use crate::config::{expand_path, Config, GitlabSettings};
use crate::errors::AppError;

use dirs::config_dir;
use slog::Logger;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use toml;

pub struct FwPaths {
  settings: PathBuf,
  base: PathBuf,
}

fn fw_path() -> Result<FwPaths, AppError> {
  let base = env::var("FW_CONFIG_PATH")
    .map(PathBuf::from)
    .ok()
    .map(expand_path)
    .or_else(|| {
      config_dir().map(|mut c| {
        c.push("fw");
        c
      })
    })
    .ok_or_else(|| AppError::InternalError("Cannot resolve fw config path"))?;

  let mut settings_path = base.clone();
  settings_path.push("settings.toml");

  Ok(FwPaths {
    settings: settings_path,
    base: base,
  })
}

fn write_settings(settings: &NSettings, paths: &FwPaths, logger: &Logger) -> Result<(), AppError> {
  let mut buffer = File::create(&paths.settings)?;
  let serialized = toml::to_string_pretty(settings)?;
  write!(buffer, "{}", serialized)?;

  Ok(())
}

pub fn write_new(config: &Config, logger: &Logger) -> Result<(), AppError> {
  let new_settings = NSettings {
    workspace: config.settings.workspace.clone(),
    shell: config.settings.shell.clone(),
    default_after_workon: config.settings.default_after_workon.clone(),
    default_after_clone: config.settings.default_after_clone.clone(),
    github_token: config.settings.github_token.clone(),
    gitlab: config.settings.gitlab.clone(),
  };
  let paths = fw_path()?;

  std::fs::create_dir_all(&paths.base)?;

  write_settings(&new_settings, &paths, &logger)?;

  Ok(())
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NSettings {
  pub workspace: String,
  pub shell: Option<Vec<String>>,
  pub default_after_workon: Option<String>,
  pub default_after_clone: Option<String>,
  pub github_token: Option<String>,
  pub gitlab: Option<GitlabSettings>,
}
