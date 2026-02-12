use crate::config::GitFetchConfig;
use clap::CommandFactory;

use clap_complete::{generate, Shell};
use std::io;
use std::process::{Command, Stdio};

pub fn easter_egg() {
    let output = Command::new("whoami")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to run whoami");

    let username = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!("{} is properly paranoid for using gitfetch v0.18", username);
}

pub fn generate_completions(shell: Shell) {
    let completion_script = match shell {
        Shell::Bash => include_str!("../../gitfetch.bash"),
        Shell::Zsh => include_str!("../../gitfetch.zsh"),
        Shell::Fish => include_str!("../../gitfetch.fish"),
        _ => {
            eprintln!("Shell {} not supported with custom completions.", shell);
            eprintln!("Falling back to basic completions...");
            let mut cmd = crate::Cli::command();
            generate(shell, &mut cmd, "gitfetch", &mut io::stdout());
            eprintln!("\nNote: Basic completions without intelligent suggestions.");
            return;
        }
    };
    
    print!("{}", completion_script);
    
    eprintln!();
    eprintln!("Save the output to the appropriate location for your shell:");
    match shell {
        Shell::Bash => {
            eprintln!("  gitfetch completions bash | sudo tee /etc/bash_completion.d/gitfetch");
            eprintln!("  # Or for user-only:");
            eprintln!("  gitfetch completions bash > ~/.local/share/bash-completion/completions/gitfetch");
        },
        Shell::Zsh => {
            eprintln!("  gitfetch completions zsh | sudo tee /usr/local/share/zsh/site-functions/_gitfetch");
            eprintln!("  # Or for user-only:");
            eprintln!("  gitfetch completions zsh > ~/.zsh/completions/_gitfetch");
            eprintln!("  # (Add 'fpath=(~/.zsh/completions $fpath)' before 'compinit' in .zshrc)");
        },
        Shell::Fish => {
            eprintln!("  gitfetch completions fish > ~/.config/fish/completions/gitfetch.fish");
        },
        _ => {}
    }
}

pub fn complete_suggestions(completion_type: &str, partial: &str) {
    match completion_type {
        "repos" => {
            let config = GitFetchConfig::load();
            for repo in &config.installed_repos {
                if repo.name.starts_with(partial) || partial.is_empty() {
                    println!("{}", repo.name);
                }
            }
        }
        "clone-targets" => {
            let config = GitFetchConfig::load();
            for repo in &config.installed_repos {
                if repo.url.contains(partial) || repo.name.contains(partial) || partial.is_empty() {
                    if repo.url.contains("github.com") {
                        if let Some(path) = repo.url.strip_prefix("https://github.com/") {
                            let short = path.trim_end_matches(".git");
                            if short.contains(partial) || partial.is_empty() {
                                println!("{}", short);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}
