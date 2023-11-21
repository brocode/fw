![](./logo/fw_rgb.png)

# fw

[![](https://img.shields.io/crates/v/fw.svg)](https://crates.io/crates/fw)

[![](https://asciinema.org/a/222856.png)](https://asciinema.org/a/222856)

## Why fw?

With `fw` you have a configuration describing your workspace. It takes
care of cloning projects and can run commands across your entire
workspace. You can start working on any project quickly, even if it\'s
not in your flat structured workspace (better than `CDPATH`!). It also
\"sets up\" your environment when you start working on a project
(compile stuff, run `make`, activate `virtualenv` or `nvm`, fire up
`sbt` shell, etc.)

[*Here\'s*]{.spurious-link target="doc/example_config"} an example
configuration that should be easy to grasp.

The default configuration location is located under your system\'s
config directory as described
[here](https://docs.rs/dirs/3.0.2/dirs/fn.config_dir.html). That is :

-   Linux: `~/.config/fw`{.verbatim}
-   MacOS: `$HOME/Library/Application Support/fw`{.verbatim}
-   Windows: `{FOLDERID_RoamingAppData}\fw`{.verbatim}

The location and can be overridden by setting `FW_CONFIG_DIR`.

Per default projects are cloned into
`${settings.workspace}/${project.name}` but you can override that by
setting an `override_path` attribute as seen in the example
configuration.

## What this is, and isn\'t

`fw` is a tool I wrote to do my bidding. It might not work for you if
your workflow differs a lot from mine or might require adjustments. Here
are the assumptions:

-   only git repositories
-   only ssh clone (easily resolveable by putting more work in the git2
    bindings usage)
-   `ssh-agent` based authentication

### If you can live with all of the above, you get:

-   workspace persistence (I can `rm -rf` my entire workspace and have
    it back in a few minutes)
-   ZERO overhead project switching with the `workon` function (need to
    activate `nvm`? Run `sbt`? Set LCD brightness to 100%? `fw` will do
    all that for you)
-   zsh completions on the project names for `workon`
-   generate projectile configuration for all your project (no need to
    `projectile-add-known-project` every time you clone some shit, it
    will just work)


## [*Installation*]{.spurious-link target="doc/installation.org"}

## [*Usage*]{.spurious-link target="doc/usage.org"}
