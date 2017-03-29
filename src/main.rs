extern crate clap;

#[macro_use]
extern crate slog;
extern crate slog_term;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate git2;

extern crate rayon;

extern crate core;

use slog::{Level, Logger, LevelFilter, DrainExt};
use clap::{Arg, App, SubCommand, AppSettings};
use errors::AppError;
use std::time::SystemTime;

fn logger_from_verbosity(verbosity: &u64) -> Logger {
  let log_level: Level = match *verbosity {
    0 => Level::Warning,
    1 => Level::Info,
    2 => Level::Debug,
    3 | _ => Level::Trace,
  };

  let drain = slog_term::StreamerBuilder::new()
    .auto_color()
    .stdout()
    .build();
  let filter = LevelFilter::new(drain, log_level);
  let logger = Logger::root(filter.fuse(), o!());
  debug!(logger, "Logger ready" ; "level" => format!("{:?}", log_level));
  logger
}

fn main() {
  let matches = App::new("fw")
    .version("0.1")
    .author("Maximilien Riehl <max@flatmap.ninja>")
    .about("fast workspace manager")
    .setting(AppSettings::SubcommandRequired)
    .arg(Arg::with_name("v")
           .short("v")
           .multiple(true)
           .help("Sets the level of verbosity"))
    .subcommand(SubCommand::with_name("sync").about("Sync workspace"))
    .subcommand(SubCommand::with_name("setup")
                  .about("Setup config from existing workspace")
                  .arg(Arg::with_name("WORKSPACE_DIR")
                         .value_name("WORKSPACE_DIR")
                         .index(1)
                         .required(true)))
    .get_matches();

  let logger = logger_from_verbosity(&matches.occurrences_of("v"));
  let config = config::get_config();

  let subcommand_name = matches
    .subcommand_name()
    .expect("subcommand required by clap.rs")
    .to_owned();
  let subcommand_matches = matches
    .subcommand_matches(&subcommand_name)
    .expect("subcommand matches enforced by clap.rs");
  let subcommand_logger = logger.new(o!("command" => subcommand_name.clone()));
  let now = SystemTime::now();
  let result: Result<String, AppError> = match subcommand_name.as_ref() {
      "sync" => sync::synchronize(config, &subcommand_logger),
      "setup" => {
        setup::setup(subcommand_matches
                       .value_of("WORKSPACE_DIR")
                       .expect("argument required by clap.rs"),
                     &subcommand_logger)
      }
      _ => Result::Err(AppError::InternalError("Command not implemented")),
    }
    .and_then(|_| now.elapsed().map_err(|e| AppError::ClockError(e)))
    .map(|duration| format!("{}sec", duration.as_secs()));

  match result {
    Ok(time) => info!(subcommand_logger, "Done"; "time" => time),
    Err(error) => {
      crit!(subcommand_logger, "Error running command"; "error" => format!("{:?}", error));
      std::process::exit(1)
    }

  }
}

mod errors;
mod config;
mod sync;
mod setup;
