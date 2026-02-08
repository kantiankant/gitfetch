#compdef gitfetch
# Zsh completion for gitfetch
# Because zsh users are special and need their own custom script

_gitfetch() {
    local line state

    _arguments -C \
        '1: :->command' \
        '*::arg:->args'

    case $state in
        command)
            _values 'gitfetch command' \
                'clone[Clone a repository (glorified git clone)]' \
                '-c[Clone a repository (short form)]' \
                'list[List installed repos]' \
                '-l[List installed repos (short form)]' \
                'search[Search for repositories by name]' \
                '-s[Search for repositories (short form)]' \
                'easter-egg[Print something utterly pointless]' \
                '-e[Easter egg (short form)]' \
                'completions[Generate shell completion scripts]' \
                '-h[Show help]' \
                '--help[Show help]' \
                '-V[Show version]' \
                '--version[Show version]'
            ;;
        args)
            case $line[1] in
                clone|-c)
                    # Dynamic completion for clone - suggest from history
                    local suggestions
                    suggestions=(${(f)"$(gitfetch complete clone-targets ${words[-1]} 2>/dev/null)"})
                    if [ ${#suggestions[@]} -gt 0 ]; then
                        _describe 'repository' suggestions
                    else
                        # Fallback to allowing any input
                        _message 'repository URL or user/repo'
                    fi
                    ;;
                search|-s)
                    _message 'repository name to search'
                    ;;
                completions)
                    _values 'shell' \
                        'bash[Bash completion]' \
                        'zsh[Zsh completion]' \
                        'fish[Fish completion]' \
                        'powershell[PowerShell completion]' \
                        'elvish[Elvish completion]'
                    ;;
                list|-l|easter-egg|-e)
                    # These take no arguments
                    ;;
            esac
            ;;
    esac
}

_gitfetch "$@"
