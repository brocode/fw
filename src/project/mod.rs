use crate::config::Config;
use crate::config::{repo_name_from_url, Project, Remote};
use crate::errors::AppError;
use crate::nconfig;
use slog::Logger;
use slog::{debug, info};
use std::fs;

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

    nconfig::write_project(&Project {
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

pub fn remove_project(maybe_config: Result<Config, AppError>, project_name: &str, purge_directory: bool, logger: &Logger) -> Result<(), AppError> {
  let config: Config = maybe_config?;

  info!(logger, "Prepare remove project entry"; "name" => project_name);

  if !config.projects.contains_key(project_name) {
    Err(AppError::UserError(format!("Project key {} does not exist in config", project_name)))
  } else if let Some(project) = config.projects.get(&project_name.to_owned()).cloned() {
    info!(logger, "Updated config"; "config" => format!("{:?}", config));

    if purge_directory {
      let path = config.actual_path_to_project(&project, logger);

      if path.exists() {
        fs::remove_dir_all(&path)?;
      }
    }
    nconfig::delete_project_config(&project)
  } else {
    Err(AppError::UserError(format!("Unknown project {}", project_name)))
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

  nconfig::write_project(&project_config)?;
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
  nconfig::write_project(&project_config)?;
  Ok(())
}

pub fn update_entry(
  maybe_config: Result<Config, AppError>,
  name: &str,
  git: Option<String>,
  after_workon: Option<String>,
  after_clone: Option<String>,
  override_path: Option<String>,
  logger: &Logger,
) -> Result<(), AppError> {
  let config: Config = maybe_config?;
  info!(logger, "Update project entry"; "name" => name);
  if name.starts_with("http") || name.starts_with("git@") {
    Err(AppError::UserError(format!(
      "{} looks like a repo URL and not like a project name, please fix",
      name
    )))
  } else if !config.projects.contains_key(name) {
    Err(AppError::UserError(format!("Project key {} does not exists. Can not update.", name)))
  } else {
    let old_project_config: Project = config.projects.get(name).expect("Already checked in the if above").clone();
    nconfig::write_project(&Project {
      git: git.unwrap_or(old_project_config.git),
      name: old_project_config.name,
      after_clone: after_clone.or(old_project_config.after_clone),
      after_workon: after_workon.or(old_project_config.after_workon),
      override_path: override_path.or(old_project_config.override_path),
      tags: old_project_config.tags,
      bare: old_project_config.bare,
      additional_remotes: old_project_config.additional_remotes,
      project_config_path: old_project_config.project_config_path,
    })?;
    Ok(())
  }
}
