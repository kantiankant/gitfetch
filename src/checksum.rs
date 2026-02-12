use crate::git;
use crate::types::RepoChecksum;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

/// Calculate SHA256 checksum of a single file
pub fn calculate_file_checksum(file_path: &Path) -> io::Result<String> {
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

/// Calculate checksums for all files in a repository
pub fn calculate_repo_checksums(repo_path: &str) -> io::Result<RepoChecksum> {
    let path = Path::new(repo_path);
    let mut file_checksums = HashMap::new();
    
    let commit_hash = git::get_commit_hash(repo_path)
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
    
    let repo_url = git::get_remote_url(repo_path)
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

/// Recursively walk directory and calculate checksums
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
        
        // Skip hidden files and common build directories
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

/// Verify repository checksums against expected values
pub fn verify_repo_checksums(repo_path: &str, expected: &RepoChecksum) -> Result<bool, String> {
    println!("Verifying repository integrity...");
    
    if let Some(current_commit) = git::get_commit_hash(repo_path) {
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
    
    // Check expected files
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
    
    // Check for unexpected files
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
