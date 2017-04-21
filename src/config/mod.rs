use errors::AppError;
use serde_json;
use slog::Logger;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
  pub workspace: String,
  pub default_after_workon: Option<String>,
  pub default_after_clone: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
  pub name: String,
  pub git: String,
  pub after_clone: Option<String>,
  pub after_workon: Option<String>,
  pub override_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
  pub projects: HashMap<String, Project>,
  pub settings: Settings,
}

fn read_config<R>(reader: Result<R, AppError>) -> Result<Config, AppError>
  where R: Read
{
  reader.and_then(|r| serde_json::de::from_reader(r).map_err(AppError::BadJson))
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

pub fn add_entry(maybe_config: Result<Config, AppError>, name: &str, url: &str, logger: &Logger) -> Result<(), AppError> {
  let mut config: Config = maybe_config?;
  info!(logger, "Prepare new project entry"; "name" => name, "url" => url);
  if name.starts_with("http") || name.starts_with("git@") {
    Err(AppError::UserError(format!("{} looks like a repo URL and not like a project name, please fix",
                                    name)))
  } else if config.projects.contains_key(name) {
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
                  });
    info!(logger, "Updated config"; "config" => format!("{:?}", config));
    write_config(&config, logger)
  }
}

pub fn write_config(config: &Config, logger: &Logger) -> Result<(), AppError> {
  let config_path = config_path()?;
  info!(logger, "Writing config"; "path" => format!("{:?}", config_path));
  let mut buffer = File::create(config_path)?;
  serde_json::ser::to_writer_pretty(&mut buffer, &config).map_err(AppError::BadJson)
}

pub fn actual_path_to_project(workspace: &str, project: &Project) -> PathBuf {
  project.override_path
         .clone()
         .map(PathBuf::from)
         .unwrap_or_else(|| Path::new(workspace).join(project.name.as_str()))
}
