use crate::sandbox::{
  bwrap,
  sandbox::{
    DeviceAccess, LaunchConfig, LaunchParams, MountConfig, NetworkMode, RuntimeEnv, SandboxConfig,
  },
  user_mapping::UserMapping,
  wine::{SyncMode, UpscaleMode},
};
use std::{collections::HashMap, path::PathBuf, str::FromStr};

pub fn run(
  environment: Vec<String>,
  no_namespace_isolation: bool,
  user_mapping: UserMapping,
  network_mode: NetworkMode,
  device_access: DeviceAccess,
  verbose: bool,
  upscale_mode: UpscaleMode,
  sync_mode: SyncMode,
  runner_path: PathBuf,
  prefix_path: PathBuf,
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
    let mount_config = MountConfig::from_str(&app_dir).map_err(|e| anyhow::anyhow!("{}", e))?;
    Some(LaunchParams::configured(
      !mount_config.writable,
      mount_config.path.to_string_lossy().to_string(),
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
  )?;
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
