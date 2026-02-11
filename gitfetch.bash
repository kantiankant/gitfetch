#!/usr/bin/env bash
# Bash completion for gitfetch v0.18
# Now with 100% more trust issues

_gitfetch_completions() {
  local cur prev words cword
  _init_completion || return

  # The command being completed
  local cmd="${words[1]}"

  # If we're still at the command level
  if [ "$cword" -eq 1 ]; then
    local commands="clone -c list -l search -s easter-egg -e completions checksum verify help --help -h --version -V"
    COMPREPLY=($(compgen -W "$commands" -- "$cur"))
    return 0
  fi

  # Handle completions based on the subcommand
  case "$cmd" in
  clone | -c)
    # CRITICAL: Check if previous word is --trust-mode FIRST
    if [[ "$prev" == "--trust-mode" ]]; then
      # Complete trust mode values
      local modes="paranoid normal yolo"
      COMPREPLY=($(compgen -W "$modes" -- "$cur"))
      return 0
    fi

    # Check if we're completing a flag
    if [[ "$cur" == -* ]]; then
      local flags="--verify-checksum -v --trust-mode"
      COMPREPLY=($(compgen -W "$flags" -- "$cur"))
      return 0
    fi

    # Otherwise, suggest repository names
    local suggestions=$(gitfetch complete clone-targets "$cur" 2>/dev/null)
    COMPREPLY=($(compgen -W "$suggestions" -- "$cur"))
    # Also allow filesystem completion as fallback
    if [ ${#COMPREPLY[@]} -eq 0 ]; then
      _filedir
    fi
    ;;
  search | -s)
    # For search, allow any input
    if [ "$cword" -eq 2 ]; then
      COMPREPLY=()
    fi
    ;;
  checksum)
    if [[ "$cur" == -* ]]; then
      local flags="--save -s"
      COMPREPLY=($(compgen -W "$flags" -- "$cur"))
    elif [ "$cword" -eq 2 ]; then
      # Suggest directories (repository paths)
      _filedir -d
    fi
    ;;
  verify)
    if [ "$cword" -eq 2 ]; then
      # Suggest directories (repository paths)
      _filedir -d
    fi
    ;;
  list | -l | easter-egg | -e | help)
    # These commands take no arguments
    COMPREPLY=()
    ;;
  completions)
    # Suggest shell types
    if [ "$cword" -eq 2 ]; then
      local shells="bash zsh fish powershell elvish"
      COMPREPLY=($(compgen -W "$shells" -- "$cur"))
    fi
    ;;
  *)
    COMPREPLY=()
    ;;
  esac

  return 0
}

# Register the completion function
complete -F _gitfetch_completions gitfetch
