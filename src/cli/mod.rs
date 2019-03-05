use clap::{crate_version, App, Arg, AppSettings, SubCommand};

pub fn build_cli<'a>() -> App<'a, 'a> {
  App::new("fw")
    .version(crate_version!())
    .author("Brocode <bros@brocode.sh>")
    .about(
      "fast workspace manager. Config set by FW_CONFIG_PATH or default.
For further information please have a look at our README https://github.com/brocode/fw/blob/master/README.org",
    )
    .global_setting(AppSettings::ColoredHelp)
    .setting(AppSettings::SubcommandRequired)
    .arg(Arg::with_name("v").short("v").multiple(true).help("Sets the level of verbosity"))
    .arg(Arg::with_name("q").short("q").help("Make fw quiet"))
    .subcommand(
      SubCommand::with_name("sync")
        .about("Sync workspace. Clones projects or updates remotes for existing projects.")
        .arg(
          Arg::with_name("no-progress-bar")
            .long("no-progress-bar")
            .short("q")
            .help("Progress bars are automatically disabled with -vv")
            .takes_value(false),
        )
        .arg(
          Arg::with_name("no-fast-forward-merge")
            .long("no-ff-merge")
            .help("No fast forward merge")
            .takes_value(false),
        )
        .arg(
          Arg::with_name("only-new")
            .long("only-new")
            .short("n")
            .help("Only clones projects. Skips all actions for projects already on your machine.")
            .takes_value(false),
        )
        .arg(
          Arg::with_name("parallelism")
            .long("parallelism")
            .short("p")
            .number_of_values(1)
            .default_value("8")
            .validator(|input| validate_number(&input, 10))
            .help("Sets the count of worker")
            .takes_value(true),
        ),
    )
    .subcommand(
      SubCommand::with_name("print-zsh-setup")
        .about("Prints zsh completion code.")
        .arg(Arg::with_name("with-fzf").long("with-fzf").short("-f").help("Integrate with fzf")),
    )
    .subcommand(
      SubCommand::with_name("print-bash-setup")
        .about("Prints bash completion code.")
        .arg(Arg::with_name("with-fzf").long("with-fzf").short("-f").help("Integrate with fzf")),
    )
    .subcommand(
      SubCommand::with_name("setup")
        .about("Setup config from existing workspace")
        .arg(Arg::with_name("WORKSPACE_DIR").value_name("WORKSPACE_DIR").index(1).required(true)),
    )
    .subcommand(
      SubCommand::with_name("reworkon")
        .aliases(&[".", "rw", "re", "fkbr"])
        .about("Re-run workon hooks for current dir (aliases: .|rw|re|fkbr)"),
    )
    .subcommand(
      SubCommand::with_name("import")
        .about("Import existing git folder to fw")
        .arg(Arg::with_name("PROJECT_DIR").value_name("PROJECT_DIR").index(1).required(true)),
    )
    .subcommand(
      SubCommand::with_name("org-import")
        .about("Import all repositories from github org into fw")
        .arg(
          Arg::with_name("include-archived")
            .value_name("include-archived")
            .long("include-archived")
            .short("a")
            .takes_value(false)
            .required(false),
        )
        .arg(Arg::with_name("ORG_NAME").value_name("ORG_NAME").index(1).required(true)),
    )
    .subcommand(SubCommand::with_name("gitlab-import").about("Import all owned repositories / your organizations repositories from gitlab into fw"))
    .subcommand(
      SubCommand::with_name("add-remote")
        .about("Add remote to project")
        .arg(Arg::with_name("NAME").value_name("NAME").index(1).required(true))
        .arg(Arg::with_name("REMOTE_NAME").value_name("REMOTE_NAME").index(2).required(true))
        .arg(Arg::with_name("URL").value_name("URL").index(3).required(true)),
    )
    .subcommand(
      SubCommand::with_name("remove-remote")
        .about("Removes remote from project (Only in the fw configuration. An existing remote will not be deleted by a sync)")
        .arg(Arg::with_name("NAME").value_name("NAME").index(1).required(true))
        .arg(Arg::with_name("REMOTE_NAME").value_name("REMOTE_NAME").index(2).required(true)),
    )
    .subcommand(
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
        )
        .arg(
          Arg::with_name("after-workon")
            .value_name("after-workon")
            .long("after-workon")
            .takes_value(true)
            .required(false),
        )
        .arg(
          Arg::with_name("after-clone")
            .value_name("after-clone")
            .long("after-clone")
            .takes_value(true)
            .required(false),
        ),
    )
    .subcommand(
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
    )
    .subcommand(
      SubCommand::with_name("foreach")
        .about("Run script on each project")
        .arg(Arg::with_name("CMD").value_name("CMD").required(true))
        .arg(
          Arg::with_name("parallel")
            .short("p")
            .help("Parallelism to use (default is set by rayon but probably equal to the number of cores)")
            .required(false)
            .validator(|input| validate_number(&input, 20))
            .takes_value(true),
        )
        .arg(
          Arg::with_name("tag")
            .long("tag")
            .short("t")
            .help("Filter projects by tag. More than 1 is allowed.")
            .required(false)
            .takes_value(true)
            .multiple(true),
        ),
    )
    .subcommand(
      SubCommand::with_name("export-project")
        .about("Exports project as fw shell script")
        .arg(Arg::with_name("PROJECT_NAME").value_name("PROJECT_NAME").index(1).required(true)),
    )
    .subcommand(
      SubCommand::with_name("export-by-tag")
        .about("Exports all projects with tag as fw shell script")
        .arg(Arg::with_name("tag_name").value_name("tag_name").required(true)),
    )
    .subcommand(
      SubCommand::with_name("export-tag")
        .about("Exports tag")
        .arg(Arg::with_name("tag_name").value_name("tag_name").required(true)),
    )
    .subcommand(
      SubCommand::with_name("print-path")
        .about("Print project path on stdout")
        .arg(Arg::with_name("PROJECT_NAME").value_name("PROJECT_NAME").index(1).required(true)),
    )
    .subcommand(SubCommand::with_name("projectile").about("Write projectile bookmarks"))
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
    )
    .subcommand(SubCommand::with_name("gen-reworkon").about("Generate sourceable shell code to re-work on project"))
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
    )
    .subcommand(
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
        )
        .arg(
          Arg::with_name("after-workon")
            .value_name("after-workon")
            .long("after-workon")
            .takes_value(true)
            .required(false),
        )
        .arg(
          Arg::with_name("after-clone")
            .value_name("after-clone")
            .long("after-clone")
            .takes_value(true)
            .required(false),
        ),
    )
    .subcommand(
      SubCommand::with_name("tag")
        .alias("tags")
        .about("Allows working with tags.")
        .subcommand(
          SubCommand::with_name("ls")
            .alias("list")
            .about("Lists tags")
            .arg(Arg::with_name("PROJECT_NAME").value_name("PROJECT_NAME").required(false)),
        )
        .subcommand(
          SubCommand::with_name("tag-project")
            .about("Add tag to project")
            .arg(Arg::with_name("PROJECT_NAME").value_name("PROJECT_NAME").required(true))
            .arg(Arg::with_name("tag-name").value_name("tag").required(true)),
        )
        .subcommand(
          SubCommand::with_name("untag-project")
            .about("Removes tag from project")
            .arg(Arg::with_name("PROJECT_NAME").value_name("PROJECT_NAME").required(true))
            .arg(Arg::with_name("tag-name").value_name("tag").required(true)),
        )
        .subcommand(
          SubCommand::with_name("autotag")
            .about("tags projects when script executes to 0")
            .arg(Arg::with_name("tag-name").value_name("tag").required(true))
            .arg(Arg::with_name("CMD").value_name("CMD").required(true))
            .arg(
              Arg::with_name("parallel")
                .short("p")
                .help("Parallelism to use (default is set by rayon but probably equal to the number of cores)")
                .required(false)
                .validator(|input| validate_number(&input, 20))
                .takes_value(true),
            ),
        )
        .subcommand(
          SubCommand::with_name("rm")
            .about("Deletes a tag. Will not untag projects.")
            .arg(Arg::with_name("tag-name").value_name("tag name").required(true)),
        )
        .subcommand(
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
            )
            .arg(
              Arg::with_name("priority")
                .value_name("priority")
                .long("priority")
                .takes_value(true)
                .required(false),
            )
            .arg(
              Arg::with_name("workspace")
                .value_name("workspace")
                .long("workspace")
                .takes_value(true)
                .required(false),
            )
            .arg(
              Arg::with_name("after-clone")
                .value_name("after-clone")
                .long("after-clone")
                .takes_value(true)
                .required(false),
            ),
        ),
    )
}

fn validate_number(input: &str, max: i32) -> std::result::Result<(), String> {
  let i = input.parse::<i32>().map_err(|_e| format!("Expected a number. Was '{}'.", input))?;
  if i > 0 && i <= max {
    Ok(())
  } else {
    Err(format!("Number must be between 1 and {}. Was {}.", max, input))
  }
}
