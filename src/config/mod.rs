use errors::AppError;
use serde_json;
use slog::Logger;
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
  pub workspace: String,
  pub default_after_workon: Option<String>,
  pub default_after_clone: Option<String>,
  pub tags: Option<BTreeMap<String, Tag>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tag {
  pub after_clone: Option<String>,
  pub after_workon: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
  pub name: String,
  pub git: String,
  pub after_clone: Option<String>,
  pub after_workon: Option<String>,
  pub override_path: Option<String>,
  pub tags: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
  pub projects: BTreeMap<String, Project>,
  pub settings: Settings,
}

impl Project {
  fn check_sanity(&self, workspace: &str) -> Result<(), AppError> {
    let path = actual_path_to_project(workspace, self);
    if path.is_absolute() {
      Ok(())
    } else {
      Err(AppError::UserError(format!("Misconfigured project {}: resolved path {:?} is relative which is not allowed",
                                      &self.name,
                                      &path)))
    }
  }
}

impl Config {
  pub fn resolve_after_workon(&self, logger: &Logger, project: &Project) -> String {
    project.after_workon
           .clone()
           .or_else(|| {
                      self.resolve_workon_from_tags(project.tags.clone(), logger)
                    })
           .map(|c| prepare_workon(&c))
           .unwrap_or_else(|| "".to_owned())
  }

  fn check_sanity(self) -> Result<Config, AppError> {
    for project in self.projects.values() {
      project.check_sanity(&self.settings.workspace)?
    }
    Ok(self)
  }


  pub fn resolve_workon_from_tags(&self, maybe_tags: Option<Vec<String>>, logger: &Logger) -> Option<String> {
    let tag_logger = logger.new(o!("tags" => format!("{:?}", maybe_tags)));
    debug!(tag_logger, "Resolving");
    if maybe_tags.is_none() || self.settings.tags.is_none() {
      None
    } else {
      let tags: Vec<String> = maybe_tags.unwrap();
      let settings_tags = self.clone().settings.tags.unwrap();
      let resolved_after_workon: Vec<String> =
        tags.iter()
            .flat_map(|t| match settings_tags.get(t) {
                      None => {
          warn!(tag_logger, "Ignoring tag since it was not found in the config"; "missing_tag" => t.clone());
          None
        }
                      Some(actual_tag) => actual_tag.after_workon.clone(),
                      })
            .collect();
      let after_workon_cmd = resolved_after_workon.join(" && ");
      debug!(tag_logger, format!("resolved {:?}", after_workon_cmd));
      Some(after_workon_cmd)
    }
  }
}

fn prepare_workon(workon: &str) -> String {
  format!(" && {}", workon)
}

fn read_config<R>(reader: Result<R, AppError>) -> Result<Config, AppError>
  where R: Read
{
  reader.and_then(|r| serde_json::de::from_reader(r).map_err(AppError::BadJson))
        .and_then(|c: Config| c.check_sanity())
}

pub fn config_path() -> Result<PathBuf, AppError> {
  let mut home: PathBuf = env::home_dir()
    .ok_or_else(|| AppError::UserError("$HOME not set".to_owned()))?;
  home.push(".fw.json");
  Ok(home)
}

fn determine_config() -> Result<File, AppError> {
  let config_file_path = config_path()?;
  let path = config_file_path.to_str()
                             .ok_or_else(|| AppError::UserError("$HOME is not valid utf8".to_owned()));
  path.and_then(|path| File::open(path).map_err(AppError::IO))
}

pub fn get_config() -> Result<Config, AppError> {
  let config_file = determine_config();
  let reader = config_file.map(BufReader::new);
  read_config(reader)
}

fn repo_name_from_url(url: &str) -> Result<&str, AppError> {
  let last_fragment = url.rsplit('/')
                         .next()
                         .ok_or_else(|| {
                                       AppError::UserError(format!("Given URL {} does not have path fragments so cannot determine project name. Please give \
                                                                    one.",
                                                                   url))
                                     })?;

  // trim_right_matches is more efficient but would fuck us up with repos like git@github.com:bauer/test.git.git (which is legal)
  Ok(if last_fragment.ends_with(".git") {
       last_fragment.split_at(last_fragment.len() - 4).0
     } else {
       last_fragment
     })
}

pub fn add_entry(maybe_config: Result<Config, AppError>, maybe_name: Option<&str>, url: &str, logger: &Logger) -> Result<(), AppError> {
  let name = maybe_name.ok_or_else(|| AppError::UserError(format!("No project name specified for {}", url)))
                       .or_else(|_| repo_name_from_url(url))?;
  let mut config: Config = maybe_config?;
  info!(logger, "Prepare new project entry"; "name" => name, "url" => url);
  if config.projects.contains_key(name) {
    Err(AppError::UserError(format!("Project key {} already exists, not gonna overwrite it for you",
                                    name)))
  } else {
    config.projects
          .insert(name.to_owned(),
                  Project {
                    git: url.to_owned(),
                    name: name.to_owned(),
                    after_clone: config.settings.default_after_clone.clone(),
                    after_workon: config.settings.default_after_workon.clone(),
                    override_path: None,
                    tags: None,
                  });
    info!(logger, "Updated config"; "config" => format!("{:?}", config));
    write_config(config, logger)
  }
}

pub fn update_entry(maybe_config: Result<Config, AppError>,
                    name: &str,
                    git: Option<String>,
                    after_workon: Option<String>,
                    after_clone: Option<String>,
                    override_path: Option<String>,
                    logger: &Logger)
                    -> Result<(), AppError> {
  let mut config: Config = maybe_config?;
  info!(logger, "Update project entry"; "name" => name);
  if name.starts_with("http") || name.starts_with("git@") {
    Err(AppError::UserError(format!("{} looks like a repo URL and not like a project name, please fix",
                                    name)))
  } else if !config.projects.contains_key(name) {
    Err(AppError::UserError(format!("Project key {} does not exists. Can not update.", name)))
  } else {
    let old_project_config: Project = config.projects
                                            .get(name)
                                            .expect("Already checked in the if above")
                                            .clone();
    config.projects
          .insert(name.to_owned(),
                  Project {
                    git: git.unwrap_or(old_project_config.git),
                    name: old_project_config.name,
                    after_clone: after_clone.or(old_project_config.after_clone),
                    after_workon: after_workon.or(old_project_config.after_workon),
                    override_path: override_path.or(old_project_config.override_path),
                    tags: None,
                  });
    info!(logger, "Updated config"; "config" => format!("{:?}", config));
    write_config(config, logger)
  }
}

pub fn write_config(config: Config, logger: &Logger) -> Result<(), AppError> {
  let config_path = config_path()?;
  info!(logger, "Writing config"; "path" => format!("{:?}", config_path));
  config.check_sanity()
        .and_then(|c| {
                    let mut buffer = File::create(config_path)?;
                    serde_json::ser::to_writer_pretty(&mut buffer, &c).map_err(AppError::BadJson)
                  })
}

fn do_expand(path: PathBuf, home_dir: Option<PathBuf>) -> PathBuf {
  if let Some(home) = home_dir {
    home.join(path.strip_prefix("~")
                  .expect("only doing this if path starts with ~"))
  } else {
    path
  }
}

pub fn expand_path(path: PathBuf) -> PathBuf {
  if path.starts_with("~") {
    do_expand(path, env::home_dir())
  } else {
    path
  }
}

pub fn actual_path_to_project(workspace: &str, project: &Project) -> PathBuf {
  let path = project.override_path
                    .clone()
                    .map(PathBuf::from)
                    .unwrap_or_else(|| Path::new(workspace).join(project.name.as_str()));
  expand_path(path)
}

#[cfg(test)]
mod tests {
  use super::*;
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
}
