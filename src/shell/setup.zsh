__fw_projects() {
  local projects;
  fw -q ls | while read line; do
      projects+=( $line );
  done;
  _describe -t projects 'project names' projects;
};

__fw_tags() {
  local tags;
  fw -q tag ls | while read line; do
      tags+=( $line );
  done;
  _describe -t tags 'tag names' tags;
};

_fw() {
  if ! command -v fw > /dev/null 2>&1; then
      _message "fw not installed";
  else
      _arguments '1: :->first' '2: :->second' '3: :->third' '4: :->fourth';

      case $state in
        first)
          actions=(
            'sync:Sync workspace'
            'setup:Setup config from existing workspace'
            'import:Import existing git folder to fw'
            'add:Add project to workspace'
            'add-remote:Add remote to project'
            'remove-remote:Removes remote from project'
            'remove:Remove project from workspace'
            'foreach:Run script on each project'
            'projectile:Create projectile bookmarks'
            'ls:List projects'
            'inspect:Inspect project'
            'update:Update project settings'
            'tag:Manipulate tags'
            'export-project:Exports a project as shell commands'
            'export-by-tag:Exports all project with a given tag'
            'export-tag:Exports a tag'
            'print-path:Print project path to stdout'
            'org-import:Import all repositories from a github org'
            'gitlab-import:Import all owned repositories / your organizations repositories from gitlab'
          );
          _describe action actions && ret=0;
        ;;
        second)
          case $words[2] in
            sync)
              _arguments '*:option:(--no-ff-merge)';
            ;;
            org-import)
              _arguments '*:option:(--include-archived)';
            ;;
            add-remote)
              __fw_projects;
            ;;
            remove-remote)
              __fw_projects;
            ;;
            print-path)
              __fw_projects;
            ;;
            inspect)
              __fw_projects;
            ;;
            update)
              __fw_projects;
            ;;
            remove)
              __fw_projects;
            ;;
            export-project)
              __fw_projects;
            ;;
            export-by-tag)
              __fw_tags;
            ;;
            export-tag)
              __fw_tags;
            ;;
            tag)
              actions=(
                'add:Adds a tag'
                'rm:Removes a tag'
                'ls:Lists tags'
                'tag-project:Add a tag to a project'
                'untag-project:Remove a tag from a project'
                'autotag:Execute command for every tagged project'
              );
              _describe action actions && ret=0;
            ;;
            *)
            ;;
          esac
        ;;
        third)
          case $words[2] in
            update)
              _arguments '*:option:(--override-path --git-url --after-clone --after-workon)';
            ;;
            remove)
              _arguments '*:option:(--purge-directory)';
            ;;
            tag)
              case $words[3] in
              tag-project)
                __fw_projects;
              ;;
              untag-project)
                __fw_projects;
              ;;
              ls)
                __fw_projects;
              ;;
              rm)
                __fw_tags;
              ;;
              *)
              ;;
              esac
            ;;
            *)
            ;;
          esac
        ;;
       fourth)
          case $words[2] in
            tag)
              case $words[3] in
              tag-project)
                __fw_tags;
              ;;
              untag-project)
                __fw_tags;
              ;;
              *)
              ;;
              esac
            ;;
            *)
            ;;
          esac
       ;;
       esac
  fi
};
compdef _fw fw;
