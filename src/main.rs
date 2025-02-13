mod cli;
mod invoker;
mod list;
mod sandbox;

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
      app_dir,
      app_bin,
      app_args,
    ),
    Commands::List { category } => list::list(category),
  }
}
