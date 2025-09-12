use super::bottles;
use super::user_mapping::UserMapping;
use super::wine::{SyncMode, UpscaleMode, is_windows_binary};
use anyhow::Context;
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::str::FromStr;

/// Represents network configuration options.
#[derive(Debug, Clone)]
pub enum NetworkMode {
  /// Allows complete network access.
  FullAccess,
  /// Allows network access but denies certain features like DNS resolving and SSL certificate
  /// access, useful for some games that require LAN.
  RestrictedAccess,
  /// Denies complete network access (recommended).
  NoAccess,
}

impl FromStr for NetworkMode {
  type Err = String;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "full_access" | "full" | "f" => Ok(NetworkMode::FullAccess),
      "restricted_access" | "restricted" | "r" => Ok(NetworkMode::RestrictedAccess),
      "no_access" | "no" | "n" => Ok(NetworkMode::NoAccess),
      _ => Err(format!("Invalid network mode: {}", s)),
    }
  }
}

#[derive(Debug, Clone)]
pub enum DeviceAccess {
  /// Allow access to all devices.
  All,
  /// Minimal set of input and GPU devices for games to work.
  Minimal,
}

impl FromStr for DeviceAccess {
  type Err = String;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s.to_lowercase().as_str() {
      "all" | "a" => Ok(DeviceAccess::All),
      "minimal" | "m" => Ok(DeviceAccess::Minimal),
      _ => Err(format!("Invalid device access mode: {}", s)),
    }
  }
}

/// Retrieves an env variable and maps the error if not found, the difference between this method
/// and using directly env::var is that this method mentions the variable name that was not found.
fn get_env_var(name: &str) -> anyhow::Result<String> {
  env::var(name).with_context(|| format!("Failed to read environment variable: {}", name))
}

// TODO: as of 2024-09 Wine Wayland is not finished, add the necessary args when it gets shipped
// with official releases. Wayland seems to also use a socket e.g. "/run/user/<uid>/wayland-0".
pub struct RuntimeEnv {
  pub home_dir: String,
  pub user_name: String,
  pub lang: String,
  pub dbus_session_bus_address: String,
  pub xdg_runtime_dir: String,
  /// Represents the unmodified value of the PATH variable.
  pub original_path: String,
  /// X11 display address, can look like `:0`, `:1` or `localhost:0.0`. This is required even if
  /// running on Wayland.
  pub display_address: String,
  /// Needed on X11 sessions, and by Gamescope.
  pub xauthority_file: String,
  /// Additional env variables set (e.g. set by the user or Bottles).
  pub overrides: Option<HashMap<String, String>>,
}

impl RuntimeEnv {
  pub fn from_env() -> anyhow::Result<Self> {
    let home_dir = get_env_var("HOME")?;
    let user_name = get_env_var("USER")?;
    let lang = env::var("LANG").unwrap_or("en_US.UTF-8".to_owned());
    let dbus_session_bus_address = get_env_var("DBUS_SESSION_BUS_ADDRESS")?;
    let xdg_runtime_dir = get_env_var("XDG_RUNTIME_DIR")?;
    let original_path = get_env_var("PATH")?;
    let display_address = get_env_var("DISPLAY")?;
    let xauthority_file = get_env_var("XAUTHORITY")?;
    Ok(Self {
      home_dir,
      user_name,
      lang,
      dbus_session_bus_address,
      xdg_runtime_dir,
      original_path,
      display_address,
      xauthority_file,
      overrides: None,
    })
  }
}

pub struct SandboxConfig {
  /// Controls whether `--unshare-{ipc,pid,cgroup,uts}` is used or not.
  pub namespace_isolation: bool,
  /// Controls the user and group id inside the sandbox.
  pub user_mapping: UserMapping,
  /// Controls network access for sandboxed programs (e.g. internet access), the bwrap default is to
  /// allow network connections, our default is to deny connections by using a separate network
  /// namespace.
  pub network_mode: NetworkMode,
  /// Controls what devices are accessible from within the sandbox.
  pub device_access: DeviceAccess,
  /// Configures various options such as WINEDEBUG and DXVK_LOG_LEVEL.
  pub verbose: bool,
}

impl Default for SandboxConfig {
  fn default() -> Self {
    SandboxConfig {
      namespace_isolation: true,
      user_mapping: UserMapping::Random,
      network_mode: NetworkMode::NoAccess,
      device_access: DeviceAccess::Minimal,
      verbose: false,
    }
  }
}

fn map_wine_app(app_bin: String, app_args: Option<Vec<String>>) -> (String, Vec<String>) {
  if !is_windows_binary(&app_bin) {
    return (app_bin, app_args.unwrap_or_default());
  }
  let new_args: Vec<String> = if let Some(mut args) = app_args {
    args.insert(0, app_bin);
    args
  } else {
    vec![app_bin]
  };
  return ("wine".to_string(), new_args);
}

fn map_wait_command(
  app_bin: String,
  app_args: Vec<String>,
  process_names: Option<Vec<String>>,
) -> (String, Vec<String>) {
  match process_names {
    Some(process_names) => {
      let current_exe = std::env::current_exe()
        .ok()
        .map(|path| path.to_string_lossy().to_string())
        .expect("Failed to get executable name");
      let mut new_args: Vec<String> = vec![
        "wait".into(),
        "-w".into(),
        process_names.join(","),
        app_bin,
        "--".into(),
      ];
      new_args.extend(app_args);
      (current_exe, new_args)
    }
    None => (app_bin, app_args),
  }
}

pub enum LaunchParams {
  /// The root directory will be mounted without any app directory.
  Unconfigured,
  /// Only the app directory is mounted; no command will be executed.
  AppDirOnly { read_only: bool, app_dir: String },
  /// App directory is mounted and specified command will be executed.
  AppDirWithCommand {
    read_only: bool,
    app_dir: String,
    app_bin: String,
    app_args: Vec<String>,
  },
}

impl LaunchParams {
  pub fn from_options(
    read_only: bool,
    app_dir: Option<String>,
    app_bin: Option<String>,
    app_args: Option<Vec<String>>,
    process_names: Option<Vec<String>>,
  ) -> Self {
    let Some(app_dir) = app_dir else {
      return LaunchParams::Unconfigured;
    };
    let Some(app_bin) = app_bin else {
      return LaunchParams::AppDirOnly { read_only, app_dir };
    };
    let (bin, args) = map_wine_app(app_bin, app_args);
    let (bin, args) = map_wait_command(bin, args, process_names);
    LaunchParams::AppDirWithCommand {
      read_only,
      app_dir,
      app_bin: bin,
      app_args: args,
    }
  }
}

// TODO: detect GPUs and configure environment to use the dedicated GPU, currently bottles uses
// lspci and grep, however it does not seem to work in many scenarios. See
// https://github.com/bottlesdevs/Bottles/blob/540f6fc0d4c2853e2a62cab98548ce3210c7352a/bottles/backend/utils/gpu.py.
pub struct LaunchConfig {
  /// Full path to the wine runner.
  pub runner_path: Option<PathBuf>,
  /// Full path to the wine prefix.
  pub prefix_path: Option<PathBuf>,
  /// Application to execute inside the sandbox, if not set, a shell will be started instead.
  pub launch_params: LaunchParams,
  /// Optional upscale mode (needs to be supported by the runner).
  pub upscale_mode: Option<UpscaleMode>,
  /// Optional Wine sync mode.
  pub sync_mode: Option<SyncMode>,
}

impl LaunchConfig {
  pub fn new(
    runner_path: Option<PathBuf>,
    prefix_path: Option<PathBuf>,
    launch_params: LaunchParams,
    upscale_mode: Option<UpscaleMode>,
    sync_mode: Option<SyncMode>,
  ) -> anyhow::Result<Self> {
    let data_root: Option<PathBuf> = if runner_path.is_some() || prefix_path.is_some() {
      Some(bottles::get_data_root()?)
    } else {
      None
    };
    let runner_path = runner_path.map(|path| match &data_root {
      Some(data_root) if !path.is_absolute() => data_root.join("runners").join(path),
      _ => path,
    });
    let prefix_path = prefix_path.map(|path| match &data_root {
      Some(data_root) if !path.is_absolute() => data_root.join("bottles").join(path),
      _ => path,
    });
    Ok(LaunchConfig {
      runner_path,
      prefix_path,
      launch_params,
      upscale_mode,
      sync_mode,
    })
  }
}
