#![allow(dead_code)]
//! Team knowledge store — shared configuration for teams.
//!
//! Team configuration is stored in `.vibecli/team.toml` (committed to git)
//! and optionally in `~/.vibecli/team.toml` (personal overrides).
//!
//! Example `.vibecli/team.toml`:
//! ```toml
//! [team]
//! name = "VibeCody Dev Team"
//!
//! [[knowledge]]
//! name = "deploy-process"
//! content = "Run `npm run deploy:staging` to deploy to staging."
//! tags = ["deployment", "ops"]
//!
//! [[shared_commands]]
//! name = "deploy-staging"
//! command = "npm run deploy:staging"
//! description = "Deploy to staging environment"
//!
//! [[shared_mcp]]
//! name = "github"
//! command = "npx @modelcontextprotocol/server-github"
//! ```
//!
//! REPL commands: `/team sync | knowledge add | knowledge list | show`

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── TeamConfig ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeamInfo {
    pub name: Option<String>,
    /// Remote URL for syncing team.toml (e.g., raw GitHub URL).
    pub knowledge_base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedCommand {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedMcp {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TeamConfig {
    #[serde(default)]
    pub team: TeamInfo,
    #[serde(default)]
    pub knowledge: Vec<KnowledgeEntry>,
    #[serde(default)]
    pub shared_commands: Vec<SharedCommand>,
    #[serde(default)]
    pub shared_mcp: Vec<SharedMcp>,
}

impl TeamConfig {
    /// Build a context block to inject into the agent system prompt.
    pub fn context_string(&self) -> String {
        if self.knowledge.is_empty() && self.shared_commands.is_empty() {
            return String::new();
        }

        let mut parts = Vec::new();

        if let Some(name) = &self.team.name {
            parts.push(format!("=== Team: {} ===", name));
        } else {
            parts.push("=== Team Knowledge ===".to_string());
        }

        if !self.knowledge.is_empty() {
            parts.push("Knowledge:".to_string());
            for k in &self.knowledge {
                let tags = if k.tags.is_empty() {
                    String::new()
                } else {
                    format!(" [{}]", k.tags.join(", "))
                };
                parts.push(format!("- {}{}: {}", k.name, tags, k.content));
            }
        }

        if !self.shared_commands.is_empty() {
            parts.push("Shared commands:".to_string());
            for cmd in &self.shared_commands {
                parts.push(format!("- {} → `{}` — {}", cmd.name, cmd.command, cmd.description));
            }
        }

        parts.join("\n") + "\n"
    }
}

// ── TeamManager ───────────────────────────────────────────────────────────────

pub struct TeamManager {
    workspace_path: Option<PathBuf>,
}

impl TeamManager {
    pub fn for_workspace(workspace_root: &Path) -> Self {
        Self { workspace_path: Some(workspace_root.to_path_buf()) }
    }

    fn team_toml_path(&self) -> Option<PathBuf> {
        self.workspace_path.as_ref().map(|p| p.join(".vibecli").join("team.toml"))
    }

    fn global_team_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".vibecli").join("team.toml"))
    }

    /// Load the team config (workspace first, then global).
    pub fn load(&self) -> TeamConfig {
        // Try workspace team.toml
        if let Some(path) = self.team_toml_path() {
            if let Ok(raw) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = toml::from_str::<TeamConfig>(&raw) {
                    return cfg;
                }
            }
        }
        // Try global ~/.vibecli/team.toml
        if let Some(path) = Self::global_team_path() {
            if let Ok(raw) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = toml::from_str::<TeamConfig>(&raw) {
                    return cfg;
                }
            }
        }
        TeamConfig::default()
    }

    /// Save the team config to the workspace.
    pub fn save(&self, config: &TeamConfig) -> Result<()> {
        let path = self.team_toml_path()
            .ok_or_else(|| anyhow::anyhow!("No workspace set for team config"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(config)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Add a knowledge entry and save.
    pub fn add_knowledge(&self, name: &str, content: &str, tags: Vec<String>) -> Result<()> {
        let mut config = self.load();
        // Remove existing entry with same name
        config.knowledge.retain(|k| k.name != name);
        config.knowledge.push(KnowledgeEntry {
            name: name.to_string(),
            content: content.to_string(),
            tags,
        });
        self.save(&config)
    }

    /// Remove a knowledge entry by name.
    pub fn remove_knowledge(&self, name: &str) -> Result<bool> {
        let mut config = self.load();
        let before = config.knowledge.len();
        config.knowledge.retain(|k| k.name != name);
        let removed = config.knowledge.len() < before;
        if removed {
            self.save(&config)?;
        }
        Ok(removed)
    }

    /// Sync team.toml from the remote URL (if configured).
    pub async fn sync(&self) -> Result<String> {
        let config = self.load();
        let url = config.team.knowledge_base_url
            .ok_or_else(|| anyhow::anyhow!("No knowledge_base_url configured in team.toml"))?;

        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await
            .map_err(|e| anyhow::anyhow!("Failed to fetch team config: {}", e))?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to sync team config: HTTP {}", resp.status());
        }
        let raw = resp.text().await?;
        let remote_cfg: TeamConfig = toml::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("Invalid remote team.toml: {}", e))?;
        let team_name = remote_cfg.team.name.clone().unwrap_or_else(|| "team".to_string());
        let knowledge_count = remote_cfg.knowledge.len();
        self.save(&remote_cfg)?;
        Ok(format!("Synced team '{}' — {} knowledge entries", team_name, knowledge_count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_default_when_no_file() {
        let tmp = TempDir::new().unwrap();
        let mgr = TeamManager::for_workspace(tmp.path());
        let cfg = mgr.load();
        assert!(cfg.knowledge.is_empty());
    }

    #[test]
    fn add_and_remove_knowledge() {
        let tmp = TempDir::new().unwrap();
        let mgr = TeamManager::for_workspace(tmp.path());
        mgr.add_knowledge("deploy", "Run `npm run deploy`", vec!["ops".to_string()]).unwrap();

        let cfg = mgr.load();
        assert_eq!(cfg.knowledge.len(), 1);
        assert_eq!(cfg.knowledge[0].name, "deploy");

        let removed = mgr.remove_knowledge("deploy").unwrap();
        assert!(removed);
        assert!(mgr.load().knowledge.is_empty());
    }

    #[test]
    fn context_string_format() {
        let cfg = TeamConfig {
            team: TeamInfo { name: Some("Acme".to_string()), knowledge_base_url: None },
            knowledge: vec![KnowledgeEntry {
                name: "tip".to_string(),
                content: "Use cargo check before cargo build".to_string(),
                tags: vec!["rust".to_string()],
            }],
            shared_commands: vec![],
            shared_mcp: vec![],
        };
        let ctx = cfg.context_string();
        assert!(ctx.contains("Acme"));
        assert!(ctx.contains("cargo check"));
    }

    // ── context_string edge cases ──────────────────────────────────────────

    #[test]
    fn context_string_empty_when_no_knowledge_or_commands() {
        let cfg = TeamConfig::default();
        assert!(cfg.context_string().is_empty());
    }

    #[test]
    fn context_string_includes_shared_commands() {
        let cfg = TeamConfig {
            team: TeamInfo { name: None, knowledge_base_url: None },
            knowledge: vec![],
            shared_commands: vec![SharedCommand {
                name: "deploy".to_string(),
                command: "npm run deploy".to_string(),
                description: "Deploy to prod".to_string(),
            }],
            shared_mcp: vec![],
        };
        let ctx = cfg.context_string();
        assert!(ctx.contains("Team Knowledge"));
        assert!(ctx.contains("npm run deploy"));
        assert!(ctx.contains("Deploy to prod"));
    }

    #[test]
    fn context_string_no_team_name_uses_default_header() {
        let cfg = TeamConfig {
            team: TeamInfo { name: None, knowledge_base_url: None },
            knowledge: vec![KnowledgeEntry {
                name: "k".into(), content: "v".into(), tags: vec![],
            }],
            shared_commands: vec![],
            shared_mcp: vec![],
        };
        let ctx = cfg.context_string();
        assert!(ctx.contains("Team Knowledge"));
    }

    #[test]
    fn context_string_knowledge_tags_shown() {
        let cfg = TeamConfig {
            team: TeamInfo { name: Some("T".into()), knowledge_base_url: None },
            knowledge: vec![KnowledgeEntry {
                name: "tip".into(),
                content: "do it".into(),
                tags: vec!["ops".into(), "dev".into()],
            }],
            shared_commands: vec![],
            shared_mcp: vec![],
        };
        let ctx = cfg.context_string();
        assert!(ctx.contains("[ops, dev]"));
    }

    #[test]
    fn context_string_knowledge_no_tags() {
        let cfg = TeamConfig {
            team: TeamInfo { name: Some("T".into()), knowledge_base_url: None },
            knowledge: vec![KnowledgeEntry {
                name: "tip".into(),
                content: "do it".into(),
                tags: vec![],
            }],
            shared_commands: vec![],
            shared_mcp: vec![],
        };
        let ctx = cfg.context_string();
        // No brackets when no tags
        assert!(!ctx.contains("["));
    }

    // ── TeamConfig serde ───────────────────────────────────────────────────

    #[test]
    fn team_config_serde_roundtrip() {
        let cfg = TeamConfig {
            team: TeamInfo { name: Some("Acme".into()), knowledge_base_url: None },
            knowledge: vec![KnowledgeEntry {
                name: "k".into(), content: "v".into(), tags: vec!["t".into()],
            }],
            shared_commands: vec![SharedCommand {
                name: "c".into(), command: "cmd".into(), description: "d".into(),
            }],
            shared_mcp: vec![SharedMcp {
                name: "m".into(), command: "mcmd".into(), args: vec!["--arg".into()],
            }],
        };
        let toml_str = toml::to_string(&cfg).unwrap();
        let back: TeamConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(back.team.name, Some("Acme".into()));
        assert_eq!(back.knowledge.len(), 1);
        assert_eq!(back.shared_commands.len(), 1);
        assert_eq!(back.shared_mcp.len(), 1);
    }

    // ── TeamManager save/load ──────────────────────────────────────────────

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let mgr = TeamManager::for_workspace(tmp.path());
        let cfg = TeamConfig {
            team: TeamInfo { name: Some("Test Team".into()), knowledge_base_url: None },
            knowledge: vec![],
            shared_commands: vec![],
            shared_mcp: vec![],
        };
        mgr.save(&cfg).unwrap();
        let loaded = mgr.load();
        assert_eq!(loaded.team.name, Some("Test Team".into()));
    }

    // ── add_knowledge deduplicates by name ─────────────────────────────────

    #[test]
    fn add_knowledge_dedup_by_name() {
        let tmp = TempDir::new().unwrap();
        let mgr = TeamManager::for_workspace(tmp.path());
        mgr.add_knowledge("deploy", "version 1", vec![]).unwrap();
        mgr.add_knowledge("deploy", "version 2", vec![]).unwrap();
        let cfg = mgr.load();
        assert_eq!(cfg.knowledge.len(), 1);
        assert!(cfg.knowledge[0].content.contains("version 2"));
    }

    // ── remove_knowledge returns false for missing ─────────────────────────

    #[test]
    fn remove_knowledge_nonexistent_returns_false() {
        let tmp = TempDir::new().unwrap();
        let mgr = TeamManager::for_workspace(tmp.path());
        let removed = mgr.remove_knowledge("nonexistent").unwrap();
        assert!(!removed);
    }
}
