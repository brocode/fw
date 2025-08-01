use crate::config;
use crate::config::project::Project;
use crate::errors::AppError;
use crate::spawn::spawn_maybe;

use std::borrow::ToOwned;
use std::env;
use yansi::Color;

pub fn gen_reworkon(maybe_config: Result<config::Config, AppError>) -> Result<(), AppError> {
	let config = maybe_config?;
	let project = current_project(&config)?;
	r#gen(&project.name, Ok(config), false)
}

fn current_project(config: &config::Config) -> Result<Project, AppError> {
	let os_current_dir = env::current_dir()?;
	let current_dir = os_current_dir.to_string_lossy().into_owned();
	let maybe_match = config
		.projects
		.values()
		.find(|&p| config.actual_path_to_project(p).to_string_lossy().eq(&current_dir));
	maybe_match
		.map(ToOwned::to_owned)
		.ok_or_else(|| AppError::UserError(format!("No project matching expanded path {current_dir} found in config")))
}

pub fn reworkon(maybe_config: Result<config::Config, AppError>) -> Result<(), AppError> {
	let config = maybe_config?;
	let project = current_project(&config)?;
	let path = config.actual_path_to_project(&project);
	let mut commands: Vec<String> = vec![format!("cd {}", path.to_string_lossy())];
	commands.extend_from_slice(&config.resolve_after_workon(&project));

	let shell = config.settings.get_shell_or_default();
	spawn_maybe(&shell, &commands.join(" && "), &path, &project.name, Color::Yellow)
}

pub fn r#gen(name: &str, maybe_config: Result<config::Config, AppError>, quick: bool) -> Result<(), AppError> {
	let config = maybe_config?;
	let project: &Project = config
		.projects
		.get(name)
		.ok_or_else(|| AppError::UserError(format!("project key {name} not found in fw.json")))?;
	let canonical_project_path = config.actual_path_to_project(project);
	let path = canonical_project_path
		.to_str()
		.ok_or(AppError::InternalError("project path is not valid unicode"))?;
	if !canonical_project_path.exists() {
		Err(AppError::UserError(format!("project key {name} found but path {path} does not exist")))
	} else {
		let mut commands: Vec<String> = vec![format!("cd '{}'", path)];
		if !quick {
			commands.extend_from_slice(&config.resolve_after_workon(project))
		}
		println!("{}", commands.join(" && "));
		Ok(())
	}
}
