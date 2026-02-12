use crate::checksum;
use crate::config::GitFetchConfig;
use crate::git;
use crate::security;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn clone_repo(repo: &str, verify_checksum: bool, trust_mode: &str) {
    security::check_bubblewrap().unwrap_or_else(|_| std::process::exit(1));
    
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
        if !security::prompt_user("WARNING: Clone from untrusted source?\nProceed? (yes/no)") {
            println!("Clone cancelled.");
            std::process::exit(0);
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("STAGE 1: FETCH (with network)");
    println!("{}", "=".repeat(60));
    
    // Stage 1: Clone with --no-checkout (network enabled, no hooks run)
    if let Err(e) = security::run_sandboxed_git(&workspace, &["clone", "--no-checkout", &repo_url], true) {
        eprintln!("Clone failed: {}", e);
        let _ = fs::remove_dir_all(&workspace);
        std::process::exit(1);
    }

    println!("\n{}", "=".repeat(60));
    println!("STAGE 2: CHECKOUT (network isolated, hooks disabled)");
    println!("{}", "=".repeat(60));
    
    let repo_in_workspace = workspace.join(&repo_name);
    
    // Stage 2: Checkout without network (hooks are disabled via env)
    if let Err(e) = security::run_sandboxed_git(
        &repo_in_workspace,
        &["checkout", "--force", "HEAD"],
        false // Network disabled
    ) {
        eprintln!("Checkout failed: {}", e);
        let _ = fs::remove_dir_all(&workspace);
        std::process::exit(1);
    }

    let mut config = GitFetchConfig::load();
    let commit_hash = git::get_commit_hash(repo_in_workspace.to_str().unwrap());
    
    // Verify checksums if available
    let mut verified = false;
    if let Some(expected_checksum) = config.get_checksum(&repo_url) {
        println!("\n{}", "=".repeat(60));
        println!("VERIFYING CHECKSUMS");
        println!("{}", "=".repeat(60));
        
        match checksum::verify_repo_checksums(repo_in_workspace.to_str().unwrap(), expected_checksum) {
            Ok(true) => {
                verified = true;
                println!("✓ Integrity verified!");
            }
            Ok(false) => {
                if trust_mode != "yolo" && !security::prompt_user("\nVerification failed. Proceed? (yes/no)") {
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
    
    let warnings = security::scan_for_suspicious_patterns(repo_in_workspace.to_str().unwrap());
    
    if !warnings.is_empty() {
        println!("⚠️  {} suspicious patterns detected:", warnings.len());
        for (i, warning) in warnings.iter().take(5).enumerate() {
            println!("  {}. {}", i + 1, warning);
        }
        if warnings.len() > 5 {
            println!("  ... and {} more", warnings.len() - 5);
        }
        
        if trust_mode == "paranoid" && !security::prompt_user("\nSuspicious code detected. Proceed? (yes/no)") {
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
    
    let final_path = if trust_mode != "paranoid" || security::prompt_user("\nCopy to current directory? (yes/no)") {
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
