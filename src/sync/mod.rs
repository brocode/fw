use ansi_term::Colour;
use atty;
use config::{Config, Project, Settings};
use std::collections::{BTreeSet};
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
use std::os::unix::fs::FileTypeExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;

pub static COLOURS: [Colour; 14] = [Colour::Green,
                                Colour::Cyan,
                                Colour::Blue,
                                Colour::Yellow,
                                Colour::RGB(255, 165, 0),
                                Colour::RGB(255, 99, 71),
                                Colour::RGB(0, 153, 255),
                                Colour::RGB(102, 0, 102),
                                Colour::RGB(102, 0, 0),
                                Colour::RGB(153, 102, 51),
                                Colour::RGB(102, 153, 0),
                                Colour::RGB(0, 0, 102),
                                Colour::RGB(255, 153, 255),
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

fn forward_process_output_to_stdout<T: std::io::Read>(read: T, prefix: &str, colour: &Colour, atty: bool, mark_err: bool) -> Result<(), AppError> {
  let mut buf = BufReader::new(read);
  loop {
    let mut line = String::new();
    let read: usize = buf.read_line(&mut line)?;
    if read == 0 {
      break;
    }
    if mark_err {
      let prefix = format!("{:>21.21} |", prefix);
      if atty {
        print!("{} {} {}",
               Colour::Red.paint("ERR"),
               colour.paint(prefix),
               line);
      } else {
        print!("ERR {} {}", prefix, line);
      };
    } else {
      let prefix = format!("{:>25.25} |", prefix);
      if atty {
        print!("{} {}", colour.paint(prefix), line);
      } else {
        print!("{} {}", prefix, line);
      };
    }
  }
  Ok(())
}

fn spawn_maybe(shell: Vec<String>, cmd: &str, workdir: &PathBuf, project_name: &str, colour: &Colour, logger: &Logger) -> Result<(), AppError> {
  let program: &str = shell.first().ok_or(AppError::UserError("shell entry in project settings must have at least one element".to_owned()))?;
  let rest: &[String] = shell.split_at(1).1;
  let mut result: Child = Command::new(program)
    .args(rest)
    .arg(cmd)
    .current_dir(&workdir)
    .env("FW_PROJECT", project_name)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .stdin(Stdio::null())
    .spawn()?;

  let stdout_child = if let Some(stdout) = result.stdout.take() {
    let colour = *colour;
    let project_name = project_name.to_owned();
    Some(thread::spawn(move || {
                         let atty: bool = is_stdout_a_tty();
                         forward_process_output_to_stdout(stdout, &project_name, &colour, atty, false)
                       }))
  } else {
    None
  };

  // stream stderr in this thread. no need to spawn another one.
  if let Some(stderr) = result.stderr.take() {
    let atty: bool = is_stderr_a_tty();
    forward_process_output_to_stdout(stderr, project_name, colour, atty, true)?
  }

  if let Some(child) = stdout_child {
    child.join().expect("Must be able to join child")?;
  }

  let status = result.wait()?;
  if status.code().unwrap_or(0) > 0 {
    error!(logger, "cmd failed");
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

fn is_stderr_a_tty() -> bool {
  atty::is(atty::Stream::Stderr)
}

fn project_shell(project_settings: &Settings) -> Vec<String> {
  project_settings.shell.clone().unwrap_or_else(|| vec!["sh".to_owned(), "-c".to_owned()])
}

pub fn foreach(maybe_config: Result<Config, AppError>, cmd: &str, tags: &BTreeSet<String>, logger: &Logger) -> Result<(), AppError> {
  let config = maybe_config?;
  let projects: Vec<&Project> = config.projects.values().collect();
  let script_results = projects
                             .par_iter()
                             .filter(|p| tags.is_empty() || p.tags.clone().unwrap_or_default().intersection(tags).count() > 0)
                             .map(|p| {
                                    let shell = project_shell(&config.settings);
                                    let project_logger = logger.new(o!("project" => p.name.clone()));
                                    let path = config.actual_path_to_project(p, &project_logger);
                                    info!(project_logger, "Entering");
                                    spawn_maybe(shell, cmd, &path, &p.name, &random_colour(), &project_logger)
                                  })
                             .collect::<Vec<Result<(), AppError>>>();

  script_results.into_iter()
                .fold(Result::Ok(()), |accu, maybe_error| accu.and(maybe_error))
}


fn sync_project(config: &Config, project: &Project, logger: &Logger, pb: &mut ProgressBar<Pipe>) -> Result<(), AppError> {
  let mut repo_builder = builder();
  pb.inc();
  let shell = project_shell(&config.settings);
  let path = config.actual_path_to_project(project, logger);
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
      let res = spawn_maybe(shell, &cmd, &path, &project.name, &random_colour(), logger);
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
  if !ssh_agent_running() {
    warn!(logger, "SSH Agent not running. Process may hang.")
  }
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

fn ssh_agent_running() -> bool {
  match std::env::var("SSH_AUTH_SOCK") {
  Ok(auth_socket) => {
    std::fs::metadata(auth_socket)
      .map(|m| m.file_type().is_socket())
      .unwrap_or(false)
  }
  Err(_) => false,
  }
}
