use config;
use config::Config;
use config::Tag;
use errors::*;
use slog::Logger;
use std::collections::{BTreeMap, BTreeSet};
use slog::{info, debug};

pub fn list_tags(maybe_config: Result<Config>, maybe_project_name: Option<String>, logger: &Logger) -> Result<()> {
  let config: Config = maybe_config?;
  if let Some(project_name) = maybe_project_name {
    debug!(logger, "Listing tags for project"; "project" => &project_name);
    list_project_tags(&config, &project_name)
  } else {
    debug!(logger, "Listing tags");
    list_all_tags(config)
  }
}

pub fn create_tag(
  maybe_config: Result<Config>,
  tag_name: String,
  after_workon: Option<String>,
  after_clone: Option<String>,
  priority: Option<u8>,
  tag_workspace: Option<String>,
  logger: &Logger,
) -> Result<()> {
  let mut config: Config = maybe_config?;
  let mut tags: BTreeMap<String, Tag> = config.settings.tags.unwrap_or_else(BTreeMap::new);
  info!(logger, "Create tag");
  let new_tag = Tag {
    after_clone,
    after_workon,
    priority,
    workspace: tag_workspace,
  };
  tags.insert(tag_name, new_tag);
  config.settings.tags = Some(tags);
  config::write_config(config, logger)
}

pub fn delete_tag(maybe_config: Result<Config>, tag_name: &str, logger: &Logger) -> Result<()> {
  let mut config: Config = maybe_config?;
  let mut tags: BTreeMap<String, Tag> = config.settings.tags.unwrap_or_else(BTreeMap::new);

  // remove tags from projects
  for (project_name, _value) in config.projects.clone().iter() {
     if let Some(mut project) = config.projects.get(&project_name.to_string()).cloned() {
       info!(logger, "Remove tag from project"; "tag" => &tag_name, "project" => &project_name);
       let mut new_tags: BTreeSet<String> = project.tags.clone().unwrap_or_else(BTreeSet::new);
       if new_tags.remove(tag_name) {
         project.tags = Some(new_tags);
         config.projects.insert(project_name.to_string(), project);
       }
     } else {
       return Err(ErrorKind::InternalError(format!("Unknown project {}", project_name)).into());
     }
  }

  info!(logger, "Delete tag"; "tag" => tag_name);
  if tags.remove(tag_name).is_some() {
    config.settings.tags = Some(tags);
    config::write_config(config, logger)
  } else {
    Ok(())
  }

}

fn list_all_tags(config: Config) -> Result<()> {
  if let Some(tags) = config.settings.tags {
    for tag_name in tags.keys() {
      println!("{}", tag_name);
    }
  }
  Ok(())
}

pub fn add_tag(maybe_config: Result<Config>, project_name: String, tag_name: String, logger: &Logger) -> Result<()> {
  let mut config: Config = maybe_config?;
  if let Some(mut project) = config.projects.get(&project_name).cloned() {
    info!(logger, "Add tag to project"; "tag" => &tag_name, "project" => &project_name);
    let mut tags: BTreeMap<String, Tag> = config.settings.tags.clone().unwrap_or_else(BTreeMap::new);
    if tags.contains_key(&tag_name) {
        let mut new_tags: BTreeSet<String> = project.tags.clone().unwrap_or_else(BTreeSet::new);
        new_tags.insert(tag_name);
        project.tags = Some(new_tags);
        config.projects.insert(project_name, project);
        config::write_config(config, logger)
    } else {
        Err(ErrorKind::UserError(format!("Unknown tag {}", tag_name)).into())
    }

  } else {
    Err(ErrorKind::UserError(format!("Unknown project {}", project_name)).into())
  }
}

pub fn remove_tag(maybe_config: Result<Config>, project_name: String, tag_name: &str, logger: &Logger) -> Result<()> {
  let mut config: Config = maybe_config?;

  if let Some(mut project) = config.projects.get(&project_name).cloned() {
    info!(logger, "Remove tag from project"; "tag" => &tag_name, "project" => &project_name);
    let mut new_tags: BTreeSet<String> = project.tags.clone().unwrap_or_else(BTreeSet::new);
    if new_tags.remove(tag_name) {
      project.tags = Some(new_tags);
      config.projects.insert(project_name, project);
      config::write_config(config, logger)
    } else {
      Ok(())
    }
  } else {
    return Err(ErrorKind::UserError(format!("Unknown project {}", project_name)).into());
  }
}

fn list_project_tags(config: &Config, project_name: &str) -> Result<()> {
  if let Some(project) = config.projects.get(project_name) {
    if let Some(tags) = project.clone().tags {
      for tag_name in tags {
        println!("{}", tag_name);
      }
    }
    Ok(())
  } else {
    Err(ErrorKind::UserError(format!("Unknown project {}", project_name)).into())
  }
}
