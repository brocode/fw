extern crate clap;

extern crate slog;
extern crate slog_async;
extern crate slog_term;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate error_chain;

extern crate dirs;

extern crate github_gql;

extern crate git2;

extern crate rayon;

extern crate core;

extern crate atty;

extern crate ansi_term;

extern crate rand;

extern crate crossbeam;

extern crate indicatif;

#[cfg(test)]
extern crate maplit;

extern crate regex;

#[cfg(test)]
extern crate spectral;

extern crate openssl_probe;

use clap::{crate_version, App, AppSettings, Arg, SubCommand};
use errors::*;
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
  let matches = app().get_matches();

  let logger = logger_from_verbosity(matches.occurrences_of("v"), matches.is_present("q"));
  let config = config::get_config(&logger);

  let subcommand_name = matches.subcommand_name().expect("subcommand required by clap.rs").to_owned();
  let subcommand_matches = matches.subcommand_matches(&subcommand_name).expect("subcommand matches enforced by clap.rs");
  let subcommand_logger = logger.new(o!("command" => subcommand_name.clone()));

  let now = SystemTime::now();
  let result: Result<String> = match subcommand_name.as_ref() {
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
      &subcommand_logger,
    ),
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
    "export" => export::export_project(config, subcommand_matches.value_of("PROJECT_NAME").expect("argument required by clap.rs")),
    "foreach" => sync::foreach(
      config,
      subcommand_matches.value_of("CMD").expect("argument required by clap.rs"),
      &subcommand_matches.values_of_lossy("tag").unwrap_or_default().into_iter().collect(),
      &subcommand_logger,
      &subcommand_matches.value_of("parallel").map(|p| p.to_owned()),
    ),
    "print-zsh-setup" => print_zsh_setup(subcommand_matches.is_present("with-fzf")),
    "tag" => {
      let subsubcommand_name: String = subcommand_matches.subcommand_name().expect("subcommand matches enforced by clap.rs").to_owned();
      let subsubcommand_matches: clap::ArgMatches = subcommand_matches
        .subcommand_matches(&subsubcommand_name)
        .expect("subcommand matches enforced by clap.rs")
        .to_owned();
      execute_tag_subcommand(config, &subsubcommand_name, &subsubcommand_matches, &subcommand_logger)
    }
    "ls" => workon::ls(config),
    _ => Err(ErrorKind::InternalError("Command not implemented".to_string()).into()),
  }.and_then(|_| now.elapsed().map_err(|e| e.into()))
  .map(|duration| format!("{}sec", duration.as_secs()));

  match result {
    Ok(time) => {
      debug!(subcommand_logger, "Done"; "time" => time);
      0
    }
    Err(e) => {
      use std::io::Write;
      let stderr = &mut ::std::io::stderr();
      let errmsg = "Error writing to stderr";

      writeln!(stderr, "error: {}", e).unwrap_or_else(|_| panic!(errmsg));

      for e in e.iter().skip(1) {
        writeln!(stderr, "caused by: {}", e).unwrap_or_else(|_| panic!(errmsg));
      }

      // The backtrace is not always generated. Try to run this example
      // with `RUST_BACKTRACE=1`.
      if let Some(backtrace) = e.backtrace() {
        writeln!(stderr, "backtrace: {:?}", backtrace).unwrap_or_else(|_| panic!(errmsg));
      }
      crit!(subcommand_logger, "Error running command"; "error" => format!("{:?}", e));
      1
    }
  }
}

fn execute_tag_subcommand(maybe_config: Result<config::Config>, tag_command_name: &str, tag_matches: &clap::ArgMatches, logger: &Logger) -> Result<()> {
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
    _ => Err(ErrorKind::InternalError("Command not implemented".to_string()).into()),
  }
}

fn print_zsh_setup(use_fzf: bool) -> Result<()> {
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

fn validate_number(input: String, max: i32) -> std::result::Result<(), String> {
  let i = input.parse::<i32>().map_err(|_e| format!("Expected a number. Was '{}'.", input))?;
  if i > 0 && i <= max {
    Ok(())
  } else {
    Err(format!("Number must be between 1 and {}. Was {}.", max, input))
  }
}

fn app<'a>() -> App<'a, 'a> {
  App::new("fw")
    .version(crate_version!())
    .author("Brocode <bros@brocode.sh>")
    .about(
      "fast workspace manager. Config set by FW_CONFIG_PATH or default.
For further information please have a look at our README https://github.com/brocode/fw/blob/master/README.org",
    ).global_setting(AppSettings::ColoredHelp)
    .setting(AppSettings::SubcommandRequired)
    .arg(Arg::with_name("v").short("v").multiple(true).help("Sets the level of verbosity"))
    .arg(Arg::with_name("q").short("q").help("Make fw quiet"))
    .subcommand(
      SubCommand::with_name("sync")
        .about("Sync workspace. Clones projects or updates remotes for existing projects.")
        .arg(Arg::with_name("no-progress-bar").long("no-progress-bar").short("q").takes_value(false))
        .arg(
          Arg::with_name("no-fast-forward-merge")
            .long("no-ff-merge")
            .help("No fast forward merge")
            .takes_value(false),
        ).arg(
          Arg::with_name("only-new")
            .long("only-new")
            .short("n")
            .help("Only clones projects. Skips all actions for projects already on your machine.")
            .takes_value(false),
        ).arg(
          Arg::with_name("parallelism")
            .long("parallelism")
            .short("p")
            .number_of_values(1)
            .default_value("8")
            .validator(|input| validate_number(input, 10))
            .help("Sets the count of worker")
            .takes_value(true),
        ),
    ).subcommand(
      SubCommand::with_name("print-zsh-setup")
        .about("Prints zsh completion code.")
        .arg(Arg::with_name("with-fzf").long("with-fzf").short("-f").help("Integrate with fzf")),
    ).subcommand(
      SubCommand::with_name("setup")
        .about("Setup config from existing workspace")
        .arg(Arg::with_name("WORKSPACE_DIR").value_name("WORKSPACE_DIR").index(1).required(true)),
    ).subcommand(
      SubCommand::with_name("reworkon")
        .aliases(&[".", "rw", "re", "fkbr"])
        .about("Re-run workon hooks for current dir (aliases: .|rw|re|fkbr)"),
    ).subcommand(
      SubCommand::with_name("import")
        .about("Import existing git folder to fw")
        .arg(Arg::with_name("PROJECT_DIR").value_name("PROJECT_DIR").index(1).required(true)),
    ).subcommand(
      SubCommand::with_name("org-import")
        .about("Import all repositories from github org into fw")
        .arg(Arg::with_name("ORG_NAME").value_name("ORG_NAME").index(1).required(true)),
    ).subcommand(
      SubCommand::with_name("add")
        .about("Add project to config")
        .arg(Arg::with_name("NAME").value_name("NAME").index(2).required(false))
        .arg(Arg::with_name("URL").value_name("URL").index(1).required(true))
        .arg(
          Arg::with_name("override-path")
            .value_name("override-path")
            .long("override-path")
            .takes_value(true)
            .required(false),
        ).arg(
          Arg::with_name("after-workon")
            .value_name("after-workon")
            .long("after-workon")
            .takes_value(true)
            .required(false),
        ).arg(
          Arg::with_name("after-clone")
            .value_name("after-clone")
            .long("after-clone")
            .takes_value(true)
            .required(false),
        ),
    ).subcommand(
      SubCommand::with_name("remove")
        .alias("rm")
        .about("Remove project from config")
        .arg(Arg::with_name("NAME").value_name("NAME").index(1).required(true))
        .arg(
          Arg::with_name("purge-directory")
            .long("purge-directory")
            .short("p")
            .help("Purges the project directory")
            .takes_value(false),
        ),
    ).subcommand(
      SubCommand::with_name("foreach")
        .about("Run script on each project")
        .arg(Arg::with_name("CMD").value_name("CMD").required(true))
        .arg(
          Arg::with_name("parallel")
            .short("p")
            .help("Parallelism to use (default is set by rayon but probably equal to the number of cores)")
            .required(false)
            .validator(|input| validate_number(input, 20))
            .takes_value(true),
        ).arg(
          Arg::with_name("tag")
            .long("tag")
            .short("t")
            .help("Filter projects by tag. More than 1 is allowed.")
            .required(false)
            .takes_value(true)
            .multiple(true),
        ),
    ).subcommand(
      SubCommand::with_name("export")
        .about("Exports project as fw shell script")
        .arg(Arg::with_name("PROJECT_NAME").value_name("PROJECT_NAME").index(1).required(true)),
    ).subcommand(
      SubCommand::with_name("print-path")
        .about("Print project path on stdout")
        .arg(Arg::with_name("PROJECT_NAME").value_name("PROJECT_NAME").index(1).required(true)),
    ).subcommand(SubCommand::with_name("projectile").about("Write projectile bookmarks"))
    .subcommand(SubCommand::with_name("ls").about("List projects"))
    .subcommand(
      SubCommand::with_name("gen-workon")
        .about("Generate sourceable shell code to work on project")
        .arg(Arg::with_name("PROJECT_NAME").value_name("PROJECT_NAME").index(1).required(true))
        .arg(
          Arg::with_name("quick")
            .required(false)
            .short("x")
            .help("Don't generate post_workon shell code, only cd into the folder"),
        ),
    ).subcommand(SubCommand::with_name("gen-reworkon").about("Generate sourceable shell code to re-work on project"))
    .subcommand(
      SubCommand::with_name("inspect")
        .about("Inspect project")
        .arg(Arg::with_name("PROJECT_NAME").value_name("PROJECT_NAME").index(1).required(true))
        .arg(
          Arg::with_name("json")
            .help("output json instead of cool text")
            .short("j")
            .long("json")
            .required(false),
        ),
    ).subcommand(
      SubCommand::with_name("update")
        .about("Modifies project settings.")
        .arg(Arg::with_name("NAME").value_name("NAME").required(true))
        .arg(Arg::with_name("git").value_name("URL").long("git-url").takes_value(true).required(false))
        .arg(
          Arg::with_name("override-path")
            .value_name("override-path")
            .long("override-path")
            .takes_value(true)
            .required(false),
        ).arg(
          Arg::with_name("after-workon")
            .value_name("after-workon")
            .long("after-workon")
            .takes_value(true)
            .required(false),
        ).arg(
          Arg::with_name("after-clone")
            .value_name("after-clone")
            .long("after-clone")
            .takes_value(true)
            .required(false),
        ),
    ).subcommand(
      SubCommand::with_name("tag")
        .alias("tags")
        .about("Allows working with tags.")
        .subcommand(
          SubCommand::with_name("ls")
            .alias("list")
            .about("Lists tags")
            .arg(Arg::with_name("PROJECT_NAME").value_name("PROJECT_NAME").required(false)),
        ).subcommand(
          SubCommand::with_name("tag-project")
            .about("Add tag to project")
            .arg(Arg::with_name("PROJECT_NAME").value_name("PROJECT_NAME").required(true))
            .arg(Arg::with_name("tag-name").value_name("tag").required(true)),
        ).subcommand(
          SubCommand::with_name("untag-project")
            .about("Removes tag from project")
            .arg(Arg::with_name("PROJECT_NAME").value_name("PROJECT_NAME").required(true))
            .arg(Arg::with_name("tag-name").value_name("tag").required(true)),
        ).subcommand(
          SubCommand::with_name("autotag")
            .about("tags projects when script executes to 0")
            .arg(Arg::with_name("tag-name").value_name("tag").required(true))
            .arg(Arg::with_name("CMD").value_name("CMD").required(true))
            .arg(
              Arg::with_name("parallel")
                .short("p")
                .help("Parallelism to use (default is set by rayon but probably equal to the number of cores)")
                .required(false)
                .validator(|input| validate_number(input, 20))
                .takes_value(true),
            ),
        ).subcommand(
          SubCommand::with_name("rm")
            .about("Deletes a tag. Will not untag projects.")
            .arg(Arg::with_name("tag-name").value_name("tag name").required(true)),
        ).subcommand(
          SubCommand::with_name("add")
            .alias("update")
            .alias("create")
            .about("Creates a new tag. Replaces existing.")
            .arg(Arg::with_name("tag-name").value_name("tag name").required(true))
            .arg(
              Arg::with_name("after-workon")
                .value_name("after-workon")
                .long("after-workon")
                .takes_value(true)
                .required(false),
            ).arg(
              Arg::with_name("priority")
                .value_name("priority")
                .long("priority")
                .takes_value(true)
                .required(false),
            ).arg(
              Arg::with_name("workspace")
                .value_name("workspace")
                .long("workspace")
                .takes_value(true)
                .required(false),
            ).arg(
              Arg::with_name("after-clone")
                .value_name("after-clone")
                .long("after-clone")
                .takes_value(true)
                .required(false),
            ),
        ),
    )
}

mod config;
mod errors;
mod export;
mod projectile;
mod setup;
mod sync;
mod tag;
mod workon;
mod ws;
