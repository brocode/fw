use config::{Config, Project};
use errors::AppError;
use std::error::Error;

pub fn export_project(maybe_config: Result<Config, AppError>, name: &str) -> Result<(), AppError> {
  let config = maybe_config?;
  let project: &Project = config
    .projects
    .get(name)
    .ok_or_else(|| AppError::UserError(format!("project {} not found", name)))?;
  println!("{}", project_to_shell_commands(&config, project)?);
  Ok(())
}

fn project_to_shell_commands(config: &Config, project: &Project) -> Result<String, AppError> {
  fn push_update(commands: &mut Vec<String>, parameter_name: &str, maybe_value: &Option<String>, project_name: &str) {
    if let Some(ref value) = *maybe_value {
      commands.push(format!(
        "fw update {} --{} \"{}\"",
        project_name, parameter_name, value
      ))
    }
  }

  let mut shell_commands: Vec<String> = Vec::new();
  shell_commands.push("# fw export".to_owned());

  shell_commands.push(format!("fw add {} {}", project.git, project.name));
  push_update(
    &mut shell_commands,
    "override-path",
    &project.override_path,
    &project.name,
  );
  push_update(
    &mut shell_commands,
    "after-workon",
    &project.after_workon,
    &project.name,
  );
  push_update(
    &mut shell_commands,
    "after-clone",
    &project.after_clone,
    &project.name,
  );

  if let Some(ref tags) = project.tags {
    for tag in tags {
      match tag_to_shell_commands(tag, config) {
        Ok(commands) => shell_commands.push(commands),
        Err(e) => shell_commands.push(format!("# Error exporting tag: {}", e.description())),
      }
      shell_commands.push(format!("fw tag tag-project {} {}", project.name, tag));
    }
  }

  Ok(shell_commands.join("\n") + "\n")
}

fn tag_to_shell_commands(tag_name: &str, config: &Config) -> Result<String, AppError> {
  if let Some(ref tags) = config.settings.tags {
    if let Some(tag) = tags.get(tag_name) {
      let after_workon = tag
        .after_workon
        .clone()
        .map(|a| format!(" --after-workon=\"{}\"", a))
        .unwrap_or_else(|| "".to_string());
      let after_clone = tag
        .after_clone
        .clone()
        .map(|a| format!(" --after-clone=\"{}\"", a))
        .unwrap_or_else(|| "".to_string());
      let priority = tag
        .priority
        .map(|p| format!(" --priority=\"{}\"", p))
        .unwrap_or_else(|| "".to_string());
      let workspace = tag
        .workspace
        .clone()
        .map(|p| format!(" --workspace=\"{}\"", p))
        .unwrap_or_else(|| "".to_string());
      Ok(format!(
        "fw tag add {}{}{}{}{}",
        tag_name, after_workon, after_clone, priority, workspace
      ))
    } else {
      Result::Err(AppError::UserError(format!("Unknown tag {}", tag_name)))
    }
  } else {
    Result::Err(AppError::UserError("No tags configured".to_string()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use config::*;
  use spectral::prelude::*;
  use std::collections::BTreeMap;

  #[test]
  fn test_workon_override_from_project() {
    let config = a_config();
    let exported_command = project_to_shell_commands(&config, config.projects.get("test1").unwrap()).expect("Should work");
    assert_that(&exported_command).is_equal_to(
      "# fw export
fw add git@github.com:codingberlin/why-i-suck.git why-i-suck
fw update why-i-suck --override-path \"/home/bauer/docs/why-i-suck\"
fw update why-i-suck --after-workon \"echo 2\"
fw update why-i-suck --after-clone \"echo 1\"
fw tag add tag1 --after-workon=\"workon1\" --after-clone=\"clone1\" --priority=\"10\"
fw tag tag-project why-i-suck tag1
fw tag add tag2 --after-workon=\"workon2\" --after-clone=\"clone2\" --priority=\"10\"
fw tag tag-project why-i-suck tag2
# Error exporting tag: Unknown tag unknown_tag
fw tag tag-project why-i-suck unknown_tag
"
        .to_owned(),
    );
  }

  fn a_config() -> Config {
    let project = Project {
      name: "why-i-suck".to_owned(),
      git: "git@github.com:codingberlin/why-i-suck.git".to_owned(),
      tags: Some(btreeset![
        "tag1".to_owned(),
        "tag2".to_owned(),
        "unknown_tag".to_owned(),
      ]),
      after_clone: Some("echo 1".to_owned()),
      after_workon: Some("echo 2".to_owned()),
      override_path: Some("/home/bauer/docs/why-i-suck".to_string()),
      bare: None,
    };
    let tag1 = Tag {
      after_clone: Some("clone1".to_owned()),
      after_workon: Some("workon1".to_owned()),
      priority: Some(10),
      workspace: None,
    };
    let tag2 = Tag {
      after_clone: Some("clone2".to_owned()),
      after_workon: Some("workon2".to_owned()),
      priority: Some(10),
      workspace: None,
    };
    let mut projects: BTreeMap<String, Project> = BTreeMap::new();
    projects.insert("test1".to_owned(), project);
    let mut tags: BTreeMap<String, Tag> = BTreeMap::new();
    tags.insert("tag1".to_owned(), tag1);
    tags.insert("tag2".to_owned(), tag2);
    let settings = Settings {
      workspace: "/test".to_owned(),
      default_after_workon: None,
      default_after_clone: None,
      default_tags: None,
      shell: None,
      tags: Some(tags),
      github_token: None,
    };
    Config {
      projects: projects,
      settings: settings,
    }
  }
}
