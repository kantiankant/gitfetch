# Fish completion for gitfetch
# Fish just HAD to have its own special syntax, didn't it?

# Helper function to check if we're at a specific position
function __gitfetch_using_command
    set -l cmd (commandline -opc)
    test (count $cmd) -gt 1 -a "$cmd[2]" = "$argv[1]"
end

# Don't suggest files by default
complete -c gitfetch -f

# Main commands
complete -c gitfetch -n "not __fish_seen_subcommand_from clone -c list -l search -s easter-egg -e completions" -a "clone" -d "Clone a repository"
complete -c gitfetch -n "not __fish_seen_subcommand_from clone -c list -l search -s easter-egg -e completions" -a "-c" -d "Clone a repository (short)"
complete -c gitfetch -n "not __fish_seen_subcommand_from clone -c list -l search -s easter-egg -e completions" -a "list" -d "List installed repos"
complete -c gitfetch -n "not __fish_seen_subcommand_from clone -c list -l search -s easter-egg -e completions" -a "-l" -d "List installed repos (short)"
complete -c gitfetch -n "not __fish_seen_subcommand_from clone -c list -l search -s easter-egg -e completions" -a "search" -d "Search for repositories"
complete -c gitfetch -n "not __fish_seen_subcommand_from clone -c list -l search -s easter-egg -e completions" -a "-s" -d "Search for repositories (short)"
complete -c gitfetch -n "not __fish_seen_subcommand_from clone -c list -l search -s easter-egg -e completions" -a "easter-egg" -d "Something pointless"
complete -c gitfetch -n "not __fish_seen_subcommand_from clone -c list -l search -s easter-egg -e completions" -a "-e" -d "Easter egg (short)"
complete -c gitfetch -n "not __fish_seen_subcommand_from clone -c list -l search -s easter-egg -e completions" -a "completions" -d "Generate shell completions"

# Help and version (available always)
complete -c gitfetch -s h -l help -d "Show help"
complete -c gitfetch -s V -l version -d "Show version"

# Clone command - suggest from history
complete -c gitfetch -n "__fish_seen_subcommand_from clone -c" -a "(gitfetch complete clone-targets (commandline -ct) 2>/dev/null)" -d "Repository"

# Search command - free form input
complete -c gitfetch -n "__fish_seen_subcommand_from search -s" -d "Repository name"

# Completions command - suggest shells
complete -c gitfetch -n "__fish_seen_subcommand_from completions" -a "bash" -d "Bash shell"
complete -c gitfetch -n "__fish_seen_subcommand_from completions" -a "zsh" -d "Zsh shell"
complete -c gitfetch -n "__fish_seen_subcommand_from completions" -a "fish" -d "Fish shell"
complete -c gitfetch -n "__fish_seen_subcommand_from completions" -a "powershell" -d "PowerShell"
complete -c gitfetch -n "__fish_seen_subcommand_from completions" -a "elvish" -d "Elvish shell"
