use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstalledRepo {
    pub name: String,
    pub url: String,
    pub path: String,
    pub commit_hash: Option<String>,
    pub verified: bool,
    pub workspace_path: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RepoChecksum {
    pub repo_url: String,
    pub commit_hash: String,
    pub file_checksums: HashMap<String, String>,
    pub total_hash: String,
    pub verified_at: String,
}

#[derive(Deserialize, Debug)]
pub struct GitHubRepo {
    pub full_name: String,
    pub html_url: String,
    pub description: Option<String>,
    pub stargazers_count: u32,
}

#[derive(Deserialize, Debug)]
pub struct GitHubSearchResponse {
    pub items: Vec<GitHubRepo>,
}
