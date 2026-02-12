use crate::checksum;
use crate::config::GitFetchConfig;
use crate::git;

pub fn checksum_command(path: &str, save: bool) {
    let abs_path = std::path::Path::new(path)
        .canonicalize()
        .expect("Can't resolve path")
        .to_string_lossy()
        .to_string();
    
    match checksum::calculate_repo_checksums(&abs_path) {
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

pub fn verify_command(path: &str) {
    let abs_path = std::path::Path::new(path)
        .canonicalize()
        .expect("Can't resolve path")
        .to_string_lossy()
        .to_string();
    
    let repo_url = git::get_remote_url(&abs_path)
        .expect("Can't get repository URL");
    
    let config = GitFetchConfig::load();
    
    match config.get_checksum(&repo_url) {
        Some(expected) => {
            match checksum::verify_repo_checksums(&abs_path, expected) {
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
