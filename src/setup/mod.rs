use slog::Logger;
use errors::AppError;
use std::path::PathBuf;
use config::Project;
use std::collections::HashMap;

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
    .and_then(|_| Result::Ok(()))
}

fn determine_projects(path: PathBuf,
                      logger: &Logger)
                      -> Result<HashMap<String, Project>, AppError> {
  Result::Err(AppError::InternalError("Not implemented"))
}
