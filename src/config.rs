use crate::types::{InstalledRepo, RepoChecksum};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct GitFetchConfig {
    pub installed_repos: Vec<InstalledRepo>,
    pub checksum_registry: HashMap<String, RepoChecksum>,
}

impl GitFetchConfig {
    pub fn load() -> Self {
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

    pub fn save(&self) {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).expect("Can't create config directory");
        }
        let contents = serde_json::to_string_pretty(self)
            .expect("Failed to serialize config");
        fs::write(&config_path, contents).expect("Can't write config file");
    }

    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").expect("No HOME directory?");
        PathBuf::from(home).join(".config").join("gitfetch").join("config.json")
    }

    pub fn add_repo(
        &mut self,
        name: String,
        url: String,
        path: String,
        commit_hash: Option<String>,
        verified: bool,
        workspace_path: Option<String>,
    ) {
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

    pub fn add_checksum(&mut self, repo_url: String, checksum: RepoChecksum) {
        self.checksum_registry.insert(repo_url, checksum);
        self.save();
    }

    pub fn get_checksum(&self, repo_url: &str) -> Option<&RepoChecksum> {
        self.checksum_registry.get(repo_url)
    }
}
