use man::prelude::*;

fn main() {
  let page = Manual::new("fw")
    .about("A fast workspace manager")
    .author(Author::new("Brocode").email("bros@brocode.sh"))
    .flag(
      Flag::new()
        .short("-h")
        .long("--help")
        .help("Print help information.")
    )
    .flag(
      Flag::new()
        .short("-q")
        .help("Make fw quiet.")
    )
    .flag(
      Flag::new()
        .short("-v")
        .help("Sets the level of verbosity.")
    )
    .flag(
      Flag::new()
        .short("-V")
        .long("--version")
        .help("Print version information.")
    )
    .option(
      Opt::new("<NAME> <URL>")
        .long("add")
        .help("Add project to config.")
    )
    .option(
      Opt::new("<NAME> <REMOTE_NAME> <URL>")
        .long("add-remote")
        .help("Add remote to project.")
    )
    .option(
      Opt::new("<CMD>")
        .long("foreach")
        .help("Run script on each project.")
    )
    .option(
      Opt::new("<OPTIONS>")
        .long("gen-reworkon")
        .help("Generate sourceable shell code to re-work on project.")
    )
    .option(
      Opt::new("<PROJECT_NAME>")
        .long("gen-workon")
        .help("Generate sourceable shell code to work on project.")
    )
    .option(
      Opt::new("<OPTIONS>")
        .long("gitlab-import")
        .help("Import all owned repositories / your organizations repositories from gitlab into fw.")
    )
    .option(
      Opt::new("<SUBCOMMANDS>")
        .long("help")
        .help("Print the help message or the help of the given subcommand(s).")
    )
    .option(
      Opt::new("<PROJECT_DIR>")
        .long("import")
        .help("Import existing git folder into fw.")
    )
    .option(
      Opt::new("<PROJECT_NAME>")
        .long("inspect")
        .help("Inspect project.")
    )
    .option(
      Opt::new("<OPTIONS>")
        .long("intellij")
        .help("Add projects to intellijs list of recent projects.")
    )
    .option(
      Opt::new("<OPTIONS>")
        .long("ls")
        .help("List projects.")
    )
    .option(
      Opt::new("<ORG_NAME>")
        .long("org-import")
        .help("Import all repositories from github org into fw. Token can be set in the settings file or provided via the environment variable FW_GITHUB_TOKEN.")
    )
    .option(
      Opt::new("<OPTIONS>")
        .long("print-bash-setup")
        .help("Prints bash completion code.")
    )
    .option(
      Opt::new("<OPTIONS>")
        .long("print-fish-setup")
        .help("Prints fish completion code.")
    )
    .option(
      Opt::new("<PROJECT_NAME>")
        .long("print-path")
        .help("Print project path on stdout.")
    )
    .option(
      Opt::new("<OPTIONS>")
        .long("print-zsh-setup")
        .help("Print zsh completion code.")
    )
    .option(
      Opt::new("")
        .long("projectile")
        .help("Write projectile bookmarks.")
    )
    .option(
      Opt::new("<NAME>")
        .long("remove")
        .help("Remove project from config.")
    )
    .option(
      Opt::new("<NAME> <REMOTE_NAME>")
        .long("remove-remote")
        .help("Removes remote from project (Only in the fw configuration. An existing remote will not be deleted by sync).")
    )
    .option(
      Opt::new("")
        .long("reworkon")
        .help("Re-run workon hooks for current dir (aliases: .|rw|re|fkbr).")
    )
    .option(
      Opt::new("<WORKSPACE_DIR>")
        .long("setup")
        .help("Setup config from existing workspace.")
    )
    .option(
      Opt::new("<OPTIONS>")
        .long("sync")
        .help("Sync workspace. Clones projects or updates remotes for existing projects.")
    )
    .option(
      Opt::new("<SUBCOMMAND>")
        .long("tag")
        .help("Allows working with tags.")
    )
    .option(
      Opt::new("<NAME>")
        .long("update")
        .help("Modifies project settings.")
    )
    .custom(
      Section::new("Why fw?")
        .paragraph("With fw you have a configuration describing your workspace. It takes care of cloning projects and can run commands across your entire workspace. You can start working on any project quickly, even if it’s not in your flat structured workspace (better than CDPATH!). It also “sets up” your environment when you start working on a project (compile stuff, run make, activate virtualenv or nvm, fire up sbt shell, etc.")
    )
   .custom(
      Section::new("What this is, and isn't")
        .paragraph("fw is a tool I wrote to do my bidding. It might not work for you if your workflow differs a lot from mine or might require adjustments. Here are the assumptions:")
        .paragraph("* only git repositories")
        .paragraph("* only ssh clone (easily resolveable by putting more work in the git2 bindings usage")
        .paragraph("* ssh-agent based authentication")
    )
    .custom(
      Section::new("If you can live with all of the above, you get:")
        .paragraph("* Workspace persistence ( I can rm -rf my entire workspace and have it back in a few minutes")
        .paragraph("* ZERO overhead project switching with the workon function (need to activate nvm ? Run sbt ? Set LCD brightness to 100% ? fw will do all that for you")
        .paragraph("* zsh completions on the project names for workon")
        .paragraph("* generate projectile configuration for all your project (no need to projectile-add-known-project every time you clone some shit, it will just work")
    )
    .custom (
      Section::new("Overriding the config file location / multiple config files (profiles)")
        .paragraph("Just set the environment variable FW_CONFIG_DIR. This is also honored by fw setup and fw org-import so you can create more than one configuration this way and switch at will.")
    )
    .custom (
      Section::new("Migration to fw / Configuration")
        .paragraph("Initial setup is done with:")
        .paragraph("$  fw setup DIR")
        .paragraph("This will look through DIR (flat structure!) and inspect all git repositories, then write the configuration in your home. You can edit the configuration manually to add stuff. If you have repositories elsewhere you will need to add them manually and set the override_path property. The configuration is portable as long as you change the workspace attribute, so you can share the file with your colleagues (projects with override_path set won’t be portable obviously. You can also add shell code to the after_clone and after_workon fields on a per-project basis. after_clone will be executed after cloning the project (interpreter is sh) and after_workon will be executed each time you workon into the project.")
        .paragraph("If you want to pull in all projects from a GitHub organization there’s fw org-import <NAME> for that (note that you need a minimal config first).")
    )
    .custom(
      Section::new("Turn fw configuration into reality")
        .paragraph("From now on you can")
        .paragraph("$  fw sync # Make sure your ssh agent has your key otherwise this command will just hang because it waits for your password (you can't enter it!).")
        .paragraph("which will clone all missing projects that are described by the configuration but not present in your workspace. Existing projects will be synced with the remote. That means a fast-forward is executed if possible.")
    )
    .custom(
      Section::new("Running command across all projects")
        .paragraph("The is also")
        .paragraph("$  fw foreach 'git remote update --prune'")
        .paragraph("which will run the command in all your projects using sh.")
    )
    .custom(
      Section::new("Updating fw configuration (adding new project)")
        .paragraph("Instead of cloning new projects you want to work on, I suggest adding a new project to your configuration. This can be done using the tool with")
        .paragraph("$  fw add git@github.com:brocode/fw.git")
        .paragraph("(you should run fw sync afterwards! If you don’t want to sync everything use fw sync -n) In case you don’t like the computed project name (the above case would be fw) you can override this (like with git clone semantics):")
        .paragraph("$  fw add git@github.com:brocode/fw.git my-fw-clone")
        .paragraph("If you're an emacs user you should always run")
        .paragraph("$  fw projectile")
        .paragraph("after a sync. This will overwrite your projectile bookmarks so that all your fw managed projects are known. Be careful: Anything that is not managed by fw will be lost.")
    )
    .custom(
      Section::new("Workon usage")
        .paragraph("Just")
        .paragraph("$  workon")
        .paragraph("It will open a fuzzy finder which you can use to select a project. Press <enter> on a selection and it will drop you into the project folder and execute all the hooks.")
        .paragraph("If you’re in a pinch and just want to check something real quick, then you can use")
        .paragraph("$  nworkon")
        .paragraph("as that will no execute any post-workon hooks and simply drop you into the project folder.")
        .paragraph("In case you’re not using fzf integration (see above) you will need to pass an argument to workon / nworkon (the project name). It comes with simple prefix-based autocompletion.")
    )
   .render();
  println!("{}", page);
}
