#!/usr/bin/env bash
# Bash completion for gitfetch
# This is the proper way to do completions, unlike those rc-injecting monstrosities

_gitfetch_completions() {
    local cur prev words cword
    _init_completion || return

    # The command being completed
    local cmd="${words[1]}"
    
    # Common options across all commands
    local common_opts="-h --help -V --version"
    
    # If we're still at the command level
    if [ "$cword" -eq 1 ]; then
        local commands="clone -c list -l search -s easter-egg -e completions --help -h --version -V"
        COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
        return 0
    fi

    # Handle completions based on the subcommand
    case "$cmd" in
        clone|-c)
            # For clone, suggest repos from history and allow any input
            if [ "$cword" -eq 2 ]; then
                # Get suggestions from gitfetch itself
                local suggestions=$(gitfetch complete clone-targets "$cur" 2>/dev/null)
                COMPREPLY=( $(compgen -W "$suggestions" -- "$cur") )
                # Also allow filesystem completion as fallback
                if [ ${#COMPREPLY[@]} -eq 0 ]; then
                    _filedir
                fi
            fi
            ;;
        search|-s)
            # For search, we can't really autocomplete but allow any input
            if [ "$cword" -eq 2 ]; then
                # Just return empty to allow free-form input
                COMPREPLY=()
            fi
            ;;
        list|-l|easter-egg|-e)
            # These commands take no arguments
            COMPREPLY=()
            ;;
        completions)
            # Suggest shell types
            if [ "$cword" -eq 2 ]; then
                local shells="bash zsh fish powershell elvish"
                COMPREPLY=( $(compgen -W "$shells" -- "$cur") )
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
