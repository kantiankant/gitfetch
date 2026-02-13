use crate::types::GitHubSearchResponse;

pub fn search_repos(query: &str) {
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
            let status = resp.status();
            
            // Check for rate limiting (403 or 429)
            if status.as_u16() == 403 || status.as_u16() == 429 {
                eprintln!("\nSod off, you've been rate limited. Maybe use the GUI sometime?");
                eprintln!("(GitHub allows 10 unauthenticated requests per minute)");
                std::process::exit(1);
            }
            
            if status.is_success() {
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
                eprintln!("GitHub API error: {}", status);
            }
        }
        Err(e) => eprintln!("Network error: {}", e),
    }
}
