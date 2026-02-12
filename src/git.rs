use std::process::Command;

/// Get the current commit hash of a git repository
pub fn get_commit_hash(repo_path: &str) -> Option<String> {
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

/// Get the remote URL of a git repository
pub fn get_remote_url(repo_path: &str) -> Option<String> {
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
