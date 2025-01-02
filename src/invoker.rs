use crate::bubblewrap::{
  bwrap,
  sandbox::{DeviceAccess, LaunchConfig, LaunchParams, NetworkMode, RuntimeEnv, SandboxConfig},
  user_mapping::UserMapping,
  wine::{SyncMode, UpscaleMode},
};
use std::collections::HashMap;

pub fn run(
  environment: Vec<String>,
  no_namespace_isolation: bool,
  user_mapping: UserMapping,
  network_mode: NetworkMode,
  device_access: DeviceAccess,
  verbose: bool,
  upscale_mode: UpscaleMode,
  sync_mode: SyncMode,
  runner_path: String,
  prefix_path: String,
  read_write: bool,
  app_dir: Option<String>,
  app_bin: Option<String>,
  app_args: Option<Vec<String>>,
) -> anyhow::Result<()> {
  let sandbox_config = SandboxConfig {
    namespace_isolation: !no_namespace_isolation,
    user_mapping,
    network_mode,
    device_access,
    verbose,
  };
  let launch_params = if let Some(app_dir) = app_dir {
    Some(LaunchParams::configured(
      Some(!read_write),
      app_dir,
      app_bin,
      app_args,
    ))
  } else {
    Some(LaunchParams::Unconfigured)
  };
  let launch_config = LaunchConfig::new(
    runner_path,
    prefix_path,
    launch_params,
    Some(upscale_mode),
    Some(sync_mode),
  );
  let env_overrides: HashMap<String, String> = environment
    .into_iter()
    .map(|item| {
      let (key, val) = item.split_once('=').unwrap_or((&item, ""));
      (key.to_string(), val.to_string())
    })
    .collect();
  let mut runtime_env = RuntimeEnv::from_env()?;
  runtime_env.overrides = Some(env_overrides);
  bwrap::run(&sandbox_config, &launch_config, &runtime_env)
}
