use crate::config::{project::Project, Config};
use crate::errors::AppError;

use crate::spawn::spawn_maybe;
use crate::util::random_colour;

use git2;
use git2::build::RepoBuilder;
use git2::{AutotagOption, Branch, Direction, FetchOptions, MergeAnalysis, ProxyOptions, Remote, RemoteCallbacks, Repository};

use regex::Regex;
use slog::Logger;
use slog::{debug, warn};
use std;
use std::borrow::ToOwned;

use std::env;

use std::path::PathBuf;

pub fn repo_name_from_url(url: &str) -> Result<&str, AppError> {
  let last_fragment = url.rsplit('/').next().ok_or_else(|| {
    AppError::UserError(format!(
      "Given URL {} does not have path fragments so cannot determine project name. Please give \
       one.",
      url
    ))
  })?;

  // trim_right_matches is more efficient but would fuck us up with repos like git@github.com:bauer/test.git.git (which is legal)
  Ok(if last_fragment.ends_with(".git") {
    last_fragment.split_at(last_fragment.len() - 4).0
  } else {
    last_fragment
  })
}

fn username_from_git_url(url: &str) -> String {
  let url_regex = Regex::new(r"([^:]+://)?((?P<user>[a-z_][a-z0-9_]{0,30})@)?").unwrap();
  if let Some(caps) = url_regex.captures(url) {
    if let Some(user) = caps.name("user") {
      return user.as_str().to_string();
    }
  }
  if let Ok(user) = env::var("USER") {
    if user != "" {
      return user;
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
  let mut proxy_options = ProxyOptions::new();
  proxy_options.auto();
  let mut fetch_options = FetchOptions::new();
  fetch_options.remote_callbacks(remote_callbacks);
  fetch_options.proxy_options(proxy_options);

  fetch_options
}

fn builder(git_user: &str) -> RepoBuilder {
  let options = agent_fetch_options(git_user);
  let mut repo_builder = RepoBuilder::new();
  repo_builder.fetch_options(options);
  repo_builder
}

fn update_remote(project: &Project, remote: &mut Remote, project_logger: &Logger) -> Result<(), AppError> {
  let git_user = username_from_git_url(&project.git);
  let remote_callbacks = agent_callbacks(&git_user);
  let mut proxy_options = ProxyOptions::new();
  proxy_options.auto();
  remote
    .connect_auth(Direction::Fetch, Some(remote_callbacks), Some(proxy_options))
    .map_err(|error| {
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

pub fn update_project_remotes(project: &Project, path: &PathBuf, project_logger: &Logger, ff_merge: bool) -> Result<(), AppError> {
  debug!(project_logger, "Update project remotes");
  let local: Repository = Repository::open(path).map_err(|error| {
    warn!(project_logger, "Error opening local repo"; "error" => format!("{}", error));
    AppError::GitError(error)
  })?;
  for desired_remote in project.additional_remotes.clone().unwrap_or_default().into_iter().chain(
    vec![crate::config::project::Remote {
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

pub fn clone_project(config: &Config, project: &Project, path: &PathBuf, project_logger: &Logger) -> Result<(), AppError> {
  let shell = config.settings.get_shell_or_default();
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

  #[test]
  fn test_repo_name_from_url() {
    let https_url = "https://github.com/mriehl/fw";
    let name = repo_name_from_url(&https_url).unwrap().to_owned();
    assert_that(&name).is_equal_to("fw".to_owned());
  }
  #[test]
  fn test_repo_name_from_ssh_pragma() {
    let ssh_pragma = "git@github.com:mriehl/fw.git";
    let name = repo_name_from_url(&ssh_pragma).unwrap().to_owned();
    assert_that(&name).is_equal_to("fw".to_owned());
  }
  #[test]
  fn test_repo_name_from_ssh_pragma_with_multiple_git_endings() {
    let ssh_pragma = "git@github.com:mriehl/fw.git.git";
    let name = repo_name_from_url(&ssh_pragma).unwrap().to_owned();
    assert_that(&name).is_equal_to("fw.git".to_owned());
  }
}
