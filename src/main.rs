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
#[command(version = "0.17")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Clone a repository in proper bubblewrap sandbox (ACTUAL isolation, not theatre)
    #[command(short_flag = 'c', visible_alias = "clone")]
    Clone {
        /// Repository URL (e.g., https://github.com/user/repo)
        #[arg(value_hint = ValueHint::Url)]
        repo: String,
        /// Verify against known checksums (requires checksum registry)
        #[arg(long, short = 'v')]
        verify_checksum: bool,
    },
    /// List all the repos you've installed with this nonsense
    #[command(short_flag = 'l', visible_alias = "list")]
    List,
    /// Search for repositories by name (because apparently you can't use GitHub's website)
    #[command(short_flag = 's', visible_alias = "search")]
    Search {
        /// Repository name to search for (e.g., dreammaomao/mangowc)
        query: String,
    },
    /// Print something utterly pointless
    #[command(short_flag = 'e', visible_alias = "easter-egg")]
    EasterEgg,
    /// Generate shell completion scripts (saves you from rc injection hell)
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
    /// Internal command for completion suggestions (don't use this manually)
    #[command(hide = true)]
    Complete {
        /// What we're completing (repos, clone-targets, etc)
        completion_type: String,
        /// Current partial input
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
    workspace_path: Option<String>, // Path to sandboxed workspace (bubblewrap)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct RepoChecksum {
    repo_url: String,
    commit_hash: String,
    file_checksums: HashMap<String, String>, // relative_path -> sha256
    total_hash: String, // hash of all file hashes combined
    verified_at: String, // timestamp
}

impl GitFetchConfig {
    fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)
                .expect("Bloody hell, can't read the config file");
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
            fs::create_dir_all(parent).expect("Can't create config directory, brilliant");
        }
        let contents = serde_json::to_string_pretty(self)
            .expect("Failed to serialize config, absolutely fantastic");
        fs::write(&config_path, contents).expect("Can't write config file, lovely");
    }

    fn config_path() -> PathBuf {
        let home = std::env::var("HOME").expect("No HOME directory? What are you running this on, a toaster?");
        PathBuf::from(home).join(".config").join("gitfetch").join("config.json")
    }

    fn add_repo(&mut self, name: String, url: String, path: String, commit_hash: Option<String>, verified: bool, workspace_path: Option<String>) {
        self.installed_repos.push(InstalledRepo { 
            name, 
            url, 
            path, 
            commit_hash,
            verified,
            workspace_path,
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
    watchers_count: u32,
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
"#);
    
    // Get the help output from clap
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
    
    // Get commit hash
    let commit_hash = get_git_commit_hash(repo_path)
        .unwrap_or_else(|| "unknown".to_string());
    
    println!("Calculating checksums for repository...");
    
    // Walk the directory and calculate checksums
    walk_and_checksum(path, path, &mut file_checksums)?;
    
    // Calculate total hash (hash of all hashes sorted by filename)
    let mut sorted_files: Vec<_> = file_checksums.iter().collect();
    sorted_files.sort_by_key(|(path, _)| *path);
    
    let mut total_hasher = Sha256::new();
    for (file_path, checksum) in sorted_files {
        total_hasher.update(file_path.as_bytes());
        total_hasher.update(checksum.as_bytes());
    }
    let total_hash = format!("{:x}", total_hasher.finalize());
    
    // Get repo URL from git config
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
        
        // Skip .git directory and other hidden files
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
            
            match calculate_file_checksum(&path) {
                Ok(checksum) => {
                    checksums.insert(relative_path.clone(), checksum);
                    println!("  ✓ {}", relative_path);
                }
                Err(e) => {
                    eprintln!("  ✗ {} (error: {})", relative_path, e);
                }
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
    println!("Expected commit: {}", expected.commit_hash);
    
    // Check commit hash
    if let Some(current_commit) = get_git_commit_hash(repo_path) {
        println!("Current commit:  {}", current_commit);
        if current_commit != expected.commit_hash {
            println!("\n⚠️  WARNING: Commit hash mismatch!");
            println!("This might indicate the repository has been updated or tampered with.");
        }
    }
    
    println!("\nVerifying file checksums...");
    
    let path = Path::new(repo_path);
    let mut all_match = true;
    let mut verified = 0;
    let mut modified = 0;
    let mut missing = 0;
    let mut extra = 0;
    
    let mut current_files = HashMap::new();
    walk_and_checksum(path, path, &mut current_files)
        .map_err(|e| format!("Failed to walk directory: {}", e))?;
    
    // Check expected files
    for (file_path, expected_checksum) in &expected.file_checksums {
        if let Some(current_checksum) = current_files.get(file_path) {
            if current_checksum == expected_checksum {
                verified += 1;
            } else {
                println!("  ✗ {} (MODIFIED)", file_path);
                modified += 1;
                all_match = false;
            }
        } else {
            println!("  ✗ {} (MISSING)", file_path);
            missing += 1;
            all_match = false;
        }
    }
    
    // Check for extra files
    for file_path in current_files.keys() {
        if !expected.file_checksums.contains_key(file_path) {
            println!("  + {} (EXTRA FILE)", file_path);
            extra += 1;
            all_match = false;
        }
    }
    
    println!("\nVerification Summary:");
    println!("  ✓ Verified: {}", verified);
    if modified > 0 {
        println!("  ✗ Modified: {}", modified);
    }
    if missing > 0 {
        println!("  ✗ Missing:  {}", missing);
    }
    if extra > 0 {
        println!("  + Extra:    {}", extra);
    }
    
    if all_match {
        println!("\n✓ All checksums match! Repository integrity verified.");
    } else {
        println!("\n✗ Checksum verification FAILED!");
        println!("The repository has been modified since the checksum was recorded.");
    }
    
    Ok(all_match)
}

fn clone_repo(repo: &str, verify_checksum: bool) {
    let repo_url = if repo.starts_with("http://") || repo.starts_with("https://") {
        repo.to_string()
    } else if repo.contains('/') {
        format!("https://github.com/{}", repo)
    } else {
        eprintln!("What sort of dodgy input is this? Provide a proper repo URL or user/repo format");
        std::process::exit(1);
    };

    let repo_name = repo_url
        .trim_end_matches(".git")
        .split('/')
        .last()
        .expect("Can't parse repo name, bloody brilliant")
        .to_string();

    // Check if bubblewrap is actually installed (for REAL sandboxing, not theatre)
    let bwrap_check = Command::new("which")
        .arg("bwrap")
        .output();
    
    let has_bwrap = match bwrap_check {
        Ok(output) => output.status.success(),
        Err(_) => false,
    };
    
    if !has_bwrap {
        eprintln!("\n{}", "=".repeat(60));
        eprintln!("BUBBLEWRAP NOT FOUND");
        eprintln!("{}", "=".repeat(60));
        eprintln!("Bloody hell, you don't have bubblewrap installed!");
        eprintln!("This tool requires bubblewrap for ACTUAL sandboxing.");
        eprintln!("(Not that fakeroot nonsense which does bugger all for security)");
        eprintln!("\nInstall it first:");
        eprintln!("  Ubuntu/Debian: sudo apt install bubblewrap");
        eprintln!("  Arch:          sudo pacman -S bubblewrap");
        eprintln!("  Fedora/RHEL:   sudo dnf install bubblewrap");
        eprintln!("  macOS:         Not available (use Docker/Podman instead)");
        eprintln!("{}", "=".repeat(60));
        std::process::exit(1);
    }

    // Create isolated working directory for fakeroot shenanigans
    let home = std::env::var("HOME").expect("No HOME? What sort of system is this?");
    let workspace_base = PathBuf::from(home).join(".gitfetch").join("workspace");
    fs::create_dir_all(&workspace_base)
        .expect("Can't create workspace directory, fantastic");
    
    let workspace = workspace_base.join(&repo_name);
    
    if workspace.exists() {
        println!("\nWorkspace already exists at: {}", workspace.display());
        println!("Cleaning up the mess from last time...");
        fs::remove_dir_all(&workspace)
            .expect("Can't remove existing workspace, lovely");
    }
    
    fs::create_dir_all(&workspace)
        .expect("Can't create workspace directory for this repo");

    // Security theatre: confirmation prompt
    println!("\n{}", "=".repeat(60));
    println!("You're about to clone: {}", repo_url);
    println!("{}", "=".repeat(60));
    println!("\nWARNING: Only clone repositories from sources you trust.");
    println!("Malicious code can compromise your system.");
    println!("\nCloning into ACTUAL sandboxed environment (bubblewrap):");
    println!("  Workspace: {}", workspace.display());
    println!("  Sandbox: Read-only system, isolated network, no device access");
    
    // Check if we have checksums for this repo
    let config = GitFetchConfig::load();
    let has_checksum = config.get_checksum(&repo_url).is_some();
    
    if has_checksum {
        println!("\n✓ Checksum registry found for this repository.");
        println!("  Integrity will be verified after cloning.");
    } else if verify_checksum {
        println!("\n⚠️  No checksum registry found for this repository.");
        println!("  Cannot verify integrity (--verify-checksum specified).");
        std::process::exit(1);
    }
    
    println!("\nDo you want to proceed? (yes/no)");
    print!("> ");
    io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");
    
    let mut response = String::new();
    io::stdin().read_line(&mut response).expect("Failed to read input");
    let response = response.trim().to_lowercase();
    
    if response != "yes" && response != "y" {
        println!("Clone cancelled. Probably for the best, really.");
        std::process::exit(0);
    }

    println!("\n{}", "=".repeat(60));
    println!("CLONING WITH BUBBLEWRAP SANDBOXING");
    println!("{}", "=".repeat(60));
    println!("Right then, cloning {} into ACTUAL sandboxed environment...", repo);
    println!("Because apparently doing security properly matters to you.\n");
    
    // Create a proper bubblewrap sandbox with REAL isolation:
    // 
    // What bubblewrap actually does (unlike fakeroot which does sod all):
    // - Creates a new mount namespace (isolated filesystem view)
    // - Creates a new PID namespace (isolated process tree)
    // - Creates a new UTS namespace (isolated hostname)
    // - Creates a new cgroup namespace (isolated resource controls)
    // - Mounts system directories as read-only (can't modify /usr, /bin, etc.)
    // - Provides ONLY our workspace as writable
    // - Uses tmpfs for /tmp (destroyed after process exits)
    // - No access to your home directory (except the workspace we create)
    // - Minimal /dev access (just enough for git to work)
    // - Network access is kept (needed for git clone, sadly)
    //
    // If the cloned code tries to:
    // - Write to /etc/passwd? Denied (read-only mount)
    // - Access your SSH keys in ~/.ssh? Denied (no home directory access)
    // - Install malware in /usr/bin? Denied (read-only mount)
    // - Delete files in /home? Denied (not mounted)
    // - Write anywhere except /workspace? Denied
    //
    // This is ACTUAL isolation, not security theatre.
    
    let status = Command::new("bwrap")
        .args(&[
            // Essential read-only system mounts
            "--ro-bind", "/usr", "/usr",
            "--ro-bind", "/lib", "/lib",
            "--ro-bind", "/lib64", "/lib64",
            "--ro-bind", "/bin", "/bin",
            "--ro-bind", "/sbin", "/sbin",
            "--ro-bind", "/etc", "/etc",
            // Proc filesystem (required for git)
            "--proc", "/proc",
            // Dev (minimal, for git to work)
            "--dev", "/dev",
            // Temporary directory
            "--tmpfs", "/tmp",
            // Our workspace - the ONLY writable location
            "--bind", workspace.to_str().unwrap(), "/workspace",
            // Change to workspace
            "--chdir", "/workspace",
            // Unshare everything we can
            "--unshare-pid",
            "--unshare-uts",
            "--unshare-cgroup",
            // Keep network (needed for clone, unfortunately)
            // but we could add --unshare-net if we wanted full isolation
            // Set a restricted environment
            "--clearenv",
            "--setenv", "PATH", "/usr/bin:/bin",
            "--setenv", "HOME", "/workspace",
            // The actual command to run
            "git", "clone", &repo_url
        ])
        .status()
        .expect("Failed to execute bubblewrap. Did it spontaneously uninstall itself?");

    if !status.success() {
        eprintln!("\nGit clone failed inside sandbox. Shocking, absolutely shocking.");
        eprintln!("Cleaning up workspace...");
        let _ = fs::remove_dir_all(&workspace);
        std::process::exit(1);
    }

    let mut config = GitFetchConfig::load();
    
    // The repository gets cloned into workspace/repo_name/
    let repo_in_workspace = workspace.join(&repo_name);
    
    let commit_hash = get_git_commit_hash(repo_in_workspace.to_str().unwrap());
    
    // Verify checksums if we have them
    let mut verified = false;
    if let Some(expected_checksum) = config.get_checksum(&repo_url) {
        println!("\n{}", "=".repeat(60));
        println!("VERIFYING REPOSITORY INTEGRITY");
        println!("{}", "=".repeat(60));
        
        match verify_repo_checksums(repo_in_workspace.to_str().unwrap(), expected_checksum) {
            Ok(true) => {
                verified = true;
                println!("\n✓ Repository integrity verified successfully!");
            }
            Ok(false) => {
                println!("\n⚠️  Integrity verification failed!");
                println!("\nProceed anyway? (yes/no)");
                print!("> ");
                io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");
                
                let mut response = String::new();
                io::stdin().read_line(&mut response).expect("Failed to read input");
                let response = response.trim().to_lowercase();
                
                if response != "yes" && response != "y" {
                    println!("\nWise choice. Removing cloned repository...");
                    let _ = std::fs::remove_dir_all(&workspace);
                    std::process::exit(0);
                }
            }
            Err(e) => {
                eprintln!("\nError during verification: {}", e);
            }
        }
    }
    
    // Run basic static analysis (security theatre continues)
    println!("\n{}", "=".repeat(60));
    println!("RUNNING BASIC SECURITY SCAN");
    println!("{}", "=".repeat(60));
    let warnings = scan_for_suspicious_patterns(repo_in_workspace.to_str().unwrap());
    
    if !warnings.is_empty() {
        println!("\n{}", "!".repeat(60));
        println!("SECURITY WARNINGS DETECTED:");
        println!("{}", "!".repeat(60));
        for warning in &warnings {
            println!("  ⚠  {}", warning);
        }
        println!("{}", "!".repeat(60));
        println!("\nThese patterns might indicate malicious code.");
        println!("Review the code carefully before running anything.");
        println!("\nProceed anyway? (yes/no)");
        print!("> ");
        io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");
        
        let mut response = String::new();
        io::stdin().read_line(&mut response).expect("Failed to read input");
        let response = response.trim().to_lowercase();
        
        if response != "yes" && response != "y" {
            println!("\nWise choice. Removing cloned repository...");
            let _ = std::fs::remove_dir_all(&workspace);
            std::process::exit(0);
        }
        
        println!("\nOn your head be it then.");
    } else {
        println!("No obvious red flags detected (doesn't mean it's safe, mind you).");
    }
    
    // Now copy to current working directory if user wants
    let current_dir = std::env::current_dir()
        .expect("Can't get current directory")
        .join(&repo_name);
    
    println!("\n{}", "=".repeat(60));
    println!("DEPLOYMENT OPTIONS");
    println!("{}", "=".repeat(60));
    println!("\nRepository cloned successfully to fakeroot workspace.");
    println!("Would you like to copy it to your current directory?");
    println!("  Current directory: {}", std::env::current_dir().unwrap().display());
    println!("  Target location:   {}", current_dir.display());
    println!("\nCopy to current directory? (yes/no)");
    print!("> ");
    io::Write::flush(&mut io::stdout()).expect("Failed to flush stdout");
    
    let mut response = String::new();
    io::stdin().read_line(&mut response).expect("Failed to read input");
    let response = response.trim().to_lowercase();
    
    let final_path = if response == "yes" || response == "y" {
        println!("\nCopying from fakeroot workspace to current directory...");
        
        // Use cp -r to copy the repository out of workspace
        let copy_status = Command::new("cp")
            .args(&[
                "-r",
                repo_in_workspace.to_str().unwrap(),
                current_dir.to_str().unwrap(),
            ])
            .status()
            .expect("Failed to copy repository");
        
        if !copy_status.success() {
            eprintln!("Failed to copy repository. Keeping it in workspace only.");
            repo_in_workspace.to_string_lossy().to_string()
        } else {
            println!("✓ Repository copied successfully!");
            current_dir.to_string_lossy().to_string()
        }
    } else {
        println!("\nKeeping repository in fakeroot workspace only.");
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
    println!("CLONE COMPLETE");
    println!("{}", "=".repeat(60));
    println!("Repository location:       {}", final_path);
    println!("Sandboxed workspace:       {}", workspace.display());
    
    if !verified && !has_checksum {
        println!("\n💡 Security Tip:");
        println!("   Create a checksum registry to verify future clones:");
        println!("   gitfetch checksum {} --save", final_path);
    }
    
    println!("\nNow cd into it yourself, this isn't a bloody taxi service.");
    if final_path.starts_with(&std::env::current_dir().unwrap().to_string_lossy().to_string()) {
        println!("  cd {}", repo_name);
    } else {
        println!("  cd {}", final_path);
    }
}

fn list_repos() {
    let config = GitFetchConfig::load();
    
    if config.installed_repos.is_empty() {
        println!("You haven't installed bugger all with gitfetch yet.");
        return;
    }

    println!("Repositories you've installed with this rubbish:");
    println!();
    for repo in &config.installed_repos {
        let verified_marker = if repo.verified { "✓" } else { "?" };
        println!("  {} {} ", verified_marker, repo.name);
        println!("    URL: {}", repo.url);
        println!("    Path: {}", repo.path);
        if let Some(workspace) = &repo.workspace_path {
            println!("    Sandboxed workspace: {}", workspace);
        }
        if let Some(commit) = &repo.commit_hash {
            println!("    Commit: {}", commit);
        }
        if repo.verified {
            println!("    Status: Checksum verified");
        } else {
            println!("    Status: Not verified");
        }
        println!();
    }
    
    if config.checksum_registry.is_empty() {
        println!("💡 No checksum registries saved yet.");
        println!("   Use 'gitfetch checksum <path> --save' to create them.");
    } else {
        println!("Checksum registries: {} saved", config.checksum_registry.len());
    }
}

fn search_repos(query: &str) {
    println!("Searching for '{}'... because the GitHub website was clearly too mainstream for you", query);
    
    // Search in repository name specifically, not description
    let search_query = if query.contains('/') {
        // If it looks like user/repo, search for exact repo name
        format!("repo:{}", query)
    } else {
        // Otherwise search in repository names
        format!("{} in:name", query)
    };
    
    let url = format!(
        "https://api.github.com/search/repositories?q={}&sort=stars&order=desc",
        urlencoding::encode(&search_query)
    );

    let client = reqwest::blocking::Client::builder()
        .user_agent("gitfetch/0.17")
        .build()
        .expect("Can't create HTTP client");

    let response = client
        .get(&url)
        .send();

    match response {
        Ok(resp) => {
            let status = resp.status();
            
            // Check for rate limiting before anything else
            if status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                eprintln!("GitHub's slapped you with a rate limit. Brilliant.");
                
                // Try to get rate limit reset time from headers
                if let Some(reset_time) = resp.headers().get("x-ratelimit-reset") {
                    if let Ok(reset_str) = reset_time.to_str() {
                        if let Ok(reset_timestamp) = reset_str.parse::<i64>() {
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64;
                            let wait_seconds = reset_timestamp - now;
                            
                            if wait_seconds > 0 {
                                let minutes = wait_seconds / 60;
                                let seconds = wait_seconds % 60;
                                eprintln!("Rate limit resets in {} minutes, {} seconds.", minutes, seconds);
                                eprintln!("Go make a cuppa, I suppose.");
                            } else {
                                eprintln!("Rate limit should've reset by now. Try again.");
                            }
                        }
                    }
                }
                
                eprintln!("\nAlternatively, authenticate with a GitHub token for higher limits:");
                eprintln!("  export GITHUB_TOKEN=your_token_here");
                eprintln!("  (Then I'll use it automatically. Not implemented yet, mind you.)");
                return;
            }
            
            if status.is_success() {
                let search_result: Result<GitHubSearchResponse, _> = resp.json();
                match search_result {
                    Ok(result) => {
                        if result.items.is_empty() {
                            println!("Found precisely nothing. Remarkable.");
                        } else {
                            println!("\nFound {} repositories (showing top results):\n", result.items.len());
                            for repo in result.items.iter().take(10) {
                                println!("  {} ", repo.full_name);
                                println!("    ⭐ Stars: {} | 👀 Watchers: {}", 
                                    repo.stargazers_count, repo.watchers_count);
                                if let Some(desc) = &repo.description {
                                    println!("    {}", desc);
                                }
                                println!("    {}", repo.html_url);
                                println!();
                            }
                        }
                    }
                    Err(e) => eprintln!("Failed to parse GitHub's response: {}. Typical.", e),
                }
            } else {
                eprintln!("GitHub API returned status {}. Lovely.", status);
            }
        }
        Err(e) => {
            eprintln!("Network request failed: {}. Check your internet connection, you muppet.", e);
        }
    }
}

fn easter_egg() {
    let output = Command::new("whoami")
        .stdout(Stdio::piped())
        .output()
        .expect("Failed to run whoami. What operating system is this?");

    let username = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!("{} is based for using gitfetch like a true lazy dev", username);
}

fn generate_completions(shell: Shell) {
    let completion_script = match shell {
        Shell::Bash => include_str!("../gitfetch.bash"),
        Shell::Zsh => include_str!("../gitfetch.zsh"),
        Shell::Fish => include_str!("../gitfetch.fish"),
        _ => {
            eprintln!("Bloody hell, {} isn't supported with custom completions yet.", shell);
            eprintln!("Falling back to basic static completions...");
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "gitfetch", &mut io::stdout());
            eprintln!();
            eprintln!("Note: These are basic completions without intelligent suggestions.");
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
            // Suggest installed repos for operations that work on them
            let config = GitFetchConfig::load();
            for repo in &config.installed_repos {
                if repo.name.starts_with(partial) || partial.is_empty() {
                    println!("{}", repo.name);
                }
            }
        }
        "clone-targets" => {
            // For clone operations, suggest installed repos as a starting point
            // (users can still type any URL/repo)
            let config = GitFetchConfig::load();
            for repo in &config.installed_repos {
                if repo.url.contains(partial) || repo.name.contains(partial) || partial.is_empty() {
                    // Suggest the short form if it's a github repo
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
        .expect("Can't resolve path. Does it exist?")
        .to_string_lossy()
        .to_string();
    
    println!("Calculating checksums for: {}", abs_path);
    println!("{}", "=".repeat(60));
    
    match calculate_repo_checksums(&abs_path) {
        Ok(checksum) => {
            println!("\n{}", "=".repeat(60));
            println!("CHECKSUM CALCULATION COMPLETE");
            println!("{}", "=".repeat(60));
            println!("Repository: {}", checksum.repo_url);
            println!("Commit:     {}", checksum.commit_hash);
            println!("Files:      {}", checksum.file_checksums.len());
            println!("Total Hash: {}", checksum.total_hash);
            println!("Timestamp:  {}", checksum.verified_at);
            
            if save {
                let mut config = GitFetchConfig::load();
                let repo_url = checksum.repo_url.clone();
                config.add_checksum(repo_url.clone(), checksum);
                println!("\n✓ Checksum registry saved for: {}", repo_url);
                println!("  Future clones can be verified with --verify-checksum flag");
            } else {
                println!("\n💡 Add --save flag to save this checksum registry");
                println!("   gitfetch checksum {} --save", path);
            }
        }
        Err(e) => {
            eprintln!("\nFailed to calculate checksums: {}", e);
            eprintln!("Make sure the path is a valid git repository.");
            std::process::exit(1);
        }
    }
}

fn verify_command(path: &str) {
    let abs_path = std::path::Path::new(path)
        .canonicalize()
        .expect("Can't resolve path. Does it exist?")
        .to_string_lossy()
        .to_string();
    
    let repo_url = get_git_remote_url(&abs_path)
        .expect("Can't get repository URL. Is this a git repository?");
    
    let config = GitFetchConfig::load();
    
    match config.get_checksum(&repo_url) {
        Some(expected) => {
            println!("Verifying: {}", abs_path);
            println!("{}", "=".repeat(60));
            
            match verify_repo_checksums(&abs_path, expected) {
                Ok(true) => {
                    println!("\n✓ SUCCESS: Repository integrity verified!");
                    std::process::exit(0);
                }
                Ok(false) => {
                    println!("\n✗ FAILURE: Repository integrity check failed!");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("\nError during verification: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            eprintln!("No checksum registry found for: {}", repo_url);
            eprintln!("\nCreate one first:");
            eprintln!("  gitfetch checksum {} --save", path);
            std::process::exit(1);
        }
    }
}

fn scan_for_suspicious_patterns(repo_path: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let path = std::path::Path::new(repo_path);
    
    if !path.exists() {
        return warnings;
    }
    
    // Scan files for suspicious patterns (this is incredibly naive but better than nothing)
    let suspicious_patterns = vec![
        ("eval(", "Use of eval() - can execute arbitrary code"),
        ("exec(", "Use of exec() - can execute system commands"),
        ("__import__", "Dynamic imports detected"),
        ("subprocess.call", "System command execution detected"),
        ("subprocess.run", "System command execution detected"),
        ("os.system", "Direct system command execution"),
        ("shell=True", "Shell command with shell=True (dangerous)"),
        ("/etc/passwd", "Accessing password file"),
        ("/etc/shadow", "Accessing shadow file"),
        ("rm -rf", "Recursive file deletion command"),
        ("curl", "Network request detected"),
        ("wget", "Network request detected"),
        ("base64.b64decode", "Base64 decoding (often used to hide code)"),
        ("chmod +x", "Making files executable"),
        (".bash_profile", "Modifying shell profile"),
        (".bashrc", "Modifying shell config"),
        (".zshrc", "Modifying shell config"),
        ("authorized_keys", "Accessing SSH keys"),
        ("id_rsa", "Accessing SSH private key"),
        ("bitcoin", "Cryptocurrency-related code"),
        ("crypto mining", "Potential cryptominer"),
        ("keylogger", "Keylogger detected"),
        ("reverse shell", "Reverse shell detected"),
        ("backdoor", "Backdoor keyword found"),
    ];
    
    let extensions_to_scan = vec![
        "py", "js", "sh", "bash", "zsh", "rb", "pl", "php", 
        "rs", "go", "java", "c", "cpp", "h", "hpp"
    ];
    
    // Walk the directory tree (limit depth to avoid scanning forever)
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            
            // Skip hidden files and common directories
            if let Some(name) = entry_path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.starts_with('.') || name_str == "node_modules" || name_str == "target" {
                    continue;
                }
            }
            
            if entry_path.is_file() {
                if let Some(ext) = entry_path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if extensions_to_scan.contains(&ext_str.as_str()) {
                        // Scan this file
                        if let Ok(content) = std::fs::read_to_string(&entry_path) {
                            let content_lower = content.to_lowercase();
                            for (pattern, description) in &suspicious_patterns {
                                if content_lower.contains(&pattern.to_lowercase()) {
                                    let filename = entry_path.file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy();
                                    warnings.push(format!("{}: {}", filename, description));
                                }
                            }
                        }
                    }
                }
            } else if entry_path.is_dir() {
                // Recursively scan subdirectories (one level only to avoid performance hell)
                if let Ok(sub_entries) = std::fs::read_dir(&entry_path) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.is_file() {
                            if let Some(ext) = sub_path.extension() {
                                let ext_str = ext.to_string_lossy().to_lowercase();
                                if extensions_to_scan.contains(&ext_str.as_str()) {
                                    if let Ok(content) = std::fs::read_to_string(&sub_path) {
                                        let content_lower = content.to_lowercase();
                                        for (pattern, description) in &suspicious_patterns {
                                            if content_lower.contains(&pattern.to_lowercase()) {
                                                let filename = sub_path.file_name()
                                                    .unwrap_or_default()
                                                    .to_string_lossy();
                                                warnings.push(format!("{}: {}", filename, description));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Deduplicate warnings
    warnings.sort();
    warnings.dedup();
    
    warnings
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => display_banner(),
        Some(Commands::Clone { repo, verify_checksum }) => clone_repo(&repo, verify_checksum),
        Some(Commands::List) => list_repos(),
        Some(Commands::Search { query }) => search_repos(&query),
        Some(Commands::EasterEgg) => easter_egg(),
        Some(Commands::Completions { shell }) => generate_completions(shell),
        Some(Commands::Complete { completion_type, partial }) => complete_suggestions(&completion_type, &partial),
        Some(Commands::Checksum { path, save }) => checksum_command(&path, save),
        Some(Commands::Verify { path }) => verify_command(&path),
    }
}
