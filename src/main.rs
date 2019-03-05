extern crate clap;

#[macro_use]
extern crate serde_derive;

#[cfg(test)]
extern crate maplit;

#[cfg(test)]
extern crate spectral;

use crate::errors::AppError;
use slog::{crit, debug, o};
use slog::{Drain, Level, LevelFilter, Logger};
use std::str::FromStr;
use std::time::SystemTime;

fn logger_from_verbosity(verbosity: u64, quiet: bool) -> Logger {
  let log_level: Level = match verbosity {
    _ if quiet => Level::Error,
    0 => Level::Warning,
    1 => Level::Info,
    2 => Level::Debug,
    3 | _ => Level::Trace,
  };

  let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
  let drain = slog_term::FullFormat::new(plain).build().fuse();
  let drain = LevelFilter::new(drain, log_level).fuse();
  let logger = Logger::root(drain, o!());
  debug!(logger, "Logger ready" ; "level" => format!("{:?}", log_level));
  logger
}

fn main() {
  openssl_probe::init_ssl_cert_env_vars();
  let return_code = _main();
  std::process::exit(return_code)
}

fn _main() -> i32 {
  let matches = cli::build_cli().get_matches();

  let logger = logger_from_verbosity(matches.occurrences_of("v"), matches.is_present("q"));
  let config = config::get_config(&logger);

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
        worker,
        &subcommand_logger,
      )
    }
    "add-remote" => {
      let name: &str = subcommand_matches.value_of("NAME").expect("argument required by clap.rs");
      let remote_name: &str = subcommand_matches.value_of("REMOTE_NAME").expect("argument required by clap.rs");
      let url: &str = subcommand_matches.value_of("URL").expect("argument required by clap.rs");
      config::add_remote(config, name, remote_name.to_string(), url.to_string(), &subcommand_logger)
    }
    "remove-remote" => {
      let name: &str = subcommand_matches.value_of("NAME").expect("argument required by clap.rs");
      let remote_name: &str = subcommand_matches.value_of("REMOTE_NAME").expect("argument required by clap.rs");
      config::remove_remote(config, name, remote_name.to_string(), &subcommand_logger)
    }
    "add" => {
      let name: Option<&str> = subcommand_matches.value_of("NAME");
      let url: &str = subcommand_matches.value_of("URL").expect("argument required by clap.rs");
      let after_workon: Option<String> = subcommand_matches.value_of("after-workon").map(str::to_string);
      let after_clone: Option<String> = subcommand_matches.value_of("after-clone").map(str::to_string);
      let override_path: Option<String> = subcommand_matches.value_of("override-path").map(str::to_string);
      config::add_entry(config, name, url, after_workon, after_clone, override_path, &subcommand_logger)
    }
    "remove" => config::remove_entry(
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
      config::update_entry(config, name, git, after_workon, after_clone, override_path, &subcommand_logger)
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
    "gitlab-import" => setup::gitlab_import(config, &subcommand_logger),
    "gen-workon" => workon::gen(
      subcommand_matches.value_of("PROJECT_NAME").expect("argument required by clap.rs"),
      config,
      subcommand_matches.is_present("quick"),
      &subcommand_logger,
    ),
    "gen-reworkon" => workon::gen_reworkon(config, &subcommand_logger),
    "reworkon" => workon::reworkon(config, &subcommand_logger),
    "inspect" => workon::inspect(
      subcommand_matches.value_of("PROJECT_NAME").expect("argument required by clap.rs"),
      config,
      subcommand_matches.is_present("json"),
      &subcommand_logger,
    ),
    "projectile" => projectile::projectile(config, &subcommand_logger),
    "print-path" => workon::print_path(
      config,
      subcommand_matches.value_of("PROJECT_NAME").expect("argument required by clap.rs"),
      &subcommand_logger,
    ),
    "export-project" => export::export_project(config, subcommand_matches.value_of("PROJECT_NAME").expect("argument required by clap.rs")),
    "export-by-tag" => {
      let tag_name: &str = subcommand_matches.value_of("tag_name").expect("argument required by clap.rs");
      export::export_tagged_projects(config, tag_name)
    }
    "export-tag" => {
      let tag_name: &str = subcommand_matches.value_of("tag_name").expect("argument required by clap.rs");
      export::export_tag(config, tag_name)
    }
    "foreach" => sync::foreach(
      config,
      subcommand_matches.value_of("CMD").expect("argument required by clap.rs"),
      &subcommand_matches.values_of_lossy("tag").unwrap_or_default().into_iter().collect(),
      &subcommand_logger,
      &subcommand_matches.value_of("parallel").map(|p| p.to_owned()),
    ),
    "print-zsh-setup" => print_zsh_setup(subcommand_matches.is_present("with-fzf")),
    "print-bash-setup" => print_bash_setup(subcommand_matches.is_present("with-fzf")),
    "tag" => {
      let subsubcommand_name: String = subcommand_matches.subcommand_name().expect("subcommand matches enforced by clap.rs").to_owned();
      let subsubcommand_matches: clap::ArgMatches = subcommand_matches
        .subcommand_matches(&subsubcommand_name)
        .expect("subcommand matches enforced by clap.rs")
        .to_owned();
      execute_tag_subcommand(config, &subsubcommand_name, &subsubcommand_matches, &subcommand_logger)
    }
    "ls" => workon::ls(config),
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
      let maybe_project_name: Option<String> = tag_matches.value_of("PROJECT_NAME").map(str::to_string);
      tag::list_tags(maybe_config, maybe_project_name, logger)
    }
    "tag-project" => {
      let project_name: String = tag_matches.value_of("PROJECT_NAME").map(str::to_string).expect("argument enforced by clap.rs");
      let tag_name: String = tag_matches.value_of("tag-name").map(str::to_string).expect("argument enforced by clap.rs");
      tag::add_tag(maybe_config, project_name, tag_name, logger)
    }
    "untag-project" => {
      let project_name: String = tag_matches.value_of("PROJECT_NAME").map(str::to_string).expect("argument enforced by clap.rs");
      let tag_name: String = tag_matches.value_of("tag-name").map(str::to_string).expect("argument enforced by clap.rs");
      tag::remove_tag(maybe_config, project_name, &tag_name, logger)
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
    "autotag" => sync::autotag(
      maybe_config,
      tag_matches.value_of("CMD").expect("argument required by clap.rs"),
      &tag_matches.value_of("tag-name").map(str::to_string).expect("argument enforced by clap.rs"),
      &logger,
      &tag_matches.value_of("parallel").map(|p| p.to_owned()),
    ),
    _ => Result::Err(AppError::InternalError("Command not implemented")),
  }
}

fn print_zsh_setup(use_fzf: bool) -> Result<(), AppError> {
  let fw_completion = include_str!("shell/setup.zsh");
  let basic_workon = include_str!("shell/workon.zsh");
  let fzf_workon = include_str!("shell/workon-fzf.zsh");
  println!("{}", fw_completion);
  if use_fzf {
    println!("{}", fzf_workon);
  } else {
    println!("{}", basic_workon);
  }
  Ok(())
}

fn print_bash_setup(use_fzf: bool) -> Result<(), AppError> {
  let setup = include_str!("shell/setup.bash");
  let basic = include_str!("shell/workon.bash");
  let fzf = include_str!("shell/workon-fzf.bash");

  println!("{}", setup);
  cli::build_cli().gen_completions_to("fw", clap::Shell::Bash, &mut std::io::stdout());
  if use_fzf {
    println!("{}", fzf);
  } else {
    println!("{}", basic);
  }

  Ok(())
}

mod cli;
mod config;
mod errors;
mod export;
mod projectile;
mod setup;
mod sync;
mod tag;
mod workon;
mod ws;
