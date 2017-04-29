use config;
use config::Config;
use config::Tag;
use errors::AppError;
use slog::Logger;
use std::collections::{BTreeMap, BTreeSet};


pub fn list_tags(maybe_config: Result<Config, AppError>, maybe_project_name: Option<String>, logger: &Logger) -> Result<(), AppError> {
  let config: Config = maybe_config?;
  if let Some(project_name) = maybe_project_name {
    debug!(logger, "Listing tags for project"; "project" => project_name);
    list_project_tags(config, &project_name)
  } else {
    debug!(logger, "Listing tags");
    list_all_tags(config)
  }
}

pub fn create_tag(maybe_config: Result<Config, AppError>,
                  tag_name: String,
                  after_workon: Option<String>,
                  after_clone: Option<String>,
                  logger: &Logger)
                  -> Result<(), AppError> {
  let mut config: Config = maybe_config?;
  let mut tags: BTreeMap<String, Tag> = config.settings.tags.unwrap_or(BTreeMap::new());
  info!(logger, "Create tag");
  let new_tag = Tag {
    after_clone: after_clone,
    after_workon: after_workon,
  };
  tags.insert(tag_name, new_tag);
  config.settings.tags = Some(tags);
  config::write_config(config, logger)
}

fn list_all_tags(config: Config) -> Result<(), AppError> {
  if let Some(tags) = config.settings.tags {
    for tag_name in tags.keys() {
      println!("{}", tag_name);
    }
  }
  Result::Ok(())
}

pub fn add_tag(maybe_config: Result<Config, AppError>, project_name: String, tag_name: String, logger: &Logger) -> Result<(), AppError> {
  let mut config: Config = maybe_config?;

  if let Some(mut project) = config.projects.get(&project_name).cloned() {
    let mut new_tags: BTreeSet<String> = project.tags.clone().unwrap_or(BTreeSet::new());
    new_tags.insert(tag_name);
    project.tags = Some(new_tags);
    config.projects.insert(project_name, project);
    config::write_config(config, logger)
  } else {
    Result::Err(AppError::UserError(format!("Unknown project {}", project_name)))
  }
}

pub fn remove_tag(maybe_config: Result<Config, AppError>, project_name: String, tag_name: String, logger: &Logger) -> Result<(), AppError> {
  let mut config: Config = maybe_config?;

  if let Some(mut project) = config.projects.get(&project_name).cloned() {
    let mut new_tags: BTreeSet<String> = project.tags.clone().unwrap_or(BTreeSet::new());
    if new_tags.remove(&tag_name) {
      project.tags = Some(new_tags);
      config.projects.insert(project_name, project);
      config::write_config(config, logger)
    } else {
      Result::Ok(())
    }
  } else {
    return Result::Err(AppError::UserError(format!("Unknown project {}", project_name)))
  }
}


fn list_project_tags(config: Config, project_name: &str) -> Result<(), AppError> {
  if let Some(project) = config.projects.get(project_name) {
    if let Some(tags) = project.clone().tags {
      for tag_name in tags {
        println!("{}", tag_name);
      }
    }
    Result::Ok(())
  } else {
    Result::Err(AppError::UserError(format!("Unknown project {}", project_name)))
  }
}
