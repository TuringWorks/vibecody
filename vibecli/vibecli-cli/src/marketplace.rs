#![allow(dead_code)]
//! Plugin Marketplace — registry client for discovering and installing plugins.
//!
//! The marketplace uses a JSON index fetched from a configurable URL.
//! Plugins are git-based and installed via the existing plugin system.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A single entry in the marketplace registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplacePlugin {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub repo_url: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub updated_at: String,
}

/// The full marketplace index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceIndex {
    #[serde(default)]
    pub plugins: Vec<MarketplacePlugin>,
    #[serde(default)]
    pub updated_at: String,
}

/// The marketplace client.
pub struct Marketplace {
    index_url: String,
    cache_path: std::path::PathBuf,
}

impl Marketplace {
    pub fn new() -> Self {
        let cache_path = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join(".vibecli")
            .join("marketplace-index.json");

        Self {
            index_url: "https://raw.githubusercontent.com/nicktrebes/vibecody-plugins/main/index.json"
                .to_string(),
            cache_path,
        }
    }

    /// Fetch the latest index from the remote URL.
    pub async fn refresh(&self) -> Result<MarketplaceIndex> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()?;

        match client.get(&self.index_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let text = resp.text().await?;
                let index: MarketplaceIndex = serde_json::from_str(&text)?;
                // Cache to disk
                let _ = std::fs::write(&self.cache_path, &text);
                Ok(index)
            }
            Ok(resp) => {
                // Fall back to cached version
                eprintln!(
                    "[marketplace] Remote returned {}, using cache",
                    resp.status()
                );
                self.load_cached()
            }
            Err(e) => {
                eprintln!("[marketplace] Fetch failed: {}, using cache", e);
                self.load_cached()
            }
        }
    }

    /// Load the cached index from disk.
    pub fn load_cached(&self) -> Result<MarketplaceIndex> {
        if self.cache_path.exists() {
            let text = std::fs::read_to_string(&self.cache_path)?;
            Ok(serde_json::from_str(&text)?)
        } else {
            // Return built-in starter index
            Ok(MarketplaceIndex {
                plugins: builtin_plugins(),
                updated_at: "2026-03-01".to_string(),
            })
        }
    }

    /// Search the marketplace index by name, description, or tags.
    pub async fn search(&self, query: &str) -> Result<Vec<MarketplacePlugin>> {
        let index = self.load_cached().unwrap_or_else(|_| MarketplaceIndex {
            plugins: builtin_plugins(),
            updated_at: String::new(),
        });

        let q = query.to_lowercase();
        let results: Vec<MarketplacePlugin> = index
            .plugins
            .into_iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.description.to_lowercase().contains(&q)
                    || p.tags.iter().any(|t| t.to_lowercase().contains(&q))
                    || p.author.to_lowercase().contains(&q)
            })
            .collect();

        Ok(results)
    }

    /// Look up a plugin by exact name.
    pub fn find_by_name(&self, name: &str) -> Result<Option<MarketplacePlugin>> {
        let index = self.load_cached()?;
        Ok(index
            .plugins
            .into_iter()
            .find(|p| p.name.eq_ignore_ascii_case(name)))
    }
}

/// Built-in starter plugins that are always available.
fn builtin_plugins() -> Vec<MarketplacePlugin> {
    vec![
        MarketplacePlugin {
            name: "vibecli-prettier".to_string(),
            description: "Auto-format code with Prettier after file writes".to_string(),
            version: "1.0.0".to_string(),
            author: "VibeCody".to_string(),
            repo_url: "https://github.com/nicktrebes/vibecli-prettier".to_string(),
            tags: vec!["formatting".into(), "prettier".into(), "hooks".into()],
            downloads: 0,
            updated_at: "2026-03-01".to_string(),
        },
        MarketplacePlugin {
            name: "vibecli-eslint".to_string(),
            description: "Run ESLint checks after TypeScript/JavaScript edits".to_string(),
            version: "1.0.0".to_string(),
            author: "VibeCody".to_string(),
            repo_url: "https://github.com/nicktrebes/vibecli-eslint".to_string(),
            tags: vec!["linting".into(), "eslint".into(), "javascript".into()],
            downloads: 0,
            updated_at: "2026-03-01".to_string(),
        },
        MarketplacePlugin {
            name: "vibecli-docker".to_string(),
            description: "Docker tools — build, run, compose from agent context".to_string(),
            version: "1.0.0".to_string(),
            author: "VibeCody".to_string(),
            repo_url: "https://github.com/nicktrebes/vibecli-docker".to_string(),
            tags: vec!["docker".into(), "devops".into(), "containers".into()],
            downloads: 0,
            updated_at: "2026-03-01".to_string(),
        },
        MarketplacePlugin {
            name: "vibecli-terraform".to_string(),
            description: "Terraform plan/apply integration with drift detection".to_string(),
            version: "1.0.0".to_string(),
            author: "VibeCody".to_string(),
            repo_url: "https://github.com/nicktrebes/vibecli-terraform".to_string(),
            tags: vec!["terraform".into(), "iac".into(), "devops".into()],
            downloads: 0,
            updated_at: "2026-03-01".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_plugins_exist() {
        let plugins = builtin_plugins();
        assert!(plugins.len() >= 3);
    }

    #[test]
    fn marketplace_index_serde() {
        let index = MarketplaceIndex {
            plugins: builtin_plugins(),
            updated_at: "2026-03-01".to_string(),
        };
        let json = serde_json::to_string(&index).unwrap();
        let parsed: MarketplaceIndex = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.plugins.len(), index.plugins.len());
    }

    #[test]
    fn marketplace_new_does_not_panic() {
        let m = Marketplace::new();
        assert!(!m.index_url.is_empty());
    }

    #[tokio::test]
    async fn search_builtin() {
        let m = Marketplace::new();
        let results = m.search("docker").await.unwrap();
        // Should find the docker plugin in builtins
        assert!(results.iter().any(|p| p.name.contains("docker")));
    }

    // ── MarketplacePlugin serde roundtrip ────────────────────────────────────

    #[test]
    fn marketplace_plugin_serde_roundtrip() {
        let plugin = MarketplacePlugin {
            name: "test-plugin".to_string(),
            description: "A test plugin".to_string(),
            version: "2.1.0".to_string(),
            author: "TestAuthor".to_string(),
            repo_url: "https://github.com/example/test-plugin".to_string(),
            tags: vec!["testing".into(), "ci".into()],
            downloads: 42,
            updated_at: "2026-03-07".to_string(),
        };
        let json = serde_json::to_string(&plugin).unwrap();
        let back: MarketplacePlugin = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "test-plugin");
        assert_eq!(back.version, "2.1.0");
        assert_eq!(back.tags, vec!["testing", "ci"]);
        assert_eq!(back.downloads, 42);
    }

    // ── MarketplacePlugin defaults for optional fields ───────────────────────

    #[test]
    fn marketplace_plugin_json_defaults() {
        let json = r#"{
            "name": "minimal",
            "description": "desc",
            "version": "0.1.0",
            "author": "anon",
            "repo_url": "https://example.com"
        }"#;
        let plugin: MarketplacePlugin = serde_json::from_str(json).unwrap();
        assert!(plugin.tags.is_empty());
        assert_eq!(plugin.downloads, 0);
        assert_eq!(plugin.updated_at, "");
    }

    // ── builtin_plugins have valid data ──────────────────────────────────────

    #[test]
    fn builtin_plugins_have_valid_urls() {
        for plugin in builtin_plugins() {
            assert!(plugin.repo_url.starts_with("https://"), "repo_url for {} should be https", plugin.name);
            assert!(!plugin.name.is_empty());
            assert!(!plugin.description.is_empty());
            assert!(!plugin.version.is_empty());
            assert!(!plugin.author.is_empty());
        }
    }

    #[test]
    fn builtin_plugins_have_unique_names() {
        let plugins = builtin_plugins();
        let mut names: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), plugins.len(), "builtin plugin names must be unique");
    }

    // ── search matches on tags ───────────────────────────────────────────────

    #[tokio::test]
    async fn search_by_tag() {
        let m = Marketplace::new();
        let results = m.search("devops").await.unwrap();
        assert!(results.len() >= 1, "should match plugins tagged with devops");
    }

    // ── search matches on author ─────────────────────────────────────────────

    #[tokio::test]
    async fn search_by_author() {
        let m = Marketplace::new();
        let results = m.search("vibecody").await.unwrap();
        assert_eq!(results.len(), builtin_plugins().len(), "all builtins are by VibeCody");
    }

    // ── search case insensitive ──────────────────────────────────────────────

    #[tokio::test]
    async fn search_case_insensitive() {
        let m = Marketplace::new();
        let results_lower = m.search("docker").await.unwrap();
        let results_upper = m.search("DOCKER").await.unwrap();
        assert_eq!(results_lower.len(), results_upper.len());
    }

    // ── search no match returns empty ────────────────────────────────────────

    #[tokio::test]
    async fn search_no_match_returns_empty() {
        let m = Marketplace::new();
        let results = m.search("zzz_nonexistent_plugin_xyz").await.unwrap();
        assert!(results.is_empty());
    }

    // ── MarketplaceIndex empty plugins ───────────────────────────────────────

    #[test]
    fn marketplace_index_empty_defaults() {
        let json = r#"{}"#;
        let index: MarketplaceIndex = serde_json::from_str(json).unwrap();
        assert!(index.plugins.is_empty());
        assert_eq!(index.updated_at, "");
    }
}
