use maplit::btreeset;
use std::collections::BTreeSet;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Remote {
  pub name: String,
  pub git: String,
}

fn empty_string() -> String {
  "".to_string()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Project {
  #[serde(default = "empty_string", skip_serializing)]
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

impl Project {
  pub fn example() -> Project {
    Project {
      name: "fw".to_owned(),
      git: "git@github.com:brocode/fw.git".to_owned(),
      tags: Some(btreeset!["rust".to_owned(), "brocode".to_owned()]),
      after_clone: Some("echo BROCODE!!".to_string()),
      after_workon: Some("echo workon fw".to_string()),
      override_path: Some("/some/fancy/path/to/fw".to_string()),
      additional_remotes: Some(vec![Remote {
        name: "upstream".to_string(),
        git: "git@...".to_string(),
      }]),
      bare: Some(false),
      project_config_path: "".to_string(), // ignored
    }
  }
}
