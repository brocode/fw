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

use clap::{App, AppSettings, Arg, SubCommand};
use errors::AppError;
use slog::{DrainExt, Level, LevelFilter, Logger};
use std::time::SystemTime;

fn logger_from_verbosity(verbosity: u64, quiet: &bool) -> Logger {
  let log_level: Level = match verbosity {
  _ if *quiet => Level::Warning,
  0 => Level::Info,
  1 => Level::Debug,
  2 | _ => Level::Trace,
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
    .arg(Arg::with_name("q").short("q").help("Make fw quiet"))
    .subcommand(SubCommand::with_name("sync").about("Sync workspace"))
    .subcommand(SubCommand::with_name("setup")
                  .about("Setup config from existing workspace")
                  .arg(Arg::with_name("WORKSPACE_DIR")
                         .value_name("WORKSPACE_DIR")
                         .index(1)
                         .required(true)))
    .subcommand(SubCommand::with_name("add")
                  .about("Add project to config")
                  .arg(Arg::with_name("NAME")
                         .value_name("NAME")
                         .index(1)
                         .required(true))
                  .arg(Arg::with_name("URL")
                         .value_name("URL")
                         .index(2)
                         .required(true)))
    .subcommand(SubCommand::with_name("foreach")
                  .about("Run script on each project")
                  .arg(Arg::with_name("CMD")
                         .value_name("CMD")
                         .index(1)
                         .required(true)))
    .subcommand(SubCommand::with_name("projectile").about("Write projectile bookmarks"))
    .subcommand(SubCommand::with_name("ls").about("List projects"))
    .subcommand(SubCommand::with_name("gen-workon")
                  .about("Generate sourceable shell code to work on project")
                  .arg(Arg::with_name("PROJECT_NAME")
                         .value_name("PROJECT_NAME")
                         .index(1)
                         .required(true)))
    .get_matches();

  let logger = logger_from_verbosity(matches.occurrences_of("v"), &matches.is_present("q"));
  let config = config::get_config();

  let subcommand_name = matches.subcommand_name()
                               .expect("subcommand required by clap.rs")
                               .to_owned();
  let subcommand_matches = matches.subcommand_matches(&subcommand_name)
                                  .expect("subcommand matches enforced by clap.rs");
  let subcommand_logger = logger.new(o!("command" => subcommand_name.clone()));
  let now = SystemTime::now();
  let result: Result<String, AppError> = match subcommand_name.as_ref() {
                                         "sync" => sync::synchronize(config, &subcommand_logger),
                                         "add" => {
                                           config::add_entry(config,
                                                             subcommand_matches.value_of("NAME")
                                                                               .expect("argument required by clap.rs"),
                                                             subcommand_matches.value_of("URL")
                                                                               .expect("argument required by clap.rs"),
                                                             &subcommand_logger)
                                         }
                                         "setup" => {
                                           setup::setup(subcommand_matches.value_of("WORKSPACE_DIR")
                                                                          .expect("argument required by clap.rs"),
                                                        &subcommand_logger)
                                         }
                                         "gen-workon" => {
                                           workon::gen(subcommand_matches.value_of("PROJECT_NAME")
                                                                         .expect("argument required by clap.rs"),
                                                       config)
                                         }
                                         "projectile" => projectile::projectile(config, &subcommand_logger),
                                         "foreach" => {
                                           sync::foreach(config,
                                                         subcommand_matches.value_of("CMD")
                                                                           .expect("argument required by clap.rs"),
                                                         &subcommand_logger)
                                         }
                                         "ls" => workon::ls(config),
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
mod workon;
mod projectile;
