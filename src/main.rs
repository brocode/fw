
use crate::errors::AppError;
use crate::util::logger_from_verbosity;
use slog::Logger;
use slog::{crit, debug, o, warn};
use std::str::FromStr;
use std::time::SystemTime;

fn main() {
  openssl_probe::init_ssl_cert_env_vars();
  let return_code = _main();
  std::process::exit(return_code)
}

fn _main() -> i32 {
  let matches = crate::app::app().get_matches();

  let logger = logger_from_verbosity(matches.occurrences_of("v"), matches.is_present("q"));

  let config = config::read_config(&logger);
  if config.is_err() {
    warn!(
      logger,
      "Could not read v2.0 config: {:?}. If you are running the setup right now this is expected.", config
    );
  };

  let subcommand_name = matches.subcommand_name().expect("subcommand required by clap.rs").to_owned();
  let subcommand_matches = matches.subcommand_matches(&subcommand_name).expect("subcommand matches enforced by clap.rs");
  let subcommand_logger = logger.new(o!("command" => subcommand_name.clone()));

  let now = SystemTime::now();
  let result: Result<String, AppError> = match subcommand_name.as_ref() {
    "sync" => {
      let worker = subcommand_matches
        .value_of("parallelism")
        .and_then(|i| i.parse::<i32>().ok())
        .expect("enforced by clap.rs");

      sync::synchronize(
        config,
        subcommand_matches.is_present("no-progress-bar"),
        subcommand_matches.is_present("only-new"),
        !subcommand_matches.is_present("no-fast-forward-merge"),
        &subcommand_matches.values_of_lossy("tag").unwrap_or_default().into_iter().collect(),
        worker,
        &subcommand_logger,
      )
    }
    "add-remote" => {
      let name: &str = subcommand_matches.value_of("NAME").expect("argument required by clap.rs");
      let remote_name: &str = subcommand_matches.value_of("REMOTE_NAME").expect("argument required by clap.rs");
      let url: &str = subcommand_matches.value_of("URL").expect("argument required by clap.rs");
      project::add_remote(config, name, remote_name.to_string(), url.to_string())
    }
    "remove-remote" => {
      let name: &str = subcommand_matches.value_of("NAME").expect("argument required by clap.rs");
      let remote_name: &str = subcommand_matches.value_of("REMOTE_NAME").expect("argument required by clap.rs");
      project::remove_remote(config, name, remote_name.to_string(), &subcommand_logger)
    }
    "add" => {
      let name: Option<&str> = subcommand_matches.value_of("NAME");
      let url: &str = subcommand_matches.value_of("URL").expect("argument required by clap.rs");
      let after_workon: Option<String> = subcommand_matches.value_of("after-workon").map(str::to_string);
      let after_clone: Option<String> = subcommand_matches.value_of("after-clone").map(str::to_string);
      let override_path: Option<String> = subcommand_matches.value_of("override-path").map(str::to_string);
      project::add_entry(config, name, url, after_workon, after_clone, override_path, &subcommand_logger)
    }
    "remove" => project::remove_project(
      config,
      subcommand_matches.value_of("NAME").expect("argument required by clap.rs"),
      subcommand_matches.is_present("purge-directory"),
      &subcommand_logger,
    ),
    "update" => {
      let name: &str = subcommand_matches.value_of("NAME").expect("argument required by clap.rs");
      let git: Option<String> = subcommand_matches.value_of("git").map(str::to_string);
      let after_workon: Option<String> = subcommand_matches.value_of("after-workon").map(str::to_string);
      let after_clone: Option<String> = subcommand_matches.value_of("after-clone").map(str::to_string);
      let override_path: Option<String> = subcommand_matches.value_of("override-path").map(str::to_string);
      project::update_entry(config, name, git, after_workon, after_clone, override_path, &subcommand_logger)
    }
    "setup" => setup::setup(
      subcommand_matches.value_of("WORKSPACE_DIR").expect("argument required by clap.rs"),
      &subcommand_logger,
    ),
    "import" => setup::import(
      config,
      subcommand_matches.value_of("PROJECT_DIR").expect("argument required by clap.rs"),
      &subcommand_logger,
    ),
    "org-import" => setup::org_import(
      config,
      subcommand_matches.value_of("ORG_NAME").expect("argument required by clap.rs"),
      subcommand_matches.is_present("include-archived"),
      &subcommand_logger,
    ),
    "gitlab-import" => {
      let state = subcommand_matches
        .value_of("include")
        .expect("argument required by clap.rs")
        .parse()
        .expect("argument values restricted by clap.rs");
      setup::gitlab_import(config, state, &subcommand_logger)
    }
    "gen-workon" => workon::gen(
      subcommand_matches.value_of("PROJECT_NAME").expect("argument required by clap.rs"),
      config,
      subcommand_matches.is_present("quick"),
      &subcommand_logger,
    ),
    "gen-reworkon" => workon::gen_reworkon(config, &subcommand_logger),
    "reworkon" => workon::reworkon(config, &subcommand_logger),
    "inspect" => project::inspect(
      subcommand_matches.value_of("PROJECT_NAME").expect("argument required by clap.rs"),
      config,
      subcommand_matches.is_present("json"),
      &subcommand_logger,
    ),
    "projectile" => projectile::projectile(config, &subcommand_logger),
    "print-path" => project::print_path(
      config,
      subcommand_matches.value_of("PROJECT_NAME").expect("argument required by clap.rs"),
      &subcommand_logger,
    ),
    "foreach" => spawn::foreach(
      config,
      subcommand_matches.value_of("CMD").expect("argument required by clap.rs"),
      &subcommand_matches.values_of_lossy("tag").unwrap_or_default().into_iter().collect(),
      &subcommand_logger,
      &subcommand_matches.value_of("parallel").map(ToOwned::to_owned),
    ),
    "print-zsh-setup" => crate::shell::print_zsh_setup(subcommand_matches.is_present("with-fzf")),
    "print-bash-setup" => crate::shell::print_bash_setup(subcommand_matches.is_present("with-fzf")),
    "print-fish-setup" => crate::shell::print_fish_setup(subcommand_matches.is_present("with-fzf")),
    "tag" => {
      let subsubcommand_name: String = subcommand_matches.subcommand_name().expect("subcommand matches enforced by clap.rs").to_owned();
      let subsubcommand_matches: clap::ArgMatches<'_> = subcommand_matches
        .subcommand_matches(&subsubcommand_name)
        .expect("subcommand matches enforced by clap.rs")
        .to_owned();
      execute_tag_subcommand(config, &subsubcommand_name, &subsubcommand_matches, &subcommand_logger)
    }
    "ls" => project::ls(config, &subcommand_matches.values_of_lossy("tag").unwrap_or_default().into_iter().collect()),
    _ => Err(AppError::InternalError("Command not implemented")),
  }
  .and_then(|_| now.elapsed().map_err(AppError::ClockError))
  .map(|duration| format!("{}sec", duration.as_secs()));

  match result {
    Ok(time) => {
      debug!(subcommand_logger, "Done"; "time" => time);
      0
    }
    Err(error) => {
      crit!(subcommand_logger, "Error running command"; "error" => format!("{:?}", error));
      1
    }
  }
}

fn execute_tag_subcommand(
  maybe_config: Result<config::Config, AppError>,
  tag_command_name: &str,
  tag_matches: &clap::ArgMatches<'_>,
  logger: &Logger,
) -> Result<(), AppError> {
  match tag_command_name {
    "ls" => {
      let maybe_project_name: Option<String> = tag_matches.value_of("PROJECT_NAME").map(str::to_string);
      tag::list_tags(maybe_config, maybe_project_name, logger)
    }
    "tag-project" => {
      let project_name: String = tag_matches.value_of("PROJECT_NAME").map(str::to_string).expect("argument enforced by clap.rs");
      let tag_name: String = tag_matches.value_of("tag-name").map(str::to_string).expect("argument enforced by clap.rs");
      tag::add_tag(&maybe_config?, project_name, tag_name, logger)
    }
    "untag-project" => {
      let project_name: String = tag_matches.value_of("PROJECT_NAME").map(str::to_string).expect("argument enforced by clap.rs");
      let tag_name: String = tag_matches.value_of("tag-name").map(str::to_string).expect("argument enforced by clap.rs");
      tag::remove_tag(maybe_config, project_name, &tag_name, logger)
    }
    "inspect" => {
      let tag_name: String = tag_matches.value_of("tag-name").map(str::to_string).expect("argument enforced by clap.rs");
      tag::inspect_tag(maybe_config, &tag_name)
    }
    "rm" => {
      let tag_name: String = tag_matches.value_of("tag-name").map(str::to_string).expect("argument enforced by clap.rs");
      tag::delete_tag(maybe_config, &tag_name, logger)
    }
    "add" => {
      let tag_name: String = tag_matches.value_of("tag-name").map(str::to_string).expect("argument enforced by clap.rs");
      let after_workon: Option<String> = tag_matches.value_of("after-workon").map(str::to_string);
      let after_clone: Option<String> = tag_matches.value_of("after-clone").map(str::to_string);
      let tag_workspace: Option<String> = tag_matches.value_of("workspace").map(str::to_string);
      let priority: Option<u8> = tag_matches
        .value_of("priority")
        .map(u8::from_str)
        .map(|p| p.expect("invalid tag priority value, must be an u8"));
      tag::create_tag(maybe_config, tag_name, after_workon, after_clone, priority, tag_workspace, logger)
    }
    "autotag" => tag::autotag(
      maybe_config,
      tag_matches.value_of("CMD").expect("argument required by clap.rs"),
      &tag_matches.value_of("tag-name").map(str::to_string).expect("argument enforced by clap.rs"),
      &logger,
      &tag_matches.value_of("parallel").map(ToOwned::to_owned),
    ),
    _ => Result::Err(AppError::InternalError("Command not implemented")),
  }
}

mod app;
mod config;
mod errors;
mod git;
mod project;
mod projectile;
mod setup;
mod shell;
mod spawn;
mod sync;
mod tag;
mod util;
mod workon;
mod ws;
