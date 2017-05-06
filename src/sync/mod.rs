use ansi_term::Colour;
use atty;
use config::{Config, Project};
use errors::AppError;
use git2;
use git2::FetchOptions;
use git2::RemoteCallbacks;
use git2::build::RepoBuilder;
use pbr::{MultiBar, Pipe, ProgressBar};
use rand;
use rand::Rng;
use rayon::prelude::*;
use slog::Logger;
use std;
use std::io::{BufRead, BufReader};
use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;

static COLOURS: [Colour; 11] = [Colour::Red,
                                Colour::Green,
                                Colour::Cyan,
                                Colour::Blue,
                                Colour::Yellow,
                                Colour::RGB(255, 165, 0),
                                Colour::RGB(255, 99, 71),
                                Colour::RGB(0, 153, 255),
                                Colour::RGB(102, 0, 102),
                                Colour::RGB(102, 0, 0),
                                Colour::Purple];

fn builder<'a>() -> RepoBuilder<'a> {
  let mut remote_callbacks = RemoteCallbacks::new();
  remote_callbacks.credentials(|_, _, _| git2::Cred::ssh_key_from_agent("git"));
  let mut fetch_options = FetchOptions::new();
  fetch_options.remote_callbacks(remote_callbacks);
  let mut repo_builder = RepoBuilder::new();
  repo_builder.fetch_options(fetch_options);
  repo_builder
}

fn spawn_maybe(cmd: &str, workdir: &PathBuf, project_name: &str, colour: &Colour, logger: &Logger) -> Result<(), AppError> {
  let mut result: Child = Command::new("sh")
    .arg("-c")
    .arg(cmd)
    .current_dir(&workdir)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

  if let Some(ref mut stdout) = result.stdout {
    let mut buf = BufReader::new(stdout);
    loop {
      let mut line = String::new();
      let read: usize = buf.read_line(&mut line)?;
      if read == 0 {
        break;
      }
      let prefix = format!("{:>25.25} |", project_name);
      if is_stdout_a_tty() {
        print!("{} {}", colour.paint(prefix), line);
      } else {
        print!("{} {}", prefix, line);
      };
    }
  }

  let mut stderr_output = String::new();
  if let Some(ref mut stderr) = result.stderr {
    stderr.read_to_string(&mut stderr_output)?;
  }

  let status = result.wait()?;
  if status.code().unwrap_or(0) > 0 {
    error!(
      logger,
      "cmd failed";
      "stderr" => stderr_output.replace("\n", "\\n"));
    Err(AppError::UserError("External command failed.".to_owned()))
  } else {
    info!(logger, "cmd finished");
    Ok(())
  }
}

fn random_colour() -> Colour {
  let mut rng = rand::thread_rng();
  rng.choose(&COLOURS)
     .map(|c| c.to_owned())
     .unwrap_or(Colour::Black)
}
fn is_stdout_a_tty() -> bool {
  atty::is(atty::Stream::Stdout)
}

pub fn foreach(maybe_config: Result<Config, AppError>, cmd: &str, logger: &Logger) -> Result<(), AppError> {
  let config = maybe_config?;
  let script_results = config.projects
                             .par_iter()
                             .map(|(_, p)| {
                                    let project_logger = logger.new(o!("project" => p.name.clone()));
                                    let path = config.actual_path_to_project(p, &project_logger);
                                    info!(project_logger, "Entering");
                                    spawn_maybe(cmd, &path, &p.name, &random_colour(), &project_logger)
                                  })
                             .collect::<Vec<Result<(), AppError>>>();

  script_results.into_iter()
                .fold(Result::Ok(()), |accu, maybe_error| accu.and(maybe_error))
}


fn sync_project(config: &Config, project: &Project, logger: &Logger, pb: &mut ProgressBar<Pipe>) -> Result<(), AppError> {
  let mut repo_builder = builder();
  pb.inc();
  let path = config.actual_path_to_project(project, &logger);
  let exists = path.exists();
  let project_logger = logger.new(o!(
        "git" => project.git.clone(),
        "exists" => exists,
        "path" => format!("{:?}", path),
      ));
  let res = if exists {
    debug!(project_logger, "NOP");
    Result::Ok(())
  } else {
    info!(project_logger, "Cloning project");
    repo_builder.clone(project.git.as_str(), &path)
                .map_err(|error| {
      warn!(project_logger, "Error cloning repo"; "error" => format!("{}", error));
      let wrapped = AppError::GitError(error);
      pb.finish_print(format!("{}: {} ({:?})",
                              Colour::Red.paint("FAILED"),
                              &project.name,
                              wrapped)
                        .as_ref());
      wrapped
    })
                .and_then(|_| match config.resolve_after_clone(&project_logger, project) {
                          Some(cmd) => {
      pb.inc();
      info!(project_logger, "Handling post hooks"; "after_clone" => cmd);
      let res = spawn_maybe(&cmd, &path, &project.name, &random_colour(), &logger);
      pb.inc();
      res.map_err(|error| {
        let wrapped = AppError::UserError(format!("Post-clone hook failed (nonzero exit code). Cause: {:?}",
                                                  error));
        pb.finish_print(format!("{}: {} ({:?})",
                                Colour::Red.paint("FAILED"),
                                &project.name,
                                wrapped)
                          .as_ref());
        error
      })
    }
                          None => Ok(()),
                          })
  };
  pb.finish();
  res
}
pub fn synchronize(maybe_config: Result<Config, AppError>, logger: &Logger) -> Result<(), AppError> {
  info!(logger, "Synchronizing everything");
  let logger = logger.new(o!());
  let config = maybe_config?;
  let mut mb = MultiBar::on(std::io::stderr());
  let mut projects: Vec<_> = config.projects
                                   .values()
                                   .map(|p| {
                                          let mut pb: ProgressBar<Pipe> = mb.create_bar(3);
                                          pb.message(&format!("{:>25.25} ", &p.name));
                                          pb.show_message = true;
                                          pb.show_speed = false;
                                          pb.tick();
                                          (p.to_owned(), pb)
                                        })
                                   .collect();
  let child = thread::spawn(move || {
                              projects.par_iter_mut()
                                      .map(|project_with_bar| {
                                             let &mut (ref project, ref mut pb) = project_with_bar;
                                             sync_project(&config, project, &logger, pb)
                                           })
                                      .collect()
                            });
  mb.listen();
  let results: Vec<Result<(), AppError>> = child.join().expect("Could not join thread.");


  results.into_iter()
         .fold(Result::Ok(()), |accu, maybe_error| accu.and(maybe_error))
}
