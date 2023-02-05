use crate::config::Config;
use crate::errors::AppError;
use slog::Logger;
use std::fs;
use std::io::Write;
use std::option::Option::Some;
use std::path::PathBuf;

pub fn intellij(maybe_config: Result<Config, AppError>, logger: &Logger, warn: bool) -> Result<(), AppError> {
  let config: Config = maybe_config?;
  let projects_paths: Vec<PathBuf> = config.projects.values().map(|p| config.actual_path_to_project(p, logger)).collect();
  let recent_projects_candidates = get_recent_projects_candidates()?;
  for candidate in recent_projects_candidates {
    let mut writer = fs::File::create(candidate)?;
    writeln!(
      writer,
      "<application><component name=\"RecentProjectsManager\"><option name=\"additionalInfo\"><map>"
    )?;
    for entry in &projects_paths {
      writeln!(writer, "<entry key=\"{}\">", entry.to_string_lossy())?;
      writeln!(writer, "<value>")?;
      writeln!(writer, "<RecentProjectMetaInfo>")?;
      writeln!(writer, "</RecentProjectMetaInfo>")?;
      writeln!(writer, "</value>")?;
      writeln!(writer, "</entry>")?;
    }
    writeln!(writer, "</map></option></component></application>")?;
  }

  let number_of_projects = projects_paths.len();

  if number_of_projects > 50 && warn {
    print_number_of_projects_warning(number_of_projects)
  }

  Ok(())
}

fn get_recent_projects_candidates() -> Result<Vec<PathBuf>, AppError> {
  let mut recent_projects_candidates: Vec<PathBuf> = Vec::new();
  let mut jetbrains_dir: PathBuf = dirs::config_dir().ok_or(AppError::InternalError("Could not resolve user configuration directory"))?;
  jetbrains_dir.push("JetBrains");
  for entry in fs::read_dir(jetbrains_dir)? {
    let path = entry?.path();
    if let Some(directory_name) = path.file_name() {
      let dir = directory_name.to_string_lossy();
      if dir.starts_with("IntelliJ") || dir.starts_with("Idea") {
        let mut recent_projects_path = path.clone();
        recent_projects_path.push("options");
        recent_projects_path.push("recentProjects.xml");
        if recent_projects_path.exists() {
          recent_projects_candidates.push(recent_projects_path);
        }
      }
    }
  }
  Ok(recent_projects_candidates)
}

fn print_number_of_projects_warning(number_of_projects: usize) {
  print!("WARNING: {number_of_projects} ");
  print!("projects were added to the list. Intellij only lists 50 projects by default. You can change this in Intellij by going to ");
  print!(r#"Settings -> Search for "recent" -> Pick the "Advanced Settings" -> adjust "Maximum number of recent projects"."#);
  println!("A high number is recommended since it won't do any harm to the system.");
}
