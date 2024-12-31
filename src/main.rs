mod bottles;
mod bubblewrap;
mod cli;
mod info;
mod installer;
mod invoker;
mod manifest;
mod sevenz;

use clap::Parser;
use cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
  let args = Cli::parse();
  match args.command {
    Commands::Run {
      environment,
      no_namespace_isolation,
      user_mapping,
      network_mode,
      device_access,
      verbose,
      upscale_mode,
      sync_mode,
      runner_path,
      prefix_path,
      read_write,
      app_dir,
      app_bin,
      app_args,
    } => invoker::run(
      environment,
      no_namespace_isolation,
      user_mapping,
      network_mode,
      device_access,
      verbose,
      upscale_mode,
      sync_mode,
      runner_path,
      prefix_path,
      read_write,
      app_dir,
      app_bin,
      app_args,
    ),
  }
}
