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
    .render();
  println!("{}", page);
}
