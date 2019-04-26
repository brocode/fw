use crate::config;
use crate::config::{expand_path, repo_name_from_url, Config, GitlabSettings, Project, Remote, Settings, Tag};
use crate::errors::AppError;

use dirs::config_dir;
use slog::{debug, info, warn, Logger};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::path::PathBuf;
use toml;
use walkdir::WalkDir;

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
    std::fs::create_dir_all(&self.base)?;
    Ok(())
  }
}

pub fn read_config(logger: &Logger) -> Result<Config, AppError> {
  let paths = fw_path()?;

  let settings_raw = read_to_string(&paths.settings)
    .map_err(|e| AppError::RuntimeError(format!("Could not read settings file ({}): {}", paths.settings.to_string_lossy(), e)))?;

  let settings: PersistedSettings = toml::from_str(&settings_raw)?;

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
        let tag: Tag = toml::from_str(&raw_tag)?;
        let tag_name: Option<String> = tag_file.file_name().to_str().map(ToOwned::to_owned);
        tags.insert(tag_name.ok_or(AppError::InternalError(""))?, tag);
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

pub fn write_settings(settings: &PersistedSettings, logger: &Logger) -> Result<(), AppError> {
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

pub fn write_project(project: &Project) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut project_path = paths.projects.to_owned();
  project_path.push(PathBuf::from(&project.project_config_path));
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

pub fn add_entry(
  maybe_config: Result<Config, AppError>,
  maybe_name: Option<&str>,
  url: &str,
  after_workon: Option<String>,
  after_clone: Option<String>,
  override_path: Option<String>,
  logger: &Logger,
) -> Result<(), AppError> {
  let name = maybe_name
    .ok_or_else(|| AppError::UserError(format!("No project name specified for {}", url)))
    .or_else(|_| repo_name_from_url(url))?;
  let config: Config = maybe_config?;
  info!(logger, "Prepare new project entry"; "name" => name, "url" => url);
  if config.projects.contains_key(name) {
    Err(AppError::UserError(format!(
      "Project key {} already exists, not gonna overwrite it for you",
      name
    )))
  } else {
    let default_after_clone = config.settings.default_after_clone.clone();
    let default_after_workon = config.settings.default_after_clone.clone();

    write_project(&Project {
      git: url.to_owned(),
      name: name.to_owned(),
      after_clone: after_clone.or(default_after_clone),
      after_workon: after_workon.or(default_after_workon),
      override_path,
      tags: config.settings.default_tags.clone(),
      bare: None,
      additional_remotes: None,
      project_config_path: "default".to_string(),
    })?;
    Ok(())
  }
}

pub fn add_remote(maybe_config: Result<Config, AppError>, name: &str, remote_name: String, git: String) -> Result<(), AppError> {
  let config: Config = maybe_config?;
  if !config.projects.contains_key(name) {
    return Err(AppError::UserError(format!("Project key {} does not exists. Can not update.", name)));
  }
  let mut project_config: Project = config.projects.get(name).expect("Already checked in the if above").clone();
  let mut additional_remotes = project_config.additional_remotes.unwrap_or_default();
  if additional_remotes.iter().any(|r| r.name == remote_name) {
    return Err(AppError::UserError(format!(
      "Remote {} for project {} does already exist. Can not add.",
      remote_name, name
    )));
  }
  additional_remotes.push(Remote { name: remote_name, git });
  project_config.additional_remotes = Some(additional_remotes);

  write_project(&project_config)?;
  Ok(())
}

pub fn remove_remote(maybe_config: Result<Config, AppError>, name: &str, remote_name: String, logger: &Logger) -> Result<(), AppError> {
  let config: Config = maybe_config?;
  if !config.projects.contains_key(name) {
    return Err(AppError::UserError(format!("Project key {} does not exists. Can not update.", name)));
  }
  let mut project_config: Project = config.projects.get(name).expect("Already checked in the if above").clone();
  let additional_remotes = project_config.additional_remotes.unwrap_or_default();
  let additional_remotes = additional_remotes.into_iter().filter(|r| r.name != remote_name).collect();
  project_config.additional_remotes = Some(additional_remotes);

  debug!(logger, "Updated config"; "config" => format!("{:?}", config));
  write_project(&project_config)?;
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
