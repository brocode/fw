use slog::Logger;
use errors::AppError;
use std::path::{PathBuf, Path};
use config::{Project, Settings, Config};
use std::collections::HashMap;
use std::io::prelude::*;
use std::fs;
use serde_json::ser;
use git2::Repository;
use config;

pub fn setup(workspace_dir: &str, logger: &Logger) -> Result<(), AppError> {
  let setup_logger = logger.new(o!("workspace" => format!("{}", workspace_dir)));
  debug!(setup_logger, "Entering setup");
  let path = PathBuf::from(workspace_dir);
  let maybe_path = if path.exists() {
    Result::Ok(path)
  } else {
    Result::Err(AppError::UserError(format!("Given workspace {} does not exist", workspace_dir)))
  };

  maybe_path
    .and_then(|path| determine_projects(path, logger))
    .and_then(|projects| write_config(projects, logger, workspace_dir))
}

fn determine_projects(path: PathBuf,
                      logger: &Logger)
                      -> Result<HashMap<String, Project>, AppError> {
  let workspace_path = path.clone();

  fs::read_dir(path)
    .and_then(|base| base.collect())
    .map_err(|e| AppError::IO(e))
    .and_then(|project_entries: Vec<fs::DirEntry>| {
      let projects: Vec<Result<Project, AppError>> = project_entries
        .into_iter()
        .map(|entry: fs::DirEntry| match entry.file_name().into_string() {
               Ok(name) => {
          let project_logger = logger.new(o!("project" => name.clone()));
          let mut path_to_repo = workspace_path.clone();
          path_to_repo.push(name.clone());
          let repo = try!(Repository::open(path_to_repo));
          let all = try!(repo.remotes());
          debug!(project_logger, "remotes"; "found" => format!("{:?}", all.len()));
          let remote = try!(repo.find_remote("origin"));
          let url = try!(remote
                           .url()
                           .ok_or(AppError::UserError(format!("invalid remote origin at {:?}",
                                                              repo.path()))));
          info!(project_logger, "git config validated");
          Ok(Project {
               name: name,
               git: url.to_owned(),
             })
        }
               Err(invalid_unicode) => Err(AppError::Utf8Error(invalid_unicode)),
             })
        .collect();

      let acc: HashMap<String, Project> = HashMap::new();
      projects
        .into_iter()
        .fold(Ok(acc),
              |maybe_accu: Result<HashMap<String, Project>, AppError>,
               project: Result<Project, AppError>| {
          match project {
            Ok(p) => {
              maybe_accu.and_then(|mut accu| {
                                    accu.insert(p.clone().name, p);
                                    Ok(accu)
                                  })
            }
            Err(e) => Err(e),
          }
        })
    })
}

fn write_config(projects: HashMap<String, Project>,
                logger: &Logger,
                workspace_dir: &str)
                -> Result<(), AppError> {
  let config = Config {
    projects: projects,
    settings: Settings { workspace: workspace_dir.to_owned() },
  };
  debug!(logger, "Finished"; "projects" => format!("{:?}", config.projects.len()));
  let config_path = try!(config::config_path());
  info!(logger, "Writing config"; "path" => format!("{:?}", config_path));
  let mut buffer = try!(fs::File::create(config_path));
  ser::to_writer_pretty(&mut buffer, &config).map_err(|e| AppError::BadJson(e))
}
