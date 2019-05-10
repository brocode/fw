use crate::config;
use crate::config::{expand_path, Config, GitlabSettings, Project, Settings, Tag};
use crate::errors::AppError;

use dirs::config_dir;
use slog::{debug, info, warn, Logger};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, read_to_string, File};
use std::io::Write;
use std::path::PathBuf;
use toml;
use walkdir::WalkDir;
use ::config as config_crate;

static CONF_MODE_HEADER: &str = "# -*- mode: Conf; -*-\n";

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PersistedSettings {
  pub workspace: String,
  pub shell: Option<Vec<String>>,
  pub default_after_workon: Option<String>,
  pub default_after_clone: Option<String>,
  pub github_token: Option<String>,
  pub gitlab: Option<GitlabSettings>,
}

impl PersistedSettings {
  fn from_settings(settings: &Settings) -> PersistedSettings {
    PersistedSettings {
      workspace: settings.workspace.clone(),
      shell: settings.shell.clone(),
      default_after_workon: settings.default_after_workon.clone(),
      default_after_clone: settings.default_after_clone.clone(),
      github_token: settings.github_token.clone(),
      gitlab: settings.gitlab.clone(),
    }
  }
}

struct FwPaths {
  settings: PathBuf,
  base: PathBuf,
  projects: PathBuf,
  tags: PathBuf,
}

impl FwPaths {
  fn ensure_base_exists(&self) -> Result<(), AppError> {
    std::fs::create_dir_all(&self.base).map_err(|e| AppError::RuntimeError(format!("Failed to create fw config base directory. {}", e)))?;
    Ok(())
  }
}

pub fn read_config(logger: &Logger) -> Result<Config, AppError> {
  let paths = fw_path()?;

  let mut config_settings = config_crate::Config::default();
  config_settings.merge(config_crate::File::with_name(&paths.settings.to_str().unwrap()).required(true))?;

  if let Ok(p) = env::var("FW_LOCAL_SETTINGS")
    .map(PathBuf::from)
    .map(expand_path) {
      config_settings.merge(config_crate::File::with_name(&p.to_str().unwrap()).required(false))?;
    }

  let settings: PersistedSettings = config_settings.try_into()?;

  debug!(logger, "read new settings ok");

  let mut projects: BTreeMap<String, Project> = BTreeMap::new();
  if paths.projects.exists() {
    for maybe_project_file in WalkDir::new(&paths.projects).follow_links(true) {
      let project_file = maybe_project_file?;
      if project_file.metadata()?.is_file() {
        let raw_project = read_to_string(project_file.path())?;
        let mut project: Project = toml::from_str(&raw_project)?;
        project.project_config_path = PathBuf::from(project_file.path().parent().ok_or(AppError::InternalError("Expected file to have a parent"))?)
          .strip_prefix(paths.projects.as_path())
          .map_err(|e| AppError::RuntimeError(format!("Failed to strip prefix: {}", e)))?
          .to_string_lossy()
          .to_string();
        if projects.contains_key(&project.name) {
          warn!(
            logger,
            "Inconsistency found: project {} defined more than once. Will use the project that is found last. Results might be inconsistent.", project.name
          );
        }
        projects.insert(project.name.clone(), project);
      }
    }
    debug!(logger, "read projects ok");
  }

  let mut tags: BTreeMap<String, Tag> = BTreeMap::new();
  if paths.tags.exists() {
    for maybe_tag_file in WalkDir::new(&paths.tags).follow_links(true) {
      let tag_file = maybe_tag_file?;
      if tag_file.metadata()?.is_file() {
        let raw_tag = read_to_string(tag_file.path())?;
        let mut tag: Tag = toml::from_str(&raw_tag)?;
        let tag_name: String = tag_file.file_name().to_str().map(ToOwned::to_owned).ok_or(AppError::InternalError(""))?;
        tag.tag_config_path = PathBuf::from(tag_file.path().parent().ok_or(AppError::InternalError("Expected file to have a parent"))?)
          .strip_prefix(paths.tags.as_path())
          .map_err(|e| AppError::RuntimeError(format!("Failed to strip prefix: {}", e)))?
          .to_string_lossy()
          .to_string();
        if tags.contains_key(&tag_name) {
          warn!(
            logger,
            "Inconsistency found: tag {} defined more than once. Will use the project that is found last. Results might be inconsistent.", tag_name
          );
        }
        tags.insert(tag_name, tag);
      }
    }
    debug!(logger, "read tags ok");
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
  let base = env::var("FW_CONFIG_DIR")
    .map(PathBuf::from)
    .ok()
    .map(expand_path)
    .or_else(|| {
      config_dir().map(|mut c| {
        c.push("fw");
        c
      })
    })
    .ok_or_else(|| AppError::InternalError("Cannot resolve fw config dir"))?;

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

pub fn write_settings(settings: &PersistedSettings, logger: &Logger) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut buffer = File::create(&paths.settings)?;
  let serialized = toml::to_string_pretty(settings)?;
  write!(buffer, "{}", serialized)?;

  debug!(logger, "Wrote settings file to {:?}", paths.settings);

  Ok(())
}

pub fn write_tag(tag_name: &str, tag: &Tag) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut tag_path = paths.tags.to_owned();
  tag_path.push(PathBuf::from(&tag.tag_config_path));
  std::fs::create_dir_all(&tag_path)
    .map_err(|e| AppError::RuntimeError(format!("Failed to create tag config path '{}'. {}", tag_path.to_string_lossy(), e)))?;

  let mut tag_file_path = tag_path.clone();
  tag_file_path.push(&tag_name);

  let mut buffer = File::create(&tag_file_path)
    .map_err(|e| AppError::RuntimeError(format!("Failed to create project config file '{}'. {}", tag_file_path.to_string_lossy(), e)))?;
  let serialized = toml::to_string_pretty(&tag)?;
  write!(buffer, "{}", CONF_MODE_HEADER)?;
  write!(buffer, "{}", serialized)?;
  Ok(())
}

pub fn delete_tag_config(tag_name: &str, tag: &Tag) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut tag_file_path = paths.tags.to_owned();
  tag_file_path.push(PathBuf::from(&tag.tag_config_path));
  tag_file_path.push(tag_name);

  fs::remove_file(&tag_file_path).map_err(|e| AppError::RuntimeError(format!("Failed to delete tag config from '{:?}': {}", tag_file_path, e)))?;
  Ok(())
}

pub fn delete_project_config(project: &Project) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut project_file_path = paths.projects.to_owned();
  project_file_path.push(PathBuf::from(&project.project_config_path));
  project_file_path.push(&project.name);

  fs::remove_file(project_file_path).map_err(|e| AppError::RuntimeError(format!("Failed to delete project config: {}", e)))?;
  Ok(())
}

fn write_example<T>(buffer: &mut File, example: T) -> Result<(), AppError>
where
  T: serde::Serialize,
{
  let example_toml = toml::to_string_pretty(&example)?;
  writeln!(buffer, "\n# Example:")?;
  for line in example_toml.split("\n") {
    if line.trim() != "" {
      writeln!(buffer, "# {}", line)?;
    }
  }
  Ok(())
}

pub fn write_project(project: &Project) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut project_path = paths.projects.to_owned();
  project_path.push(PathBuf::from(&project.project_config_path));
  std::fs::create_dir_all(&project_path)
    .map_err(|e| AppError::RuntimeError(format!("Failed to create project config path '{}'. {}", project_path.to_string_lossy(), e)))?;

  let mut project_file_path = project_path.clone();
  project_file_path.push(&project.name);

  let mut buffer: File = File::create(&project_file_path)
    .map_err(|e| AppError::RuntimeError(format!("Failed to create project config file '{}'. {}", project_file_path.to_string_lossy(), e)))?;
  let serialized = toml::to_string_pretty(&project)?;

  write!(buffer, "{}", CONF_MODE_HEADER)?;
  write!(buffer, "{}", serialized)?;
  write_example(&mut buffer, Project::example())?;
  Ok(())
}

fn migrate_write_tags(config: &Config, logger: &Logger) -> Result<(), AppError> {
  for (name, tag) in config.settings.tags.clone().unwrap_or_default() {
    let mut tag = tag.clone();
    tag.tag_config_path = "default".to_string();
    write_tag(&name, &tag)?;
  }

  debug!(logger, "Wrote tags");
  Ok(())
}

fn migrate_write_projects(config: &Config, logger: &Logger) -> Result<(), AppError> {
  for project in config.projects.values() {
    let mut p = project.clone();
    p.project_config_path = "default".to_string();
    write_project(&p)?;
  }

  debug!(logger, "Wrote projects");
  Ok(())
}

pub fn write_new(config: &Config, logger: &Logger) -> Result<(), AppError> {
  let new_settings = PersistedSettings::from_settings(&config.settings);

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
    warn!(logger, "Written v2.0 config with {} projects", written_config.projects.values().len());
    Ok(())
  } else {
    Err(AppError::RuntimeError("Could not load old config".to_string()))
  }
}
