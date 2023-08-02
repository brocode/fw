workon () {
  SCRIPT="$(fw gen-workon $@)";
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

nworkon () {
  SCRIPT="$(fw gen-workon -x $@)";
  if [ $? -eq 0 ]; then
    eval "$SCRIPT";
  else
    printf "$SCRIPT\n";
  fi
};

_workon() {
  if ! command -v fw > /dev/null 2>&1; then
      _message "fw not installed";
  else
    __fw_projects;
  fi
};

compdef _workon workon;
compdef _workon nworkon;
