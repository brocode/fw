# Usage

### Overriding the config file location / multiple config files (profiles)

Just set the environment variable `FW_CONFIG_DIR`. This is also honored
by `fw setup` and `fw org-import` so you can create more than one
configuration this way and switch at will.

### Migrating to `fw` / Configuration

Initial setup is done with

``` bash
fw setup DIR
```

This will look through `DIR` (flat structure!) and inspect all git
repositories, then write the configuration in your home. You can edit
the configuration manually to add stuff. If you have repositories
elsewhere you will need to add them manually and set the `override_path`
property. The configuration is portable as long as you change the
`workspace` attribute, so you can share the file with your colleagues
(projects with `override_path` set won\'t be portable obviously. You can
also add shell code to the `after_clone` and `after_workon` fields on a
per-project basis. `after_clone` will be executed after cloning the
project (interpreter is `sh`) and `after_workon` will be executed each
time you `workon` into the project.

If you want to pull in all projects from a GitHub organization there\'s
`fw org-import <NAME>` for that (note that you need a minimal config
first).

### Turn `fw` configuration into reality

From now on you can

``` bash
fw sync # Make sure your ssh agent has your key otherwise this command will just hang because it waits for your password (you can't enter it!).
```

which will clone all missing projects that are described by the
configuration but not present in your workspace. Existing projects will
be synced with the remote. That means a fast-forward is executed if
possible.

### Running command across all projects

There is also

``` bash
fw foreach 'git remote update --prune'
```

which will run the command in all your projects using `sh`.

### Updating `fw` configuration (adding new project)

Instead of cloning new projects you want to work on, I suggest adding a
new project to your configuration. This can be done using the tool with

``` bash
fw add git@github.com:brocode/fw.git
```

(you should run `fw` sync afterwards! If you don\'t want to sync
everything use `fw sync -n`) In case you don\'t like the computed
project name (the above case would be `fw`) you can override this (like
with `git clone` semantics):

``` bash
fw add git@github.com:brocode/fw.git my-fw-clone
```

If you\'re an emacs user you should always run

``` bash
fw projectile
```

after a `sync`. This will overwrite your projectile bookmarks so that
all your `fw` managed projects are known. Be careful: Anything that is
not managed by fw will be lost.

## workon usage

Just

``` bash
workon
```

It will open a fuzzy finder which you can use to select a project. Press
\<enter\> on a selection and it will drop you into the project folder
and execute all the hooks.

If you\'re in a pinch and just want to check something real quick, then
you can use

    nworkon

as that will no execute any post-workon hooks and simply drop you into
the project folder.

In case you\'re not using `fzf` integration (see above) you will need to
pass an argument to `workon` / `nworkon` (the project name). It comes
with simple prefix-based autocompletion.
