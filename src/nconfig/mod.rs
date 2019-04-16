use crate::config;
use crate::config::{expand_path, Config, GitlabSettings, Project, Settings, Tag};
use crate::errors::AppError;
use slog::info;

use dirs::config_dir;
use slog::{debug, Logger};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::PathBuf;
use toml;
use walkdir::WalkDir;

static CONF_MODE_HEADER: &str = "# -*- mode: Conf; -*-\n";

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NSettings {
  pub workspace: String,
  pub shell: Option<Vec<String>>,
  pub default_after_workon: Option<String>,
  pub default_after_clone: Option<String>,
  pub github_token: Option<String>,
  pub gitlab: Option<GitlabSettings>,
}

struct FwPaths {
  settings: PathBuf,
  base: PathBuf,
  projects: PathBuf,
  tags: PathBuf,
}

impl FwPaths {
  fn ensure_base_exists(&self) -> Result<(), AppError> {
    std::fs::create_dir_all(&self.base)?;
    Ok(())
  }
}

pub fn read_config(logger: &Logger) -> Result<Config, AppError> {
  let paths = fw_path()?;

  let settings_raw = read_to_string(&paths.settings)
    .map_err(|e| AppError::RuntimeError(format!("Could not read settings file ({}): {}", paths.settings.to_string_lossy(), e)))?;

  let settings: NSettings = toml::from_str(&settings_raw)?;

  debug!(logger, "read new settings ok");

  let mut projects: BTreeMap<String, Project> = BTreeMap::new();
  for maybe_project_file in WalkDir::new(&paths.projects).follow_links(true) {
    let project_file = maybe_project_file?;
    if project_file.metadata()?.is_file() {
      let raw_project = read_to_string(project_file.path())?;
      let project: Project = toml::from_str(&raw_project)?;
      projects.insert(project.name.clone(), project);
    }
  }
  debug!(logger, "read new projects ok");

  let mut tags: BTreeMap<String, Tag> = BTreeMap::new();
  for maybe_tag_file in WalkDir::new(&paths.tags).follow_links(true) {
    let tag_file = maybe_tag_file?;
    if tag_file.metadata()?.is_file() {
      let raw_tag = read_to_string(tag_file.path())?;
      let tag: Tag = toml::from_str(&raw_tag)?;
      let tag_name: Option<String> = tag_file.file_name().to_str().map(ToOwned::to_owned);
      tags.insert(tag_name.ok_or(AppError::InternalError(""))?, tag);
    }
  }

  let default_tags: BTreeSet<String> = tags
    .iter()
    .filter(|(_, value)| value.default.unwrap_or_default())
    .map(|(key, _)| key.to_string())
    .collect();

  Ok(Config {
    projects,
    settings: Settings {
      tags: Some(tags),
      workspace: settings.workspace,
      shell: settings.shell,
      default_after_workon: settings.default_after_workon,
      default_after_clone: settings.default_after_clone,
      default_tags: Some(default_tags),
      github_token: settings.github_token,
      gitlab: settings.gitlab,
    },
  })
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

pub fn write_settings(settings: &NSettings, logger: &Logger) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut buffer = File::create(&paths.settings)?;
  let serialized = toml::to_string_pretty(settings)?;
  write!(buffer, "{}", serialized)?;

  debug!(logger, "Wrote settings file to {:?}", paths.settings);

  Ok(())
}

pub fn write_tag(tag_name: &str, tag: &Tag, tag_config_path: &str) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut tag_path = paths.tags.to_owned();
  tag_path.push(PathBuf::from(tag_config_path));
  std::fs::create_dir_all(&tag_path)?;

  let mut tag_file_path = tag_path.clone();
  tag_file_path.push(&tag_name);

  let mut buffer = File::create(&tag_file_path)?;
  let serialized = toml::to_string_pretty(&tag)?;
  write!(buffer, "{}", CONF_MODE_HEADER)?;
  write!(buffer, "{}", serialized)?;
  Ok(())
}

pub fn write_project(project: &Project, project_config_path: &str) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut project_path = paths.projects.to_owned();
  project_path.push(PathBuf::from(project_config_path));
  std::fs::create_dir_all(&project_path)?;

  let mut project_file_path = project_path.clone();
  project_file_path.push(&project.name);

  let mut buffer = File::create(&project_file_path)?;
  let serialized = toml::to_string_pretty(&project)?;
  write!(buffer, "{}", CONF_MODE_HEADER)?;
  write!(buffer, "{}", serialized)?;
  Ok(())
}

fn migrate_write_tags(config: &Config, logger: &Logger) -> Result<(), AppError> {
  for (name, tag) in config.settings.tags.clone().unwrap_or_default() {
    write_tag(&name, &tag, "default")?;
  }

  debug!(logger, "Wrote tags");
  Ok(())
}

fn migrate_write_projects(config: &Config, logger: &Logger) -> Result<(), AppError> {
  for project in config.projects.values() {
    write_project(&project, "default")?;
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

  write_settings(&new_settings, &logger)?;
  migrate_write_projects(&config, &logger)?;
  migrate_write_tags(&config, &logger)?;

  Ok(())
}

pub fn migrate(logger: &Logger) -> Result<(), AppError> {
  let config = config::get_config(&logger);

  // write 2.0 for compat
  if let Ok(ref c) = config {
    write_new(&c, &logger).expect("Failed to write v2.0 config");
    info!(logger, "Wrote new config");
    // TODO remove me, just for testing
    let written_config = read_config(&logger).expect("oh noes");
    info!(logger, "Written config be like: {:?}", written_config);
    Ok(())
  } else {
    Err(AppError::RuntimeError("Could not load old config".to_string()))
  }
}
