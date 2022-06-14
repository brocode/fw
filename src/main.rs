use crate::errors::AppError;
use crate::util::logger_from_verbosity;
use setup::ProjectState;
use slog::Logger;
use slog::{crit, debug, o, warn};
use std::collections::BTreeSet;
use std::time::SystemTime;

fn main() {
  openssl_probe::init_ssl_cert_env_vars();
  let return_code = _main();
  std::process::exit(return_code)
}

fn _main() -> i32 {
  let matches = crate::app::app().get_matches();

  let logger = logger_from_verbosity(matches.get_one::<u8>("v").copied().unwrap_or_default().into(), matches.contains_id("q"));

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
      let worker = subcommand_matches.get_one::<i32>("parallelism").expect("enforced by clap.rs").to_owned();

      sync::synchronize(
        config,
        subcommand_matches.contains_id("no-progress-bar"),
        subcommand_matches.contains_id("only-new"),
        !subcommand_matches.contains_id("no-fast-forward-merge"),
        &subcommand_matches
          .get_many::<String>("tag")
          .unwrap_or_default()
          .into_iter()
          .map(ToOwned::to_owned)
          .collect(),
        worker,
        &subcommand_logger,
      )
    }
    "add-remote" => {
      let name: &str = subcommand_matches.get_one::<String>("NAME").expect("argument required by clap.rs");
      let remote_name: &str = subcommand_matches.get_one::<String>("REMOTE_NAME").expect("argument required by clap.rs");
      let url: &str = subcommand_matches.get_one::<String>("URL").expect("argument required by clap.rs");
      project::add_remote(config, name, remote_name.to_string(), url.to_string())
    }
    "remove-remote" => {
      let name: &str = subcommand_matches.get_one::<String>("NAME").expect("argument required by clap.rs");
      let remote_name: &str = subcommand_matches.get_one::<String>("REMOTE_NAME").expect("argument required by clap.rs");
      project::remove_remote(config, name, remote_name.to_string(), &subcommand_logger)
    }
    "add" => {
      let name: Option<String> = subcommand_matches.get_one::<String>("NAME").map(ToOwned::to_owned);
      let url: &str = subcommand_matches.get_one::<String>("URL").expect("argument required by clap.rs");
      let after_workon: Option<String> = subcommand_matches.get_one::<String>("after-workon").map(ToOwned::to_owned);
      let after_clone: Option<String> = subcommand_matches.get_one::<String>("after-clone").map(ToOwned::to_owned);
      let override_path: Option<String> = subcommand_matches.get_one::<String>("override-path").map(ToOwned::to_owned);
      let tags: Option<BTreeSet<String>> = subcommand_matches
        .get_many::<String>("tag")
        .map(|v| v.into_iter().map(ToOwned::to_owned).collect());
      project::add_entry(config, name, url, after_workon, after_clone, override_path, tags, &subcommand_logger)
    }
    "remove" => project::remove_project(
      config,
      subcommand_matches.get_one::<String>("NAME").expect("argument required by clap.rs"),
      subcommand_matches.contains_id("purge-directory"),
      &subcommand_logger,
    ),
    "update" => {
      let name: &str = subcommand_matches.get_one::<String>("NAME").expect("argument required by clap.rs");
      let git: Option<String> = subcommand_matches.get_one::<String>("git").map(ToOwned::to_owned);
      let after_workon: Option<String> = subcommand_matches.get_one::<String>("after-workon").map(ToOwned::to_owned);
      let after_clone: Option<String> = subcommand_matches.get_one::<String>("after-clone").map(ToOwned::to_owned);
      let override_path: Option<String> = subcommand_matches.get_one::<String>("override-path").map(ToOwned::to_owned);
      project::update_entry(config, name, git, after_workon, after_clone, override_path, &subcommand_logger)
    }
    "setup" => setup::setup(
      subcommand_matches.get_one::<String>("WORKSPACE_DIR").expect("argument required by clap.rs"),
      &subcommand_logger,
    ),
    "import" => setup::import(
      config,
      subcommand_matches.get_one::<String>("PROJECT_DIR").expect("argument required by clap.rs"),
      &subcommand_logger,
    ),
    "org-import" => setup::org_import(
      config,
      subcommand_matches.get_one::<String>("ORG_NAME").expect("argument required by clap.rs"),
      subcommand_matches.contains_id("include-archived"),
      &subcommand_logger,
    ),
    "gitlab-import" => {
      let state = *subcommand_matches.get_one::<ProjectState>("include").expect("argument required by clap.rs");
      setup::gitlab_import(config, state, &subcommand_logger)
    }
    "gen-workon" => workon::gen(
      subcommand_matches.get_one::<String>("PROJECT_NAME").expect("argument required by clap.rs"),
      config,
      subcommand_matches.contains_id("quick"),
      &subcommand_logger,
    ),
    "gen-reworkon" => workon::gen_reworkon(config, &subcommand_logger),
    "reworkon" => workon::reworkon(config, &subcommand_logger),
    "inspect" => project::inspect(
      subcommand_matches.get_one::<String>("PROJECT_NAME").expect("argument required by clap.rs"),
      config,
      subcommand_matches.contains_id("json"),
      &subcommand_logger,
    ),
    "projectile" => projectile::projectile(config, &subcommand_logger),
    "intellij" => intellij::intellij(config, &subcommand_logger, !subcommand_matches.contains_id("no-warn")),
    "print-path" => project::print_path(
      config,
      subcommand_matches.get_one::<String>("PROJECT_NAME").expect("argument required by clap.rs"),
      &subcommand_logger,
    ),
    "foreach" => spawn::foreach(
      config,
      subcommand_matches.get_one::<String>("CMD").expect("argument required by clap.rs"),
      &subcommand_matches
        .get_many::<String>("tag")
        .unwrap_or_default()
        .into_iter()
        .map(ToOwned::to_owned)
        .collect(),
      &subcommand_logger,
      &subcommand_matches.get_one::<String>("parallel").map(ToOwned::to_owned),
    ),
    "print-zsh-setup" => crate::shell::print_zsh_setup(subcommand_matches.contains_id("with-fzf"), subcommand_matches.contains_id("with-skim")),
    "print-bash-setup" => crate::shell::print_bash_setup(subcommand_matches.contains_id("with-fzf"), subcommand_matches.contains_id("with-skim")),
    "print-fish-setup" => crate::shell::print_fish_setup(subcommand_matches.contains_id("with-fzf"), subcommand_matches.contains_id("with-skim")),
    "tag" => {
      let subsubcommand_name: String = subcommand_matches.subcommand_name().expect("subcommand matches enforced by clap.rs").to_owned();
      let subsubcommand_matches: clap::ArgMatches = subcommand_matches
        .subcommand_matches(&subsubcommand_name)
        .expect("subcommand matches enforced by clap.rs")
        .to_owned();
      execute_tag_subcommand(config, &subsubcommand_name, &subsubcommand_matches, &subcommand_logger)
    }
    "ls" => project::ls(
      config,
      &subcommand_matches
        .get_many::<String>("tag")
        .unwrap_or_default()
        .into_iter()
        .map(ToOwned::to_owned)
        .collect(),
    ),
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
  tag_matches: &clap::ArgMatches,
  logger: &Logger,
) -> Result<(), AppError> {
  match tag_command_name {
    "ls" => {
      let maybe_project_name: Option<String> = tag_matches.get_one::<String>("PROJECT_NAME").map(ToOwned::to_owned);
      tag::list_tags(maybe_config, maybe_project_name, logger)
    }
    "tag-project" => {
      let project_name: String = tag_matches
        .get_one::<String>("PROJECT_NAME")
        .map(ToOwned::to_owned)
        .expect("argument enforced by clap.rs");
      let tag_name: String = tag_matches
        .get_one::<String>("tag-name")
        .map(ToOwned::to_owned)
        .expect("argument enforced by clap.rs");
      tag::add_tag(&maybe_config?, project_name, tag_name, logger)
    }
    "untag-project" => {
      let project_name: String = tag_matches
        .get_one::<String>("PROJECT_NAME")
        .map(ToOwned::to_owned)
        .expect("argument enforced by clap.rs");
      let tag_name: String = tag_matches
        .get_one::<String>("tag-name")
        .map(ToOwned::to_owned)
        .expect("argument enforced by clap.rs");
      tag::remove_tag(maybe_config, project_name, &tag_name, logger)
    }
    "inspect" => {
      let tag_name: String = tag_matches
        .get_one::<String>("tag-name")
        .map(ToOwned::to_owned)
        .expect("argument enforced by clap.rs");
      tag::inspect_tag(maybe_config, &tag_name)
    }
    "rm" => {
      let tag_name: String = tag_matches
        .get_one::<String>("tag-name")
        .map(ToOwned::to_owned)
        .expect("argument enforced by clap.rs");
      tag::delete_tag(maybe_config, &tag_name, logger)
    }
    "add" => {
      let tag_name: String = tag_matches
        .get_one::<String>("tag-name")
        .map(ToOwned::to_owned)
        .expect("argument enforced by clap.rs");
      let after_workon: Option<String> = tag_matches.get_one::<String>("after-workon").map(ToOwned::to_owned);
      let after_clone: Option<String> = tag_matches.get_one::<String>("after-clone").map(ToOwned::to_owned);
      let tag_workspace: Option<String> = tag_matches.get_one::<String>("workspace").map(ToOwned::to_owned);
      let priority: Option<u8> = tag_matches.get_one::<u8>("priority").map(ToOwned::to_owned);
      tag::create_tag(maybe_config, tag_name, after_workon, after_clone, priority, tag_workspace, logger)
    }
    "autotag" => tag::autotag(
      maybe_config,
      tag_matches.get_one::<String>("CMD").expect("argument required by clap.rs"),
      &tag_matches
        .get_one::<String>("tag-name")
        .map(ToOwned::to_owned)
        .expect("argument enforced by clap.rs"),
      logger,
      &tag_matches.get_one::<String>("parallel").map(ToOwned::to_owned),
    ),
    _ => Result::Err(AppError::InternalError("Command not implemented")),
  }
}

mod app;
mod config;
mod errors;
mod git;
mod intellij;
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
