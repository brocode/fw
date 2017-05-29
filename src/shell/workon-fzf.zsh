__workon () {
  PROJECT="$(fw -q ls | fzf --cycle --query=$1 --color=light --preview-window=top:50% --preview='~/.cargo/bin/fw -q inspect {}' --no-mouse)"
  SCRIPT="$(~/.cargo/bin/fw -q gen-workon $2 $PROJECT)";
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
