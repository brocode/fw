#[macro_use]
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

#[cfg(test)]
#[macro_use]
extern crate maplit;

extern crate regex;

#[cfg(test)]
extern crate spectral;

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
    .stderr()
    .build();
  let filter = LevelFilter::new(drain, log_level);
  let logger = Logger::root(filter.fuse(), o!());
  debug!(logger, "Logger ready" ; "level" => format!("{:?}", log_level));
  logger
}

fn main() {
  let matches = App::new("fw")
    .version(crate_version!())
    .author("Maximilien Riehl <max@flatmap.ninja>")
    .about("fast workspace manager")
    .setting(AppSettings::SubcommandRequired)
    .arg(Arg::with_name("v")
           .short("v")
           .multiple(true)
           .help("Sets the level of verbosity"))
    .arg(Arg::with_name("q").short("q").help("Make fw quiet"))
    .subcommand(SubCommand::with_name("sync").about("Sync workspace"))
    .subcommand(SubCommand::with_name("print-zsh-setup").about("Prints zsh completion code."))
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
                         .index(2)
                         .required(false))
                  .arg(Arg::with_name("URL")
                         .value_name("URL")
                         .index(1)
                         .required(true)))
    .subcommand(SubCommand::with_name("foreach")
                  .about("Run script on each project")
                  .arg(Arg::with_name("CMD")
                         .value_name("CMD")
                         .index(1)
                         .required(true)))
    .subcommand(SubCommand::with_name("print-path")
                  .about("Print project path on stdout")
                  .arg(Arg::with_name("PROJECT_NAME")
                         .value_name("PROJECT_NAME")
                         .index(1)
                         .required(true)))
    .subcommand(SubCommand::with_name("projectile").about("Write projectile bookmarks"))
    .subcommand(SubCommand::with_name("ls").about("List projects"))
    .subcommand(SubCommand::with_name("gen-workon")
                  .about("Generate sourceable shell code to work on project")
                  .arg(Arg::with_name("PROJECT_NAME")
                         .value_name("PROJECT_NAME")
                         .index(1)
                         .required(true))
                  .arg(Arg::with_name("quick")
                         .required(false)
                         .short("x")
                         .help("Don't generate post_workon shell code, only cd into the folder")))
    .subcommand(SubCommand::with_name("update")
                  .about("Modifies project settings.")
                  .arg(Arg::with_name("NAME").value_name("NAME").required(true))
                  .arg(Arg::with_name("git")
                         .value_name("URL")
                         .long("git-url")
                         .takes_value(true)
                         .required(false))
                  .arg(Arg::with_name("override-path")
                         .value_name("override-path")
                         .long("override-path")
                         .takes_value(true)
                         .required(false))
                  .arg(Arg::with_name("after-workon")
                         .value_name("after-workon")
                         .long("after-workon")
                         .takes_value(true)
                         .required(false))
                  .arg(Arg::with_name("after-clone")
                         .value_name("after-clone")
                         .long("after-clone")
                         .takes_value(true)
                         .required(false)))
    .subcommand(SubCommand::with_name("tag")
                  .alias("tags")
                  .about("Allows working with tags.")
                  .subcommand(SubCommand::with_name("ls")
                                .alias("list")
                                .about("Lists tags")
                                .arg(Arg::with_name("PROJECT_NAME")
                                       .value_name("PROJECT_NAME")
                                       .required(false)))
                  .subcommand(SubCommand::with_name("tag-project")
                                .about("Add tag to project")
                                .arg(Arg::with_name("PROJECT_NAME")
                                       .value_name("PROJECT_NAME")
                                       .required(true))
                                .arg(Arg::with_name("tag-name")
                                       .value_name("tag")
                                       .required(true)))
                  .subcommand(SubCommand::with_name("untag-project")
                                .about("Removes tag from project")
                                .arg(Arg::with_name("PROJECT_NAME")
                                       .value_name("PROJECT_NAME")
                                       .required(true))
                                .arg(Arg::with_name("tag-name")
                                       .value_name("tag")
                                       .required(true)))
                  .subcommand(SubCommand::with_name("rm")
                                .about("Deletes a tag. Will not untag projects.")
                                .arg(Arg::with_name("tag-name")
                                       .value_name("tag name")
                                       .required(true)))
                  .subcommand(SubCommand::with_name("add")
                                .alias("update")
                                .alias("create")
                                .about("Creates a new tag. Replaces existing.")
                                .arg(Arg::with_name("tag-name")
                                       .value_name("tag name")
                                       .required(true))
                                .arg(Arg::with_name("after-workon")
                                       .value_name("after-workon")
                                       .long("after-workon")
                                       .takes_value(true)
                                       .required(false))
                                .arg(Arg::with_name("after-clone")
                                       .value_name("after-clone")
                                       .long("after-clone")
                                       .takes_value(true)
                                       .required(false))))
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
                                                             subcommand_matches.value_of("NAME"),
                                                             subcommand_matches.value_of("URL")
                                                                               .expect("argument required by clap.rs"),
                                                             &subcommand_logger)
                                         }
                                         "update" => {
    let name: &str = subcommand_matches.value_of("NAME")
                                       .expect("argument required by clap.rs");
    let git: Option<String> = subcommand_matches.value_of("git").map(str::to_string);
    let after_workon: Option<String> = subcommand_matches.value_of("after-workon")
                                                         .map(str::to_string);
    let after_clone: Option<String> = subcommand_matches.value_of("after-clone")
                                                        .map(str::to_string);
    let override_path: Option<String> = subcommand_matches.value_of("override-path")
                                                          .map(str::to_string);
    config::update_entry(config,
                         name,
                         git,
                         after_workon,
                         after_clone,
                         override_path,
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
                                                       config,
                                                       subcommand_matches.is_present("quick"),
                                                       &subcommand_logger)
                                         }
                                         "projectile" => projectile::projectile(config, &subcommand_logger),
                                         "print-path" => {
                                           workon::print_path(config,
                                                              subcommand_matches.value_of("PROJECT_NAME")
                                                                                .expect("argument required by clap.rs"))
                                         }
                                         "foreach" => {
                                           sync::foreach(config,
                                                         subcommand_matches.value_of("CMD")
                                                                           .expect("argument required by clap.rs"),
                                                         &subcommand_logger)
                                         }
                                         "print-zsh-setup" => print_zsh_setup(),
                                         "tag" => {
    let subsubcommand_name: String = subcommand_matches.subcommand_name()
                                                       .expect("subcommand matches enforced by clap.rs")
                                                       .to_owned();
    let subsubcommand_matches: clap::ArgMatches =
      subcommand_matches.subcommand_matches(&subsubcommand_name)
                        .expect("subcommand matches enforced by clap.rs")
                        .to_owned();
    execute_tag_subcommand(config,
                           subsubcommand_name,
                           subsubcommand_matches,
                           &subcommand_logger)
  }
                                         "ls" => workon::ls(config),
                                         _ => Result::Err(AppError::InternalError("Command not implemented")),
                                         }
                                         .and_then(|_| now.elapsed().map_err(AppError::ClockError))
                                         .map(|duration| format!("{}sec", duration.as_secs()));

  match result {
  Ok(time) => debug!(subcommand_logger, "Done"; "time" => time),
  Err(error) => {
    crit!(subcommand_logger, "Error running command"; "error" => format!("{:?}", error));
    std::process::exit(1)
  }

  }
}

fn execute_tag_subcommand(maybe_config: Result<config::Config, AppError>,
                          tag_command_name: String,
                          tag_matches: clap::ArgMatches,
                          logger: &Logger)
                          -> Result<(), AppError> {
  match tag_command_name.as_ref() {
  "ls" => {
    let maybe_project_name: Option<String> = tag_matches.value_of("PROJECT_NAME").map(str::to_string);
    tag::list_tags(maybe_config, maybe_project_name, logger)
  },
  "tag-project" => {
    let project_name: String = tag_matches.value_of("PROJECT_NAME").map(str::to_string)
                                      .expect("argument enforced by clap.rs");
    let tag_name: String = tag_matches.value_of("tag-name")
                                      .map(str::to_string)
                                      .expect("argument enforced by clap.rs");
    tag::add_tag(maybe_config, project_name, tag_name, logger)
  },
  "untag-project" => {
    let project_name: String = tag_matches.value_of("PROJECT_NAME").map(str::to_string)
                                      .expect("argument enforced by clap.rs");
    let tag_name: String = tag_matches.value_of("tag-name")
                                      .map(str::to_string)
                                      .expect("argument enforced by clap.rs");
    tag::remove_tag(maybe_config, project_name, tag_name, logger)
  },
  "rm" => {
    let tag_name: String = tag_matches.value_of("tag-name")
                                      .map(str::to_string)
                                      .expect("argument enforced by clap.rs");
    tag::delete_tag(maybe_config, tag_name, logger)
  }
  "add" => {
    let tag_name: String = tag_matches.value_of("tag-name")
                                      .map(str::to_string)
                                      .expect("argument enforced by clap.rs");
    let after_workon: Option<String> = tag_matches.value_of("after-workon").map(str::to_string);
    let after_clone: Option<String> = tag_matches.value_of("after-clone").map(str::to_string);
    tag::create_tag(maybe_config, tag_name, after_workon, after_clone, logger)
  }
  _ => Result::Err(AppError::InternalError("Command not implemented")),
  }
}

fn print_zsh_setup() -> Result<(), AppError> {
  let completion = include_str!("setup.zsh");
  println!("{}", completion);
  Ok(())
}

mod errors;
mod config;
mod sync;
mod setup;
mod workon;
mod projectile;
mod tag;
