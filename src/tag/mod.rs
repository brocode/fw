use config::Config;
use errors::AppError;
use slog::Logger;


pub fn list_tags(maybe_config: Result<Config, AppError>, maybe_project_name: Option<String>, logger: &Logger) -> Result<(), AppError> {
  let config: Config = maybe_config?;
  if let Some(project_name) = maybe_project_name {
    debug!(logger, "Listing tags for project"; "project" => project_name);
    list_project_tags(config, &project_name)
  } else {
    debug!(logger, "Listing tags");
    list_all_tags(config)
  }
}

fn list_all_tags(config: Config) -> Result<(), AppError> {
  if let Some(tags) = config.settings.tags {
    for tag_name in tags.keys() {
      println!("{}", tag_name);
    }
  }
  Result::Ok(())
}

fn list_project_tags(config: Config, project_name: &str) -> Result<(), AppError> {
  if let Some(project) = config.projects.get(project_name) {
    if let Some(tags) = project.clone().tags {
      for tag_name in tags {
        println!("{}", tag_name);
      }
    }
    Result::Ok(())
  } else {
    Result::Err(AppError::UserError(format!("Unknown project {}", project_name)))
  }
}
