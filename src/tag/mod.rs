use crate::config;
use crate::config::settings::Tag;
use crate::config::{Config, project::Project};
use crate::errors::AppError;
use crate::spawn::init_threads;
use crate::spawn::spawn_maybe;
use crate::util::random_color;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use yansi::Paint;

pub fn list_tags(maybe_config: Result<Config, AppError>, maybe_project_name: Option<String>) -> Result<(), AppError> {
	let config: Config = maybe_config?;
	if let Some(project_name) = maybe_project_name {
		list_project_tags(&config, &project_name)
	} else {
		list_all_tags(config);
		Ok(())
	}
}

pub fn delete_tag(maybe_config: Result<Config, AppError>, tag_name: &str) -> Result<(), AppError> {
	let config: Config = maybe_config?;
	let tags: BTreeMap<String, Tag> = config.settings.tags.unwrap_or_default();

	// remove tags from projects
	for mut project in config.projects.values().cloned() {
		let mut new_tags: BTreeSet<String> = project.tags.clone().unwrap_or_default();
		if new_tags.remove(tag_name) {
			project.tags = Some(new_tags);
			config::write_project(&project)?;
		}
	}

	if let Some(tag) = tags.get(tag_name) {
		config::delete_tag_config(tag_name, tag)
	} else {
		Ok(())
	}
}

fn list_all_tags(config: Config) {
	if let Some(tags) = config.settings.tags {
		for tag_name in tags.keys() {
			println!("{}", tag_name);
		}
	}
}

pub fn add_tag(config: &Config, project_name: String, tag_name: String) -> Result<(), AppError> {
	if let Some(mut project) = config.projects.get(&project_name).cloned() {
		let tags: BTreeMap<String, Tag> = config.settings.tags.clone().unwrap_or_default();
		if tags.contains_key(&tag_name) {
			let mut new_tags: BTreeSet<String> = project.tags.clone().unwrap_or_default();
			new_tags.insert(tag_name);
			project.tags = Some(new_tags);
			config::write_project(&project)?;
			Ok(())
		} else {
			Err(AppError::UserError(format!("Unknown tag {}", tag_name)))
		}
	} else {
		Err(AppError::UserError(format!("Unknown project {}", project_name)))
	}
}

pub fn create_tag(
	maybe_config: Result<Config, AppError>,
	tag_name: String,
	after_workon: Option<String>,
	after_clone: Option<String>,
	priority: Option<u8>,
	tag_workspace: Option<String>,
) -> Result<(), AppError> {
	let config: Config = maybe_config?;
	let tags: BTreeMap<String, Tag> = config.settings.tags.unwrap_or_default();

	if tags.contains_key(&tag_name) {
		Err(AppError::UserError(format!("Tag {} already exists, not gonna overwrite it for you", tag_name)))
	} else {
		let new_tag = Tag {
			after_clone,
			after_workon,
			priority,
			workspace: tag_workspace,
			default: None,
			tag_config_path: "default".to_string(),
		};
		config::write_tag(&tag_name, &new_tag)?;
		Ok(())
	}
}

pub fn inspect_tag(maybe_config: Result<Config, AppError>, tag_name: &str) -> Result<(), AppError> {
	let config: Config = maybe_config?;
	let tags: BTreeMap<String, Tag> = config.settings.tags.unwrap_or_default();
	if let Some(tag) = tags.get(tag_name) {
		println!("{}", Paint::new(tag_name).bold().underline());
		println!("{:<20}: {}", "config path", tag.tag_config_path);
		println!("{:<20}: {}", "after workon", tag.after_workon.clone().unwrap_or_default());
		println!("{:<20}: {}", "after clone", tag.after_clone.clone().unwrap_or_default());
		println!("{:<20}: {}", "priority", tag.priority.map(|n| n.to_string()).unwrap_or_default());
		println!("{:<20}: {}", "workspace", tag.workspace.clone().unwrap_or_default());
		println!("{:<20}: {}", "default", tag.default.map(|n| n.to_string()).unwrap_or_default());
		println!();
		println!("{}", Paint::new("projects".to_string()).bold().underline());
		for project in config.projects.values().cloned() {
			if project.tags.unwrap_or_default().contains(tag_name) {
				println!("{}", project.name)
			}
		}
		Ok(())
	} else {
		Err(AppError::UserError(format!("Unkown tag {}", tag_name)))
	}
}

pub fn remove_tag(maybe_config: Result<Config, AppError>, project_name: String, tag_name: &str) -> Result<(), AppError> {
	let config: Config = maybe_config?;

	if let Some(mut project) = config.projects.get(&project_name).cloned() {
		let mut new_tags: BTreeSet<String> = project.tags.clone().unwrap_or_default();
		if new_tags.remove(tag_name) {
			project.tags = Some(new_tags);
			config::write_project(&project)
		} else {
			Ok(())
		}
	} else {
		Err(AppError::UserError(format!("Unknown project {}", project_name)))
	}
}

fn list_project_tags(config: &Config, project_name: &str) -> Result<(), AppError> {
	if let Some(project) = config.projects.get(project_name) {
		if let Some(tags) = project.clone().tags {
			for tag_name in tags {
				println!("{}", tag_name);
			}
		}
		Ok(())
	} else {
		Err(AppError::UserError(format!("Unknown project {}", project_name)))
	}
}

pub fn autotag(maybe_config: Result<Config, AppError>, cmd: &str, tag_name: &str, parallel_raw: &Option<String>) -> Result<(), AppError> {
	let config = maybe_config?;

	let tags: BTreeMap<String, Tag> = config.settings.tags.clone().unwrap_or_default();
	if tags.contains_key(tag_name) {
		init_threads(parallel_raw)?;

		let projects: Vec<&Project> = config.projects.values().collect();

		let script_results = projects
			.par_iter()
			.map(|p| {
				let shell = config.settings.get_shell_or_default();
				let path = &config.actual_path_to_project(p);
				spawn_maybe(&shell, cmd, path, &p.name, random_color())
			})
			.collect::<Vec<Result<(), AppError>>>();

		// map with projects and filter if result == 0
		let filtered_projects: Vec<&Project> = script_results
			.into_iter()
			.zip(projects)
			.filter(|(x, _)| x.is_ok())
			.map(|(_, p)| p)
			.collect::<Vec<&Project>>();

		for project in filtered_projects.iter() {
			add_tag(&config, project.name.clone(), tag_name.to_string())?;
		}
		Ok(())
	} else {
		Err(AppError::UserError(format!("Unknown tag {}", tag_name)))
	}
}
