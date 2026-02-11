#compdef gitfetch
# Zsh completion for gitfetch v0.18
# Because zsh users are special and deserve special treatment

_gitfetch() {
    local line state

    _arguments -C \
        '1: :->command' \
        '*::arg:->args'

    case $state in
        command)
            _values 'gitfetch command' \
                'clone[Clone a repository with ACTUAL security]' \
                '-c[Clone a repository (short form)]' \
                'list[List installed repos]' \
                '-l[List installed repos (short form)]' \
                'search[Search for repositories by name]' \
                '-s[Search for repositories (short form)]' \
                'easter-egg[Print something utterly pointless]' \
                '-e[Easter egg (short form)]' \
                'completions[Generate shell completion scripts]' \
                'checksum[Calculate checksums for a cloned repository]' \
                'verify[Verify repository integrity against saved checksums]' \
                'help[Print help message]' \
                '-h[Show help]' \
                '--help[Show help]' \
                '-V[Show version]' \
                '--version[Show version]'
            ;;
        args)
            case $line[1] in
                clone|-c)
                    _arguments \
                        '1:repository:->repos' \
                        '(--verify-checksum -v)'{--verify-checksum,-v}'[Verify against known checksums]' \
                        '--trust-mode=[Trust mode]:mode:(paranoid normal yolo)'
                    
                    case $state in
                        repos)
                            # Dynamic completion for clone - suggest from history
                            local suggestions
                            suggestions=(${(f)"$(gitfetch complete clone-targets ${words[-1]} 2>/dev/null)"})
                            if [ ${#suggestions[@]} -gt 0 ]; then
                                _describe 'repository' suggestions
                            else
                                _message 'repository URL or user/repo'
                            fi
                            ;;
                    esac
                    ;;
                search|-s)
                    _message 'repository name to search'
                    ;;
                checksum)
                    _arguments \
                        '1:repository path:_files -/' \
                        '--save[Save checksum to registry]' \
                        '-s[Save checksum to registry]'
                    ;;
                verify)
                    _arguments \
                        '1:repository path:_files -/'
                    ;;
                completions)
                    _values 'shell' \
                        'bash[Bash completion]' \
                        'zsh[Zsh completion]' \
                        'fish[Fish completion]' \
                        'powershell[PowerShell completion]' \
                        'elvish[Elvish completion]'
                    ;;
                list|-l|easter-egg|-e|help)
                    # These take no arguments
                    ;;
            esac
            ;;
    esac
}

_gitfetch "$@"
