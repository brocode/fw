use std::collections::HashMap;
use std::io::Read;
use serde_json;
use errors::AppError;
use std::path::PathBuf;
use std::env;
use std::io::BufReader;
use std::fs::File;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
  pub workspace: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
  pub name: String,
  pub git: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
  pub projects: HashMap<String, Project>,
  pub settings: Settings,
}

fn read_config<R>(reader: Result<R, AppError>) -> Result<Config, AppError>
  where R: Read
{
  reader.and_then(|r| serde_json::de::from_reader(r).map_err(|error| AppError::BadJson(error)))
}

fn determine_config(name: &str) -> Result<File, AppError> {
  let mut home: PathBuf = try!(env::home_dir().ok_or(AppError::UserError("$HOME not set"
                                                                           .to_owned())));
  home.push(name);
  let path = home.to_str().ok_or(AppError::UserError("$HOME is not valid utf8".to_owned()));
  path.and_then(|path| File::open(path).map_err(|err| AppError::IO(err)))
}

pub fn get_config(name: &str) -> Result<Config, AppError> {
  let config_file = determine_config(name);
  let reader = config_file.map(|f| BufReader::new(f));
  read_config(reader)
}
