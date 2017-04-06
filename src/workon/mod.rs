use errors::AppError;
use config;
use config::Project;
use std::path::PathBuf;


pub fn ls(maybe_config: Result<config::Config, AppError>) -> Result<(), AppError> {
  let config = try!(maybe_config);
  config.projects.into_iter().map(|(_, p)| println!("{}", p.name)).collect::<Vec<()>>();
  Ok(())
}

pub fn gen(name: &str) -> Result<(), AppError> {
  let config = try!(config::get_config());
  let project: &Project =
    try!(config
           .projects
           .get(name)
           .ok_or(AppError::UserError(format!("project key {} not found in ~/.fw.json", name))));
  let mut canonical_project_path = PathBuf::from(config.settings.workspace);
  canonical_project_path.push(project.name.clone());
  let path = try!(canonical_project_path
                    .to_str()
                    .ok_or(AppError::InternalError("project path is not valid unicode")));
  if !canonical_project_path.exists() {
    Err(AppError::UserError(format!("project key {} found but path {} does not exist",
                                    name,
                                    path)))
  } else {
    let after_workon = project
      .after_workon
      .clone()
      .map(|cmd| format!(" && {}", cmd))
      .unwrap_or("".to_owned());
    println!("cd {}{}", path, after_workon);
    Ok(())
  }
}
