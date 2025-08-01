use crate::config;
use crate::config::Config;
use crate::config::{project::Project, project::Remote};
use crate::errors::AppError;
use crate::git::repo_name_from_url;
use std::collections::BTreeSet;
use std::{fs, path};
use yansi::Paint;

pub fn add_entry(
	maybe_config: Result<Config, AppError>,
	maybe_name: Option<String>,
	url: &str,
	after_workon: Option<String>,
	after_clone: Option<String>,
	override_path: Option<String>,
	tags: Option<BTreeSet<String>>,
	trusted: bool,
) -> Result<(), AppError> {
	let name = maybe_name
		.ok_or_else(|| AppError::UserError(format!("No project name specified for {url}")))
		.or_else(|_| repo_name_from_url(url).map(ToOwned::to_owned))?;
	let config: Config = maybe_config?;
	if config.projects.contains_key(&name) {
		Err(AppError::UserError(format!(
			"Project key {name} already exists, not gonna overwrite it for you"
		)))
	} else {
		let default_after_clone = config.settings.default_after_clone.clone();
		let default_after_workon = config.settings.default_after_workon.clone();

		let project_tags: Option<BTreeSet<String>> = if tags.is_some() && config.settings.default_tags.is_some() {
			tags.zip(config.settings.default_tags).map(|(t1, t2)| t1.union(&t2).cloned().collect())
		} else {
			tags.or(config.settings.default_tags)
		};

		config::write_project(&Project {
			git: url.to_owned(),
			name,
			after_clone: after_clone.or(default_after_clone),
			after_workon: after_workon.or(default_after_workon),
			override_path,
			tags: project_tags,
			bare: None,
			additional_remotes: None,
			trusted,
			project_config_path: "default".to_string(),
		})?;
		Ok(())
	}
}

pub fn remove_project(maybe_config: Result<Config, AppError>, project_name: &str, purge_directory: bool) -> Result<(), AppError> {
	let config: Config = maybe_config?;

	if !config.projects.contains_key(project_name) {
		Err(AppError::UserError(format!("Project key {project_name} does not exist in config")))
	} else if let Some(project) = config.projects.get(project_name).cloned() {
		if purge_directory {
			let path = config.actual_path_to_project(&project);

			if path.exists() {
				fs::remove_dir_all(&path)?;
			}
		}
		config::delete_project_config(&project)
	} else {
		Err(AppError::UserError(format!("Unknown project {project_name}")))
	}
}

pub fn add_remote(maybe_config: Result<Config, AppError>, name: &str, remote_name: String, git: String) -> Result<(), AppError> {
	let config: Config = maybe_config?;
	if !config.projects.contains_key(name) {
		return Err(AppError::UserError(format!("Project key {name} does not exists. Can not update.")));
	}
	let mut project_config: Project = config.projects.get(name).expect("Already checked in the if above").clone();
	let mut additional_remotes = project_config.additional_remotes.unwrap_or_default();
	if additional_remotes.iter().any(|r| r.name == remote_name) {
		return Err(AppError::UserError(format!(
			"Remote {remote_name} for project {name} does already exist. Can not add."
		)));
	}
	additional_remotes.push(Remote { name: remote_name, git });
	project_config.additional_remotes = Some(additional_remotes);

	config::write_project(&project_config)?;
	Ok(())
}

pub fn remove_remote(maybe_config: Result<Config, AppError>, name: &str, remote_name: String) -> Result<(), AppError> {
	let config: Config = maybe_config?;
	if !config.projects.contains_key(name) {
		return Err(AppError::UserError(format!("Project key {name} does not exists. Can not update.")));
	}
	let mut project_config: Project = config.projects.get(name).expect("Already checked in the if above").clone();
	let additional_remotes = project_config.additional_remotes.unwrap_or_default();
	let additional_remotes = additional_remotes.into_iter().filter(|r| r.name != remote_name).collect();
	project_config.additional_remotes = Some(additional_remotes);

	config::write_project(&project_config)?;
	Ok(())
}

pub fn update_entry(
	maybe_config: Result<Config, AppError>,
	name: &str,
	git: Option<String>,
	after_workon: Option<String>,
	after_clone: Option<String>,
	override_path: Option<String>,
) -> Result<(), AppError> {
	let config: Config = maybe_config?;
	if name.starts_with("http") || name.starts_with("git@") {
		Err(AppError::UserError(format!(
			"{name} looks like a repo URL and not like a project name, please fix"
		)))
	} else if !config.projects.contains_key(name) {
		Err(AppError::UserError(format!("Project key {name} does not exists. Can not update.")))
	} else {
		let old_project_config: Project = config.projects.get(name).expect("Already checked in the if above").clone();
		config::write_project(&Project {
			git: git.unwrap_or(old_project_config.git),
			name: old_project_config.name,
			after_clone: after_clone.or(old_project_config.after_clone),
			after_workon: after_workon.or(old_project_config.after_workon),
			override_path: override_path.or(old_project_config.override_path),
			tags: old_project_config.tags,
			bare: old_project_config.bare,
			trusted: old_project_config.trusted,
			additional_remotes: old_project_config.additional_remotes,
			project_config_path: old_project_config.project_config_path,
		})?;
		Ok(())
	}
}

pub fn ls(maybe_config: Result<Config, AppError>, tags: &BTreeSet<String>) -> Result<(), AppError> {
	let config = maybe_config?;
	for (name, project) in config.projects {
		if tags.is_empty() || project.tags.unwrap_or_default().intersection(tags).count() > 0 {
			println!("{name}")
		}
	}
	Ok(())
}

pub fn print_path(maybe_config: Result<Config, AppError>, name: &str) -> Result<(), AppError> {
	let config = maybe_config?;
	let project = config
		.projects
		.get(name)
		.ok_or_else(|| AppError::UserError(format!("project {name} not found")))?;
	let canonical_project_path = config.actual_path_to_project(project);
	let path = canonical_project_path
		.to_str()
		.ok_or(AppError::InternalError("project path is not valid unicode"))?;
	println!("{path}");
	Ok(())
}

pub fn inspect(name: &str, maybe_config: Result<Config, AppError>, json: bool) -> Result<(), AppError> {
	let config = maybe_config?;
	let project = config
		.projects
		.get(name)
		.ok_or_else(|| AppError::UserError(format!("project {name} not found")))?;
	if json {
		println!("{}", serde_json::to_string(project)?);
		return Ok(());
	}
	let canonical_project_path = config.actual_path_to_project(project);
	let path = canonical_project_path
		.to_str()
		.ok_or(AppError::InternalError("project path is not valid unicode"))?;
	println!("{}", Paint::new(project.name.to_owned()).bold().underline());
	println!("{:<20}: {}", "Path", path);
	println!("{:<20}: {}", "config path", project.project_config_path);
	let tags = project
		.tags
		.clone()
		.map(|t| {
			let project_tags: Vec<String> = t.into_iter().collect();
			project_tags.join(", ")
		})
		.unwrap_or_else(|| "None".to_owned());
	println!("{:<20}: {}", "Tags", tags);
	let additional_remotes = project
		.additional_remotes
		.clone()
		.map(|t| {
			let project_tags: Vec<String> = t.into_iter().map(|r| format!("{} - {}", r.name, r.git)).collect();
			project_tags.join(", ")
		})
		.unwrap_or_else(|| "None".to_owned());
	println!("{:<20}: {}", "Additional remotes", additional_remotes);
	let git = project.git.clone();
	println!("{:<20}: {}", "Git", git);
	Ok(())
}

pub(crate) fn move_project(maybe_config: Result<Config, AppError>, name: &str, destination: &str) -> Result<(), AppError> {
	let config = maybe_config?;
	let project = config
		.projects
		.get(name)
		.ok_or_else(|| AppError::UserError(format!("project {name} not found")))?;
	let canonical_project_path = config.actual_path_to_project(project);

	let mut path = path::absolute(destination)?;

	fs::rename(canonical_project_path, path.clone())?;

	path = fs::canonicalize(path)?;
	let project_path = path.to_str().ok_or(AppError::InternalError("project path is not valid unicode"))?.to_owned();
	update_entry(Ok(config), name, None, None, None, Some(project_path))?;
	Ok(())
}
