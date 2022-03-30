use crate::config;
use crate::config::Config;
use crate::config::{project::Project, project::Remote};
use crate::errors::AppError;
use crate::git::repo_name_from_url;
use ansi_term::Style;
use slog::Logger;
use slog::{debug, info};
use std::collections::BTreeSet;
use std::fs;

pub fn add_entry(
  maybe_config: Result<Config, AppError>,
  maybe_name: Option<&str>,
  url: &str,
  after_workon: Option<String>,
  after_clone: Option<String>,
  override_path: Option<String>,
  tags: Option<BTreeSet<String>>,
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
    let default_after_workon = config.settings.default_after_workon.clone();

    let project_tags: Option<BTreeSet<String>> = if tags.is_some() && config.settings.default_tags.is_some() {
      tags.zip(config.settings.default_tags).map(|(t1, t2)| t1.union(&t2).cloned().collect())
    } else {
      tags.or(config.settings.default_tags)
    };

    config::write_project(&Project {
      git: url.to_owned(),
      name: name.to_owned(),
      after_clone: after_clone.or(default_after_clone),
      after_workon: after_workon.or(default_after_workon),
      override_path,
      tags: project_tags,
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
    config::delete_project_config(&project)
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

  config::write_project(&project_config)?;
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
  config::write_project(&project_config)?;
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
    config::write_project(&Project {
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

pub fn ls(maybe_config: Result<Config, AppError>, tags: &BTreeSet<String>) -> Result<(), AppError> {
  let config = maybe_config?;
  for (name, project) in config.projects {
    if tags.is_empty() || project.tags.unwrap_or_default().intersection(tags).count() > 0 {
      println!("{}", name)
    }
  }
  Ok(())
}

pub fn print_path(maybe_config: Result<Config, AppError>, name: &str, logger: &Logger) -> Result<(), AppError> {
  let config = maybe_config?;
  let project = config
    .projects
    .get(name)
    .ok_or_else(|| AppError::UserError(format!("project {} not found", name)))?;
  let canonical_project_path = config.actual_path_to_project(project, logger);
  let path = canonical_project_path
    .to_str()
    .ok_or(AppError::InternalError("project path is not valid unicode"))?;
  println!("{}", path);
  Ok(())
}

pub fn inspect(name: &str, maybe_config: Result<Config, AppError>, json: bool, logger: &Logger) -> Result<(), AppError> {
  let config = maybe_config?;
  let project = config
    .projects
    .get(name)
    .ok_or_else(|| AppError::UserError(format!("project {} not found", name)))?;
  if json {
    println!("{}", serde_json::to_string(project)?);
    return Ok(());
  }
  let canonical_project_path = config.actual_path_to_project(project, logger);
  let path = canonical_project_path
    .to_str()
    .ok_or(AppError::InternalError("project path is not valid unicode"))?;
  println!("{}", Style::new().underline().bold().paint(project.name.clone()));
  println!("{:<20}: {}", "Path", path);
  println!("{:<20}: {}", "config path", project.project_config_path);
  let tags = project
    .tags
    .clone()
    .map(|t| {
      let project_tags: Vec<String> = t.into_iter().collect();
      project_tags.join(", ")
    })
    .unwrap_or_else(|| "None".to_owned());
  println!("{:<20}: {}", "Tags", tags);
  let additional_remotes = project
    .additional_remotes
    .clone()
    .map(|t| {
      let project_tags: Vec<String> = t.into_iter().map(|r| format!("{} - {}", r.name, r.git)).collect();
      project_tags.join(", ")
    })
    .unwrap_or_else(|| "None".to_owned());
  println!("{:<20}: {}", "Additional remotes", additional_remotes);
  let git = project.git.clone();
  println!("{:<20}: {}", "Git", git);
  Ok(())
}
