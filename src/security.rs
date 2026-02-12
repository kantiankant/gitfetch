use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

/// Check if bubblewrap is installed
pub fn check_bubblewrap() -> Result<(), String> {
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

/// Run git command in a sandboxed environment using bubblewrap
pub fn run_sandboxed_git(workspace: &Path, args: &[&str], with_network: bool) -> Result<(), String> {
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

/// Scan repository for suspicious code patterns
pub fn scan_for_suspicious_patterns(repo_path: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let path = Path::new(repo_path);
    
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
    
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            
            // Skip hidden files and node_modules
            if let Some(name) = entry_path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.starts_with('.') || name_str == "node_modules" {
                    continue;
                }
            }
            
            if entry_path.is_file() {
                if let Some(ext) = entry_path.extension() {
                    if extensions.contains(&ext.to_string_lossy().as_ref()) {
                        if let Ok(content) = fs::read_to_string(&entry_path) {
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

/// Prompt user for yes/no confirmation
pub fn prompt_user(message: &str) -> bool {
    println!("{}", message);
    print!("> ");
    io::stdout().flush().expect("Failed to flush stdout");
    
    let mut response = String::new();
    io::stdin().read_line(&mut response).expect("Failed to read input");
    let response = response.trim().to_lowercase();
    
    response == "yes" || response == "y"
}
