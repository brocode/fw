command -v fw >/dev/null 2>&1 &&

__fw_complete()
{
    __fw_comp()
    {
        local cur_="${3-$cur}"

        case "$cur_" in
            --*=)
                ;;
            *)
                local c i=0 IFS=$' \t\n'
                for c in $1; do
                    c="$c${4-}"
                    if [[ $c == "$cur_"* ]]; then
                        case $c in
                            --*=*|*.) ;;
                            *) c="$c " ;;
                        esac
                        COMPREPLY[i++]="${2-}$c"
                    fi
                done
                ;;
        esac
    }

    __fw_commands()
    {
        local cmds=(
            'add-remote'
            'add'
            'export-by-tag'
            'export-project'
            'export-tag'
            'foreach'
            'gitlab-import'
            'help '
            'import'
            'inspect'
            'ls'
            'org-import'
            'print-path'
            'projectile'
            'remove-remote'
            'remove'
            'reworkon'
            'setup'
            'sync'
            'tag'
            'update'
        )
        echo "${cmds[@]}"
    }

    __fw_tags()
    {
        local tags=()
        while read line; do
            tags+=($line)
        done < <(fw -q tag ls)
        echo ${tags[@]}
    }
    
    __find_on_cmdline() {
        local word subcommand c=1
        while [ $c -lt $cword ]; do
            word="${words[c]}"
            for subcommand in $1; do
                if [ "$subcommand" = "$word" ]; then
                    echo "$subcommand"
                    return
                fi
            done
            ((c++))
        done
    }

    _fw_add() {
        case "$cur" in
            --*) __fw_comp "--after-clone --after-workon --override-path" ; return ;;
        esac
    }

    _fw_add_remote() {
        __fw_comp "$(__fw_projects)"
    }

    _fw_export_by_tag () {
        __fw_comp "$(__fw_tags)"
    }

    _fw_export_project () {
        __fw_comp "$(__fw_projects)"
    }

    _fw_export_tag () {
        __fw_comp "$(__fw_tags)"
    }

    _fw_foreach () {
        case "$prev" in 
            --tag|-t) __fw_comp "$(__fw_tags)" ; return ;;
        esac

        case "$cur" in
            --*) __fw_comp "--parallel --tag" ; return ;;
        esac
    }

    _fw_help () {
        __fw_comp "$(__fw_commands)"
    }

    _fw_import () {
        __fw_comp "$(__fw_projects)"
    }

    _fw_inspect () {
        case "$cur" in
            --*) __fw_comp "--json" ; return ;;
        esac

        __fw_comp "$(__fw_projects)"
    }

    _fw_org_import () {
        case "$cur" in
            --*) __fw_comp "--include-archived" ; return ;;
        esac
    }

    _fw_print_path () {
        __fw_comp "$(__fw_projects)"
    }

    _fw_remove_remote () {
        __fw_comp "$(__fw_projects)"
    }

    _fw_remove () {
        case "$cur" in
            --*) __fw_comp "--purge-directory" ; return ;;
        esac

        __fw_comp "$(__fw_projects)"
    }

    # _fw_reworkon() {
    # }

    _fw_sync () {
        case "$cur" in
            --*) __fw_comp "--no-ff-merge --no-progress-bar --only-new --parallelism" ; return ;;
        esac
    }

    _fw_tag () {
        local subcommands='add autotag help ls rm tag-project untag-project '
        local subcommand="$(__find_on_cmdline "$subcommands")"
        case "$subcommand,$cur" in
            ,*) __fw_comp "$subcommands" ;;
            *)
                local func="_fw_tag_${subcommand//-/_}"
                declare -f $func >/dev/null && $func && return
            ;;
        esac
    }

    _fw_tag_add() {
        case "$cur" in
            --*) __fw_comp "--after-clone --after-workon --override-path" ; return ;;
        esac
        __fw_comp "$(__fw_tags)"
    }

    _fw_tag_autotag() {
        case "$cur" in
            --*) __fw_comp "--parallel" ; return ;;
        esac
        __fw_comp "$(__fw_tags)"
    }

    _fw_tag_help() {
        __fw_comp 'add autotag ls rm tag-project untag-project'
    }

    _fw_tag_ls() {
        __fw_comp "$(__fw_projects)"
    }

    _fw_tag_rm() {
        __fw_comp "$(__fw_tags)"
    }

    _fw_tag_tag_project() {
        local project="$(__find_on_cmdline "$(__fw_projects)")"
        if [ -z "$project" ]; then
            __fw_comp "$(__fw_projects)"
        else
            __fw_comp "$(__fw_tags)"
        fi
    }

    _fw_tag_untag_project() {
        local project="$(__find_on_cmdline "$(__fw_projects)")"
        if [ -z "$project" ]; then
            __fw_comp "$(__fw_projects)"
        else
            __fw_comp "$(__fw_tags)"
        fi
    }

    _fw_update () {
        case "$prev" in
            --*) return ;;
        esac

        case "$cur" in
            --*) __fw_comp "--after-clone --after-workon --git-url --override-path" ; return ;;
        esac

        __fw_comp "$(__fw_projects)"
    }

    # ---------------------------------------------------------------------------------------------------

    __fw_main()
    {
        local i command c=1
        while [ $c -lt $cword ]; do
            i="${words[c]}"
            case $i in
                --help) command="help"; break ;;
                -*) ;;
                *) command="$i"; break ;;
                esac
            ((c++))
        done

        if [ -z "$command" ]; then
            case "$cur" in
            --*)   __fw_comp " --help" ;;
            *)     __fw_comp "$(__fw_commands)" ;;
            esac
            return
        fi

        local completion_func="_fw_${command//-/_}"
        declare -f $completion_func >/dev/null && $completion_func && return
    }

    __fw_wrap()
    {
        local cur words cword prev
        _get_comp_words_by_ref -n =: cur words cword prev
        __fw_main
    }

    complete -o bashdefault -o default -o nospace -F __fw_wrap $1 2>/dev/null \
        || complete -o default -o nospace -F __fw_wrap $1
} &&

__fw_complete fw

# This is needed for workon's completion
__fw_projects()
{
    local projects=()
    while read line; do
        projects+=($line)
    done < <(fw -q ls)
    echo ${projects[@]}
}
