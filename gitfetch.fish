# Fish completion for gitfetch v0.18
# For those who prefer their shells friendly

# Disable file completion by default
complete -c gitfetch -f

# Main commands
complete -c gitfetch -n __fish_use_subcommand -s c -l clone -d "Clone a repository with ACTUAL security"
complete -c gitfetch -n __fish_use_subcommand -a clone -d "Clone a repository with ACTUAL security"
complete -c gitfetch -n __fish_use_subcommand -s l -l list -d "List installed repos"
complete -c gitfetch -n __fish_use_subcommand -a list -d "List installed repos"
complete -c gitfetch -n __fish_use_subcommand -s s -l search -d "Search for repositories"
complete -c gitfetch -n __fish_use_subcommand -a search -d "Search for repositories"
complete -c gitfetch -n __fish_use_subcommand -s e -l easter-egg -d "Print something utterly pointless"
complete -c gitfetch -n __fish_use_subcommand -a easter-egg -d "Print something utterly pointless"
complete -c gitfetch -n __fish_use_subcommand -a completions -d "Generate shell completion scripts"
complete -c gitfetch -n __fish_use_subcommand -a checksum -d "Calculate checksums for a repository"
complete -c gitfetch -n __fish_use_subcommand -a verify -d "Verify repository integrity"
complete -c gitfetch -n __fish_use_subcommand -a help -d "Print help message"
complete -c gitfetch -n __fish_use_subcommand -s h -l help -d "Show help"
complete -c gitfetch -n __fish_use_subcommand -s V -l version -d "Show version"

# Clone command options
complete -c gitfetch -n "__fish_seen_subcommand_from clone -c" -s v -l verify-checksum -d "Verify against known checksums"
complete -c gitfetch -n "__fish_seen_subcommand_from clone -c" -l trust-mode -d "Trust mode" -x -a "paranoid normal yolo"

# Clone repository suggestions (dynamic)
complete -c gitfetch -n "__fish_seen_subcommand_from clone -c; and not __fish_seen_subcommand_from --verify-checksum -v --trust-mode" -a "(gitfetch complete clone-targets (commandline -ct) 2>/dev/null)"

# Checksum command options
complete -c gitfetch -n "__fish_seen_subcommand_from checksum" -s s -l save -d "Save checksum to registry"
complete -c gitfetch -n "__fish_seen_subcommand_from checksum" -F -d "Repository path"

# Verify command
complete -c gitfetch -n "__fish_seen_subcommand_from verify" -F -d "Repository path"

# Completions command - shell types
complete -c gitfetch -n "__fish_seen_subcommand_from completions" -a "bash zsh fish powershell elvish" -d "Shell type"

# Search command - no specific completions, allow any input
