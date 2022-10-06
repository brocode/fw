use crate::errors::AppError;
use serde::{Deserialize, Serialize};
use slog::{debug, o, trace, warn, Logger};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

static CONF_MODE_HEADER: &str = "# -*- mode: Conf; -*-\n";

pub mod metadata_from_repository;
mod path;
pub mod project;
pub mod settings;
use path::{expand_path, fw_path};

use project::Project;
use settings::{PersistedSettings, Settings, Tag};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
  pub projects: BTreeMap<String, Project>,
  pub settings: Settings,
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
      if project_file.metadata()?.is_file() && !project_file.file_name().to_os_string().eq(".DS_Store") {
        let raw_project = read_to_string(project_file.path())?;
        let mut project: Project = match toml::from_str(&raw_project) {
          o @ Ok(_) => o,
          e @ Err(_) => {
            eprintln!("There is an issue in your config for project {}", project_file.file_name().to_string_lossy());
            e
          }
        }?;

        project.name = project_file
          .file_name()
          .to_str()
          .map(ToOwned::to_owned)
          .ok_or(AppError::InternalError("Failed to get project name"))?;
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

      if tag_file.metadata()?.is_file() && !tag_file.file_name().to_os_string().eq(".DS_Store") {
        let raw_tag = read_to_string(tag_file.path())?;
        let mut tag: Tag = toml::from_str(&raw_tag)?;
        let tag_name: String = tag_file
          .file_name()
          .to_str()
          .map(ToOwned::to_owned)
          .ok_or(AppError::InternalError("Failed to get tag name"))?;
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

pub fn write_settings(settings: &PersistedSettings, logger: &Logger) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut buffer = File::create(&paths.settings)?;
  let serialized = toml::to_string_pretty(settings)?;
  write!(buffer, "{}", serialized)?;
  write_example(&mut buffer, PersistedSettings::example())?;

  debug!(logger, "Wrote settings file to {:?}", paths.settings);

  Ok(())
}

pub fn write_tag(tag_name: &str, tag: &Tag) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut tag_path = paths.tags;
  tag_path.push(PathBuf::from(&tag.tag_config_path));
  std::fs::create_dir_all(&tag_path)
    .map_err(|e| AppError::RuntimeError(format!("Failed to create tag config path '{}'. {}", tag_path.to_string_lossy(), e)))?;

  let mut tag_file_path = tag_path;
  tag_file_path.push(tag_name);

  let mut buffer = File::create(&tag_file_path)
    .map_err(|e| AppError::RuntimeError(format!("Failed to create project config file '{}'. {}", tag_file_path.to_string_lossy(), e)))?;
  let serialized = toml::to_string_pretty(&tag)?;
  write!(buffer, "{}", CONF_MODE_HEADER)?;
  write!(buffer, "{}", serialized)?;
  write_example(&mut buffer, Tag::example())?;
  Ok(())
}

pub fn delete_tag_config(tag_name: &str, tag: &Tag) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut tag_file_path = paths.tags;
  tag_file_path.push(PathBuf::from(&tag.tag_config_path));
  tag_file_path.push(tag_name);

  fs::remove_file(&tag_file_path).map_err(|e| AppError::RuntimeError(format!("Failed to delete tag config from '{:?}': {}", tag_file_path, e)))?;
  Ok(())
}

pub fn delete_project_config(project: &Project) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut project_file_path = paths.projects;
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
  for line in example_toml.split('\n') {
    if line.trim() != "" {
      writeln!(buffer, "# {}", line)?;
    }
  }
  Ok(())
}

pub fn write_project(project: &Project) -> Result<(), AppError> {
  let paths = fw_path()?;
  paths.ensure_base_exists()?;

  let mut project_path = paths.projects;
  project_path.push(PathBuf::from(&project.project_config_path));
  std::fs::create_dir_all(&project_path)
    .map_err(|e| AppError::RuntimeError(format!("Failed to create project config path '{}'. {}", project_path.to_string_lossy(), e)))?;

  let mut project_file_path = project_path;
  project_file_path.push(&project.name);

  let mut buffer: File = File::create(&project_file_path)
    .map_err(|e| AppError::RuntimeError(format!("Failed to create project config file '{}'. {}", project_file_path.to_string_lossy(), e)))?;
  let serialized = toml::to_string_pretty(&project)?;

  write!(buffer, "{}", CONF_MODE_HEADER)?;
  write!(buffer, "{}", serialized)?;
  write_example(&mut buffer, Project::example())?;
  Ok(())
}

impl Config {
  pub fn actual_path_to_project(&self, project: &Project, logger: &Logger) -> PathBuf {
    let path = project
      .override_path
      .clone()
      .map(PathBuf::from)
      .unwrap_or_else(|| Path::new(self.resolve_workspace(logger, project).as_str()).join(project.name.as_str()));
    expand_path(path)
  }

  fn resolve_workspace(&self, logger: &Logger, project: &Project) -> String {
    let mut x = self.resolve_from_tags(|tag| tag.workspace.clone(), project.tags.clone(), logger);
    let workspace = x.pop().unwrap_or_else(|| self.settings.workspace.clone());
    trace!(logger, "resolved"; "workspace" => &workspace);
    workspace
  }
  pub fn resolve_after_clone(&self, logger: &Logger, project: &Project) -> Vec<String> {
    let mut commands: Vec<String> = vec![];
    commands.extend_from_slice(&self.resolve_after_clone_from_tags(project.tags.clone(), logger));
    let commands_from_project: Vec<String> = project.after_clone.clone().into_iter().collect();
    commands.extend_from_slice(&commands_from_project);
    commands
  }
  pub fn resolve_after_workon(&self, logger: &Logger, project: &Project) -> Vec<String> {
    let mut commands: Vec<String> = vec![];
    commands.extend_from_slice(&self.resolve_workon_from_tags(project.tags.clone(), logger));
    let commands_from_project: Vec<String> = project.after_workon.clone().into_iter().collect();
    commands.extend_from_slice(&commands_from_project);
    commands
  }

  fn resolve_workon_from_tags(&self, maybe_tags: Option<BTreeSet<String>>, logger: &Logger) -> Vec<String> {
    self.resolve_from_tags(|t| t.clone().after_workon, maybe_tags, logger)
  }
  fn resolve_after_clone_from_tags(&self, maybe_tags: Option<BTreeSet<String>>, logger: &Logger) -> Vec<String> {
    self.resolve_from_tags(|t| t.clone().after_clone, maybe_tags, logger)
  }

  fn tag_priority_or_fallback(&self, name: &str, tag: &Tag, logger: &Logger) -> u8 {
    match tag.priority {
      None => {
        debug!(logger, r#"No tag priority set, will use default (50).
Tags with low priority are applied first and if they all have the same priority
they will be applied in alphabetical name order so it is recommended you make a
conscious choice and set the value."#;
            "tag_name" => name, "tag_def" => format!("{:?}", tag));
        50
      }
      Some(p) => p,
    }
  }

  fn resolve_from_tags<F>(&self, resolver: F, maybe_tags: Option<BTreeSet<String>>, logger: &Logger) -> Vec<String>
  where
    F: Fn(&Tag) -> Option<String>,
  {
    let tag_logger = logger.new(o!("tags" => format!("{:?}", maybe_tags)));
    trace!(tag_logger, "Resolving");
    if let (Some(tags), Some(settings_tags)) = (maybe_tags, self.clone().settings.tags) {
      let mut resolved_with_priority: Vec<(String, u8)> = tags
        .iter()
        .flat_map(|t| match settings_tags.get(t) {
          None => {
            warn!(tag_logger, "Ignoring tag since it was not found in the config"; "missing_tag" => t.clone());
            None
          }
          Some(actual_tag) => resolver(actual_tag).map(|val| (val, self.tag_priority_or_fallback(t, actual_tag, logger))),
        })
        .collect();
      trace!(logger, "before sort"; "tags" => format!("{:?}", resolved_with_priority));
      resolved_with_priority.sort_by_key(|resolved_and_priority| resolved_and_priority.1);
      trace!(logger, "after sort"; "tags" => format!("{:?}", resolved_with_priority));
      resolved_with_priority.into_iter().map(|r| r.0).collect()
    } else {
      vec![]
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use maplit::btreeset;

  #[test]
  fn test_workon_from_tags() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_workon(&logger, config.projects.get("test1").unwrap());
    assert_eq!(resolved, vec!["workon1".to_string(), "workon2".to_string()]);
  }
  #[test]
  fn test_workon_from_tags_prioritized() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_workon(&logger, config.projects.get("test5").unwrap());
    assert_eq!(resolved, vec!["workon4".to_string(), "workon3".to_string()]);
  }
  #[test]
  fn test_after_clone_from_tags() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_clone(&logger, config.projects.get("test1").unwrap());
    assert_eq!(resolved, vec!["clone1".to_string(), "clone2".to_string()]);
  }
  #[test]
  fn test_after_clone_from_tags_prioritized() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_clone(&logger, config.projects.get("test5").unwrap());
    assert_eq!(resolved, vec!["clone4".to_string(), "clone3".to_string()]);
  }
  #[test]
  fn test_workon_from_tags_missing_one_tag_graceful() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_workon(&logger, config.projects.get("test2").unwrap());
    assert_eq!(resolved, vec!["workon1".to_owned()]);
  }
  #[test]
  fn test_workon_from_tags_missing_all_tags_graceful() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_workon(&logger, config.projects.get("test4").unwrap());
    assert_eq!(resolved, Vec::<String>::new());
  }
  #[test]
  fn test_after_clone_from_tags_missing_all_tags_graceful() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_clone(&logger, config.projects.get("test4").unwrap());
    assert_eq!(resolved, Vec::<String>::new());
  }
  #[test]
  fn test_after_clone_from_tags_missing_one_tag_graceful() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_clone(&logger, config.projects.get("test2").unwrap());
    assert_eq!(resolved, vec!["clone1".to_owned()]);
  }
  #[test]
  fn test_workon_override_from_project() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_workon(&logger, config.projects.get("test3").unwrap());
    assert_eq!(resolved, vec!["workon1".to_string(), "workon override in project".to_owned()]);
  }
  #[test]
  fn test_after_clone_override_from_project() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_clone(&logger, config.projects.get("test3").unwrap());
    assert_eq!(resolved, vec!["clone1".to_string(), "clone override in project".to_owned()]);
  }

  fn a_config() -> Config {
    let project = Project {
      name: "test1".to_owned(),
      git: "irrelevant".to_owned(),
      tags: Some(btreeset!["tag1".to_owned(), "tag2".to_owned()]),
      after_clone: None,
      after_workon: None,
      override_path: None,
      additional_remotes: None,
      bare: None,
      trusted: false,
      project_config_path: "".to_string(),
    };
    let project2 = Project {
      name: "test2".to_owned(),
      git: "irrelevant".to_owned(),
      tags: Some(btreeset!["tag1".to_owned(), "tag-does-not-exist".to_owned(),]),
      after_clone: None,
      after_workon: None,
      override_path: None,
      additional_remotes: None,
      bare: None,
      trusted: false,
      project_config_path: "".to_string(),
    };
    let project3 = Project {
      name: "test3".to_owned(),
      git: "irrelevant".to_owned(),
      tags: Some(btreeset!["tag1".to_owned()]),
      after_clone: Some("clone override in project".to_owned()),
      after_workon: Some("workon override in project".to_owned()),
      override_path: None,
      additional_remotes: None,
      bare: None,
      trusted: false,
      project_config_path: "".to_string(),
    };
    let project4 = Project {
      name: "test4".to_owned(),
      git: "irrelevant".to_owned(),
      tags: Some(btreeset!["tag-does-not-exist".to_owned()]),
      after_clone: None,
      after_workon: None,
      override_path: None,
      additional_remotes: None,
      bare: None,
      trusted: false,
      project_config_path: "".to_string(),
    };
    let project5 = Project {
      name: "test5".to_owned(),
      git: "irrelevant".to_owned(),
      tags: Some(btreeset!["tag3".to_owned(), "tag4".to_owned()]),
      after_clone: None,
      after_workon: None,
      override_path: None,
      additional_remotes: None,
      bare: None,
      trusted: false,
      project_config_path: "".to_string(),
    };
    let tag1 = Tag {
      after_clone: Some("clone1".to_owned()),
      after_workon: Some("workon1".to_owned()),
      priority: None,
      workspace: None,
      default: None,
      tag_config_path: "".to_string(),
    };
    let tag2 = Tag {
      after_clone: Some("clone2".to_owned()),
      after_workon: Some("workon2".to_owned()),
      priority: None,
      workspace: None,
      default: None,
      tag_config_path: "".to_string(),
    };
    let tag3 = Tag {
      after_clone: Some("clone3".to_owned()),
      after_workon: Some("workon3".to_owned()),
      priority: Some(100),
      workspace: None,
      default: None,
      tag_config_path: "".to_string(),
    };
    let tag4 = Tag {
      after_clone: Some("clone4".to_owned()),
      after_workon: Some("workon4".to_owned()),
      priority: Some(0),
      workspace: None,
      default: None,
      tag_config_path: "".to_string(),
    };
    let mut projects: BTreeMap<String, Project> = BTreeMap::new();
    projects.insert("test1".to_owned(), project);
    projects.insert("test2".to_owned(), project2);
    projects.insert("test3".to_owned(), project3);
    projects.insert("test4".to_owned(), project4);
    projects.insert("test5".to_owned(), project5);
    let mut tags: BTreeMap<String, Tag> = BTreeMap::new();
    tags.insert("tag1".to_owned(), tag1);
    tags.insert("tag2".to_owned(), tag2);
    tags.insert("tag3".to_owned(), tag3);
    tags.insert("tag4".to_owned(), tag4);
    let settings = Settings {
      workspace: "/test".to_owned(),
      default_after_workon: None,
      default_after_clone: None,
      default_tags: None,
      shell: None,
      tags: Some(tags),
      github_token: None,
      gitlab: None,
    };
    Config { projects, settings }
  }

  fn a_logger() -> Logger {
    use slog::Drain;
    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let drain = slog_term::FullFormat::new(plain).build().fuse();
    Logger::root(drain, o!())
  }
}
