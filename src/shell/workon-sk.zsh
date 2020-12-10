__workon () {
  PROJECT="$(fw -q ls | sk --query=$1 --color=light --preview-window=top:50% --preview='fw -q inspect {}' --no-mouse)"
  SCRIPT="$(fw -q gen-workon $2 $PROJECT)";
  if [ $? -eq 0 ]; then
    eval "$SCRIPT";
  else
    printf "$SCRIPT\n";
  fi
};

reworkon () {
  SCRIPT="$(fw -q gen-reworkon $@)";
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
