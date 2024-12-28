use crate::bubblewrap::{
  bwrap::{DeviceAccess, NetworkMode},
  user_mapping::UserMapping,
  wine::{SyncMode, UpscaleMode},
};
use clap::{ArgAction, Parser};
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub enum Commands {
  /// Run application sandboxed.
  #[command(arg_required_else_help = true)]
  Run {
    /// Environment variable overrides.
    #[arg(short = 'e', long = "setenv", value_name="KEY=VALUE", action = ArgAction::Append)]
    environment: Vec<String>,
    /// Disable namespace isolation.
    #[arg(long, default_value = "false")]
    no_namespace_isolation: bool,
    /// Use specific user and group id.
    #[arg(long, value_name = "UID:GID", default_value = "random", value_parser)]
    user_mapping: UserMapping,
    /// Configure network access.
    #[arg(long, value_name = "MODE", default_value = "no_access", value_parser)]
    network_mode: NetworkMode,
    /// Sandbox device access.
    #[arg(long, value_name = "ACCESS", default_value = "minimal", value_parser)]
    device_access: DeviceAccess,
    /// Print additional troubleshooting information.
    #[arg(short, long, default_value = "false")]
    verbose: bool,
    /// One of none, dlss, fsr:mode:stre.
    #[arg(long, value_name = "MODE", default_value = "none", value_parser)]
    upscale_mode: UpscaleMode,
    /// Configure Wine sync mode.
    #[arg(long, value_name = "MODE", default_value = "none", value_parser)]
    sync_mode: SyncMode,
    /// Path of the Wine runner.
    #[arg(short, long = "runner", value_name = "PATH")]
    runner_path: String,
    /// Path of the Wine prefix.
    #[arg(short, long = "prefix", value_name = "PATH")]
    prefix_path: String,
    /// Make app directory read-write.
    #[arg(short = 'w', long = "writable", default_value = "false")]
    read_write: bool,
    /// Path that contains the application files.
    #[arg(short, long = "appdir", value_name = "PATH")]
    app_dir: Option<String>,
    /// Path of the executable file relative to appdir.
    #[arg(short = 'b', long = "appbin", value_name = "BIN")]
    app_bin: Option<String>,
    /// Optional game arguments, need to be placed after a double dash.
    app_args: Option<Vec<String>>,
  },
  // TODO: List available runner versions.
  // List {},
  // TODO: Download the specified runner version.
  // #[command(arg_required_else_help = true)]
  // Download {},
  // TODO: Download and install a dependency into the specified prefix.
  // #[command(arg_required_else_help = true)]
  // Get {},
}

#[derive(Debug, Parser)]
#[command(about = "Run and manage games inside of a sandbox", long_about = None)]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,
}
