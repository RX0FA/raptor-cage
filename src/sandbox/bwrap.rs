use super::display::Display;
use super::sandbox::{
  DeviceAccess, LaunchConfig, LaunchParams, NetworkMode, RuntimeEnv, SandboxConfig,
};
use super::wine::{SyncMode, UpscaleMode};
use anyhow::Context;
use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{
  fs,
  os::unix::fs::FileTypeExt,
  process::{Command, Stdio},
};
use tempfile::NamedTempFile;

fn current_timestamp_hex() -> String {
  let start = SystemTime::now();
  let since_epoch = start.duration_since(UNIX_EPOCH).unwrap();
  let seconds = since_epoch.as_secs();
  format!("{:x}", seconds)
}

fn find_nvidia_devices() -> anyhow::Result<Vec<String>> {
  let mut nvidia_devices = Vec::new();
  let entries = fs::read_dir("/dev")?;
  for entry in entries.flatten() {
    let path = entry.path();
    let metadata = path.metadata()?;
    if metadata.file_type().is_char_device() {
      let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();
      if file_name.starts_with("nvidia") {
        if let Some(path_str) = path.to_str() {
          nvidia_devices.push(path_str.to_string());
        }
      }
    }
  }
  Ok(nvidia_devices)
}

/// Gets the corresponding bwrap parameters for the selected DeviceAccess option.
pub fn get_device_args(device_access: &DeviceAccess) -> anyhow::Result<Vec<String>> {
  match device_access {
    DeviceAccess::All => {
      // NOTE: "bwrap --dev /dev ..." does not work as expected, so using "--dev-bind" instead.
      let args = vec!["--dev-bind", "/dev", "/dev"];
      Ok(args.into_iter().map(String::from).collect())
    }
    DeviceAccess::Minimal => {
      let nvidia_devices = find_nvidia_devices()?;
      // TODO: check if /dev/snd/seq is needed.
      let mut devices: Vec<String> = vec!["/dev/input", "/dev/uinput", "/dev/dri"]
        .into_iter()
        .map(String::from)
        .collect();
      devices.extend(nvidia_devices);
      let args: Vec<String> = devices
        .into_iter()
        .flat_map(|d| vec!["--dev-bind".to_string(), d.to_owned(), d.to_owned()])
        .collect();
      Ok(args)
    }
  }
}

const INNER_WINE_ROOT: &str = "/opt/wine";
const INNER_WINE_PREFIX: &str = "/var/lib/wine";
const INNER_APP_DIR: &str = "/app";

fn build_args(
  sandbox_config: &SandboxConfig,
  launch_config: &LaunchConfig,
  runtime_env: &RuntimeEnv,
  empty_file_path: &str,
) -> anyhow::Result<Vec<String>> {
  let mut args = vec![
    // Kill processes in sandbox when bwrap dies.
    "--die-with-parent",
  ];
  // With user isolation the uid and gid will change inside the container, outside the container
  // they will still be the same as the invoking user.
  let uid: String;
  let gid: String;
  if sandbox_config.namespace_isolation {
    // Need to keep IPC namespace (i.e. no --unshare-ipc) because it breaks some GUI applications
    // i.e. when quickly moving the mouse cursor over the WinRAR menu bar, the application will
    // crash with a "X Error of failed request:  BadValue (integer parameter out of range for
    // operation)" error.
    // TODO: consider bringing back the --unshare-ipc parameter, it seems limited to X11, see also
    // flatpak docs about the IPC issue.
    args.extend(["--unshare-pid", "--unshare-cgroup"]);
    (uid, gid) = sandbox_config.user_mapping.get_uid_gid_string();
    args.extend(["--unshare-user", "--uid", &uid, "--gid", &gid]);
  }
  // Use a new UTS space and a hostname based on the current timestamp.
  let timestamp = current_timestamp_hex();
  args.extend(["--unshare-uts", "--hostname", &timestamp]);
  // Share devices, if NVIDIA devices are missing, weird/misleading gstreamer errors may appear when
  // playing games, like telling you that a gst plugin is missing.
  let device_args = get_device_args(&sandbox_config.device_access)?;
  args.extend(device_args.iter().map(|a| a.as_str()));
  // System binaries and libraries.
  args.extend([
    "--ro-bind",
    "/bin",
    "/bin",
    "--ro-bind",
    "/lib64",
    "/lib64",
    "--ro-bind",
    "/sbin",
    "/sbin",
    "--ro-bind",
    "/usr",
    "/usr",
    "--symlink",
    "/usr/lib",
    "/lib",
  ]);
  // Need to bind /run because it allows D-Bus to work (needed for some gamepads), and /sys to
  // provide access to kernel and hardware information.
  // Binding /run works but it exposes more than we need, so only bind D-Bus related paths,
  // i.e. sandboxed apps shouldn't be able to run "DOCKER_HOST=unix:///run/docker.sock docker ps",
  // the aforementioned command works even if --ro-bind was used.
  // TODO: check if more paths are needed, and if they could be restricted even further.
  args.extend([
    "--ro-bind",
    "/run/dbus",
    "/run/dbus",
    "--ro-bind",
    "/run/user",
    "/run/user",
    "--ro-bind",
    "/sys",
    "/sys",
  ]);
  // There are just so many things that could be needed under /etc to the point
  // that is not reliable to selectively mount directories under /etc
  // (e.g. DOOM 2016 will fail if no /etc/vulkan is present), so mount all /etc.
  args.extend([
    "--bind",
    "/etc",
    "/etc",
    "--ro-bind",
    empty_file_path,
    "/etc/hostname",
  ]);
  // Setup networking, the bwrap default is enabled, our default will be to have it disabled.
  match sandbox_config.network_mode {
    NetworkMode::FullAccess => (), // No extra arguments required
    NetworkMode::RestrictedAccess => {
      args.extend([
        "--tmpfs",
        "/etc/ca-certificates",
        "--tmpfs",
        "/etc/ssl",
        "--tmpfs",
        "/etc/NetworkManager",
        "--ro-bind",
        empty_file_path,
        "/etc/resolv.conf",
        "--ro-bind",
        empty_file_path,
        "/etc/nsswitch.conf",
        "--ro-bind",
        empty_file_path,
        "/etc/hosts",
      ]);
    }
    NetworkMode::NoAccess => {
      args.push("--unshare-net");
    }
  }
  // While --dir itself doesn't inherently leak data from the host, it provides less protection
  // because it allows the container to manage files on a persistent basis (even if those files are
  // contained within the sandbox), in other words, it has greater attack surface in case a
  // vulnerability in Bubblewrap is found. In contrast, --tmpfs ensures a clean and isolated
  // environment with no chance of interaction with the host filesystem.
  args.extend([
    "--tmpfs",
    "/var",
    "--proc",
    "proc",
    "--tmpfs",
    &runtime_env.home_dir,
  ]);
  // Mount the directory that contains the Wine binaries and libraries (a.k.a. runner), the Wine
  // version to be mounted must be statically compiled in order to not rely on any host library
  // i.e. the runners downloaded by Bottles are statically compiled.
  args.extend([
    "--tmpfs",
    "/opt",
    "--ro-bind",
    &launch_config.wine_root,
    INNER_WINE_ROOT,
  ]);
  // Prefix needs to be read-write because some dependencies may be installed or system files change
  // while wine is running, even changing the registry requires write access.
  args.extend(["--bind", &launch_config.wine_prefix, INNER_WINE_PREFIX]);
  // Mount X11 socket to allow running GUI apps. Using the same X11 display number as the host
  // because using a different number will not work despite being the first recommendation in the
  // ArchWiki: https://wiki.archlinux.org/title/Bubblewrap#Using_X11.
  let display = Display::from_str(&runtime_env.display_address)?;
  let x11_socket = display.get_socket_path();
  args.extend([
    "--tmpfs",
    "/tmp",
    "--bind",
    &x11_socket,
    &x11_socket,
    "--ro-bind",
    &runtime_env.xauthority_file,
    &runtime_env.xauthority_file,
  ]);
  // Clear env and set minimal required variables, we need to make sure that all needed variables
  // are being passed otherwise games may crash or have no sound.
  args.extend([
    "--clearenv",
    "--setenv",
    "HOME",
    &runtime_env.home_dir,
    "--setenv",
    "XAUTHORITY",
    &runtime_env.xauthority_file,
    "--setenv",
    "DBUS_SESSION_BUS_ADDRESS",
    &runtime_env.dbus_session_bus_address,
    "--setenv",
    "XDG_RUNTIME_DIR",
    &runtime_env.xdg_runtime_dir,
    "--setenv",
    "DISPLAY",
    &runtime_env.display_address,
    "--setenv",
    "WINEPREFIX",
    INNER_WINE_PREFIX,
    "--setenv",
    "WINEDLLOVERRIDES",
    "winemenubuilder=''",
    "--setenv",
    "WINE_LARGE_ADDRESS_AWARE",
    "1",
  ]);
  // Allow gamepad hotplugging, otherwise network access or --share-net would be required.
  args.extend(["--setenv", "SDL_JOYSTICK_DISABLE_UDEV", "1"]);
  // Extend the PATH to have access to the Wine binaries without full paths.
  let new_path = format!("{}/bin:{}", INNER_WINE_ROOT, runtime_env.original_path);
  args.extend(["--setenv", "PATH", &new_path]);
  // GPU cache is saved in the game directory by default, this is undesired because most of the time
  // this directory will be read-only, so put the caches under the prefix (Bottles does the same).
  let gl_cache_path = format!("{}/cache/gl_shader", INNER_WINE_PREFIX);
  let dxvk_cache_path = format!("{}/cache/dxvk_state", INNER_WINE_PREFIX);
  let mesa_cache_path = format!("{}/cache/mesa_shader", INNER_WINE_PREFIX);
  let vkd3d_cache_path = format!("{}/cache/vkd3d_shader", INNER_WINE_PREFIX);
  args.extend([
    "--setenv",
    "__GL_SHADER_DISK_CACHE",
    "1",
    "--setenv",
    "__GL_SHADER_DISK_CACHE_PATH",
    &gl_cache_path,
    "--setenv",
    "DXVK_STATE_CACHE_PATH",
    &dxvk_cache_path,
    "--setenv",
    "MESA_SHADER_CACHE_DIR",
    &mesa_cache_path,
    "--setenv",
    "VKD3D_SHADER_CACHE_PATH",
    &vkd3d_cache_path,
  ]);
  // Configure upscale mode.
  let fsr_mode: String;
  let fsr_strength: String;
  match &launch_config.upscale_mode {
    None | Some(UpscaleMode::None) => (),
    Some(UpscaleMode::Fsr { mode, strength }) => {
      fsr_mode = mode.to_string();
      fsr_strength = strength.to_string();
      args.extend([
        "--setenv",
        "WINE_FULLSCREEN_FSR",
        "1",
        "--setenv",
        "WINE_FULLSCREEN_FSR_MODE",
        &fsr_mode,
        "--setenv",
        "WINE_FULLSCREEN_FSR_STRENGTH",
        &fsr_strength,
      ]);
    }
    Some(UpscaleMode::Dlss) => {
      args.extend([
        "--setenv",
        "DXVK_NVAPIHACK",
        "0",
        "--setenv",
        "DXVK_ENABLE_NVAPI",
        "1",
      ]);
    }
  }
  // Configure Wine sync mode, only one mode can be set at time.
  match launch_config.sync_mode {
    None | Some(SyncMode::None) => (),
    Some(SyncMode::Fsync) => {
      args.extend(["--setenv", "WINEFSYNC", "1"]); // Default for soda runner
    }
    Some(SyncMode::Esync) => {
      args.extend(["--setenv", "WINEESYNC", "1"]);
    }
  }
  // Configure verbosity.
  if !sandbox_config.verbose {
    args.extend([
      "--setenv",
      "WINEDEBUG",
      "fixme-all",
      "--setenv",
      "DXVK_LOG_LEVEL",
      "warn",
    ]);
  }
  // Set custom environment variables overrides. If there are 2 variables with the same name set by
  // --setenv, bwrap will use the rightmost one.
  if let Some(env_overrides) = &runtime_env.overrides {
    for (key, value) in env_overrides.into_iter() {
      args.extend(["--setenv", &key, &value])
    }
  }
  // Method return contains a Vec<String> because it needs to own each element, we initially declare
  // args as Vec<&str> to make it easier to add elements (so we avoid String::from() or .into() on
  // each element), however args still needs to be converted to Vec<String> at the end.
  let mut final_args: Vec<String> = args.into_iter().map(String::from).collect();
  let term = env::var("TERM").unwrap_or("xterm-256color".into());
  let shell = env::var("SHELL").unwrap_or("bash".into());
  let shell_params: Vec<String> = vec!["--setenv".into(), "TERM".into(), term, shell];
  // Depending on the launch params, add the necessary arguments to start a regular shell or execute
  // a command under wine.
  match &launch_config.launch_params {
    LaunchParams::Unconfigured => {
      // No launch params, so start with a regular shell.
      final_args.extend(["--chdir".into(), "/".into()]);
      final_args.extend(shell_params);
    }
    LaunchParams::Configured {
      read_only,
      app_dir,
      app_bin,
      app_args,
    } => {
      // Most games can work without issues when mounted as read-only. This also prevents polluting
      // the game directory. Also, setting the working directory is important for many games.
      final_args.extend([
        if *read_only {
          "--ro-bind".into()
        } else {
          "--bind".into()
        },
        app_dir.into(),
        INNER_APP_DIR.into(),
        "--chdir".into(),
        INNER_APP_DIR.into(),
      ]);
      if let Some(app_bin) = app_bin {
        let bin_buf = PathBuf::from(INNER_APP_DIR).join(app_bin);
        let bin_path = bin_buf
          .to_str()
          .with_context(|| format!("invalid path: {}", bin_buf.to_string_lossy()))?;
        final_args.extend(["wine".into(), bin_path.into()]);
        final_args.extend(app_args.to_owned());
      } else {
        // Only app_dir was set (not app_bin), so start with default shell (useful for maintenance).
        final_args.extend(shell_params);
      }
    }
  }
  Ok(final_args)
}

/// Execute a program under a restricted Bubblewrap container, the output will be inherited by the
/// current terminal and printed in real-time. See detailed parameter information at
/// https://man.archlinux.org/man/extra/bubblewrap/bwrap.1.en.
/// **NOTE:** keep in mind that even if runners (Wine custom builds downloaded through Bottles) are
/// statically compiled, it does not mean they will run without additional dependencies, they are
/// kinda independent of glibc and similar lower level stuff, however they still need the OS to
/// provide the right dependencies, otherwise not even `notepad.exe` will run, to install these
/// dependencies, just install `steam-native-runtime` on Arch/Manjaro.
pub fn run(
  sandbox_config: &SandboxConfig,
  launch_config: &LaunchConfig,
  runtime_env: &RuntimeEnv,
) -> anyhow::Result<()> {
  // Temporary file will be automatically removed when variable goes out of scope.
  let temp_file = NamedTempFile::new()?;
  let temp_file_path = temp_file
    .path()
    .to_str()
    .context("could not get temporary file path")?;
  let args = build_args(sandbox_config, launch_config, &runtime_env, temp_file_path)?;
  let mut cmd = Command::new("bwrap")
    .args(args)
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit())
    .spawn()
    .map_err(|e| anyhow::anyhow!("could not spawn bwrap: {}", e))?;
  let status = cmd.wait()?;
  if status.success() {
    return Ok(());
  }
  Err(anyhow::anyhow!("bwrap exited with non-zero exit code"))
}
