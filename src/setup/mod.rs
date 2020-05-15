use crate::config::{self, project::Project, settings::Settings, Config};
use crate::errors::AppError;
use crate::ws::github;
use git2::Repository;
use slog::Logger;
use slog::{debug, info, o, warn};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::iter::Iterator;
use std::path::{Path, PathBuf};

pub fn setup(workspace_dir: &str, logger: &Logger) -> Result<(), AppError> {
  let setup_logger = logger.new(o!("workspace" => workspace_dir.to_owned()));
  debug!(setup_logger, "Entering setup");
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
    .and_then(|path| determine_projects(path, logger))
    .and_then(|projects| write_new_config_with_projects(projects, logger, workspace_dir))
}

fn determine_projects(path: PathBuf, logger: &Logger) -> Result<BTreeMap<String, Project>, AppError> {
  let workspace_path = path.clone();

  let project_entries: Vec<fs::DirEntry> = fs::read_dir(path).and_then(Iterator::collect).map_err(AppError::IO)?;

  let mut projects: BTreeMap<String, Project> = BTreeMap::new();
  for entry in project_entries {
    let path = entry.path();
    if path.is_dir() {
      match entry.file_name().into_string() {
        Ok(name) => {
          let mut path_to_repo = workspace_path.clone();
          path_to_repo.push(&name);
          match load_project(None, path_to_repo, &name, logger) {
            Ok(project) => {
              projects.insert(project.name.clone(), project);
            }
            Err(e) => warn!(logger, "Error while importing folder. Skipping it."; "entry" => name, "error" => format!("{}", e)),
          }
        }
        Err(_) => warn!(logger, "Failed to parse directory name as unicode. Skipping it."),
      }
    }
  }

  Ok(projects)
}

pub fn gitlab_import(maybe_config: Result<Config, AppError>, logger: &Logger) -> Result<(), AppError> {
  use gitlab::api::Query;
  let current_config = maybe_config?;

  let gitlab_config = current_config.settings.gitlab.clone().ok_or_else(|| {
    AppError::UserError(
      r#"Can't call Gitlab API, because no gitlab settings ("gitlab": { "token": "some-token", "url": "some-url" }) specified in the configuration."#
        .to_string(),
    )
  })?;

  let gitlab_client =
    gitlab::Gitlab::new(gitlab_config.host, gitlab_config.token).map_err(|e| AppError::RuntimeError(format!("Failed to create gitlab client: {}", e)))?;

  // owned repos and your organizations repositories
  let owned_projects: Vec<gitlab::Project> = gitlab::api::paged(
    gitlab::api::projects::Projects::builder().owned(true).build().unwrap(),
    gitlab::api::Pagination::All,
  )
  .query(&gitlab_client)
  .map_err(|e| AppError::RuntimeError(format!("Failed to query gitlab: {}", e)))?;

  let names_and_urls: Vec<(String, String)> = owned_projects
    .iter()
    .map(|repo| (repo.name.to_owned(), repo.ssh_url_to_repo.to_owned()))
    .collect();

  let after_clone = current_config.settings.default_after_clone.clone();
  let after_workon = current_config.settings.default_after_workon.clone();
  let tags = current_config.settings.default_tags.clone();
  let mut current_projects = current_config.projects;

  for (name, url) in names_and_urls {
    let p = Project {
      name,
      git: url,
      after_clone: after_clone.clone(),
      after_workon: after_workon.clone(),
      override_path: None,
      tags: tags.clone(),
      additional_remotes: None,
      bare: None,
      project_config_path: "gitlab".to_string(),
    };

    if current_projects.contains_key(&p.name) {
      info!(
        logger,
          "Skipping new project from Gitlab import because it already exists in the current fw config"; "project_name" => &p.name);
    } else {
      info!(logger, "Saving new project"; "project_name" => &p.name);
      config::write_project(&p)?; // TODO not sure if this should be default or gitlab subfolder? or even user specified?
      current_projects.insert(p.name.clone(), p); // to ensure no duplicated name encountered during processing
    }
  }

  Ok(())
}

pub fn org_import(maybe_config: Result<Config, AppError>, org_name: &str, include_archived: bool, logger: &Logger) -> Result<(), AppError> {
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
      project_config_path: org_name.to_string(),
    };

    if current_projects.contains_key(&p.name) {
      info!(
        logger,
          "Skipping new project from Github import because it already exists in the current fw config"; "project_name" => &p.name);
    } else {
      info!(logger, "Saving new project"; "project_name" => &p.name);
      config::write_project(&p)?;
      current_projects.insert(p.name.clone(), p); // to ensure no duplicated name encountered during processing
    }
  }
  Ok(())
}

pub fn import(maybe_config: Result<Config, AppError>, path: &str, logger: &Logger) -> Result<(), AppError> {
  let path = fs::canonicalize(Path::new(path))?;
  let project_path = path.to_str().ok_or(AppError::InternalError("project path is not valid unicode"))?.to_owned();
  let file_name = AppError::require(path.file_name(), AppError::UserError("Import path needs to be valid".to_string()))?;
  let project_name: String = file_name.to_string_lossy().into_owned();
  let maybe_settings = maybe_config.ok().map(|c| c.settings);
  let new_project = load_project(maybe_settings, path.clone(), &project_name, logger)?;
  let new_project_with_path = Project {
    override_path: Some(project_path),
    ..new_project
  };
  config::write_project(&new_project_with_path)?;
  Ok(())
}

fn load_project(maybe_settings: Option<Settings>, path_to_repo: PathBuf, name: &str, logger: &Logger) -> Result<Project, AppError> {
  let project_logger = logger.new(o!("project" => name.to_string()));
  let repo: Repository = Repository::open(path_to_repo)?;
  let all = repo.remotes()?;
  debug!(project_logger, "remotes"; "found" => format!("{:?}", all.len()));
  let remote = repo.find_remote("origin")?;
  let url = remote
    .url()
    .ok_or_else(|| AppError::UserError(format!("invalid remote origin at {:?}", repo.path())))?;
  info!(project_logger, "git config validated");
  Ok(Project {
    name: name.to_owned(),
    git: url.to_owned(),
    after_clone: maybe_settings.clone().and_then(|s| s.default_after_clone),
    after_workon: maybe_settings.clone().and_then(|s| s.default_after_workon),
    override_path: None,
    additional_remotes: None, // TODO: use remotes
    tags: maybe_settings.and_then(|s| s.default_tags),
    bare: None,
    project_config_path: "default".to_string(),
  })
}

fn write_new_config_with_projects(projects: BTreeMap<String, Project>, logger: &Logger, workspace_dir: &str) -> Result<(), AppError> {
  let settings: config::settings::PersistedSettings = config::settings::PersistedSettings {
    workspace: workspace_dir.to_owned(),
    default_after_workon: None,
    default_after_clone: None,
    shell: None,
    github_token: None,
    gitlab: None,
  };
  config::write_settings(&settings, &logger)?;
  for p in projects.values() {
    config::write_project(&p)?;
  }
  debug!(logger, "Finished"; "projects" => format!("{:?}", projects.len()));
  Ok(())
}
