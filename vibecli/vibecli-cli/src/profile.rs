//! Named configuration profiles.
//!
//! Profiles live at `~/.vibecli/profiles/<name>.toml`. Each profile overrides
//! a subset of the base config — only the fields present in the profile file
//! are applied; everything else falls back to `~/.vibecli/config.toml`.
//!
//! ## Example profile  (`~/.vibecli/profiles/work.toml`)
//!
//! ```toml
//! # Profile name is derived from the filename.
//! description = "Work profile — Claude with auto-edit"
//!
//! [provider]
//! type = "claude"
//! model = "claude-opus-4-6"
//!
//! [safety]
//! approval_policy = "auto-edit"
//! sandbox = true
//! ```
//!
//! ## Usage
//!
//! ```bash
//! vibecli --profile work
//! vibecli /profile list
//! vibecli /profile show work
//! vibecli /profile switch personal
//! vibecli /profile create myprofile
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── Profile manifest ──────────────────────────────────────────────────────────

/// Partial config overrides stored in a profile file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    /// Human-readable description shown in `profile list`.
    #[serde(default)]
    pub description: String,

    /// Provider overrides (`[provider]` section).
    pub provider: Option<ProfileProvider>,

    /// Safety overrides (`[safety]` section).
    pub safety: Option<ProfileSafety>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileProvider {
    /// Provider type: "ollama", "claude", "openai", "gemini", "grok".
    #[serde(rename = "type")]
    pub provider_type: Option<String>,
    /// Model name (overrides config default).
    pub model: Option<String>,
    /// API URL (for Ollama host override).
    pub api_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSafety {
    /// "suggest" | "auto-edit" | "full-auto"
    pub approval_policy: Option<String>,
    /// Wrap tool calls in OS sandbox.
    pub sandbox: Option<bool>,
}

// ── ProfileManager ─────────────────────────────────────────────────────────────

/// Manages the profiles directory at `~/.vibecli/profiles/`.
pub struct ProfileManager {
    pub profiles_dir: PathBuf,
}

impl ProfileManager {
    /// Create a manager rooted at `~/.vibecli/profiles/`.
    pub fn new() -> Self {
        let dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vibecli")
            .join("profiles");
        Self { profiles_dir: dir }
    }

    /// Load a specific profile by name.
    pub fn load(&self, name: &str) -> Result<Profile> {
        let path = self.profiles_dir.join(format!("{}.toml", name));
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Profile '{}' not found at {}", name, path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Invalid profile TOML in {}", path.display()))
    }

    /// List all installed profile names and their descriptions.
    pub fn list(&self) -> Vec<(String, String)> {
        let Ok(entries) = std::fs::read_dir(&self.profiles_dir) else {
            return vec![];
        };
        let mut profiles: Vec<(String, String)> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let path = e.path();
                if path.extension().and_then(|x| x.to_str()) == Some("toml") {
                    let name = path.file_stem()?.to_str()?.to_string();
                    let desc = std::fs::read_to_string(&path)
                        .ok()
                        .and_then(|s| toml::from_str::<Profile>(&s).ok())
                        .map(|p| p.description)
                        .unwrap_or_default();
                    Some((name, desc))
                } else {
                    None
                }
            })
            .collect();
        profiles.sort_by(|a, b| a.0.cmp(&b.0));
        profiles
    }

    /// Create a new profile with the given name and an example template.
    pub fn create(&self, name: &str, provider: &str, approval: &str) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.profiles_dir)?;
        let path = self.profiles_dir.join(format!("{}.toml", name));
        if path.exists() {
            anyhow::bail!("Profile '{}' already exists at {}", name, path.display());
        }
        let content = format!(
            r#"# VibeCLI profile: {}
description = "{} — {} ({})"

[provider]
type = "{}"

[safety]
approval_policy = "{}"
sandbox = false
"#,
            name, name, provider, approval, provider, approval
        );
        std::fs::write(&path, content)?;
        Ok(path)
    }

    /// Delete a profile.
    pub fn delete(&self, name: &str) -> Result<()> {
        let path = self.profiles_dir.join(format!("{}.toml", name));
        if !path.exists() {
            anyhow::bail!("Profile '{}' does not exist.", name);
        }
        std::fs::remove_file(&path)?;
        Ok(())
    }

    /// Read/write the "active profile" name from `~/.vibecli/active_profile`.
    pub fn active_profile_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".vibecli").join("active_profile"))
    }

    pub fn read_active() -> Option<String> {
        Self::active_profile_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn write_active(name: &str) -> Result<()> {
        let path = Self::active_profile_path()
            .context("Cannot determine home directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, name)?;
        Ok(())
    }
}

// ── Apply profile to CLI args ─────────────────────────────────────────────────

/// Values extracted from a profile that the CLI needs.
#[derive(Debug, Default)]
pub struct ProfileOverrides {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub approval_policy: Option<String>,
    pub sandbox: Option<bool>,
}

impl ProfileOverrides {
    /// Load a profile by name and return its overrides.
    pub fn load(profile_name: &str) -> Result<Self> {
        let mgr = ProfileManager::new();
        let profile = mgr.load(profile_name)?;
        Ok(Self {
            provider: profile.provider.as_ref().and_then(|p| p.provider_type.clone()),
            model: profile.provider.as_ref().and_then(|p| p.model.clone()),
            approval_policy: profile.safety.as_ref().and_then(|s| s.approval_policy.clone()),
            sandbox: profile.safety.as_ref().and_then(|s| s.sandbox),
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_load_profile() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ProfileManager { profiles_dir: tmp.path().to_path_buf() };

        let path = mgr.create("work", "claude", "auto-edit").unwrap();
        assert!(path.exists());

        let profile = mgr.load("work").unwrap();
        let provider = profile.provider.unwrap();
        assert_eq!(provider.provider_type.as_deref(), Some("claude"));
        let safety = profile.safety.unwrap();
        assert_eq!(safety.approval_policy.as_deref(), Some("auto-edit"));
    }

    #[test]
    fn test_list_profiles() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ProfileManager { profiles_dir: tmp.path().to_path_buf() };
        mgr.create("alpha", "ollama", "suggest").unwrap();
        mgr.create("beta", "openai", "full-auto").unwrap();

        let list = mgr.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].0, "alpha");
        assert_eq!(list[1].0, "beta");
    }

    #[test]
    fn test_delete_profile() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ProfileManager { profiles_dir: tmp.path().to_path_buf() };
        mgr.create("test", "ollama", "suggest").unwrap();
        assert_eq!(mgr.list().len(), 1);
        mgr.delete("test").unwrap();
        assert!(mgr.list().is_empty());
    }

    #[test]
    fn test_duplicate_create_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ProfileManager { profiles_dir: tmp.path().to_path_buf() };
        mgr.create("p", "ollama", "suggest").unwrap();
        assert!(mgr.create("p", "claude", "full-auto").is_err());
    }

    #[test]
    fn test_load_nonexistent_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = ProfileManager { profiles_dir: tmp.path().to_path_buf() };
        assert!(mgr.load("nope").is_err());
    }
}
