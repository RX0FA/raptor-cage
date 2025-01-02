use super::user_mapping::UserMapping;
use super::wine::{SyncMode, UpscaleMode};
use anyhow::Context;
use std::collections::HashMap;
use std::env;
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
    read_only: Option<bool>,
    app_dir: String,
    app_bin: Option<String>,
    app_args: Option<Vec<String>>,
  ) -> Self {
    LaunchParams::Configured {
      read_only: read_only.unwrap_or(true),
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
  pub wine_root: String,
  pub wine_prefix: String,
  /// Application to execute inside the sandbox, if not set, a shell will be started instead.
  pub launch_params: LaunchParams,
  /// Optional upscale mode (needs to be supported by the runner).
  pub upscale_mode: Option<UpscaleMode>,
  /// Optional Wine sync mode.
  pub sync_mode: Option<SyncMode>,
}

impl LaunchConfig {
  pub fn new(
    wine_root: String,
    wine_prefix: String,
    launch_params: Option<LaunchParams>,
    upscale_mode: Option<UpscaleMode>,
    sync_mode: Option<SyncMode>,
  ) -> Self {
    LaunchConfig {
      wine_root,
      wine_prefix,
      launch_params: launch_params.unwrap_or(LaunchParams::Unconfigured),
      upscale_mode,
      sync_mode,
    }
  }
}
