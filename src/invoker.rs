use crate::{
  inhibitor,
  sandbox::{
    bwrap,
    mount::{MountConfig, MountMapping},
    sandbox::{
      DeviceAccess, DisplayProtocol, LaunchConfig, LaunchParams, NetworkMode, RuntimeEnv,
      SandboxConfig,
    },
    user_mapping::UserMapping,
    wine::{SyncMode, UpscaleMode, get_wine_user},
  },
};
use std::{collections::HashMap, path::PathBuf, str::FromStr};

// TODO: deny "/" and other important dirs.
fn parse_mappings(volumes: &[String]) -> anyhow::Result<Vec<MountMapping>> {
  let mut mappings: Vec<MountMapping> = Vec::with_capacity(volumes.len());
  for volume in volumes {
    let mapping =
      MountMapping::from_str(volume).map_err(|e| anyhow::anyhow!("Volume error: {}", e))?;
    mappings.push(mapping);
  }
  Ok(mappings)
}

pub async fn run(
  environment: &[String],
  volumes: &[String],
  no_namespace_isolation: bool,
  user_mapping: UserMapping,
  display_protocol: DisplayProtocol,
  network_mode: NetworkMode,
  device_access: DeviceAccess,
  verbose: bool,
  upscale_mode: UpscaleMode,
  sync_mode: SyncMode,
  process_names: Option<Vec<String>>,
  runner_path: Option<PathBuf>,
  prefix_path: Option<PathBuf>,
  app_dir: Option<String>,
  app_bin: Option<String>,
  app_args: Option<Vec<String>>,
) -> anyhow::Result<()> {
  if runner_path.as_ref().xor(prefix_path.as_ref()).is_some() {
    anyhow::bail!("Either both runner and prefix paths are required, or neither");
  }
  let sandbox_config = SandboxConfig {
    namespace_isolation: !no_namespace_isolation,
    user_mapping,
    display_protocol,
    network_mode,
    device_access,
    verbose,
  };
  let (app_dir, read_only) = if let Some(dir) = app_dir {
    let mount_config = MountConfig::from_str(&dir).map_err(|e| anyhow::anyhow!("{}", e))?;
    (Some(mount_config.path.to_string_lossy().to_string()), !mount_config.writable)
  } else {
    (None, true)
  };
  let launch_params =
    LaunchParams::from_options(read_only, app_dir, app_bin, app_args, process_names);
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
  if let Some(wine_prefix) = &launch_config.prefix_path {
    // Need to set current user because some games rely on this variable to build the save/config
    // path. If not set, the behavior depends on the game, common symptoms include saving data to
    // the root path (e.g., "/My Games"), silently failing to save settings or broken UI.
    let wine_user = get_wine_user(&wine_prefix, &runtime_env.user_name)?;
    runtime_env.user_name = wine_user;
  }
  runtime_env.overrides = Some(env_overrides);
  let mount_mappings = parse_mappings(volumes)?;
  // Inhibit the system so screen does not dim while running a game, inhibition will be
  // automatically released when inhibit_handle is dropped.
  let inhibit_handle = inhibitor::inhibit_idle().await;
  if let Err(inhibit_error) = &inhibit_handle {
    println!("Inhibition failed: {}", inhibit_error.to_string());
  }
  bwrap::run(&sandbox_config, &launch_config, &runtime_env, &mount_mappings)
}
