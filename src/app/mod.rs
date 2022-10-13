use clap::{builder::EnumValueParser, crate_version, value_parser, Arg, ArgAction, Command};

use crate::setup::ProjectState;

pub fn app() -> Command {
  let arg_with_fzf = Arg::new("with-fzf")
    .long("with-fzf")
    .short('f')
    .num_args(0)
    .action(ArgAction::SetTrue)
    .help("Integrate with fzf");
  let arg_with_skim = Arg::new("with-skim")
    .long("with-skim")
    .short('s')
    .help("Integrate with skim")
    .conflicts_with("with-fzf")
    .action(ArgAction::SetTrue)
    .num_args(0);

  Command::new("fw")
    .version(crate_version!())
    .author("Brocode <bros@brocode.sh>")
    .about(
      "fast workspace manager. Config set by FW_CONFIG_DIR or default.
For further information please have a look at our README https://github.com/brocode/fw/blob/master/README.org",
    )
    .subcommand_required(true)
    .arg(
      Arg::new("v")
        .short('v')
        .num_args(0)
        .action(ArgAction::Count)
        .help("Sets the level of verbosity"),
    )
    .arg(Arg::new("q").short('q').help("Make fw quiet").action(ArgAction::SetTrue).num_args(0))
    .subcommand(
      Command::new("sync")
        .about("Sync workspace. Clones projects or updates remotes for existing projects.")
        .arg(
          Arg::new("tag")
            .long("tag")
            .short('t')
            .help("Filter projects by tag. More than 1 is allowed.")
            .required(false)
            .num_args(1)
            .action(ArgAction::Append),
        )
        .arg(
          Arg::new("no-progress-bar")
            .long("no-progress-bar")
            .short('q')
            .help("Progress bars are automatically disabled with -vv")
            .num_args(0)
            .action(ArgAction::SetTrue),
        )
        .arg(
          Arg::new("no-fast-forward-merge")
            .long("no-ff-merge")
            .help("No fast forward merge")
            .action(ArgAction::SetTrue)
            .num_args(0),
        )
        .arg(
          Arg::new("only-new")
            .long("only-new")
            .short('n')
            .help("Only clones projects. Skips all actions for projects already on your machine.")
            .num_args(0)
            .action(ArgAction::SetTrue),
        )
        .arg(
          Arg::new("parallelism")
            .long("parallelism")
            .short('p')
            .default_value("8")
            .value_parser(clap::builder::RangedI64ValueParser::<i32>::new().range(0..=128))
            .help("Sets the count of worker")
            .num_args(1),
        ),
    )
    .subcommand(
      Command::new("print-zsh-setup")
        .about("Prints zsh completion code.")
        .arg(arg_with_fzf.clone())
        .arg(arg_with_skim.clone()),
    )
    .subcommand(
      Command::new("print-bash-setup")
        .about("Prints bash completion code.")
        .arg(arg_with_fzf.clone())
        .arg(arg_with_skim.clone()),
    )
    .subcommand(
      Command::new("print-fish-setup")
        .about("Prints fish completion code.")
        .arg(arg_with_fzf)
        .arg(arg_with_skim),
    )
    .subcommand(
      Command::new("setup")
        .about("Setup config from existing workspace")
        .arg(Arg::new("WORKSPACE_DIR").value_name("WORKSPACE_DIR").index(1).required(true)),
    )
    .subcommand(
      Command::new("reworkon")
        .aliases([".", "rw", "re", "fkbr"])
        .about("Re-run workon hooks for current dir (aliases: .|rw|re|fkbr)"),
    )
    .subcommand(
      Command::new("import")
        .about("Import existing git folder to fw")
        .arg(Arg::new("PROJECT_DIR").value_name("PROJECT_DIR").index(1).required(true)),
    )
    .subcommand(
      Command::new("org-import")
        .about(
          "Import all repositories from github org into fw. Token can be set in the settings file or provided via the environment variable FW_GITHUB_TOKEN",
        )
        .arg(
          Arg::new("include-archived")
            .value_name("include-archived")
            .long("include-archived")
            .short('a')
            .num_args(0)
            .action(ArgAction::SetTrue)
            .required(false),
        )
        .arg(Arg::new("ORG_NAME").value_name("ORG_NAME").index(1).required(true)),
    )
    .subcommand(
      Command::new("gitlab-import")
        .about("Import all owned repositories / your organizations repositories from gitlab into fw")
        .arg(
          Arg::new("include")
            .help("Filter projects to import by state")
            .long("include")
            .short('a')
            .num_args(1)
            .value_name("state")
            .value_parser(EnumValueParser::<ProjectState>::new())
            .default_value("active"),
        ),
    )
    .subcommand(
      Command::new("add-remote")
        .about("Add remote to project")
        .arg(Arg::new("NAME").value_name("NAME").index(1).required(true))
        .arg(Arg::new("REMOTE_NAME").value_name("REMOTE_NAME").index(2).required(true))
        .arg(Arg::new("URL").value_name("URL").index(3).required(true)),
    )
    .subcommand(
      Command::new("remove-remote")
        .about("Removes remote from project (Only in the fw configuration. An existing remote will not be deleted by a sync)")
        .arg(Arg::new("NAME").value_name("NAME").index(1).required(true))
        .arg(Arg::new("REMOTE_NAME").value_name("REMOTE_NAME").index(2).required(true)),
    )
    .subcommand(
      Command::new("add")
        .about("Add project to config")
        .arg(Arg::new("NAME").value_name("NAME").index(2).required(false))
        .arg(Arg::new("URL").value_name("URL").index(1).required(true))
        .arg(
          Arg::new("override-path")
            .value_name("override-path")
            .long("override-path")
            .num_args(1)
            .required(false),
        )
        .arg(
          Arg::new("after-workon")
            .value_name("after-workon")
            .long("after-workon")
            .num_args(1)
            .required(false),
        )
        .arg(
          Arg::new("tag")
            .long("tag")
            .short('t')
            .help("Add tag to project")
            .required(false)
            .num_args(1)
            .action(ArgAction::Append),
        )
        .arg(
          Arg::new("after-clone")
            .value_name("after-clone")
            .long("after-clone")
            .num_args(1)
            .required(false),
        )
        .arg(Arg::new("trusted").long("trusted").num_args(0).required(false).action(ArgAction::SetTrue)),
    )
    .subcommand(
      Command::new("remove")
        .alias("rm")
        .about("Remove project from config")
        .arg(Arg::new("NAME").value_name("NAME").index(1).required(true))
        .arg(
          Arg::new("purge-directory")
            .long("purge-directory")
            .short('p')
            .help("Purges the project directory")
            .num_args(0)
            .action(ArgAction::SetTrue),
        ),
    )
    .subcommand(
      Command::new("foreach")
        .about("Run script on each project")
        .arg(Arg::new("CMD").value_name("CMD").required(true))
        .arg(
          Arg::new("parallel")
            .short('p')
            .help("Parallelism to use (default is set by rayon but probably equal to the number of cores)")
            .required(false)
            .value_parser(clap::builder::RangedI64ValueParser::<i32>::new().range(0..=128))
            .num_args(1),
        )
        .arg(
          Arg::new("tag")
            .long("tag")
            .short('t')
            .help("Filter projects by tag. More than 1 is allowed.")
            .required(false)
            .num_args(1)
            .action(ArgAction::Append),
        ),
    )
    .subcommand(
      Command::new("print-path")
        .about("Print project path on stdout")
        .arg(Arg::new("PROJECT_NAME").value_name("PROJECT_NAME").index(1).required(true)),
    )
    .subcommand(Command::new("projectile").about("Write projectile bookmarks"))
    .subcommand(
      Command::new("intellij").about("Add projects to intellijs list of recent projects").arg(
        Arg::new("no-warn")
          .long("no-warn")
          .short('n')
          .help("Disables warning message if more than 50 projects would be added"),
      ),
    )
    .subcommand(
      Command::new("ls").about("List projects").arg(
        Arg::new("tag")
          .long("tag")
          .short('t')
          .help("Filter projects by tag. More than 1 is allowed.")
          .required(false)
          .num_args(1)
          .action(ArgAction::Append),
      ),
    )
    .subcommand(
      Command::new("gen-workon")
        .about("Generate sourceable shell code to work on project")
        .arg(Arg::new("PROJECT_NAME").value_name("PROJECT_NAME").index(1).required(true))
        .arg(
          Arg::new("quick")
            .required(false)
            .short('x')
            .help("Don't generate post_workon shell code, only cd into the folder"),
        ),
    )
    .subcommand(Command::new("gen-reworkon").about("Generate sourceable shell code to re-work on project"))
    .subcommand(
      Command::new("inspect")
        .about("Inspect project")
        .arg(Arg::new("PROJECT_NAME").value_name("PROJECT_NAME").index(1).required(true))
        .arg(
          Arg::new("json")
            .help("output json instead of cool text")
            .short('j')
            .long("json")
            .required(false),
        ),
    )
    .subcommand(
      Command::new("update")
        .about("Modifies project settings.")
        .arg(Arg::new("NAME").value_name("NAME").required(true))
        .arg(Arg::new("git").value_name("URL").long("git-url").num_args(1).required(false))
        .arg(
          Arg::new("override-path")
            .value_name("override-path")
            .long("override-path")
            .num_args(1)
            .required(false),
        )
        .arg(
          Arg::new("after-workon")
            .value_name("after-workon")
            .long("after-workon")
            .num_args(1)
            .required(false),
        )
        .arg(
          Arg::new("after-clone")
            .value_name("after-clone")
            .long("after-clone")
            .num_args(1)
            .required(false),
        ),
    )
    .subcommand(
      Command::new("tag")
        .alias("tags")
        .about("Allows working with tags.")
        .subcommand_required(true)
        .subcommand(
          Command::new("ls")
            .alias("list")
            .about("Lists tags")
            .arg(Arg::new("PROJECT_NAME").value_name("PROJECT_NAME").required(false)),
        )
        .subcommand(
          Command::new("tag-project")
            .about("Add tag to project")
            .arg(Arg::new("PROJECT_NAME").value_name("PROJECT_NAME").required(true))
            .arg(Arg::new("tag-name").value_name("tag").required(true)),
        )
        .subcommand(
          Command::new("untag-project")
            .about("Removes tag from project")
            .arg(Arg::new("PROJECT_NAME").value_name("PROJECT_NAME").required(true))
            .arg(Arg::new("tag-name").value_name("tag").required(true)),
        )
        .subcommand(
          Command::new("autotag")
            .about("tags projects when CMD returns exit code 0")
            .arg(Arg::new("tag-name").value_name("tag").required(true))
            .arg(Arg::new("CMD").value_name("CMD").required(true))
            .arg(
              Arg::new("parallel")
                .short('p')
                .help("Parallelism to use (default is set by rayon but probably equal to the number of cores)")
                .required(false)
                .value_parser(clap::builder::RangedI64ValueParser::<i32>::new().range(0..=128))
                .num_args(1),
            ),
        )
        .subcommand(
          Command::new("inspect")
            .about("Inspect a tag")
            .arg(Arg::new("tag-name").value_name("tag name").required(true)),
        )
        .subcommand(
          Command::new("rm")
            .about("Deletes a tag. Will not untag projects.")
            .arg(Arg::new("tag-name").value_name("tag name").required(true)),
        )
        .subcommand(
          Command::new("add")
            .alias("update")
            .alias("create")
            .about("Creates a new tag. Replaces existing.")
            .arg(Arg::new("tag-name").value_name("tag name").required(true))
            .arg(
              Arg::new("after-workon")
                .value_name("after-workon")
                .long("after-workon")
                .num_args(1)
                .required(false),
            )
            .arg(
              Arg::new("priority")
                .value_name("priority")
                .long("priority")
                .value_parser(value_parser!(u8))
                .num_args(1)
                .required(false),
            )
            .arg(Arg::new("workspace").value_name("workspace").long("workspace").num_args(1).required(false))
            .arg(
              Arg::new("after-clone")
                .value_name("after-clone")
                .long("after-clone")
                .num_args(1)
                .required(false),
            ),
        ),
    )
}
