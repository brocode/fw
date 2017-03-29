use slog::Logger;
use errors::AppError;
use std::path::PathBuf;
use config::Project;
use std::collections::HashMap;
use std::fs;

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
    .map(|_| ())
}

fn determine_projects(path: PathBuf,
                      logger: &Logger)
                      -> Result<HashMap<String, Project>, AppError> {

  fs::read_dir(path)
    .and_then(|base| base.collect()).map_err(|e| AppError::IO(e))
    .and_then(|project_entries: Vec<fs::DirEntry>| {
      let projects: Vec<Result<Project, AppError>> = project_entries
        .into_iter()
        .map(|entry: fs::DirEntry| match entry.file_name().into_string() {
               Ok(name) => {
                 info!(logger, "processing"; "project" => name);
                 Ok(Project {
                      name: name,
                      git: "".to_owned(),
                    })
               }
               Err(invalid_unicode) => Err(AppError::Utf8Error(invalid_unicode)),
             })
        .collect();

      let acc: HashMap<String, Project> = HashMap::new();
      let ok_projects: Result<HashMap<String, Project>, AppError> =
        projects.into_iter().fold(Ok(acc), |maybe_accu: Result<HashMap<String, Project>, AppError>, project: Result<Project, AppError>| {
          match project {
            Ok(p) =>
              maybe_accu.and_then(|mut accu| {accu.insert(p.clone().name, p);
                                         Ok(accu)}),
            Err(e) => Err(e),
          }
      });

      ok_projects
    })
}
