workon()
{
    local SCRIPT="$(fw -q gen-workon $@)"
    [ $? -eq 0 ] && eval "$SCRIPT" || printf "$SCRIPT"
}

reworkon()
{
    local SCRIPT="$(fw -q gen-reworkon $@)"
    [ $? -eq 0 ] && eval "$SCRIPT" || printf "$SCRIPT"
}

reworkon()
{
    local SCRIPT="$(fw -q gen-workon -x $@)"
    [ $? -eq 0 ] && eval "$SCRIPT" || printf "$SCRIPT"
}

_workon()
{
    COMPREPLY=($(compgen -W "$(__fw_projects)" -- ${COMP_WORDS[1]}))
}

complete -F _workon workon
