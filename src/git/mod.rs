use crate::config::{project::Project, Config};
use crate::errors::AppError;

use crate::spawn::spawn_maybe;
use crate::util::random_color;

use git2::build::RepoBuilder;
use git2::{AutotagOption, Branch, Direction, FetchOptions, MergeAnalysis, ProxyOptions, Remote, RemoteCallbacks, Repository};

use std::borrow::ToOwned;

use std::path::Path;

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

fn agent_callbacks() -> git2::RemoteCallbacks<'static> {
    let mut remote_callbacks = RemoteCallbacks::new();
    remote_callbacks.credentials(move |_url, username_from_url, _allowed_types| git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git")));
    remote_callbacks
}

fn agent_fetch_options() -> git2::FetchOptions<'static> {
    let remote_callbacks = agent_callbacks();
    let mut proxy_options = ProxyOptions::new();
    proxy_options.auto();
    let mut fetch_options = FetchOptions::new();
    fetch_options.remote_callbacks(remote_callbacks);
    fetch_options.proxy_options(proxy_options);

    fetch_options
}

fn builder() -> RepoBuilder<'static> {
    let options = agent_fetch_options();
    let mut repo_builder = RepoBuilder::new();
    repo_builder.fetch_options(options);
    repo_builder
}

fn update_remote(remote: &mut Remote<'_>) -> Result<(), AppError> {
    let remote_callbacks = agent_callbacks();
    let mut proxy_options = ProxyOptions::new();
    proxy_options.auto();
    remote
        .connect_auth(Direction::Fetch, Some(remote_callbacks), Some(proxy_options))
        .map_err(AppError::GitError)?;
    let mut options = agent_fetch_options();
    remote.download::<String>(&[], Some(&mut options)).map_err(AppError::GitError)?;
    remote.disconnect()?;
    remote.update_tips(None, true, AutotagOption::Unspecified, None)?;
    Ok(())
}

pub fn update_project_remotes(project: &Project, path: &Path, ff_merge: bool) -> Result<(), AppError> {
    let local: Repository = Repository::open(path).map_err(AppError::GitError)?;
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

        update_remote(&mut remote)?;
    }

    if ff_merge {
        // error does not matter. fast forward not possible
        let _ = fast_forward_merge(&local);
    }

    Ok(())
}

fn fast_forward_merge(local: &Repository) -> Result<(), AppError> {
    let head_ref = local.head()?;
    if head_ref.is_branch() {
        let branch = Branch::wrap(head_ref);
        let upstream = branch.upstream()?;
        let upstream_commit = local.reference_to_annotated_commit(upstream.get())?;

        let (analysis_result, _) = local.merge_analysis(&[&upstream_commit])?;
        if MergeAnalysis::is_fast_forward(&analysis_result) {
            let target_id = upstream_commit.id();
            local.checkout_tree(&local.find_object(upstream_commit.id(), None)?, None)?;
            local.head()?.set_target(target_id, "fw fast-forward")?;
        }
    }
    Ok(())
}

pub fn clone_project(config: &Config, project: &Project, path: &Path) -> Result<(), AppError> {
    let shell = config.settings.get_shell_or_default();
    let mut repo_builder = builder();
    repo_builder
        .bare(project.bare.unwrap_or_default())
        .clone(project.git.as_str(), path)
        .map_err(AppError::GitError)
        .and_then(|repo| init_additional_remotes(project, repo))
        .and_then(|_| {
            let after_clone = config.resolve_after_clone(project);
            if !after_clone.is_empty() {
                spawn_maybe(&shell, &after_clone.join(" && "), path, &project.name, random_color())
                    .map_err(|error| AppError::UserError(format!("Post-clone hook failed (nonzero exit code). Cause: {:?}", error)))
            } else {
                Ok(())
            }
        })
}

fn init_additional_remotes(project: &Project, repository: Repository) -> Result<(), AppError> {
    if let Some(additional_remotes) = &project.additional_remotes {
        for remote in additional_remotes {
            let mut git_remote = repository.remote(&remote.name, &remote.git)?;
            update_remote(&mut git_remote)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_name_from_url() {
        let https_url = "https://github.com/mriehl/fw";
        let name = repo_name_from_url(https_url).unwrap().to_owned();
        assert_eq!(name, "fw".to_owned());
    }
    #[test]
    fn test_repo_name_from_ssh_pragma() {
        let ssh_pragma = "git@github.com:mriehl/fw.git";
        let name = repo_name_from_url(ssh_pragma).unwrap().to_owned();
        assert_eq!(name, "fw".to_owned());
    }
    #[test]
    fn test_repo_name_from_ssh_pragma_with_multiple_git_endings() {
        let ssh_pragma = "git@github.com:mriehl/fw.git.git";
        let name = repo_name_from_url(ssh_pragma).unwrap().to_owned();
        assert_eq!(name, "fw.git".to_owned());
    }
}
