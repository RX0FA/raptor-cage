use super::bottles;
use super::user_mapping::UserMapping;
use super::wine::{SyncMode, UpscaleMode};
use anyhow::Context;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::{env, fmt, fs};

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
      _ => Err(format!("invalid network mode: {}", s)),
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
      _ => Err(format!("invalid device access mode: {}", s)),
    }
  }
}

#[derive(Debug, PartialEq)]
pub enum MountConfigError {
  EmptyPath,
  DisallowedPath(PathBuf),
  PathError(PathBuf, String),
}

impl fmt::Display for MountConfigError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      MountConfigError::EmptyPath => write!(f, "path must not be empty"),
      MountConfigError::DisallowedPath(path) => {
        write!(f, "path is not allowed: {}", path.to_string_lossy())
      }
      MountConfigError::PathError(path, error) => write!(
        f,
        "path \"{}\" can not be resolved: {}",
        path.to_string_lossy(),
        error
      ),
    }
  }
}

#[derive(Debug, PartialEq)]
pub struct MountConfig {
  pub path: PathBuf,
  pub writable: bool,
}

impl FromStr for MountConfig {
  type Err = MountConfigError;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let mut parts: Vec<&str> = s.rsplitn(2, ":").collect();
    parts.reverse();
    if parts[0].is_empty() {
      return Err(MountConfigError::EmptyPath);
    }
    let path = PathBuf::from(parts[0]);
    // Resolve symlinks and relative paths, this will help us identify paths that we want to be
    // forbidden to mount (such as "/"), bwrap will take care of making sure that the path is valid.
    let canonical_path = fs::canonicalize(&path)
      .map_err(|e| MountConfigError::PathError(path.clone(), e.to_string()))?;
    if canonical_path.to_string_lossy().to_string() == "/" {
      return Err(MountConfigError::DisallowedPath(path));
    }
    let writable = parts.len() > 1 && parts[1].split(',').any(|flag| flag == "rw");
    Ok(Self { path, writable })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;
  use std::str::FromStr;

  #[test]
  fn test_mount_config_parsing() {
    let test_cases = vec![
      ("", Err(MountConfigError::EmptyPath)),
      (":", Err(MountConfigError::EmptyPath)),
      (
        "/usr/bin",
        Ok(MountConfig {
          path: PathBuf::from("/usr/bin"),
          writable: false,
        }),
      ),
      (
        "/usr/bin:",
        Ok(MountConfig {
          path: PathBuf::from("/usr/bin"),
          writable: false,
        }),
      ),
      (
        "/usr/bin:rw",
        Ok(MountConfig {
          path: PathBuf::from("/usr/bin"),
          writable: true,
        }),
      ),
      (
        "/usr/bin:noexec,rw",
        Ok(MountConfig {
          path: PathBuf::from("/usr/bin"),
          writable: true,
        }),
      ),
      (
        "/does/not/exist",
        Err(MountConfigError::PathError(
          PathBuf::from("/does/not/exist"),
          "No such file or directory (os error 2)".to_string(),
        )),
      ),
      (
        "/",
        Err(MountConfigError::DisallowedPath(PathBuf::from("/"))),
      ),
      (
        "/./",
        Err(MountConfigError::DisallowedPath(PathBuf::from("/"))),
      ),
    ];
    for (input, expected) in test_cases {
      let result = MountConfig::from_str(input);
      assert_eq!(result, expected);
    }
  }
}

fn get_env_var(name: &str) -> anyhow::Result<String> {
  env::var(name).with_context(|| format!("failed to read environment variable: {}", name))
}

// TODO: as of 2024-09 Wine Wayland is not finished, add the necessary args when it gets shipped
// with official releases. Wayland seems to also use a socket e.g. "/run/user/<uid>/wayland-0".
pub struct RuntimeEnv {
  pub home_dir: String,
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
    let dbus_session_bus_address = get_env_var("DBUS_SESSION_BUS_ADDRESS")?;
    let xdg_runtime_dir = get_env_var("XDG_RUNTIME_DIR")?;
    let original_path = get_env_var("PATH")?;
    let display_address = get_env_var("DISPLAY")?;
    let xauthority_file = get_env_var("XAUTHORITY")?;
    Ok(Self {
      home_dir,
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

pub enum LaunchParams {
  Unconfigured,
  Configured {
    read_only: bool,
    app_dir: String,
    app_bin: Option<String>,
    app_args: Vec<String>,
  },
}

impl LaunchParams {
  pub fn configured(
    read_only: bool,
    app_dir: String,
    app_bin: Option<String>,
    app_args: Option<Vec<String>>,
  ) -> Self {
    LaunchParams::Configured {
      read_only,
      app_dir,
      app_bin,
      app_args: app_args.unwrap_or(vec![]),
    }
  }
}

// TODO: detect GPUs and configure environment to use the dedicated GPU, currently bottles uses
// lspci and grep, however it does not seem to work in many scenarios. See
// https://github.com/bottlesdevs/Bottles/blob/540f6fc0d4c2853e2a62cab98548ce3210c7352a/bottles/backend/utils/gpu.py.
pub struct LaunchConfig {
  pub runner_path: PathBuf,
  pub prefix_path: PathBuf,
  /// Application to execute inside the sandbox, if not set, a shell will be started instead.
  pub launch_params: LaunchParams,
  /// Optional upscale mode (needs to be supported by the runner).
  pub upscale_mode: Option<UpscaleMode>,
  /// Optional Wine sync mode.
  pub sync_mode: Option<SyncMode>,
}

impl LaunchConfig {
  pub fn new(
    runner_path: PathBuf,
    prefix_path: PathBuf,
    launch_params: Option<LaunchParams>,
    upscale_mode: Option<UpscaleMode>,
    sync_mode: Option<SyncMode>,
  ) -> anyhow::Result<Self> {
    let data_root = bottles::get_data_root()?;
    let runner_path = if runner_path.is_absolute() {
      runner_path
    } else {
      data_root.join("runners").join(runner_path)
    };
    let prefix_path = if prefix_path.is_absolute() {
      prefix_path
    } else {
      data_root.join("bottles").join(prefix_path)
    };
    Ok(LaunchConfig {
      runner_path,
      prefix_path,
      launch_params: launch_params.unwrap_or(LaunchParams::Unconfigured),
      upscale_mode,
      sync_mode,
    })
  }
}
