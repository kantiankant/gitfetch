use clap::{Parser, Subcommand, CommandFactory, ValueHint};
use clap_complete::{generate, Shell};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Parser)]
#[command(name = "gitfetch")]
#[command(about = "A GitHub Package Manager from Hell", long_about = None)]
#[command(version = "0.18")]
struct Cli {
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
        #[arg(long, default_value = "normal", value_parser = ["paranoid", "normal", "yolo"])]
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

#[derive(Serialize, Deserialize, Debug)]
struct GitFetchConfig {
    installed_repos: Vec<InstalledRepo>,
    checksum_registry: HashMap<String, RepoChecksum>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct InstalledRepo {
    name: String,
    url: String,
    path: String,
    commit_hash: Option<String>,
    verified: bool,
    workspace_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RepoChecksum {
    repo_url: String,
    commit_hash: String,
    file_checksums: HashMap<String, String>,
    total_hash: String,
    verified_at: String,
}

impl GitFetchConfig {
    fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)
                .expect("Can't read config file");
            serde_json::from_str(&contents).unwrap_or_else(|_| GitFetchConfig {
                installed_repos: vec![],
                checksum_registry: HashMap::new(),
            })
        } else {
            GitFetchConfig {
                installed_repos: vec![],
                checksum_registry: HashMap::new(),
            }
        }
    }

    fn save(&self) {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).expect("Can't create config directory");
        }
        let contents = serde_json::to_string_pretty(self)
            .expect("Failed to serialize config");
        fs::write(&config_path, contents).expect("Can't write config file");
    }

    fn config_path() -> PathBuf {
        let home = std::env::var("HOME").expect("No HOME directory?");
        PathBuf::from(home).join(".config").join("gitfetch").join("config.json")
    }

    fn add_repo(&mut self, name: String, url: String, path: String, commit_hash: Option<String>, verified: bool, workspace_path: Option<String>) {
        self.installed_repos.push(InstalledRepo { 
            name, url, path, commit_hash, verified, workspace_path,
        });
        self.save();
    }

    fn add_checksum(&mut self, repo_url: String, checksum: RepoChecksum) {
        self.checksum_registry.insert(repo_url, checksum);
        self.save();
    }

    fn get_checksum(&self, repo_url: &str) -> Option<&RepoChecksum> {
        self.checksum_registry.get(repo_url)
    }
}

#[derive(Deserialize, Debug)]
struct GitHubRepo {
    full_name: String,
    html_url: String,
    description: Option<String>,
    stargazers_count: u32,
}

#[derive(Deserialize, Debug)]
struct GitHubSearchResponse {
    items: Vec<GitHubRepo>,
}

fn display_banner() {
    println!(r#"
 ██████╗ ██╗████████╗███████╗███████╗████████╗ ██████╗██╗  ██╗
██╔════╝ ██║╚══██╔══╝██╔════╝██╔════╝╚══██╔══╝██╔════╝██║  ██║
██║  ███╗██║   ██║   █████╗  █████╗     ██║   ██║     ███████║
██║   ██║██║   ██║   ██╔══╝  ██╔══╝     ██║   ██║     ██╔══██║
╚██████╔╝██║   ██║   ██║     ███████╗   ██║   ╚██████╗██║  ██║
 ╚═════╝ ╚═╝   ╚═╝   ╚═╝     ╚══════╝   ╚═╝    ╚═════╝╚═╝  ╚═╝
                                                        v0.18
"#);
    let mut cmd = Cli::command();
    let _ = cmd.print_help();
    println!();
}

fn get_git_commit_hash(repo_path: &str) -> Option<String> {
    let output = Command::new("git")
        .args(&["-C", repo_path, "rev-parse", "HEAD"])
        .output()
        .ok()?;
    
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn calculate_file_checksum(file_path: &Path) -> io::Result<String> {
    let mut file = fs::File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}

fn calculate_repo_checksums(repo_path: &str) -> io::Result<RepoChecksum> {
    let path = Path::new(repo_path);
    let mut file_checksums = HashMap::new();
    
    let commit_hash = get_git_commit_hash(repo_path)
        .unwrap_or_else(|| "unknown".to_string());
    
    println!("Calculating checksums...");
    walk_and_checksum(path, path, &mut file_checksums)?;
    
    let mut sorted_files: Vec<_> = file_checksums.iter().collect();
    sorted_files.sort_by_key(|(path, _)| *path);
    
    let mut total_hasher = Sha256::new();
    for (file_path, checksum) in sorted_files {
        total_hasher.update(file_path.as_bytes());
        total_hasher.update(checksum.as_bytes());
    }
    let total_hash = format!("{:x}", total_hasher.finalize());
    
    let repo_url = get_git_remote_url(repo_path)
        .unwrap_or_else(|| "unknown".to_string());
    
    let verified_at = chrono::Utc::now().to_rfc3339();
    
    Ok(RepoChecksum {
        repo_url,
        commit_hash,
        file_checksums,
        total_hash,
        verified_at,
    })
}

fn walk_and_checksum(
    current_path: &Path,
    base_path: &Path,
    checksums: &mut HashMap<String, String>,
) -> io::Result<()> {
    if !current_path.exists() {
        return Ok(());
    }
    
    for entry in fs::read_dir(current_path)? {
        let entry = entry?;
        let path = entry.path();
        
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') || name_str == "node_modules" || name_str == "target" {
                continue;
            }
        }
        
        if path.is_file() {
            let relative_path = path.strip_prefix(base_path)
                .unwrap()
                .to_string_lossy()
                .to_string();
            
            if let Ok(checksum) = calculate_file_checksum(&path) {
                checksums.insert(relative_path, checksum);
            }
        } else if path.is_dir() {
            walk_and_checksum(&path, base_path, checksums)?;
        }
    }
    
    Ok(())
}

fn get_git_remote_url(repo_path: &str) -> Option<String> {
    let output = Command::new("git")
        .args(&["-C", repo_path, "config", "--get", "remote.origin.url"])
        .output()
        .ok()?;
    
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn verify_repo_checksums(repo_path: &str, expected: &RepoChecksum) -> Result<bool, String> {
    println!("Verifying repository integrity...");
    
    if let Some(current_commit) = get_git_commit_hash(repo_path) {
        if current_commit != expected.commit_hash {
            println!("⚠️  Commit hash mismatch!");
        }
    }
    
    let path = Path::new(repo_path);
    let mut all_match = true;
    let mut verified = 0;
    let mut issues = 0;
    
    let mut current_files = HashMap::new();
    walk_and_checksum(path, path, &mut current_files)
        .map_err(|e| format!("Failed to walk directory: {}", e))?;
    
    for (file_path, expected_checksum) in &expected.file_checksums {
        if let Some(current_checksum) = current_files.get(file_path) {
            if current_checksum == expected_checksum {
                verified += 1;
            } else {
                issues += 1;
                all_match = false;
            }
        } else {
            issues += 1;
            all_match = false;
        }
    }
    
    for file_path in current_files.keys() {
        if !expected.file_checksums.contains_key(file_path) {
            issues += 1;
            all_match = false;
        }
    }
    
    println!("✓ Verified: {} | ✗ Issues: {}", verified, issues);
    
    if all_match {
        println!("✓ Repository integrity verified!");
    } else {
        println!("✗ Verification FAILED!");
    }
    
    Ok(all_match)
}

fn check_bubblewrap() -> Result<(), String> {
    let bwrap_check = Command::new("which")
        .arg("bwrap")
        .output();
    
    match bwrap_check {
        Ok(output) if output.status.success() => Ok(()),
        _ => {
            eprintln!("\n{}", "=".repeat(60));
            eprintln!("BUBBLEWRAP NOT FOUND");
            eprintln!("{}", "=".repeat(60));
            eprintln!("Install bubblewrap for actual sandboxing:");
            eprintln!("  Ubuntu/Debian: sudo apt install bubblewrap");
            eprintln!("  Arch:          sudo pacman -S bubblewrap");
            eprintln!("  Fedora/RHEL:   sudo dnf install bubblewrap");
            eprintln!("{}", "=".repeat(60));
            Err("Bubblewrap not installed".to_string())
        }
    }
}

fn run_sandboxed_git(workspace: &Path, args: &[&str], with_network: bool) -> Result<(), String> {
    let mut cmd = Command::new("timeout");
    cmd.arg("300") // 5 minute timeout
        .arg("bwrap")
        // Read-only system mounts
        .args(&["--ro-bind", "/usr", "/usr"])
        .args(&["--ro-bind", "/lib", "/lib"])
        .args(&["--ro-bind", "/lib64", "/lib64"])
        .args(&["--ro-bind", "/bin", "/bin"])
        .args(&["--ro-bind", "/sbin", "/sbin"])
        .args(&["--ro-bind", "/etc", "/etc"])
        .args(&["--proc", "/proc"])
        // Minimal /dev (only what git needs)
        .args(&["--dev-bind", "/dev/null", "/dev/null"])
        .args(&["--dev-bind", "/dev/zero", "/dev/zero"])
        .args(&["--dev-bind", "/dev/urandom", "/dev/urandom"])
        .args(&["--tmpfs", "/tmp"])
        // Workspace
        .args(&["--bind", workspace.to_str().unwrap(), "/workspace"])
        .args(&["--chdir", "/workspace"])
        // Isolation
        .args(&["--unshare-pid"])
        .args(&["--unshare-uts"])
        .args(&["--unshare-cgroup"])
        .args(&["--die-with-parent"])
        .args(&["--cap-drop", "ALL"]);
    
    // Network isolation for checkout phase
    if !with_network {
        cmd.args(&["--unshare-net"]);
    }
    
    // Restricted environment - DISABLE GIT HOOKS
    cmd.args(&["--clearenv"])
        .args(&["--setenv", "PATH", "/usr/bin:/bin"])
        .args(&["--setenv", "HOME", "/workspace"])
        .args(&["--setenv", "GIT_CONFIG_COUNT", "1"])
        .args(&["--setenv", "GIT_CONFIG_KEY_0", "core.hooksPath"])
        .args(&["--setenv", "GIT_CONFIG_VALUE_0", "/dev/null"]);
    
    // Git command
    cmd.arg("git").args(args);
    
    let status = cmd.status()
        .map_err(|e| format!("Failed to execute bubblewrap: {}", e))?;
    
    if status.success() {
        Ok(())
    } else {
        Err("Git command failed in sandbox".to_string())
    }
}

fn scan_for_suspicious_patterns(repo_path: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let path = std::path::Path::new(repo_path);
    
    if !path.exists() {
        return warnings;
    }
    
    let suspicious_patterns = vec![
        ("eval(", "eval() usage"),
        ("exec(", "exec() usage"),
        ("subprocess", "subprocess usage"),
        ("os.system", "os.system usage"),
        ("shell=True", "shell=True"),
        ("/etc/passwd", "password file access"),
        ("rm -rf", "recursive deletion"),
        ("curl", "network request"),
        ("base64.b64decode", "base64 decode"),
        ("authorized_keys", "SSH keys access"),
        ("bitcoin", "crypto-related"),
    ];
    
    let extensions = vec!["py", "js", "sh", "bash", "rb", "pl", "php", "rs"];
    
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            
            if let Some(name) = entry_path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.starts_with('.') || name_str == "node_modules" {
                    continue;
                }
            }
            
            if entry_path.is_file() {
                if let Some(ext) = entry_path.extension() {
                    if extensions.contains(&ext.to_string_lossy().as_ref()) {
                        if let Ok(content) = std::fs::read_to_string(&entry_path) {
                            let content_lower = content.to_lowercase();
                            for (pattern, desc) in &suspicious_patterns {
                                if content_lower.contains(&pattern.to_lowercase()) {
                                    let filename = entry_path.file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy();
                                    warnings.push(format!("{}: {}", filename, desc));
                                    break; // One warning per file
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    warnings.sort();
    warnings.dedup();
    warnings
}

fn prompt_user(message: &str) -> bool {
    println!("{}", message);
    print!("> ");
    io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");
    
    let mut response = String::new();
    io::stdin().read_line(&mut response).expect("Failed to read input");
    let response = response.trim().to_lowercase();
    
    response == "yes" || response == "y"
}

fn clone_repo(repo: &str, verify_checksum: bool, trust_mode: &str) {
    check_bubblewrap().unwrap_or_else(|_| std::process::exit(1));
    
    let repo_url = if repo.starts_with("http://") || repo.starts_with("https://") {
        repo.to_string()
    } else if repo.contains('/') {
        format!("https://github.com/{}", repo)
    } else {
        eprintln!("Invalid repository format");
        std::process::exit(1);
    };

    let repo_name = repo_url
        .trim_end_matches(".git")
        .split('/')
        .last()
        .expect("Can't parse repo name")
        .to_string();

    let home = std::env::var("HOME").expect("No HOME?");
    let workspace_base = PathBuf::from(home).join(".gitfetch").join("workspace");
    fs::create_dir_all(&workspace_base).expect("Can't create workspace");
    
    let workspace = workspace_base.join(&repo_name);
    
    if workspace.exists() {
        fs::remove_dir_all(&workspace).expect("Can't remove existing workspace");
    }
    
    fs::create_dir_all(&workspace).expect("Can't create workspace");

    println!("\n{}", "=".repeat(60));
    println!("CLONING: {}", repo_url);
    println!("{}", "=".repeat(60));
    println!("Trust mode: {}", trust_mode);
    
    let config = GitFetchConfig::load();
    let has_checksum = config.get_checksum(&repo_url).is_some();
    
    if verify_checksum && !has_checksum {
        eprintln!("No checksum registry found (--verify-checksum specified)");
        std::process::exit(1);
    }
    
    // Paranoid mode: always prompt
    // Normal mode: prompt unless checksum verified
    // YOLO mode: never prompt
    let should_prompt = match trust_mode {
        "paranoid" => true,
        "yolo" => false,
        _ => !has_checksum, // normal
    };
    
    if should_prompt {
        if !prompt_user("WARNING: Clone from untrusted source?\nProceed? (yes/no)") {
            println!("Clone cancelled.");
            std::process::exit(0);
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("STAGE 1: FETCH (with network)");
    println!("{}", "=".repeat(60));
    
    // Stage 1: Clone with --no-checkout (network enabled, no hooks run)
    if let Err(e) = run_sandboxed_git(&workspace, &["clone", "--no-checkout", &repo_url], true) {
        eprintln!("Clone failed: {}", e);
        let _ = fs::remove_dir_all(&workspace);
        std::process::exit(1);
    }

    println!("\n{}", "=".repeat(60));
    println!("STAGE 2: CHECKOUT (network isolated, hooks disabled)");
    println!("{}", "=".repeat(60));
    
    let repo_in_workspace = workspace.join(&repo_name);
    
    // Stage 2: Checkout without network (hooks are disabled via env)
    if let Err(e) = run_sandboxed_git(
        &repo_in_workspace,
        &["checkout", "--force", "HEAD"],
        false // Network disabled
    ) {
        eprintln!("Checkout failed: {}", e);
        let _ = fs::remove_dir_all(&workspace);
        std::process::exit(1);
    }

    let mut config = GitFetchConfig::load();
    let commit_hash = get_git_commit_hash(repo_in_workspace.to_str().unwrap());
    
    // Verify checksums if available
    let mut verified = false;
    if let Some(expected_checksum) = config.get_checksum(&repo_url) {
        println!("\n{}", "=".repeat(60));
        println!("VERIFYING CHECKSUMS");
        println!("{}", "=".repeat(60));
        
        match verify_repo_checksums(repo_in_workspace.to_str().unwrap(), expected_checksum) {
            Ok(true) => {
                verified = true;
                println!("✓ Integrity verified!");
            }
            Ok(false) => {
                if trust_mode != "yolo" && !prompt_user("\nVerification failed. Proceed? (yes/no)") {
                    let _ = fs::remove_dir_all(&workspace);
                    std::process::exit(0);
                }
            }
            Err(e) => eprintln!("Verification error: {}", e),
        }
    }
    
    // Security scan
    println!("\n{}", "=".repeat(60));
    println!("SECURITY SCAN");
    println!("{}", "=".repeat(60));
    
    let warnings = scan_for_suspicious_patterns(repo_in_workspace.to_str().unwrap());
    
    if !warnings.is_empty() {
        println!("⚠️  {} suspicious patterns detected:", warnings.len());
        for (i, warning) in warnings.iter().take(5).enumerate() {
            println!("  {}. {}", i + 1, warning);
        }
        if warnings.len() > 5 {
            println!("  ... and {} more", warnings.len() - 5);
        }
        
        if trust_mode == "paranoid" && !prompt_user("\nSuspicious code detected. Proceed? (yes/no)") {
            let _ = fs::remove_dir_all(&workspace);
            std::process::exit(0);
        }
    } else {
        println!("No obvious threats detected.");
    }
    
    // Copy to current directory
    let current_dir = std::env::current_dir()
        .expect("Can't get current directory")
        .join(&repo_name);
    
    let final_path = if trust_mode != "paranoid" || prompt_user("\nCopy to current directory? (yes/no)") {
        let copy_status = Command::new("cp")
            .args(&["-r", repo_in_workspace.to_str().unwrap(), current_dir.to_str().unwrap()])
            .status()
            .expect("Failed to copy");
        
        if copy_status.success() {
            current_dir.to_string_lossy().to_string()
        } else {
            repo_in_workspace.to_string_lossy().to_string()
        }
    } else {
        repo_in_workspace.to_string_lossy().to_string()
    };
    
    config.add_repo(
        repo_name.clone(),
        repo_url.clone(),
        final_path.clone(),
        commit_hash,
        verified,
        Some(workspace.to_string_lossy().to_string())
    );

    println!("\n{}", "=".repeat(60));
    println!("✓ CLONE COMPLETE");
    println!("{}", "=".repeat(60));
    println!("Location: {}", final_path);
    
    if !verified && !has_checksum {
        println!("\n💡 Create checksum: gitfetch checksum {} --save", final_path);
    }
}

fn list_repos() {
    let config = GitFetchConfig::load();
    
    if config.installed_repos.is_empty() {
        println!("No repositories installed yet.");
        return;
    }

    println!("Installed repositories:\n");
    for repo in &config.installed_repos {
        let marker = if repo.verified { "✓" } else { "?" };
        println!("  {} {}", marker, repo.name);
        println!("    {}", repo.url);
        println!("    {}", repo.path);
        if let Some(commit) = &repo.commit_hash {
            println!("    Commit: {:.8}", commit);
        }
        println!();
    }
}

fn search_repos(query: &str) {
    let search_query = if query.contains('/') {
        format!("repo:{}", query)
    } else {
        format!("{} in:name", query)
    };
    
    let url = format!(
        "https://api.github.com/search/repositories?q={}&sort=stars&order=desc",
        urlencoding::encode(&search_query)
    );

    let client = reqwest::blocking::Client::builder()
        .user_agent("gitfetch/0.18")
        .build()
        .expect("Can't create HTTP client");

    match client.get(&url).send() {
        Ok(resp) => {
            if resp.status().is_success() {
                if let Ok(result) = resp.json::<GitHubSearchResponse>() {
                    if result.items.is_empty() {
                        println!("No repositories found.");
                    } else {
                        println!("\nFound {} repositories:\n", result.items.len());
                        for repo in result.items.iter().take(10) {
                            println!("  {}", repo.full_name);
                            println!("    ⭐ {}", repo.stargazers_count);
                            if let Some(desc) = &repo.description {
                                println!("    {}", desc);
                            }
                            println!("    {}\n", repo.html_url);
                        }
                    }
                }
            } else {
                eprintln!("GitHub API error: {}", resp.status());
            }
        }
        Err(e) => eprintln!("Network error: {}", e),
    }
}

fn easter_egg() {
    let output = Command::new("whoami")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to run whoami");

    let username = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!("{} is properly paranoid for using gitfetch v0.18", username);
}

fn generate_completions(shell: Shell) {
    let completion_script = match shell {
        Shell::Bash => include_str!("../gitfetch.bash"),
        Shell::Zsh => include_str!("../gitfetch.zsh"),
        Shell::Fish => include_str!("../gitfetch.fish"),
        _ => {
            eprintln!("Shell {} not supported with custom completions.", shell);
            eprintln!("Falling back to basic completions...");
            let mut cmd = Cli::command();
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

fn complete_suggestions(completion_type: &str, partial: &str) {
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

fn checksum_command(path: &str, save: bool) {
    let abs_path = std::path::Path::new(path)
        .canonicalize()
        .expect("Can't resolve path")
        .to_string_lossy()
        .to_string();
    
    match calculate_repo_checksums(&abs_path) {
        Ok(checksum) => {
            println!("\n{}", "=".repeat(60));
            println!("Repository: {}", checksum.repo_url);
            println!("Commit:     {}", checksum.commit_hash);
            println!("Files:      {}", checksum.file_checksums.len());
            println!("Total Hash: {}", checksum.total_hash);
            
            if save {
                let mut config = GitFetchConfig::load();
                config.add_checksum(checksum.repo_url.clone(), checksum);
                println!("✓ Checksum saved");
            }
        }
        Err(e) => {
            eprintln!("Failed: {}", e);
            std::process::exit(1);
        }
    }
}

fn verify_command(path: &str) {
    let abs_path = std::path::Path::new(path)
        .canonicalize()
        .expect("Can't resolve path")
        .to_string_lossy()
        .to_string();
    
    let repo_url = get_git_remote_url(&abs_path)
        .expect("Can't get repository URL");
    
    let config = GitFetchConfig::load();
    
    match config.get_checksum(&repo_url) {
        Some(expected) => {
            match verify_repo_checksums(&abs_path, expected) {
                Ok(true) => std::process::exit(0),
                Ok(false) => std::process::exit(1),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            eprintln!("No checksum found for: {}", repo_url);
            std::process::exit(1);
        }
    }
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
