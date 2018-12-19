use crate::config;
use crate::config::Project;
use crate::errors::*;
use crate::sync;
use ansi_term::Colour;
use ansi_term::Style;
use serde_json;
use slog::debug;
use slog::Logger;
use std::env;

pub fn ls(maybe_config: Result<config::Config>) -> Result<()> {
  let config = maybe_config?;
  for (name, _) in config.projects {
    println!("{}", name)
  }
  Ok(())
}

pub fn print_path(maybe_config: Result<config::Config>, name: &str, logger: &Logger) -> Result<()> {
  let config = maybe_config?;
  let project = config
    .projects
    .get(name)
    .chain_err(|| ErrorKind::UserError(format!("project {} not found", name)))?;
  let canonical_project_path = config.actual_path_to_project(project, logger);
  let path = canonical_project_path
    .to_str()
    .chain_err(|| ErrorKind::InternalError("project path is not valid unicode".to_string()))?;
  println!("{}", path);
  Ok(())
}

pub fn inspect(name: &str, maybe_config: Result<config::Config>, json: bool, logger: &Logger) -> Result<()> {
  let config = maybe_config?;
  let project = config
    .projects
    .get(name)
    .chain_err(|| ErrorKind::UserError(format!("project {} not found", name)))?;
  if json {
    println!("{}", serde_json::to_string(project)?);
    return Ok(());
  }
  let canonical_project_path = config.actual_path_to_project(project, logger);
  let path = canonical_project_path
    .to_str()
    .chain_err(|| ErrorKind::InternalError("project path is not valid unicode".to_string()))?;
  println!("{}", Style::new().underline().bold().paint(project.name.clone()));
  println!("{:<20}: {}", "Path", path);
  let tags = project
    .tags
    .clone()
    .map(|t| {
      let project_tags: Vec<String> = t.into_iter().collect();
      project_tags.join(", ")
    })
    .unwrap_or_else(|| "None".to_owned());
  println!("{:<20}: {}", "Tags", tags);
  let git = project.git.clone();
  println!("{:<20}: {}", "Git", git);
  Ok(())
}

pub fn gen_reworkon(maybe_config: Result<config::Config>, logger: &Logger) -> Result<()> {
  let config = maybe_config?;
  let project = current_project(&config, logger)?;
  gen(&project.name, Ok(config), false, logger)
}

fn current_project(config: &config::Config, logger: &Logger) -> Result<Project> {
  let os_current_dir = env::current_dir()?;
  let current_dir = os_current_dir.to_string_lossy().to_owned();
  let maybe_match = config
    .projects
    .values()
    .find(|&p| config.actual_path_to_project(p, logger).to_string_lossy().eq(&current_dir));
  maybe_match
    .map(|p| p.to_owned())
    .chain_err(|| ErrorKind::UserError(format!("No project matching expanded path {} found in config", current_dir)))
}

pub fn reworkon(maybe_config: Result<config::Config>, logger: &Logger) -> Result<()> {
  let config = maybe_config?;
  let project = current_project(&config, logger)?;
  let path = config.actual_path_to_project(&project, logger);
  let mut commands: Vec<String> = vec![];
  commands.push(format!("cd {}", path.to_string_lossy()));
  commands.extend_from_slice(&config.resolve_after_workon(logger, &project));

  debug!(logger, "Reworkon match: {:?} with command {:?}", project, commands);
  let shell = sync::project_shell(&config.settings);
  sync::spawn_maybe(&shell, &commands.join(" && "), &path, &project.name, Colour::Yellow, logger)
}

pub fn gen(name: &str, maybe_config: Result<config::Config>, quick: bool, logger: &Logger) -> Result<()> {
  let config = maybe_config?;
  let project: &Project = config
    .projects
    .get(name)
    .chain_err(|| ErrorKind::UserError(format!("project key {} not found in fw.json", name)))?;
  let canonical_project_path = config.actual_path_to_project(project, logger);
  let path = canonical_project_path
    .to_str()
    .chain_err(|| ErrorKind::InternalError("project path is not valid unicode".to_string()))?;
  if !canonical_project_path.exists() {
    Err(ErrorKind::UserError(format!("project key {} found but path {} does not exist", name, path)).into())
  } else {
    let mut commands: Vec<String> = vec![];
    commands.push(format!("cd {}", path));
    if !quick {
      commands.extend_from_slice(&config.resolve_after_workon(logger, project))
    }
    println!("{}", commands.join(" && "));
    Ok(())
  }
}
