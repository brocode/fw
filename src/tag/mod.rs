use crate::config::Config;
use crate::config::Tag;
use crate::errors::AppError;
use crate::nconfig;
use slog::Logger;
use slog::{debug, info};
use std::collections::{BTreeMap, BTreeSet};

pub fn list_tags(maybe_config: Result<Config, AppError>, maybe_project_name: Option<String>, logger: &Logger) -> Result<(), AppError> {
  let config: Config = maybe_config?;
  if let Some(project_name) = maybe_project_name {
    debug!(logger, "Listing tags for project"; "project" => &project_name);
    list_project_tags(&config, &project_name)
  } else {
    debug!(logger, "Listing tags");
    list_all_tags(config)
  }
}

pub fn delete_tag(maybe_config: Result<Config, AppError>, tag_name: &str, logger: &Logger) -> Result<(), AppError> {
  let config: Config = maybe_config?;
  let tags: BTreeMap<String, Tag> = config.settings.tags.unwrap_or_else(BTreeMap::new);

  // remove tags from projects
  for project_name in config.projects.keys().cloned() {
    if let Some(mut project) = config.projects.get(&project_name).cloned() {
      info!(logger, "Remove tag from project"; "tag" => &tag_name, "project" => &project_name);
      let mut new_tags: BTreeSet<String> = project.tags.clone().unwrap_or_else(BTreeSet::new);
      if new_tags.remove(tag_name) {
        project.tags = Some(new_tags);
        nconfig::write_project(&project)?;
      }
    } else {
      return Err(AppError::UserError(format!("Unknown project {}", project_name)));
    }
  }

  info!(logger, "Delete tag"; "tag" => tag_name);
  if let Some(tag) = tags.get(tag_name) {
    nconfig::delete_tag_config(tag_name, tag)
  } else {
    Ok(())
  }
}

fn list_all_tags(config: Config) -> Result<(), AppError> {
  if let Some(tags) = config.settings.tags {
    for tag_name in tags.keys() {
      println!("{}", tag_name);
    }
  }
  Ok(())
}

pub fn add_tag(config: &Config, project_name: String, tag_name: String, logger: &Logger) -> Result<(), AppError> {
  if let Some(mut project) = config.projects.get(&project_name).cloned() {
    info!(logger, "Add tag to project"; "tag" => &tag_name, "project" => &project_name);
    let tags: BTreeMap<String, Tag> = config.settings.tags.clone().unwrap_or_else(BTreeMap::new);
    if tags.contains_key(&tag_name) {
      let mut new_tags: BTreeSet<String> = project.tags.clone().unwrap_or_else(BTreeSet::new);
      new_tags.insert(tag_name);
      project.tags = Some(new_tags);
      nconfig::write_project(&project)?;
      Ok(())
    } else {
      Err(AppError::UserError(format!("Unknown tag {}", tag_name)))
    }
  } else {
    Err(AppError::UserError(format!("Unknown project {}", project_name)))
  }
}

pub fn create_tag(
  maybe_config: Result<Config, AppError>,
  tag_name: String,
  after_workon: Option<String>,
  after_clone: Option<String>,
  priority: Option<u8>,
  tag_workspace: Option<String>,
  logger: &Logger,
) -> Result<(), AppError> {
  let config: Config = maybe_config?;
  let tags: BTreeMap<String, Tag> = config.settings.tags.unwrap_or_else(BTreeMap::new);
  info!(logger, "Create tag");

  if tags.contains_key(&tag_name) {
    Err(AppError::UserError(format!("Tag {} already exists, not gonna overwrite it for you", tag_name)))
  } else {
    let new_tag = Tag {
      after_clone,
      after_workon,
      priority,
      workspace: tag_workspace,
      default: None,
      tag_config_path: "default".to_string(),
    };
    nconfig::write_tag(&tag_name, &new_tag)?;
    Ok(())
  }
}

pub fn remove_tag(maybe_config: Result<Config, AppError>, project_name: String, tag_name: &str, logger: &Logger) -> Result<(), AppError> {
  let config: Config = maybe_config?;

  if let Some(mut project) = config.projects.get(&project_name).cloned() {
    info!(logger, "Remove tag from project"; "tag" => &tag_name, "project" => &project_name);
    let mut new_tags: BTreeSet<String> = project.tags.clone().unwrap_or_else(BTreeSet::new);
    if new_tags.remove(tag_name) {
      project.tags = Some(new_tags);
      nconfig::write_project(&project)
    } else {
      Ok(())
    }
  } else {
    return Err(AppError::UserError(format!("Unknown project {}", project_name)));
  }
}

fn list_project_tags(config: &Config, project_name: &str) -> Result<(), AppError> {
  if let Some(project) = config.projects.get(project_name) {
    if let Some(tags) = project.clone().tags {
      for tag_name in tags {
        println!("{}", tag_name);
      }
    }
    Ok(())
  } else {
    Err(AppError::UserError(format!("Unknown project {}", project_name)))
  }
}
