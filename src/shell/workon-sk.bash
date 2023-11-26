__workon()
{
    local PROJECT="$(fw ls | sk --query=$1 --preview-window=up:50% --preview='fw inspect {}' --no-mouse --select-1)"
    local SCRIPT="$(fw gen-workon $2 $PROJECT)"
    case $(uname -s) in
        MINGW*|MSYS*) SCRIPT="cd $(echo "/${SCRIPT:3}" | sed -e 's/\\/\//g' -e 's/://')" ;;
    esac
    [ $? -eq 0 ] && eval "$SCRIPT" || printf "$SCRIPT\n"
}

reworkon()
{
    local SCRIPT="$(fw gen-reworkon $@)"
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
