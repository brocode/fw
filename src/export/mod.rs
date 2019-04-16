use crate::config::{Config, Project};
use crate::errors::AppError;
use std::error::Error;

pub fn export_project(maybe_config: Result<Config, AppError>, name: &str) -> Result<(), AppError> {
  let config = maybe_config?;
  let project: &Project = config
    .projects
    .get(name)
    .ok_or_else(|| AppError::UserError(format!("project {} not found", name)))?;
  println!("{}", projects_to_shell_commands(&config, &[project])?);
  Ok(())
}

pub fn export_tagged_projects(maybe_config: Result<Config, AppError>, tag_name: &str) -> Result<(), AppError> {
  let config = maybe_config?;
  let mut projects: Vec<&Project> = Vec::new();

  for (_project_name, project) in config.projects.iter() {
    if let Some(ref project_tags) = project.tags {
      if project_tags.contains(tag_name) {
        projects.push(&project);
      }
    }
  }

  println!("{}", projects_to_shell_commands(&config, &projects)?);

  Ok(())
}

pub fn export_tag(maybe_config: Result<Config, AppError>, tag_name: &str) -> Result<(), AppError> {
  let config = maybe_config?;
  println!("{}", tag_to_shell_commands(tag_name, &config)?);
  Ok(())
}

fn projects_to_shell_commands(config: &Config, projects: &[&Project]) -> Result<String, AppError> {
  fn push_update(commands: &mut Vec<String>, parameter_name: &str, maybe_value: &Option<String>, project_name: &str) {
    if let Some(ref value) = *maybe_value {
      let mut value_string = value.to_string();
      value_string = value_string.replace("'", "'\\''");
      commands.push(format!("fw update {} --{} '{}'", project_name, parameter_name, value_string))
    }
  }

  let mut project_commands: Vec<String> = Vec::new();
  let mut tag_commands: Vec<String> = Vec::new();
  tag_commands.push("# fw export projects".to_owned());

  for project in projects {
    project_commands.push(format!("fw add {} {}", project.git, project.name));
    push_update(&mut project_commands, "override-path", &project.override_path, &project.name);
    push_update(&mut project_commands, "after-workon", &project.after_workon, &project.name);
    push_update(&mut project_commands, "after-clone", &project.after_clone, &project.name);

    if let Some(ref tags) = project.tags {
      for tag in tags {
        match tag_to_shell_commands(tag, config) {
          Ok(commands) => tag_commands.push(commands),
          Err(e) => tag_commands.push(format!("# Error exporting tag: {}", e.description())),
        }
        project_commands.push(format!("fw tag tag-project {} {}", project.name, tag));
      }
    }
    if let Some(ref additional_remotes) = project.additional_remotes {
      for remote in additional_remotes {
        project_commands.push(format!("fw add-remote {} {} {}", project.name, remote.name, remote.git));
      }
    }
  }

  tag_commands.sort_unstable();
  tag_commands.dedup();

  tag_commands.append(&mut project_commands);

  Ok(tag_commands.join("\n") + "\n")
}

fn tag_to_shell_commands(tag_name: &str, config: &Config) -> Result<String, AppError> {
  if let Some(ref tags) = config.settings.tags {
    if let Some(tag) = tags.get(tag_name) {
      let after_workon = tag
        .after_workon
        .clone()
        .map(|a| format!(" --after-workon=\'{}\'", a))
        .unwrap_or_else(|| "".to_string());
      let after_clone = tag
        .after_clone
        .clone()
        .map(|a| format!(" --after-clone=\'{}\'", a))
        .unwrap_or_else(|| "".to_string());
      let priority = tag.priority.map(|p| format!(" --priority=\'{}\'", p)).unwrap_or_else(|| "".to_string());
      let workspace = tag
        .workspace
        .clone()
        .map(|p| format!(" --workspace=\'{}\'", p))
        .unwrap_or_else(|| "".to_string());
      Ok(format!("fw tag add {}{}{}{}{}", tag_name, after_workon, after_clone, priority, workspace))
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
  use crate::config::*;
  use maplit::btreeset;
  use spectral::prelude::*;
  use std::collections::BTreeMap;

  #[test]
  fn test_workon_override_from_project() {
    let config = a_config();
    let exported_command = projects_to_shell_commands(&config, &[config.projects.get("test1").unwrap()]).expect("Should work");
    assert_that(&exported_command).is_equal_to(
      "# Error exporting tag: Unknown tag unknown_tag
# fw export projects
fw tag add tag1 --after-workon=\'workon1\' --after-clone=\'clone1\' --priority=\'10\'
fw tag add tag2 --after-workon=\'workon2\' --after-clone=\'clone2\' --priority=\'10\'
fw add git@github.com:codingberlin/why-i-suck.git why-i-suck
fw update why-i-suck --override-path \'/home/bauer/docs/why-i-suck\'
fw update why-i-suck --after-workon \'echo test\'\\'\'s\'
fw update why-i-suck --after-clone \'echo 1\'
fw tag tag-project why-i-suck tag1
fw tag tag-project why-i-suck tag2
fw tag tag-project why-i-suck unknown_tag
"
      .to_owned(),
    );
  }

  fn a_config() -> Config {
    let project = Project {
      name: "why-i-suck".to_owned(),
      git: "git@github.com:codingberlin/why-i-suck.git".to_owned(),
      tags: Some(btreeset!["tag1".to_owned(), "tag2".to_owned(), "unknown_tag".to_owned(),]),
      after_clone: Some("echo 1".to_owned()),
      after_workon: Some("echo test's".to_owned()),
      override_path: Some("/home/bauer/docs/why-i-suck".to_string()),
      additional_remotes: None,
      bare: None,
    };
    let tag1 = Tag {
      after_clone: Some("clone1".to_owned()),
      after_workon: Some("workon1".to_owned()),
      priority: Some(10),
      workspace: None,
      default: None,
    };
    let tag2 = Tag {
      after_clone: Some("clone2".to_owned()),
      after_workon: Some("workon2".to_owned()),
      priority: Some(10),
      workspace: None,
      default: None,
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
      gitlab: None,
    };
    Config { projects, settings }
  }
}
