workon () {
  PROJECT="$(fw -q ls | fzf --cycle --query=$1 --color=light --preview-window=top:50% --preview='~/.cargo/bin/fw -q gen-workon {}') --no-mouse"
  SCRIPT="$(~/.cargo/bin/fw -q gen-workon $PROJECT)";
  if [ $? -eq 0 ]; then
    eval "$SCRIPT";
  else
    printf "$SCRIPT\n";
  fi
};

nworkon () {
  PROJECT="$(fw -q ls | fzf --cycle --color=light)"
  SCRIPT="$(~/.cargo/bin/fw -q gen-workon -x $PROJECT)";
  if [ $? -eq 0 ]; then
    eval "$SCRIPT";
  else
    printf "$SCRIPT\n";
  fi
};
