use crate::config::{self, Config, project::Project, settings::Settings};
use crate::errors::AppError;
use crate::ws::github;
use clap::builder::PossibleValue;
use git2::Repository;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::iter::Iterator;
use std::path::{Path, PathBuf};

#[derive(Copy, Clone)]
pub enum ProjectState {
	Active,
	Archived,
	Both,
}

impl clap::ValueEnum for ProjectState {
	fn value_variants<'a>() -> &'a [Self] {
		&[Self::Active, Self::Archived, Self::Both]
	}

	fn to_possible_value(&self) -> Option<PossibleValue> {
		match self {
			Self::Active => Some(PossibleValue::new("active")),
			Self::Archived => Some(PossibleValue::new("archived")),
			Self::Both => Some(PossibleValue::new("both")),
		}
	}
}

impl std::str::FromStr for ProjectState {
	type Err = AppError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"active" => Ok(Self::Active),
			"archived" => Ok(Self::Archived),
			"both" => Ok(Self::Both),
			_ => Err(AppError::InternalError("invalid value for ProjectState")), // TODO should this be unreachable?,
		}
	}
}

pub fn setup(workspace_dir: &str) -> Result<(), AppError> {
	let path = PathBuf::from(workspace_dir);
	let maybe_path = if path.exists() {
		Ok(path)
	} else {
		Err(AppError::UserError(format!("Given workspace path {} does not exist", workspace_dir)))
	};

	maybe_path
		.and_then(|path| {
			if path.is_absolute() {
				Ok(path)
			} else {
				Err(AppError::UserError(format!("Workspace path {} needs to be absolute", workspace_dir)))
			}
		})
		.and_then(determine_projects)
		.and_then(|projects| write_new_config_with_projects(projects, workspace_dir))
}

fn determine_projects(path: PathBuf) -> Result<BTreeMap<String, Project>, AppError> {
	let workspace_path = path.clone();

	let project_entries: Vec<fs::DirEntry> = fs::read_dir(path).and_then(Iterator::collect).map_err(AppError::Io)?;

	let mut projects: BTreeMap<String, Project> = BTreeMap::new();
	for entry in project_entries {
		let path = entry.path();
		if path.is_dir() {
			match entry.file_name().into_string() {
				Ok(name) => {
					let mut path_to_repo = workspace_path.clone();
					path_to_repo.push(&name);
					match load_project(None, path_to_repo, &name) {
						Ok(project) => {
							projects.insert(project.name.clone(), project);
						}
						Err(e) => eprintln!("Error while importing folder. Skipping it. {}", e),
					}
				}
				Err(_) => eprintln!("Failed to parse directory name as unicode. Skipping it."),
			}
		}
	}

	Ok(projects)
}

pub fn org_import(maybe_config: Result<Config, AppError>, org_name: &str, include_archived: bool) -> Result<(), AppError> {
	let current_config = maybe_config?;
	let token = env::var_os("FW_GITHUB_TOKEN")
		.map(|s| s.to_string_lossy().to_string())
		.or_else(|| current_config.settings.github_token.clone())
		.ok_or_else(|| {
			AppError::UserError(format!(
				"Can't call GitHub API for org {} because no github oauth token (settings.github_token) specified in the configuration.",
				org_name
			))
		})?;
	let mut api = github::github_api(&token)?;
	let org_repository_names: Vec<String> = api.list_repositories(org_name, include_archived)?;
	let after_clone = current_config.settings.default_after_clone.clone();
	let after_workon = current_config.settings.default_after_workon.clone();
	let tags = current_config.settings.default_tags.clone();
	let mut current_projects = current_config.projects;

	for name in org_repository_names {
		let p = Project {
			name: name.clone(),
			git: format!("git@github.com:{}/{}.git", org_name, name),
			after_clone: after_clone.clone(),
			after_workon: after_workon.clone(),
			override_path: None,
			tags: tags.clone(),
			additional_remotes: None,
			bare: None,
			trusted: false,
			project_config_path: org_name.to_string(),
		};

		if current_projects.contains_key(&p.name) {
			//     "Skipping new project from Github import because it already exists in the current fw config
		} else {
			config::write_project(&p)?;
			current_projects.insert(p.name.clone(), p); // to ensure no duplicated name encountered during processing
		}
	}
	Ok(())
}

pub fn import(maybe_config: Result<Config, AppError>, path: &str) -> Result<(), AppError> {
	let path = fs::canonicalize(Path::new(path))?;
	let project_path = path.to_str().ok_or(AppError::InternalError("project path is not valid unicode"))?.to_owned();
	let file_name = AppError::require(path.file_name(), AppError::UserError("Import path needs to be valid".to_string()))?;
	let project_name: String = file_name.to_string_lossy().into_owned();
	let maybe_settings = maybe_config.ok().map(|c| c.settings);
	let new_project = load_project(maybe_settings, path.clone(), &project_name)?;
	let new_project_with_path = Project {
		override_path: Some(project_path),
		..new_project
	};
	config::write_project(&new_project_with_path)?;
	Ok(())
}

fn load_project(maybe_settings: Option<Settings>, path_to_repo: PathBuf, name: &str) -> Result<Project, AppError> {
	let repo: Repository = Repository::open(path_to_repo)?;
	let remote = repo.find_remote("origin")?;
	let url = remote
		.url()
		.ok_or_else(|| AppError::UserError(format!("invalid remote origin at {:?}", repo.path())))?;
	Ok(Project {
		name: name.to_owned(),
		git: url.to_owned(),
		after_clone: maybe_settings.clone().and_then(|s| s.default_after_clone),
		after_workon: maybe_settings.clone().and_then(|s| s.default_after_workon),
		override_path: None,
		additional_remotes: None, // TODO: use remotes
		tags: maybe_settings.and_then(|s| s.default_tags),
		bare: None,
		trusted: false,
		project_config_path: "default".to_string(),
	})
}

fn write_new_config_with_projects(projects: BTreeMap<String, Project>, workspace_dir: &str) -> Result<(), AppError> {
	let settings: config::settings::PersistedSettings = config::settings::PersistedSettings {
		workspace: workspace_dir.to_owned(),
		default_after_workon: None,
		default_after_clone: None,
		shell: None,
		github_token: None,
	};
	config::write_settings(&settings)?;
	for p in projects.values() {
		config::write_project(p)?;
	}
	Ok(())
}
