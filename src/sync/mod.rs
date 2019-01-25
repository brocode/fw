use crate::config;
use crate::config::Tag;
use crate::config::{Config, Project, Settings};
use crate::errors::AppError;
use crate::tag;
use ansi_term::Colour;
use atty;
use crossbeam::queue::MsQueue;
use git2;
use git2::build::RepoBuilder;
use git2::{AutotagOption, Branch, Direction, FetchOptions, MergeAnalysis, Remote, RemoteCallbacks, Repository};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use rand;
use rand::Rng;
use rayon;
use rayon::prelude::*;
use regex::Regex;
use slog::Logger;
use slog::{debug, error, info, o, warn};
use std;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::env;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::FileTypeExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;

pub static COLOURS: [Colour; 14] = [
  Colour::Green,
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
  Colour::Purple,
];

fn username_from_git_url(url: &str) -> String {
  let url_regex = Regex::new(r"([^:]+://)?((?P<user>[a-z_][a-z0-9_]{0,30})@)?").unwrap();
  if let Some(caps) = url_regex.captures(url) {
    if let Some(user) = caps.name("user") {
      return user.as_str().to_string();
    }
  }
  if let Ok(user) = env::var("USER") {
    if user != "" {
      return user.to_string();
    }
  }
  "git".to_string()
}

fn agent_callbacks(git_user: &str) -> git2::RemoteCallbacks {
  let mut remote_callbacks = RemoteCallbacks::new();
  remote_callbacks.credentials(move |_, _, _| git2::Cred::ssh_key_from_agent(git_user));
  remote_callbacks
}

fn agent_fetch_options(git_user: &str) -> git2::FetchOptions {
  let remote_callbacks = agent_callbacks(git_user);
  let mut fetch_options = FetchOptions::new();
  fetch_options.remote_callbacks(remote_callbacks);

  fetch_options
}

fn builder(git_user: &str) -> RepoBuilder {
  let options = agent_fetch_options(git_user);
  let mut repo_builder = RepoBuilder::new();
  repo_builder.fetch_options(options);
  repo_builder
}

fn forward_process_output_to_stdout<T: std::io::Read>(read: T, prefix: &str, colour: Colour, atty: bool, mark_err: bool) -> Result<(), AppError> {
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
        print!("{} {} {}", Colour::Red.paint("ERR"), colour.paint(prefix), line);
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

pub fn spawn_maybe(shell: &[String], cmd: &str, workdir: &PathBuf, project_name: &str, colour: Colour, logger: &Logger) -> Result<(), AppError> {
  let program: &str = shell
    .first()
    .ok_or_else(|| AppError::UserError("shell entry in project settings must have at least one element".to_owned()))?;
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
    let project_name = project_name.to_owned();
    Some(thread::spawn(move || {
      let atty: bool = is_stdout_a_tty();
      forward_process_output_to_stdout(stdout, &project_name, colour, atty, false)
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
  rng.choose(&COLOURS).map(|c| c.to_owned()).unwrap_or(Colour::Black)
}
fn is_stdout_a_tty() -> bool {
  atty::is(atty::Stream::Stdout)
}

fn is_stderr_a_tty() -> bool {
  atty::is(atty::Stream::Stderr)
}

pub fn project_shell(project_settings: &Settings) -> Vec<String> {
  project_settings.shell.clone().unwrap_or_else(|| vec!["sh".to_owned(), "-c".to_owned()])
}

pub fn init_threads(parallel_raw: &Option<String>, logger: &Logger) -> Result<(), AppError> {
  if let Some(ref raw_num) = *parallel_raw {
    let num_threads = raw_num.parse::<usize>()?;
    rayon::ThreadPoolBuilder::new().num_threads(num_threads).build_global().expect(
      "Tried to initialize rayon more than once (this is a software bug on fw side, please file an issue at https://github.com/brocode/fw/issues/new )",
    );
    debug!(logger, "Rayon rolling with thread pool of size {}", raw_num)
  }
  Ok(())
}

pub fn foreach(
  maybe_config: Result<Config, AppError>,
  cmd: &str,
  tags: &BTreeSet<String>,
  logger: &Logger,
  parallel_raw: &Option<String>,
) -> Result<(), AppError> {
  let config = maybe_config?;
  init_threads(parallel_raw, logger)?;

  let projects: Vec<&Project> = config.projects.values().collect();
  let script_results = projects
    .par_iter()
    .filter(|p| tags.is_empty() || p.tags.clone().unwrap_or_default().intersection(tags).count() > 0)
    .map(|p| {
      let shell = project_shell(&config.settings);
      let project_logger = logger.new(o!("project" => p.name.clone()));
      let path = config.actual_path_to_project(p, &project_logger);
      info!(project_logger, "Entering");
      spawn_maybe(&shell, cmd, &path, &p.name, random_colour(), &project_logger)
    })
    .collect::<Vec<Result<(), AppError>>>();

  script_results.into_iter().fold(Ok(()), |accu, maybe_error| accu.and(maybe_error))
}

pub fn autotag(maybe_config: Result<Config, AppError>, cmd: &str, tag_name: &str, logger: &Logger, parallel_raw: &Option<String>) -> Result<(), AppError> {
  let mut config = maybe_config?;

  let tags: BTreeMap<String, Tag> = config.settings.tags.clone().unwrap_or_else(BTreeMap::new);
  if tags.contains_key(tag_name) {
    init_threads(parallel_raw, logger)?;

    let config2 = &config.clone();
    let projects: Vec<&Project> = config2.projects.values().collect();

    let script_results = projects
      .par_iter()
      .map(|p| {
        let shell = project_shell(&config2.settings);
        let project_logger = logger.new(o!("project" => p.name.clone()));
        let path = &config2.actual_path_to_project(p, &project_logger);
        info!(project_logger, "Entering");
        spawn_maybe(&shell, cmd, &path, &p.name, random_colour(), &project_logger)
      })
      .collect::<Vec<Result<(), AppError>>>();

    // map with projects and filter if result == 0
    let filtered_projects: Vec<&Project> = script_results
      .into_iter()
      .zip(projects.into_iter())
      .filter(|(x, _)| x.is_ok())
      .map(|(_, p)| p)
      .collect::<Vec<&Project>>();

    for project in filtered_projects.iter() {
      config = tag::add_tag_project(Ok(config), project.name.clone(), tag_name.to_string(), logger)?;
    }
    config::write_config(config, logger)
  } else {
    Err(AppError::UserError(format!("Unknown tag {}", tag_name)))
  }
}

fn init_additional_remotes(project: &Project, repository: Repository, project_logger: &Logger) -> Result<(), AppError> {
  if let Some(additional_remotes) = &project.additional_remotes {
    for remote in additional_remotes {
      let mut git_remote = repository.remote(&remote.name, &remote.git)?;
      update_remote(project, &mut git_remote, project_logger)?;
      debug!(project_logger, "Added remote"; "remote" => remote.name.to_string())
    }
  }
  Ok(())
}

fn update_remote(project: &Project, remote: &mut Remote, project_logger: &Logger) -> Result<(), AppError> {
  let git_user = username_from_git_url(&project.git);
  let remote_callbacks = agent_callbacks(&git_user);
  remote.connect_auth(Direction::Fetch, Some(remote_callbacks), None).map_err(|error| {
    warn!(project_logger, "Error connecting remote"; "error" => format!("{}", error), "project" => &project.name);
    AppError::GitError(error)
  })?;
  let mut options = agent_fetch_options(&git_user);
  remote.download(&[], Some(&mut options)).map_err(|error| {
    warn!(project_logger, "Error downloading for remote"; "error" => format!("{}", error), "project" => &project.name);
    AppError::GitError(error)
  })?;
  remote.disconnect();
  remote.update_tips(None, true, AutotagOption::Unspecified, None)?;
  Ok(())
}

fn update_project_remotes(project: &Project, path: &PathBuf, project_logger: &Logger, ff_merge: bool) -> Result<(), AppError> {
  debug!(project_logger, "Update project remotes");
  let local: Repository = Repository::open(path).map_err(|error| {
    warn!(project_logger, "Error opening local repo"; "error" => format!("{}", error));
    AppError::GitError(error)
  })?;
  for desired_remote in project.additional_remotes.clone().unwrap_or_default().into_iter().chain(
    vec![crate::config::Remote {
      name: "origin".to_string(),
      git: project.git.to_owned(),
    }]
    .into_iter(),
  ) {
    let remote = local
      .find_remote(&desired_remote.name)
      .or_else(|_| local.remote(&desired_remote.name, &desired_remote.git))?;

    let mut remote = match remote.url() {
      Some(url) if url == desired_remote.git => remote,
      _ => {
        local.remote_set_url(&desired_remote.name, &desired_remote.git)?;
        local.find_remote(&desired_remote.name)?
      }
    };

    update_remote(project, &mut remote, project_logger)?;
  }

  if ff_merge {
    if let Err(error) = fast_forward_merge(&local, project_logger) {
      debug!(project_logger, "Fast forward failed: {}", error)
    }
  }

  Ok(())
}

fn fast_forward_merge(local: &Repository, project_logger: &Logger) -> Result<(), AppError> {
  let head_ref = local.head()?;
  if head_ref.is_branch() {
    let branch = Branch::wrap(head_ref);
    let upstream = branch.upstream()?;
    let upstream_commit = local.reference_to_annotated_commit(upstream.get())?;

    debug!(project_logger, "Check fast forward for {:?} {:?}", branch.name(), upstream.name());

    let (analysis_result, _) = local.merge_analysis(&[&upstream_commit])?;
    if MergeAnalysis::is_fast_forward(&analysis_result) {
      debug!(project_logger, "Fast forward possible");
      let target_id = upstream_commit.id();
      local.checkout_tree(&local.find_object(upstream_commit.id(), None)?, None)?;
      local.head()?.set_target(target_id, "fw fast-forward")?;
    } else {
      debug!(project_logger, "Fast forward not possible: {:?}", analysis_result)
    }
  }
  Ok(())
}

fn clone_project(config: &Config, project: &Project, path: &PathBuf, project_logger: &Logger) -> Result<(), AppError> {
  let shell = project_shell(&config.settings);
  let git_user = username_from_git_url(&project.git);
  let mut repo_builder = builder(&git_user);
  debug!(project_logger, "Cloning project");
  repo_builder
    .bare(project.bare.unwrap_or_default())
    .clone(project.git.as_str(), path)
    .map_err(|error| {
      warn!(project_logger, "Error cloning repo"; "error" => format!("{}", error));
      AppError::GitError(error)
    })
    .and_then(|repo| init_additional_remotes(project, repo, project_logger))
    .and_then(|_| {
      let after_clone = config.resolve_after_clone(project_logger, project);
      if !after_clone.is_empty() {
        debug!(project_logger, "Handling post hooks"; "after_clone" => format!("{:?}", after_clone));
        spawn_maybe(&shell, &after_clone.join(" && "), path, &project.name, random_colour(), project_logger)
          .map_err(|error| AppError::UserError(format!("Post-clone hook failed (nonzero exit code). Cause: {:?}", error)))
      } else {
        Ok(())
      }
    })
}

fn sync_project(config: &Config, project: &Project, logger: &Logger, only_new: bool, ff_merge: bool) -> Result<(), AppError> {
  let path = config.actual_path_to_project(project, logger);
  let exists = path.exists();
  let project_logger = logger.new(o!(
    "git" => project.git.clone(),
    "exists" => exists,
    "path" => format!("{:?}", path),
  ));
  if exists {
    if only_new {
      Ok(())
    } else {
      update_project_remotes(project, &path, &project_logger, ff_merge)
    }
  } else {
    clone_project(config, project, &path, &project_logger)
  }
}

pub fn synchronize(
  maybe_config: Result<Config, AppError>,
  no_progress_bar: bool,
  only_new: bool,
  ff_merge: bool,
  worker: i32,
  logger: &Logger,
) -> Result<(), AppError> {
  eprintln!("Synchronizing everything");
  if !ssh_agent_running() {
    warn!(logger, "SSH Agent not running. Process may hang.")
  }
  let config = Arc::new(maybe_config?);

  let projects: Vec<Project> = config.projects.values().map(|p| p.to_owned()).collect();
  let q: Arc<MsQueue<Project>> = Arc::new(MsQueue::new());
  let projects_count = projects.len() as u64;
  for project in projects {
    q.push(project);
  }

  let spinner_style = ProgressStyle::default_spinner()
    .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷⣿")
    .template("{prefix:.bold.dim} {spinner} {wide_msg}");

  let m = MultiProgress::new();
  m.set_draw_target(if no_progress_bar {
    ProgressDrawTarget::hidden()
  } else {
    ProgressDrawTarget::stderr()
  });

  let job_results: Arc<MsQueue<Result<(), AppError>>> = Arc::new(MsQueue::new());
  let progress_bars = (1..=worker).map(|i| {
    let pb = m.add(ProgressBar::new(projects_count));
    pb.set_style(spinner_style.clone());
    pb.set_prefix(&format!("[{}/{}]", i, worker));
    pb.set_message("initializing...");
    pb.tick();
    pb.enable_steady_tick(250);
    pb
  });
  for pb in progress_bars {
    let job_q = Arc::clone(&q);
    let job_config = Arc::clone(&config);
    let job_logger = logger.clone();
    let job_result_queue = Arc::clone(&job_results);
    thread::spawn(move || {
      let mut job_result: Result<(), AppError> = Result::Ok(());
      loop {
        if let Some(project) = job_q.try_pop() {
          pb.set_message(&project.name);
          let sync_result = sync_project(&job_config, &project, &job_logger, only_new, ff_merge);
          job_result = job_result.and(sync_result);
        } else {
          pb.finish_with_message("waiting...");
          break;
        }
      }
      job_result_queue.push(job_result);
    });
  }
  m.join_and_clear().unwrap();

  let mut synchronize_result: Result<(), AppError> = Result::Ok(());
  while let Some(result) = job_results.try_pop() {
    synchronize_result = synchronize_result.and(result);
  }
  synchronize_result
}

fn ssh_agent_running() -> bool {
  match std::env::var("SSH_AUTH_SOCK") {
    Ok(auth_socket) => std::fs::metadata(auth_socket).map(|m| m.file_type().is_socket()).unwrap_or(false),
    Err(_) => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use spectral::prelude::*;

  #[test]
  fn test_username_from_git_url() {
    let user = env::var("USER").unwrap();
    assert_that(&username_from_git_url(&"git+ssh://git@fkbr.org:sxoe.git")).is_equal_to("git".to_string());
    assert_that(&username_from_git_url(&"ssh://aur@aur.archlinux.org/fw.git")).is_equal_to("aur".to_string());
    assert_that(&username_from_git_url(&"aur@github.com:21re/fkbr.git")).is_equal_to("aur".to_string());
    assert_that(&username_from_git_url(&"aur_fkbr_1@github.com:21re/fkbr.git")).is_equal_to("aur_fkbr_1".to_string());
    assert_that(&username_from_git_url(&"github.com:21re/fkbr.git")).is_equal_to(user.to_string());
    assert_that(&username_from_git_url(&"git://fkbr.org/sxoe.git")).is_equal_to(user.to_string());

    assert_that(&username_from_git_url(&"https://github.com/brocode/fw.git")).is_equal_to(user.to_string());
    assert_that(&username_from_git_url(&"https://kuci@github.com/brocode/fw.git")).is_equal_to("kuci".to_string());
  }
}
