function __fish_fw_use_script
  if test $argv[1] -eq 0
    echo $argv[2] | source
  else
    printf "$argv[2]\n"
  end
end

function __workon
  set -l project (fw -q ls | fzf --cycle --query=$argv[1] --color=light --preview-window=top:50% --preview='fw -q inspect {}' --no-mouse --select-1)
  set -l script (fw -q gen-workon $argv[2] $project)
  __fish_fw_use_script $status $script
end

function workon
  __workon $argv[1]
end

function nworkon
  __workon $argv[1] -x
end

function reworkon
  set -l script (fw -q gen-reworkon $argv)
  __fish_fw_use_script $status $script
end

complete -c workon -f -xa "(__fw_projects)"
complete -c nworkon -f -xa "(__fw_projects)"
