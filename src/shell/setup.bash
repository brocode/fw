command -v fw >/dev/null 2>&1 &&

_fw()
{
    local cur prev pprev
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    pprev="${COMP_WORDS[COMP_CWORD-2]}"

    # looking for the command
    local nwords=${#COMP_WORDS[@]}
    local cmd_i cmd dd_i
    for (( cmd_i=1; cmd_i<$COMP_WORDS; cmd_i++ )); do
		if [[ ! "${COMP_WORDS[$cmd_i]}" =~ ^[+-] ]]; then
			cmd="${COMP_WORDS[$cmd_i]}"
			break
		fi
	done

    # Find the location of the -- separator.
	for (( dd_i=1; dd_i<$nwords-1; dd_i++ )); do
		if [[ "${words[$dd_i]}" = "--" ]]; then
			break
		fi
	done

    local opt_help='-h --help'
    local opt_verbose='-v --verbose'
    local opt_quiet='-q'
    local opt_common="$opt_help $opt_verbose $opt_quiet"
    local opt_proj_cfg="--after-clone --after-workon --override-path"
    local opt_parallel="-p"
    local opt_tag="-t --tag"
    local opt_json="-j --json"
    local opt_purge="-p --purge-directory"

    local opt__nocmd="$opt_common -V --version"
    local opt__add="$opt_common $opt_proj_cfg"
    local opt__add_remote="$opt_common"
    local opt__export_by_tag="$opt_common"
    local opt__export_project="$opt_common"
    local opt__export_tag="$opt_common"
    local opt__foreach="$opt_common $opt_parallel $opt_tags"
    local opt__gitlab_import="$opt_common"
    local opt__help="$opt_common"
    local opt__import="$opt_common"
    local opt__inspect="$opt_common $opt_json"
    local opt__ls="$opt_common"
    local opt__org_import="$opt_common -a --include-archived"
    local opt__print_path="$opt_common"
    local opt__projectile="$opt_common"
    local opt__remove="$opt_common $opt_purge"
    local opt__remove_remote="$opt_common"
    local opt__setup="$opt_common"
    local opt__sync="$opt_common $opt_parallel --no-ff-merge -q --no-progress-bar -n --only-new"
    local opt__tag="$opt_common"
    local opt__update="$opt_common $opt_proj_cfg --git-url"

    if [[ $COMP_CWORD -gt $dd_i ]]; then
        # completion after -- separator
        COMPREPLY=( $( compgen -f  -- "${COMP_WORDS[${COMP_CWORD}]}" ) )
    elif [[ $COMP_CWORD -le $cmd_i ]]; then
        # at either before or at the command
        if [[ "$cur" == -* ]]; then
            COMPREPLY=( $( compgen -W "${opt___nocmd}" -- "$cur" ) )
        else
            COMPREPLY=( $( compgen -W "$(__fw_commands)" -- "$cur" ) )
        fi
    else
        case "${prev}" in
            help) COMPREPLY=( $( compgen -W "$(__fw_commands)" -- "$cur" ) ) ;;
            tag) COMPREPLY=( $( compgen -W "$(__fw_tag_commands)" -- "$cur" ) ) ;;
            remove) COMPREPLY=( $( compgen -W "$(__fw_projects)" -- "$cur" ) ) ;;
            *)
                local opt_var=opt__${cmd//-/_}
				if [[ -z "${!opt_var}" ]]; then
                    # fallback to filename completion
                    COMPREPLY=( $( compgen -f  -- "${COMP_WORDS[${COMP_CWORD}]}" ) )
                else
                    COMPREPLY=( $( compgen -W "${!opt_var}" -- "$cur" ) )
                fi
                ;;
        esac
    fi

    return 0
} &&
complete -F _fw fw

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

__fw_tag_commands()
{
    local cmds=(
        'add'
        'autotag'
        'help'
        'ls'
        'rm'
        'tag-project'
        'untag-project'
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
