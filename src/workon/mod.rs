use config;
use config::Project;
use errors::AppError;
use std::path::PathBuf;


pub fn ls(maybe_config: Result<config::Config, AppError>) -> Result<(), AppError> {
  let config = maybe_config?;
  config.projects
        .into_iter()
        .map(|(_, p)| println!("{}", p.name))
        .collect::<Vec<()>>();
  Ok(())
}

pub fn gen(name: &str, maybe_config: Result<config::Config, AppError>, quick: bool) -> Result<(), AppError> {
  let config = maybe_config?;
  let project: &Project = config.projects
                                .get(name)
                                .ok_or(AppError::UserError(format!("project key {} not found in ~/.fw.json", name)))?;
  let mut canonical_project_path = PathBuf::from(config.settings.workspace);
  canonical_project_path.push(project.name.clone());
  let path = canonical_project_path.to_str()
                                   .ok_or(AppError::InternalError("project path is not valid unicode"))?;
  if !canonical_project_path.exists() {
    Err(AppError::UserError(format!("project key {} found but path {} does not exist",
                                    name,
                                    path)))
  } else {
    let after_workon = if !quick {
      project.after_workon
             .clone()
             .map(|cmd| format!(" && {}", cmd))
             .unwrap_or("".to_owned())
    } else {
      String::new()
    };
    println!("cd {}{}", path, after_workon);
    Ok(())
  }
}
