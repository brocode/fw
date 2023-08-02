workon()
{
    local SCRIPT="$(fw gen-workon $@)"
    case $(uname -s) in
        MINGW*|MSYS*) SCRIPT="cd $(echo "/${SCRIPT:3}" | sed -e 's/\\/\//g' -e 's/://')" ;;
    esac
    [ $? -eq 0 ] && eval "$SCRIPT" || printf "$SCRIPT"
}

reworkon()
{
    local SCRIPT="$(fw gen-reworkon $@)"
    case $(uname -s) in
        MINGW*|MSYS*) SCRIPT="cd $(echo "/${SCRIPT:3}" | sed -e 's/\\/\//g' -e 's/://')" ;;
    esac
    [ $? -eq 0 ] && eval "$SCRIPT" || printf "$SCRIPT"
}

reworkon()
{
    local SCRIPT="$(fw gen-workon -x $@)"
    case $(uname -s) in
        MINGW*|MSYS*) SCRIPT="cd $(echo "/${SCRIPT:3}" | sed -e 's/\\/\//g' -e 's/://')" ;;
    esac
    [ $? -eq 0 ] && eval "$SCRIPT" || printf "$SCRIPT"
}

_workon()
{
    COMPREPLY=($(compgen -W "$(__fw_projects)" -- ${COMP_WORDS[1]}))
}

complete -F _workon workon
