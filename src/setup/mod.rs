use config;
use config::{Config, Project, Settings};
use errors::AppError;
use git2::Repository;
use slog::Logger;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

pub fn setup(workspace_dir: &str, logger: &Logger) -> Result<(), AppError> {
  let setup_logger = logger.new(o!("workspace" => workspace_dir.to_owned()));
  debug!(setup_logger, "Entering setup");
  let path = PathBuf::from(workspace_dir);
  let maybe_path = if path.exists() {
    Result::Ok(path)
  } else {
    Result::Err(AppError::UserError(format!("Given workspace {} does not exist", workspace_dir)))
  };

  maybe_path.and_then(|path| determine_projects(path, logger))
            .and_then(|projects| write_config(projects, logger, workspace_dir))
}

fn determine_projects(path: PathBuf, logger: &Logger) -> Result<BTreeMap<String, Project>, AppError> {
  let workspace_path = path.clone();

  let project_entries: Vec<fs::DirEntry> = fs::read_dir(path)
    .and_then(|base| base.collect())
    .map_err(AppError::IO)?;
  let projects: Vec<Result<Project, AppError>> =
    project_entries.into_iter()
                   .map(|entry: fs::DirEntry| match entry.file_name().into_string() {
                        Ok(name) => {
      let project_logger = logger.new(o!("project" => name.clone()));
      let mut path_to_repo = workspace_path.clone();
      path_to_repo.push(name.clone());
      let repo = Repository::open(path_to_repo)?;
      let all = repo.remotes()?;
      debug!(project_logger, "remotes"; "found" => format!("{:?}", all.len()));
      let remote = repo.find_remote("origin")?;
      let url = remote.url()
                      .ok_or_else(|| AppError::UserError(format!("invalid remote origin at {:?}", repo.path())))?;
      info!(project_logger, "git config validated");
      Ok(Project {
           name: name,
           git: url.to_owned(),
           after_clone: None,
           after_workon: None,
           override_path: None,
         })
    }
                        Err(invalid_unicode) => Err(AppError::Utf8Error(invalid_unicode)),
                        })
                   .collect();

  let acc: BTreeMap<String, Project> = BTreeMap::new();
  projects.into_iter()
          .fold(Ok(acc),
                |maybe_accu: Result<BTreeMap<String, Project>, AppError>, project: Result<Project, AppError>| match project {
                Ok(p) => {
                  maybe_accu.and_then(|mut accu| {
                                        accu.insert(p.clone().name, p);
                                        Ok(accu)
                                      })
                }
                Err(e) => Err(e),
                })
}

fn write_config(projects: BTreeMap<String, Project>, logger: &Logger, workspace_dir: &str) -> Result<(), AppError> {
  let config = Config {
    projects: projects,
    settings: Settings {
      workspace: workspace_dir.to_owned(),
      default_after_workon: None,
      default_after_clone: None,
    },
  };
  debug!(logger, "Finished"; "projects" => format!("{:?}", config.projects.len()));
  config::write_config(&config, logger)
}
