__workon () {
  PROJECT="$(fw ls | fzf --cycle --query=$1 --preview-window=top:50% --preview='fw inspect {}' --no-mouse --select-1)"
  SCRIPT="$(fw gen-workon $2 $PROJECT)";
  if [ $? -eq 0 ]; then
    eval "$SCRIPT";
  else
    printf "$SCRIPT\n";
  fi
};

reworkon () {
  SCRIPT="$(fw gen-reworkon $@)";
  if [ $? -eq 0 ]; then
    eval "$SCRIPT";
  else
    printf "$SCRIPT\n";
  fi
};

workon () {
  __workon "$1"
};

nworkon () {
  __workon "$1" "-x"
};
