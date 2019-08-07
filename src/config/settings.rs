use std::collections::{BTreeMap, BTreeSet};

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

impl Tag {
  pub fn example() -> Tag {
    Tag {
      after_clone: Some("echo after clone from tag".to_owned()),
      after_workon: Some("echo after workon from tag".to_owned()),
      priority: Some(0),
      workspace: Some("/home/other".to_string()),
      default: Some(false),
      tag_config_path: "".to_string(), // ignored
    }
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GitlabSettings {
  pub token: String,
  pub host: String,
}

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

impl Settings {
  pub fn get_shell_or_default(self: &Settings) -> Vec<String> {
    self.shell.clone().unwrap_or_else(|| vec!["sh".to_owned(), "-c".to_owned()])
  }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PersistedSettings {
  pub workspace: String,
  pub shell: Option<Vec<String>>,
  pub default_after_workon: Option<String>,
  pub default_after_clone: Option<String>,
  pub github_token: Option<String>,
  pub gitlab: Option<GitlabSettings>,
}

impl PersistedSettings {
  pub fn example() -> PersistedSettings {
    PersistedSettings {
      workspace: "~/workspace".to_owned(),
      default_after_workon: Some("echo default after workon".to_string()),
      default_after_clone: Some("echo default after clone".to_string()),
      shell: Some(vec!["/usr/bin/zsh".to_string(), "-c".to_string()]),
      github_token: Some("githubtokensecret".to_string()),
      gitlab: Some(GitlabSettings {
        host: "localhost".to_string(),
        token: "token".to_string(),
      }),
    }
  }
}
