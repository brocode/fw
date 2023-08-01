use crate::config::{project::Project, Config};
use crate::errors::AppError;

use rayon::prelude::*;
use std::collections::BTreeSet;
use yansi::{Color, Paint};

use std::borrow::ToOwned;

use crate::util::random_color;
use std::io::IsTerminal;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

use std::thread;

fn forward_process_output_to_stdout<T: std::io::Read>(read: T, prefix: &str, color: Color, atty: bool, mark_err: bool) -> Result<(), AppError> {
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
        print!("{} {} {}", Paint::red("ERR"), color.paint(prefix), line);
      } else {
        print!("ERR {} {}", prefix, line);
      };
    } else {
      let prefix = format!("{:>25.25} |", prefix);
      if atty {
        print!("{} {}", color.paint(prefix), line);
      } else {
        print!("{} {}", prefix, line);
      };
    }
  }
  Ok(())
}

fn is_stdout_a_tty() -> bool {
  std::io::stdout().is_terminal()
}

fn is_stderr_a_tty() -> bool {
  std::io::stderr().is_terminal()
}

pub fn spawn_maybe(shell: &[String], cmd: &str, workdir: &Path, project_name: &str, color: Color) -> Result<(), AppError> {
  let program: &str = shell
    .first()
    .ok_or_else(|| AppError::UserError("shell entry in project settings must have at least one element".to_owned()))?;
  let rest: &[String] = shell.split_at(1).1;
  let mut result: Child = Command::new(program)
    .args(rest)
    .arg(cmd)
    .current_dir(workdir)
    .env("FW_PROJECT", project_name)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .stdin(Stdio::null())
    .spawn()?;

  let stdout_child = if let Some(stdout) = result.stdout.take() {
    let project_name = project_name.to_owned();
    Some(thread::spawn(move || {
      let atty: bool = is_stdout_a_tty();
      forward_process_output_to_stdout(stdout, &project_name, color, atty, false)
    }))
  } else {
    None
  };

  // stream stderr in this thread. no need to spawn another one.
  if let Some(stderr) = result.stderr.take() {
    let atty: bool = is_stderr_a_tty();
    forward_process_output_to_stdout(stderr, project_name, color, atty, true)?
  }

  if let Some(child) = stdout_child {
    child.join().expect("Must be able to join child")?;
  }

  let status = result.wait()?;
  if status.code().unwrap_or(0) > 0 {
    Err(AppError::UserError("External command failed.".to_owned()))
  } else {
    Ok(())
  }
}

pub fn init_threads(parallel_raw: &Option<String>) -> Result<(), AppError> {
  if let Some(ref raw_num) = *parallel_raw {
    let num_threads = raw_num.parse::<usize>()?;
    rayon::ThreadPoolBuilder::new().num_threads(num_threads).build_global().expect(
      "Tried to initialize rayon more than once (this is a software bug on fw side, please file an issue at https://github.com/brocode/fw/issues/new )",
    );
  }
  Ok(())
}

pub fn foreach(maybe_config: Result<Config, AppError>, cmd: &str, tags: &BTreeSet<String>, parallel_raw: &Option<String>) -> Result<(), AppError> {
  let config = maybe_config?;
  init_threads(parallel_raw)?;

  let projects: Vec<&Project> = config.projects.values().collect();
  let script_results = projects
    .par_iter()
    .filter(|p| tags.is_empty() || p.tags.clone().unwrap_or_default().intersection(tags).count() > 0)
    .map(|p| {
      let shell = config.settings.get_shell_or_default();
      let path = config.actual_path_to_project(p);
      spawn_maybe(&shell, cmd, &path, &p.name, random_color())
    })
    .collect::<Vec<Result<(), AppError>>>();

  script_results.into_iter().fold(Ok(()), Result::and)
}
