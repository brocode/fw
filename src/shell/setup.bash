__fw_commands()
{
    local cmds=(
        'sync'
        'setup'
        'import'
        'add'
        'add-remote'
        'remove-remote'
        'remove'
        'foreach'
        'projectile'
        'ls'
        'inspect'
        'update'
        'tag'
        'export-project'
        'export-by-tag'
        'export-tag'
        'print-path'
        'org-import'
        'gitlab-import'
    )
    echo "${cmds[@]}"
}

__fw_projects()
{
    local projects=()
    while read line; do
        projects+=($line)
    done < <(fw -q ls)
    echo ${projects[@]}
}
