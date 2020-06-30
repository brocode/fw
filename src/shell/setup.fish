function __fw_projects
  fw -q ls
end

function __fw_tags
  fw -q tag ls
end

function __fw_subcommands
  set -l __fw_subcommands_in_zsh_format \
    'sync:Sync workspace' \
    'setup:Setup config from existing workspace' \
    'import:Import existing git folder to fw' \
    'add:Add project to workspace' \
    'add-remote:Add remote to project' \
    'remove-remote:Removes remote from project' \
    'remove:Remove project from workspace' \
    'foreach:Run script on each project' \
    'projectile:Create projectile bookmarks' \
    'ls:List projects' \
    'inspect:Inspect project' \
    'update:Update project settings' \
    'tag:Manipulate tags' \
    'print-path:Print project path to stdout' \
    'org-import:Import all repositories from a github org' \
    'gitlab-import:Import all owned repositories / your organizations repositories from gitlab'

  for subcmd in $__fw_subcommands_in_zsh_format
    echo (string replace -r ':' '\t' $subcmd)
  end
end

function __fw_tag_subcommands
  set -l __fw_tag_subcommands_in_zsh_format \
    'add:Adds a tag' \
    'rm:Removes a tag' \
    'ls:Lists tags' \
    'inspect:inspect a tag' \
    'tag-project:Add a tag to a project' \
    'untag-project:Remove a tag from a project' \
    'autotag:Execute command for every tagged project'

  for subcmd in $__fw_tag_subcommands_in_zsh_format
    echo (string replace -r ':' '\t' $subcmd)
  end
end

function __fish_fw_is_arg_n -d 'Indicates if completion is for the nth argument (ignoring options)'
  set -l args (__fish_print_cmd_args_without_options)

  test (count $args) -eq $argv[1]
end

function __fish_fw_needs_command
  __fish_fw_is_arg_n 1
end

function __fish_fw_command_in
  set -l args (__fish_print_cmd_args_without_options)

  contains -- $args[2] $argv
end

function __fish_fw_subcommand_in
  set -l args (__fish_print_cmd_args_without_options)

  contains -- $args[3] $argv
end

function __fish_fw_completion_for_command
  __fish_fw_is_arg_n 2; and __fish_fw_command_in $argv[1]
end

function __fish_fw_completion_for_command_subcommand
  __fish_fw_is_arg_n 3; and __fish_fw_command_in $argv[1]; and __fish_fw_subcommand_in $argv[2]
end

function __fish_fw_needs_project_arg
  if __fish_fw_is_arg_n 2
    __fish_fw_command_in add-remote remove-remote print-path inspect update remove
  else if __fish_fw_is_arg_n 3 and __fish_fw_command_in tag
    __fish_fw_subcommand_in ls tag-project untag-project
  else
    return 1
  end
end

function __fish_fw_needs_tag_arg
  if ! __fish_fw_command_in tag
    return 1
  end

  if __fish_fw_is_arg_n 3
    __fish_fw_subcommand_in inspect rm
  else if __fish_fw_is_arg_n 4
    __fish_fw_subcommand_in tag-project untag-project
  else
    return 1
  end
end

complete -ec fw

complete -c fw -n '__fish_fw_needs_command' -f -xa '(__fw_subcommands)'
complete -c fw -n '__fish_fw_command_in tag; and __fish_fw_is_arg_n 2' -f -xa '(__fw_tag_subcommands)'
complete -c fw -n '__fish_fw_needs_project_arg' -f -xa '(__fw_projects)'
complete -c fw -n '__fish_fw_needs_tag_arg' -f -xa '(__fw_tags)'

complete -c fw -n '__fish_fw_is_arg_n 1' -s V -l version -d 'Print version information'
complete -c fw -n '__fish_fw_is_arg_n 1' -s h -l help    -d 'Print help information'
complete -c fw -n '__fish_fw_is_arg_n 1' -s q            -d 'Make fw quiet'
complete -c fw -n '__fish_fw_is_arg_n 1' -s v            -d 'Set the level of verbosity'

complete -c fw -n 'not __fish_fw_is_arg_n 1' -l help -s h -d 'Print help information for subcommand'

complete -c fw -n '__fish_fw_completion_for_command sync'      -l no-ff-merge \
  -d 'No fast forward merge'
complete -c fw -n '__fish_fw_completion_for_command sync' -s q -l no-progress-bar
complete -c fw -n '__fish_fw_completion_for_command sync' -s n -l only-new \
  -d 'Only clones projects, skips all actions for projects already on your machine.'
complete -c fw -n '__fish_fw_completion_for_command sync' -s p -l parallelism \
  -d 'Set the number of threads'

complete -c fw -n '__fish_fw_completion_for_command org-import' -s a -l include-archived

complete -c fw -n '__fish_fw_completion_for_command foreach' -s p \
  -d 'Set the number of threads'
complete -c fw -n '__fish_fw_completion_for_command foreach' -s t -l tag \
  -d 'Filter projects by tag. More than 1 is allowed.'

complete -c fw -n '__fish_fw_completion_for_command ls' -s t -l tag \
  -d 'Filter projects by tag. More than 1 is allowed.'

complete -c fw -n '__fish_fw_completion_for_command update' -l after-clone
complete -c fw -n '__fish_fw_completion_for_command update' -l after-workon
complete -c fw -n '__fish_fw_completion_for_command update' -l git-url
complete -c fw -n '__fish_fw_completion_for_command update' -l override-path

complete -c fw -n '__fish_fw_completion_for_command_subcommand tag add' -l after-clone
complete -c fw -n '__fish_fw_completion_for_command_subcommand tag add' -l after-workon
complete -c fw -n '__fish_fw_completion_for_command_subcommand tag add' -l git-url
complete -c fw -n '__fish_fw_completion_for_command_subcommand tag add' -l override-path

complete -c fw -n '__fish_fw_completion_for_command_subcommand tag autotag' -s p \
  -d 'Set the number of threads'
