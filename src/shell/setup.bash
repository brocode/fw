command -v fw >/dev/null 2>&1 &&

__fw_complete()
{
    #
    # This is taken from bash-completion https://github.com/scop/bash-completion
    #
    _get_comp_words_by_ref()
    {
        _upvar()
        {
            if unset -v "$1"; then
                if (( $# == 2 )); then
                    eval $1=\"\$2\"
                else
                    eval $1=\(\"\${@:2}\"\)
                fi
            fi
        }

        _upvars()
        {
            if ! (( $# )); then
                echo "${FUNCNAME[0]}: usage: ${FUNCNAME[0]} [-v varname"\
                    "value] | [-aN varname [value ...]] ..." 1>&2
                return 2
            fi
            while (( $# )); do
                case $1 in
                    -a*)
                        [[ ${1#-a} ]] || { echo "bash: ${FUNCNAME[0]}: \`$1': missing"\
                            "number specifier" 1>&2; return 1; }
                        printf %d "${1#-a}" &> /dev/null || { echo "bash:"\
                            "${FUNCNAME[0]}: \`$1': invalid number specifier" 1>&2
                            return 1; }
                        [[ "$2" ]] && unset -v "$2" && eval $2=\(\"\${@:3:${1#-a}}\"\) &&
                        shift $((${1#-a} + 2)) || { echo "bash: ${FUNCNAME[0]}:"\
                            "\`$1${2+ }$2': missing argument(s)" 1>&2; return 1; }
                        ;;
                    -v)
                        [[ "$2" ]] && unset -v "$2" && eval $2=\"\$3\" &&
                        shift 3 || { echo "bash: ${FUNCNAME[0]}: $1: missing"\
                        "argument(s)" 1>&2; return 1; }
                        ;;
                    *)
                        echo "bash: ${FUNCNAME[0]}: $1: invalid option" 1>&2
                        return 1 ;;
                esac
            done
        }

        __reassemble_comp_words_by_ref()
        {
            local exclude i j line ref
            if [[ $1 ]]; then
                exclude="${1//[^$COMP_WORDBREAKS]}"
            fi

            printf -v "$3" %s "$COMP_CWORD"
            if [[ $exclude ]]; then
                line=$COMP_LINE
                for (( i=0, j=0; i < ${#COMP_WORDS[@]}; i++, j++)); do
                    while [[ $i -gt 0 && ${COMP_WORDS[$i]} == +([$exclude]) ]]; do
                        [[ $line != [[:blank:]]* ]] && (( j >= 2 )) && ((j--))
                        ref="$2[$j]"
                        printf -v "$ref" %s "${!ref}${COMP_WORDS[i]}"
                        [[ $i == $COMP_CWORD ]] && printf -v "$3" %s "$j"
                        line=${line#*"${COMP_WORDS[$i]}"}
                        [[ $line == [[:blank:]]* ]] && ((j++))
                        (( $i < ${#COMP_WORDS[@]} - 1)) && ((i++)) || break 2
                    done
                    ref="$2[$j]"
                    printf -v "$ref" %s "${!ref}${COMP_WORDS[i]}"
                    line=${line#*"${COMP_WORDS[i]}"}
                    [[ $i == $COMP_CWORD ]] && printf -v "$3" %s "$j"
                done
                [[ $i == $COMP_CWORD ]] && printf -v "$3" %s "$j"
            else
                for i in ${!COMP_WORDS[@]}; do
                    printf -v "$2[i]" %s "${COMP_WORDS[i]}"
                done
            fi
        }

        __get_cword_at_cursor_by_ref()
        {
            local cword words=()
            __reassemble_comp_words_by_ref "$1" words cword

            local i cur index=$COMP_POINT lead=${COMP_LINE:0:$COMP_POINT}
            if [[ $index -gt 0 && ( $lead && ${lead//[[:space:]]} ) ]]; then
                cur=$COMP_LINE
                for (( i = 0; i <= cword; ++i )); do
                    while [[
                        ${#cur} -ge ${#words[i]} &&
                        "${cur:0:${#words[i]}}" != "${words[i]}"
                    ]]; do
                        cur="${cur:1}"
                        [[ $index -gt 0 ]] && ((index--))
                    done

                    if [[ $i -lt $cword ]]; then
                        local old_size=${#cur}
                        cur="${cur#"${words[i]}"}"
                        local new_size=${#cur}
                        index=$(( index - old_size + new_size ))
                    fi
                done
                [[ $cur && ! ${cur//[[:space:]]} ]] && cur=
                [[ $index -lt 0 ]] && index=0
            fi

            local "$2" "$3" "$4" && _upvars -a${#words[@]} $2 "${words[@]}" \
                -v $3 "$cword" -v $4 "${cur:0:$index}"
        }

        local exclude flag i OPTIND=1
        local cur cword words=()
        local upargs=() upvars=() vcur vcword vprev vwords

        while getopts "c:i:n:p:w:" flag "$@"; do
            case $flag in
                c) vcur=$OPTARG ;;
                i) vcword=$OPTARG ;;
                n) exclude=$OPTARG ;;
                p) vprev=$OPTARG ;;
                w) vwords=$OPTARG ;;
            esac
        done
        while [[ $# -ge $OPTIND ]]; do
            case ${!OPTIND} in
                cur)   vcur=cur ;;
                prev)  vprev=prev ;;
                cword) vcword=cword ;;
                words) vwords=words ;;
                *) echo "bash: $FUNCNAME(): \`${!OPTIND}': unknown argument" \
                    1>&2; return 1
            esac
            let "OPTIND += 1"
        done

        __get_cword_at_cursor_by_ref "$exclude" words cword cur

        [[ $vcur   ]] && { upvars+=("$vcur"  ); upargs+=(-v $vcur   "$cur"  ); }
        [[ $vcword ]] && { upvars+=("$vcword"); upargs+=(-v $vcword "$cword"); }
        [[ $vprev && $cword -ge 1 ]] && { upvars+=("$vprev" ); upargs+=(-v $vprev
            "${words[cword - 1]}"); }
        [[ $vwords ]] && { upvars+=("$vwords"); upargs+=(-a${#words[@]} $vwords
            "${words[@]}"); }

        (( ${#upvars[@]} )) && local "${upvars[@]}" && _upvars "${upargs[@]}"
    }

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
        done < <(fw tag ls)
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
            --*) __fw_comp "--after-clone --after-workon --workspace" ; return ;;
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
    done < <(fw ls)
    echo ${projects[@]}
}
