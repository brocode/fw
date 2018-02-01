use config;
use config::{Config, Project, Settings};
use errors::AppError;
use git2::Repository;
use slog::Logger;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use ws::github;

pub fn setup(workspace_dir: &str, logger: &Logger) -> Result<(), AppError> {
  let setup_logger = logger.new(o!("workspace" => workspace_dir.to_owned()));
  debug!(setup_logger, "Entering setup");
  let path = PathBuf::from(workspace_dir);
  let maybe_path = if path.exists() {
    Result::Ok(path)
  } else {
    Result::Err(AppError::UserError(format!(
      "Given workspace path {} does not exist",
      workspace_dir
    )))
  };

  maybe_path
    .and_then(|path| {
      if path.is_absolute() {
        Ok(path)
      } else {
        Err(AppError::UserError(format!(
          "Workspace path {} needs to be absolute",
          workspace_dir
        )))
      }
    })
    .and_then(|path| determine_projects(path, logger))
    .and_then(|projects| write_config(projects, logger, workspace_dir))
}

fn determine_projects(path: PathBuf, logger: &Logger) -> Result<BTreeMap<String, Project>, AppError> {
  let workspace_path = path.clone();

  let project_entries: Vec<fs::DirEntry> = fs::read_dir(path)
    .and_then(|base| base.collect())
    .map_err(AppError::IO)?;
  let mut projects: BTreeMap<String, Project> = BTreeMap::new();
  for entry in project_entries {
    let path = entry.path();
    if path.is_dir() {
      match entry.file_name().into_string() {
        Ok(name) => {
          let mut path_to_repo = workspace_path.clone();
          path_to_repo.push(&name);
          match load_project(path_to_repo, &name, logger) {
            Ok(project) => {
              projects.insert(project.name.clone(), project);
            }
            Err(e) => warn!(logger, "Error while importing folder. Skipping it."; "entry" => name, "error" => format!("{}", e)),
          }
        }
        Err(_) => warn!(
          logger,
          "Failed to parse directory name as unicode. Skipping it."
        ),
      }
    }
  }

  Ok(projects)
}

pub fn org_import(maybe_config: Result<Config, AppError>, org_name: &str, logger: &Logger) -> Result<(), AppError> {
  let mut current_config = maybe_config?;
  let token = current_config
    .settings
    .github_token
    .clone()
    .ok_or_else(|| {
      AppError::UserError(format!(
        "Can't call GitHub API for org {} because no github oauth token (settings.github_token) specified in the configuration.",
        org_name
      ))
    })?;
  let mut api = github::github_api(token)?;
  let mut current_projects = current_config.projects.clone();
  let org_repository_names: Vec<String> = api.list_repositories(org_name)?;
  let new_projects = org_repository_names.into_iter().map(|repo_name| Project {
    name: repo_name.clone(),
    git: format!("git@github.com:{}/{}.git", org_name, repo_name),
    after_clone: None,
    after_workon: None,
    override_path: None,
    tags: None,
  });
  for new_project in new_projects {
    if current_projects.contains_key(&new_project.name) {
      warn!(
        logger,
          "Skipping new project from org import because it already exists in the current fw config"; "project_name" => &new_project.name);
    } else {
      info!(logger, "Adding new project"; "project_name" => &new_project.name);
      current_projects.insert(new_project.name.clone(), new_project);
    }
  }
  current_config.projects = current_projects;
  config::write_config(current_config, logger)?;
  Ok(())
}

pub fn import(maybe_config: Result<Config, AppError>, path: &str, logger: &Logger) -> Result<(), AppError> {
  let mut config: Config = maybe_config?;
  let path = fs::canonicalize(Path::new(path))?;
  let project_path = path
    .to_str()
    .ok_or(AppError::InternalError("project path is not valid unicode"))?
    .to_owned();
  let file_name = AppError::require(
    path.file_name(),
    AppError::UserError("Import path needs to be valid".to_string()),
  )?;
  let project_name: String = file_name.to_string_lossy().into_owned();
  let new_project = load_project(path.clone(), &project_name, logger)?;
  let new_project_with_path = Project {
    override_path: Some(project_path),
    ..new_project
  };
  config.projects.insert(project_name, new_project_with_path);
  info!(logger, "Updated config"; "config" => format!("{:?}", config));
  config::write_config(config, logger)
}

fn load_project(path_to_repo: PathBuf, name: &str, logger: &Logger) -> Result<Project, AppError> {
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
    after_clone: None,
    after_workon: None,
    override_path: None,
    tags: None,
  })
}

fn write_config(projects: BTreeMap<String, Project>, logger: &Logger, workspace_dir: &str) -> Result<(), AppError> {
  let config = Config {
    projects: projects,
    settings: Settings {
      workspace: workspace_dir.to_owned(),
      default_after_workon: None,
      default_after_clone: None,
      default_tags: None,
      shell: None,
      tags: Some(BTreeMap::new()),
      github_token: None,
    },
  };
  debug!(logger, "Finished"; "projects" => format!("{:?}", config.projects.len()));
  config::write_config(config, logger)
}
