use crate::errors::AppError;
use dirs;
use serde_json;
use slog::Logger;
use slog::{debug, o, trace, warn};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
  pub workspace: String,
  pub shell: Option<Vec<String>>,
  pub default_after_workon: Option<String>,
  pub default_after_clone: Option<String>,
  pub default_tags: Option<BTreeSet<String>>,
  pub tags: Option<BTreeMap<String, Tag>>,
  pub github_token: Option<String>,
  pub gitlab: Option<GitlabSettings>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitlabSettings {
  pub token: String,
  pub host: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tag {
  pub after_clone: Option<String>,
  pub after_workon: Option<String>,
  pub priority: Option<u8>,
  pub workspace: Option<String>,
  pub default: Option<bool>,

  #[serde(skip)]
  pub tag_config_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Remote {
  pub name: String,
  pub git: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
  pub name: String,
  pub git: String,
  pub after_clone: Option<String>,
  pub after_workon: Option<String>,
  pub override_path: Option<String>,
  pub bare: Option<bool>,
  pub tags: Option<BTreeSet<String>>,
  pub additional_remotes: Option<Vec<Remote>>,

  #[serde(skip)]
  pub project_config_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
  pub projects: BTreeMap<String, Project>,
  pub settings: Settings,
}

impl Project {
  fn check_sanity(&self, config: &Config, logger: &Logger) -> Result<(), AppError> {
    let sanity_logger = logger.new(o!("task" => "check_sanity"));
    let path = config.actual_path_to_project(self, &sanity_logger);
    if path.is_absolute() {
      Ok(())
    } else {
      Err(AppError::UserError(format!(
        "Misconfigured project {}: resolved path {:?} is relative which is not allowed",
        &self.name, &path
      )))
    }
  }
}

fn fw_path() -> Result<PathBuf, AppError> {
  let raw_path = match env::var("FW_CONFIG_PATH") {
    Ok(path) => Ok(PathBuf::from(path)),
    Err(_) => Ok(PathBuf::from("~/.fw.json")),
  };
  raw_path.map(expand_path)
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

  fn check_sanity(self, logger: &Logger) -> Result<Config, AppError> {
    for project in self.projects.values() {
      project.check_sanity(&self, logger)?
    }
    Ok(self)
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
    if maybe_tags.is_none() || self.settings.tags.is_none() {
      vec![]
    } else {
      let tags: BTreeSet<String> = maybe_tags.unwrap();
      let settings_tags = self.clone().settings.tags.unwrap();
      let mut resolved_with_priority: Vec<(String, u8)> = tags
        .iter()
        .flat_map(|t| match settings_tags.get(t) {
          None => {
            warn!(tag_logger, "Ignoring tag since it was not found in the config"; "missing_tag" => t.clone());
            None
          }
          Some(actual_tag) => resolver(actual_tag)
            .clone()
            .map(|val| (val, self.tag_priority_or_fallback(t, actual_tag, logger))),
        })
        .collect();
      trace!(logger, "before sort"; "tags" => format!("{:?}", resolved_with_priority));
      resolved_with_priority.sort_by_key(|resolved_and_priority| resolved_and_priority.1);
      trace!(logger, "after sort"; "tags" => format!("{:?}", resolved_with_priority));
      resolved_with_priority.into_iter().map(|r| r.0).collect()
    }
  }
}

fn read_config<R>(reader: Result<R, AppError>, logger: &Logger) -> Result<Config, AppError>
where
  R: Read,
{
  reader
    .and_then(|r| serde_json::de::from_reader(r).map_err(AppError::BadJson))
    .and_then(|c: Config| c.check_sanity(logger))
}

fn open_config() -> Result<File, AppError> {
  let config_file_path = fw_path()?;
  let path = config_file_path
    .to_str()
    .ok_or_else(|| AppError::UserError("$HOME is not valid utf8".to_owned()));
  path.and_then(|path| File::open(path).map_err(AppError::IO))
}

pub fn get_config(logger: &Logger) -> Result<Config, AppError> {
  let config_file = open_config();
  let reader = config_file.map(BufReader::new);
  read_config(reader, logger)
}

pub fn repo_name_from_url(url: &str) -> Result<&str, AppError> {
  let last_fragment = url.rsplit('/').next().ok_or_else(|| {
    AppError::UserError(format!(
      "Given URL {} does not have path fragments so cannot determine project name. Please give \
       one.",
      url
    ))
  })?;

  // trim_right_matches is more efficient but would fuck us up with repos like git@github.com:bauer/test.git.git (which is legal)
  Ok(if last_fragment.ends_with(".git") {
    last_fragment.split_at(last_fragment.len() - 4).0
  } else {
    last_fragment
  })
}

fn do_expand(path: PathBuf, home_dir: Option<PathBuf>) -> PathBuf {
  if let Some(home) = home_dir {
    home.join(path.strip_prefix("~").expect("only doing this if path starts with ~"))
  } else {
    path
  }
}

pub fn expand_path(path: PathBuf) -> PathBuf {
  if path.starts_with("~") {
    do_expand(path, dirs::home_dir())
  } else {
    path
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use maplit::btreeset;
  use spectral::prelude::*;

  #[test]
  fn test_repo_name_from_url() {
    let https_url = "https://github.com/mriehl/fw";
    let name = repo_name_from_url(&https_url).unwrap().to_owned();
    assert_that(&name).is_equal_to("fw".to_owned());
  }
  #[test]
  fn test_repo_name_from_ssh_pragma() {
    let ssh_pragma = "git@github.com:mriehl/fw.git";
    let name = repo_name_from_url(&ssh_pragma).unwrap().to_owned();
    assert_that(&name).is_equal_to("fw".to_owned());
  }
  #[test]
  fn test_repo_name_from_ssh_pragma_with_multiple_git_endings() {
    let ssh_pragma = "git@github.com:mriehl/fw.git.git";
    let name = repo_name_from_url(&ssh_pragma).unwrap().to_owned();
    assert_that(&name).is_equal_to("fw.git".to_owned());
  }
  #[test]
  fn test_do_not_expand_path_without_tilde() {
    let path = PathBuf::from("/foo/bar");
    assert_that(&expand_path(path.clone())).is_equal_to(&path);
  }
  #[test]
  fn test_do_expand_path() {
    let path = PathBuf::from("~/foo/bar");
    let home = PathBuf::from("/my/home");
    assert_that(&do_expand(path, Some(home))).is_equal_to(PathBuf::from("/my/home/foo/bar"));
  }
  #[test]
  fn test_workon_from_tags() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_workon(&logger, config.projects.get("test1").unwrap());
    assert_that(&resolved).is_equal_to(vec!["workon1".to_string(), "workon2".to_string()]);
  }
  #[test]
  fn test_workon_from_tags_prioritized() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_workon(&logger, config.projects.get("test5").unwrap());
    assert_that(&resolved).is_equal_to(vec!["workon4".to_string(), "workon3".to_string()]);
  }
  #[test]
  fn test_after_clone_from_tags() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_clone(&logger, config.projects.get("test1").unwrap());
    assert_that(&resolved).is_equal_to(vec!["clone1".to_string(), "clone2".to_string()]);
  }
  #[test]
  fn test_after_clone_from_tags_prioritized() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_clone(&logger, config.projects.get("test5").unwrap());
    assert_that(&resolved).is_equal_to(vec!["clone4".to_string(), "clone3".to_string()]);
  }
  #[test]
  fn test_workon_from_tags_missing_one_tag_graceful() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_workon(&logger, config.projects.get("test2").unwrap());
    assert_that(&resolved).is_equal_to(vec!["workon1".to_owned()]);
  }
  #[test]
  fn test_workon_from_tags_missing_all_tags_graceful() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_workon(&logger, config.projects.get("test4").unwrap());
    assert_that(&resolved).is_equal_to(vec![]);
  }
  #[test]
  fn test_after_clone_from_tags_missing_all_tags_graceful() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_clone(&logger, config.projects.get("test4").unwrap());
    assert_that(&resolved).is_equal_to(vec![]);
  }
  #[test]
  fn test_after_clone_from_tags_missing_one_tag_graceful() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_clone(&logger, config.projects.get("test2").unwrap());
    assert_that(&resolved).is_equal_to(vec!["clone1".to_owned()]);
  }
  #[test]
  fn test_workon_override_from_project() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_workon(&logger, config.projects.get("test3").unwrap());
    assert_that(&resolved).is_equal_to(vec!["workon1".to_string(), "workon override in project".to_owned()]);
  }
  #[test]
  fn test_after_clone_override_from_project() {
    let config = a_config();
    let logger = a_logger();
    let resolved = config.resolve_after_clone(&logger, config.projects.get("test3").unwrap());
    assert_that(&resolved).is_equal_to(vec!["clone1".to_string(), "clone override in project".to_owned()]);
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
    use slog_term;
    use std;
    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let drain = slog_term::FullFormat::new(plain).build().fuse();
    Logger::root(drain, o!())
  }
}
