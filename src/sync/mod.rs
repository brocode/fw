use crate::config;
use crate::config::metadata_from_repository::MetadataFromRepository;
use crate::config::{Config, project::Project};
use crate::errors::AppError;
use std::collections::BTreeSet;
use std::fs::read_to_string;
use std::path::Path;
use std::time::Duration;

use crate::git::{clone_project, update_project_remotes};

use crossbeam::queue::SegQueue;

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};

use std::borrow::ToOwned;

use std::sync::Arc;
use std::thread;

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;

fn sync_project(config: &Config, project: &Project, only_new: bool, ff_merge: bool) -> Result<(), AppError> {
	let path = config.actual_path_to_project(project);
	let exists = path.exists();
	let result = if exists {
		if only_new {
			Ok(())
		} else {
			update_project_remotes(project, &path, ff_merge).and_then(|_| synchronize_metadata_if_trusted(project, &path))
		}
	} else {
		clone_project(config, project, &path).and_then(|_| synchronize_metadata_if_trusted(project, &path))
	};
	result.map_err(|e| AppError::RuntimeError(format!("Failed to sync {}: {}", project.name, e)))
}

pub fn synchronize_metadata_if_trusted(project: &Project, path: &Path) -> Result<(), AppError> {
	if !project.trusted {
		Ok(())
	} else {
		let metadata_file = path.join("fw.toml");

		if metadata_file.exists() {
			let content = read_to_string(metadata_file)?;
			let metadata_from_repository = toml::from_str::<MetadataFromRepository>(&content)?;

			let new_project = Project {
				tags: metadata_from_repository.tags,
				..project.to_owned()
			};

			config::write_project(&new_project)
		} else {
			Ok(())
		}
	}
}

pub fn synchronize(maybe_config: Result<Config, AppError>, only_new: bool, ff_merge: bool, tags: &BTreeSet<String>, worker: i32) -> Result<(), AppError> {
	eprintln!("Synchronizing everything");
	if !ssh_agent_running() {
		eprintln!("SSH Agent not running. Process may hang.")
	}
	let config = Arc::new(maybe_config?);

	let projects: Vec<Project> = config.projects.values().map(ToOwned::to_owned).collect();
	let q: Arc<SegQueue<Project>> = Arc::new(SegQueue::new());
	let projects_count = projects.len() as u64;

	projects
		.into_iter()
		.filter(|p| tags.is_empty() || p.tags.clone().unwrap_or_default().intersection(tags).count() > 0)
		.for_each(|p| q.push(p));

	let spinner_style = ProgressStyle::default_spinner()
		.tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷⣿")
		.template("{prefix:.bold.dim} {spinner} {wide_msg}")
		.map_err(|e| AppError::RuntimeError(format!("Invalid Template: {e}")))?;

	let m = MultiProgress::new();
	m.set_draw_target(ProgressDrawTarget::stderr());

	let job_results: Arc<SegQueue<Result<(), AppError>>> = Arc::new(SegQueue::new());
	let progress_bars = (1..=worker).map(|i| {
		let pb = m.add(ProgressBar::new(projects_count));
		pb.set_style(spinner_style.clone());
		pb.set_prefix(format!("[{i: >2}/{worker}]"));
		pb.set_message("initializing...");
		pb.tick();
		pb.enable_steady_tick(Duration::from_millis(250));
		pb
	});
	let mut thread_handles: Vec<thread::JoinHandle<()>> = Vec::new();
	for pb in progress_bars {
		let job_q = Arc::clone(&q);
		let job_config = Arc::clone(&config);
		let job_result_queue = Arc::clone(&job_results);
		thread_handles.push(thread::spawn(move || {
			let mut job_result: Result<(), AppError> = Result::Ok(());
			loop {
				if let Some(project) = job_q.pop() {
					pb.set_message(project.name.to_string());
					let sync_result = sync_project(&job_config, &project, only_new, ff_merge);
					let msg = match sync_result {
						Ok(_) => format!("DONE: {}", project.name),
						Err(ref e) => format!("FAILED: {} - {}", project.name, e),
					};
					pb.println(&msg);
					job_result = job_result.and(sync_result);
				} else {
					pb.finish_and_clear();
					break;
				}
			}
			job_result_queue.push(job_result);
		}));
	}

	while let Some(cur_thread) = thread_handles.pop() {
		cur_thread.join().unwrap();
	}

	let mut synchronize_result: Result<(), AppError> = Result::Ok(());
	while let Some(result) = job_results.pop() {
		synchronize_result = synchronize_result.and(result);
	}

	m.clear().unwrap();

	synchronize_result
}

fn ssh_agent_running() -> bool {
	match std::env::var("SSH_AUTH_SOCK") {
		Ok(auth_socket) => is_socket(&auth_socket),
		Err(_) => false,
	}
}

#[cfg(unix)]
fn is_socket(path: &str) -> bool {
	std::fs::metadata(path).map(|m| m.file_type().is_socket()).unwrap_or(false)
}

#[cfg(not(unix))]
fn is_socket(_: &str) -> bool {
	false
}
