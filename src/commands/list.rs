use crate::config::GitFetchConfig;

pub fn list_repos() {
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
