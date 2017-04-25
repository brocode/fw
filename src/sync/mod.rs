use config;
use config::Config;
use errors::AppError;
use git2;
use git2::FetchOptions;
use git2::RemoteCallbacks;
use git2::build::RepoBuilder;
use rayon::prelude::*;
use slog::Logger;
use std;
use std::path::PathBuf;
use std::process::{Command, Output};

fn builder<'a>() -> RepoBuilder<'a> {
  let mut remote_callbacks = RemoteCallbacks::new();
  remote_callbacks.credentials(|_, _, _| git2::Cred::ssh_key_from_agent("git"));
  let mut fetch_options = FetchOptions::new();
  fetch_options.remote_callbacks(remote_callbacks);
  let mut repo_builder = RepoBuilder::new();
  repo_builder.fetch_options(fetch_options);
  repo_builder
}

fn spawn_maybe(cmd: &str, workdir: &PathBuf, logger: &Logger) -> Result<(), AppError> {
  let result = Command::new("sh")
    .arg("-c")
    .arg(cmd)
    .current_dir(&workdir)
    .output()?;
  match result {
  Output {
    status,
    ref stdout,
    ref stderr,
  } => {
    let ok_stderr = std::str::from_utf8(stderr)?.replace("\n", "\\n");
    let ok_stdout = std::str::from_utf8(stdout)?.replace("\n", "\\n");
    info!(
                    logger,
                    "cmd finished";
                    "stderr" => ok_stderr,
                    "stdout" => ok_stdout);
    if status.success() {
      Ok(())
    } else {
      Err(AppError::UserError(format!("cmd {} blew up in project at {:?} (this was the first error, there might have been more subsequent failures)", cmd, workdir)))
    }
  }
  }

}

pub fn foreach(maybe_config: Result<Config, AppError>, cmd: &str, logger: &Logger) -> Result<(), AppError> {
  let config = maybe_config?;
  let workspace = config.settings.workspace;
  let script_results = config.projects
                             .par_iter()
                             .map(|(_, p)| {
                                    let path = config::actual_path_to_project(&workspace, p);
                                    let project_logger = logger.new(o!("project" => p.name.clone()));
                                    info!(project_logger, "Entering");
                                    spawn_maybe(cmd, &path, &project_logger)
                                  })
                             .collect::<Vec<Result<(), AppError>>>();

  script_results.into_iter()
                .fold(Result::Ok(()), |accu, maybe_error| accu.and(maybe_error))
}



pub fn synchronize(maybe_config: Result<Config, AppError>, logger: &Logger) -> Result<(), AppError> {
  info!(logger, "Synchronizing everything");
  let config = maybe_config?;
  let workspace = config.settings.workspace;
  let results: Vec<Result<(), AppError>> = config.projects
                                                 .par_iter()
                                                 .map(|(_, project)| {
    let mut repo_builder = builder();
    let path = config::actual_path_to_project(&workspace, project);
    let exists = path.exists();
    let project_logger = logger.new(o!(
        "git" => project.git.clone(),
        "exists" => exists,
        "path" => format!("{:?}", path),
      ));
    if exists {
      info!(project_logger, "NOP");
      Result::Ok(())
    } else {
      info!(project_logger, "Cloning project");
      repo_builder.clone(project.git.as_str(), &path)
                  .map_err(|error| {
                             warn!(project_logger, "Error cloning repo"; "error" => format!("{}", error));
                             AppError::GitError(error)
                           })
                  .and_then(|_| match project.clone().after_clone {
                            Some(cmd) => {
        info!(project_logger, "Handling post hooks"; "after_clone" => cmd);
        spawn_maybe(&cmd, &path, logger)
      }
                            None => Ok(()),
                            })
    }
  })
                                                 .collect();

  results.into_iter()
         .fold(Result::Ok(()), |accu, maybe_error| accu.and(maybe_error))
}
