use slog::Logger;
use errors::AppError;
use std::path::{PathBuf, Path};
use config::{Project, Settings, Config};
use std::collections::HashMap;
use std::fs;
use serde_json::ser;
use git2::Repository;

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

  fs::read_dir(path)
    .and_then(|base| base.collect())
    .map_err(|e| AppError::IO(e))
    .and_then(|project_entries: Vec<fs::DirEntry>| {
      let projects: Vec<Result<Project, AppError>> = project_entries
        .into_iter()
        .map(|entry: fs::DirEntry| match entry.file_name().into_string() {
               Ok(name) => {
          // todo unwrap calls
                 // todo path to repo fix
          let repo = Repository::open("").unwrap();
          let remote = repo.find_remote("origin").unwrap();
          let url = remote.url().unwrap();
          info!(logger, "processing"; "project" => name);
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
  //ser::to_writer_pretty(writer, config);
  unimplemented!()
}
