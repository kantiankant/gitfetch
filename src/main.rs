use clap::{Parser, Subcommand, CommandFactory, ValueHint};
use clap_complete::Shell;

mod types;
mod config;
mod git;
mod checksum;
mod security;
mod commands;

use commands::*;

#[derive(Parser)]
#[command(name = "gitfetch")]
#[command(about = "A GitHub Package Manager from Hell", long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]

#[command(after_help = "TRUST MODES (for clone command):\n  \
    paranoid  - Maximum security: verify everything, prompt for all decisions, isolate network\n  \
    normal    - Default: balanced security with reasonable prompts and standard checks\n  \
    yolo      - Minimal security: trust the source, minimal prompts (use with caution)\n\n\
    Usage: gitfetch clone <repo> --trust-mode <mode>")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Clone a repository with ACTUAL security (hooks disabled, network isolated)
    #[command(short_flag = 'c', visible_alias = "clone")]
    Clone {
        /// Repository URL (e.g., https://github.com/user/repo)
        #[arg(value_hint = ValueHint::Url)]
        repo: String,
        /// Verify against known checksums (requires checksum registry)
        #[arg(long, short = 'v')]
        verify_checksum: bool,
        /// Trust mode: paranoid (max security), normal (default), yolo (minimal prompts)
        #[arg(long, default_value = "normal", value_parser = ["paranoid", "normal", "yolo"], 
              long_help = "Set the trust level for cloning operations.\n\n\
                          PARANOID: Maximum security. Verifies checksums, prompts for every decision,\n\
                          disables all git hooks, isolates network access. Use when cloning untrusted repos.\n\n\
                          NORMAL: Default mode. Balanced security with standard checks and reasonable prompts.\n\
                          Disables hooks and provides basic isolation.\n\n\
                          YOLO: Minimal security checks and prompts. Trusts the repository source.\n\
                          Only use with repositories you absolutely trust.\n\n\
                          Examples:\n  \
                          gitfetch clone https://github.com/user/repo --trust-mode paranoid\n  \
                          gitfetch clone https://github.com/user/repo --trust-mode yolo")]
        trust_mode: String,
    },
    /// List all the repos you've installed with this nonsense
    #[command(short_flag = 'l', visible_alias = "list")]
    List,
    /// Search for repositories by name
    #[command(short_flag = 's', visible_alias = "search")]
    Search {
        /// Repository name to search for
        query: String,
    },
    /// Print something utterly pointless
    #[command(short_flag = 'e', visible_alias = "easter-egg")]
    EasterEgg,
    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Calculate checksums for a cloned repository
    #[command(visible_alias = "checksum")]
    Checksum {
        /// Path to the repository
        path: String,
        /// Save checksum to registry
        #[arg(long, short = 's')]
        save: bool,
    },
    /// Verify repository integrity against saved checksums
    #[command(visible_alias = "verify")]
    Verify {
        /// Path to the repository
        path: String,
    },
    /// Internal command for completion suggestions
    #[command(hide = true)]
    Complete {
        completion_type: String,
        #[arg(default_value = "")]
        partial: String,
    },
}

fn display_banner() {
    println!(r#"
 ██████╗ ██╗████████╗███████╗███████╗████████╗ ██████╗██╗  ██╗
██╔════╝ ██║╚══██╔══╝██╔════╝██╔════╝╚══██╔══╝██╔════╝██║  ██║
██║  ███╗██║   ██║   █████╗  █████╗     ██║   ██║     ███████║
██║   ██║██║   ██║   ██╔══╝  ██╔══╝     ██║   ██║     ██╔══██║
╚██████╔╝██║   ██║   ██║     ███████╗   ██║   ╚██████╗██║  ██║
 ╚═════╝ ╚═╝   ╚═╝   ╚═╝     ╚══════╝   ╚═╝    ╚═════╝╚═╝  ╚═╝
                                                        v0.18.2
"#);
    let mut cmd = Cli::command();
    let _ = cmd.print_help();
    println!();
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => display_banner(),
        Some(Commands::Clone { repo, verify_checksum, trust_mode }) => {
            clone_repo(&repo, verify_checksum, &trust_mode)
        }
        Some(Commands::List) => list_repos(),
        Some(Commands::Search { query }) => search_repos(&query),
        Some(Commands::EasterEgg) => easter_egg(),
        Some(Commands::Completions { shell }) => generate_completions(shell),
        Some(Commands::Complete { completion_type, partial }) => {
            complete_suggestions(&completion_type, &partial)
        }
        Some(Commands::Checksum { path, save }) => checksum_command(&path, save),
        Some(Commands::Verify { path }) => verify_command(&path),
    }
}
