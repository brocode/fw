workon () {
  PROJECT="$(fw -q ls | fzf)"
  SCRIPT="$(~/.cargo/bin/fw -q gen-workon $PROJECT)";
  if [ $? -eq 0 ]; then
    eval "$SCRIPT";
  else
    printf "$SCRIPT\n";
  fi
};

nworkon () {
  PROJECT="$(fw -q ls | fzf)"
  SCRIPT="$(~/.cargo/bin/fw -q gen-workon -x $PROJECT)";
  if [ $? -eq 0 ]; then
    eval "$SCRIPT";
  else
    printf "$SCRIPT\n";
  fi
};
