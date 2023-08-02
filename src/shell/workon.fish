function __fish_fw_use_script
  if test $argv[1] -eq 0
    echo $argv[2] | source
  else
    printf "$argv[2]\n"
  end
end

function workon
  set -l script (fw gen-workon $argv)
  __fish_fw_use_script $status $script
end

function nworkon
  set -l script (fw gen-workon -x $argv)
  __fish_fw_use_script $status $script
end

function reworkon
  set -l script (fw gen-reworkon $argv)
  __fish_fw_use_script $status $script
end

complete -c workon -f -xa "(__fw_projects)"
complete -c nworkon -f -xa "(__fw_projects)"
