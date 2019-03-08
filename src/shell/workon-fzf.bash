__workon()
{
    local PROJECT="$(fw -q ls | fzf --cycle --query=$1 --color=light --preview-window=top:50% --preview='fw -q inspect {}' --no-mouse)"
    local SCRIPT="$(fw -q gen-workon $2 $PROJECT)"
    case $(uname -s) in
        MINGW*|MSYS*) SCRIPT="cd $(echo "/${SCRIPT:3}" | sed -e 's/\\/\//g' -e 's/://')" ;; 
    esac
    [ $? -eq 0 ] && eval "$SCRIPT" || printf "$SCRIPT\n"
}

reworkon()
{
    local SCRIPT="$(fw -q gen-reworkon $@)"
    case $(uname -s) in
        MINGW*|MSYS*) SCRIPT="cd $(echo "/${SCRIPT:3}" | sed -e 's/\\/\//g' -e 's/://')" ;; 
    esac
    [ $? -eq 0 ] && eval "$SCRIPT" || printf "$SCRIPT"
}

workon()
{
    __workon "$1"
}

nworkon()
{
    __workon "$1" "-x"
}
