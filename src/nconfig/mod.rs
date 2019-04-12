use crate::config::{expand_path, Config, GitlabSettings};
use crate::errors::AppError;

use dirs::config_dir;
use slog::{debug, Logger};
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use toml;

pub struct FwPaths {
  settings: PathBuf,
  base: PathBuf,
  projects: PathBuf,
  tags: PathBuf,
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

  let mut settings = base.clone();
  settings.push("settings.toml");

  let mut projects = base.clone();
  projects.push("projects");

  let mut tags = base.clone();
  tags.push("tags");

  Ok(FwPaths {
    settings,
    base,
    projects,
    tags,
  })
}

fn write_settings(settings: &NSettings, paths: &FwPaths, logger: &Logger) -> Result<(), AppError> {
  let mut buffer = File::create(&paths.settings)?;
  let serialized = toml::to_string_pretty(settings)?;
  write!(buffer, "{}", serialized)?;

  debug!(logger, "Wrote settings file to {:?}", paths.settings);

  Ok(())
}

fn write_tags(config: &Config, fw_paths: &FwPaths, logger: &Logger) -> Result<(), AppError> {
  let mut default_tags_path = fw_paths.tags.clone();
  default_tags_path.push("default");
  std::fs::create_dir_all(&default_tags_path)?;

  for (name, tag) in config.settings.tags.clone().unwrap_or_default() {
    let mut tag_path = default_tags_path.clone();
    tag_path.push(format!("{}.toml", name));
    let mut buffer = File::create(&tag_path)?;
    let serialized = toml::to_string_pretty(&tag)?;
    write!(buffer, "{}", serialized)?;
  }

  debug!(logger, "Wrote tags");
  Ok(())
}

fn write_projects(config: &Config, fw_paths: &FwPaths, logger: &Logger) -> Result<(), AppError> {
  let mut default_projects_path = fw_paths.projects.clone();
  default_projects_path.push("default");
  std::fs::create_dir_all(&default_projects_path)?;

  for project in config.projects.values() {
    let mut project_path = default_projects_path.clone();
    project_path.push(format!("{}.toml", project.name));
    let mut buffer = File::create(&project_path)?;
    let serialized = toml::to_string_pretty(&project)?;
    write!(buffer, "{}", serialized)?;
  }

  debug!(logger, "Wrote projects");
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
  write_projects(&config, &paths, &logger)?;
  write_tags(&config, &paths, &logger)?;

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
