//! VibeCLI plugin system.
//!
//! Plugins live at `~/.vibecli/plugins/<name>/` and are distributable bundles
//! of skills, hooks, and custom REPL commands. Install from a local path or
//! a git repository URL.
//!
//! ## Plugin layout
//!
//! ```text
//! ~/.vibecli/plugins/
//! └── rust-safety/
//!     ├── plugin.toml          — manifest (required)
//!     ├── skills/              — .md files auto-activated in agent
//!     │   └── rust-safety.md
//!     ├── hooks/               — shell scripts fired on agent events
//!     │   ├── pre-tool.sh
//!     │   └── post-write.sh
//!     └── commands/            — scripts run as /plugin-name REPL commands
//!         └── lint.sh
//! ```
//!
//! ## Plugin manifest (`plugin.toml`)
//!
//! ```toml
//! name = "rust-safety"
//! version = "0.1.0"
//! description = "Enforce Rust safety rules in agent sessions"
//! author = "you"
//! vibecli_min_version = "0.1.0"   # optional
//!
//! [[hooks]]
//! event = "PostToolUse"
//! tools = ["write_file", "apply_patch"]
//! command = "hooks/post-write.sh"   # relative to plugin dir
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── Manifest ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub vibecli_min_version: Option<String>,
    /// Hook configurations bundled in this plugin.
    #[serde(default)]
    pub hooks: Vec<PluginHookConfig>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

/// A hook entry inside `plugin.toml` (mirrors `HookConfig` but path is relative).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHookConfig {
    pub event: String,
    #[serde(default)]
    pub tools: Vec<String>,
    /// Path to the hook script, relative to the plugin directory.
    pub command: String,
    #[serde(default)]
    pub r#async: bool,
}

// ── Plugin ────────────────────────────────────────────────────────────────────

/// A loaded plugin with resolved paths.
#[derive(Debug, Clone)]
pub struct Plugin {
    pub manifest: PluginManifest,
    /// Directory where the plugin is installed.
    pub dir: PathBuf,
}

impl Plugin {
    /// Path to the `skills/` sub-directory.
    pub fn skills_dir(&self) -> PathBuf {
        self.dir.join("skills")
    }

    /// Path to the `commands/` sub-directory.
    pub fn commands_dir(&self) -> PathBuf {
        self.dir.join("commands")
    }

    /// Resolved hook entries (command paths made absolute).
    pub fn resolved_hooks(&self) -> Vec<(String, Vec<String>, PathBuf, bool)> {
        self.manifest
            .hooks
            .iter()
            .map(|h| {
                let abs_cmd = self.dir.join(&h.command);
                (h.event.clone(), h.tools.clone(), abs_cmd, h.r#async)
            })
            .collect()
    }
}

// ── PluginLoader ──────────────────────────────────────────────────────────────

/// Manages the plugin directory and provides access to installed plugins.
pub struct PluginLoader {
    pub plugins_dir: PathBuf,
}

impl PluginLoader {
    /// Create a loader rooted at `~/.vibecli/plugins/`.
    pub fn new() -> Self {
        let dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vibecli")
            .join("plugins");
        Self { plugins_dir: dir }
    }

    /// Load all valid plugins from the plugins directory.
    pub fn load_all(&self) -> Vec<Plugin> {
        let Ok(entries) = std::fs::read_dir(&self.plugins_dir) else {
            return vec![];
        };

        entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .filter_map(|e| self.load_plugin(&e.path()).ok())
            .collect()
    }

    /// Load a single plugin from a directory.
    pub fn load_plugin(&self, dir: &Path) -> Result<Plugin> {
        let manifest_path = dir.join("plugin.toml");
        let toml_str = std::fs::read_to_string(&manifest_path)
            .with_context(|| format!("Cannot read plugin manifest: {}", manifest_path.display()))?;
        let manifest: PluginManifest = toml::from_str(&toml_str)
            .with_context(|| format!("Invalid plugin.toml in {}", dir.display()))?;
        Ok(Plugin { manifest, dir: dir.to_path_buf() })
    }

    // ── Install / Remove ──────────────────────────────────────────────────────

    /// Install a plugin from a local path.
    pub fn install_from_path(&self, src: &Path) -> Result<Plugin> {
        // Determine the plugin name from the source directory name.
        let name = src
            .file_name()
            .and_then(|n| n.to_str())
            .context("Cannot determine plugin name from path")?;

        let dest = self.plugins_dir.join(name);
        if dest.exists() {
            anyhow::bail!(
                "Plugin '{}' is already installed at {}. Use `plugin remove {}` first.",
                name, dest.display(), name
            );
        }

        // Validate the source has a plugin.toml
        let manifest_path = src.join("plugin.toml");
        if !manifest_path.exists() {
            anyhow::bail!("No plugin.toml found in {}", src.display());
        }

        std::fs::create_dir_all(&self.plugins_dir)?;
        copy_dir_all(src, &dest)
            .with_context(|| format!("Failed to copy plugin from {}", src.display()))?;

        self.load_plugin(&dest)
    }

    /// Install a plugin from a git repository URL.
    pub fn install_from_git(&self, url: &str) -> Result<Plugin> {
        // Derive a local name from the URL (last path component, strip .git)
        let name = url
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("plugin")
            .trim_end_matches(".git");

        let dest = self.plugins_dir.join(name);
        if dest.exists() {
            anyhow::bail!(
                "Plugin '{}' is already installed. Use `plugin remove {}` first.",
                name, name
            );
        }

        std::fs::create_dir_all(&self.plugins_dir)?;
        let status = std::process::Command::new("git")
            .args(["clone", "--depth", "1", url, &dest.to_string_lossy()])
            .status()
            .context("git clone failed (is git installed?)")?;

        if !status.success() {
            if dest.exists() {
                let _ = std::fs::remove_dir_all(&dest);
            }
            anyhow::bail!("git clone exited with status: {}", status);
        }

        self.load_plugin(&dest)
    }

    /// Remove an installed plugin by name.
    pub fn remove(&self, name: &str) -> Result<()> {
        let dir = self.plugins_dir.join(name);
        if !dir.exists() {
            anyhow::bail!("Plugin '{}' is not installed.", name);
        }
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("Failed to remove plugin directory {}", dir.display()))?;
        Ok(())
    }

    /// List all installed plugin names and their descriptions.
    pub fn list(&self) -> Vec<(String, String, String)> {
        self.load_all()
            .into_iter()
            .map(|p| (p.manifest.name.clone(), p.manifest.version.clone(), p.manifest.description.clone()))
            .collect()
    }

    // ── Skill aggregation ─────────────────────────────────────────────────────

    /// Collect all skill file paths from all installed plugins.
    pub fn all_skill_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        for plugin in self.load_all() {
            let skills_dir = plugin.skills_dir();
            if let Ok(entries) = std::fs::read_dir(&skills_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("md") {
                        paths.push(path);
                    }
                }
            }
        }
        paths
    }
}

// ── Filesystem helper ─────────────────────────────────────────────────────────

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_plugin(base: &Path, name: &str) -> PathBuf {
        let dir = base.join(name);
        fs::create_dir_all(dir.join("skills")).unwrap();
        fs::create_dir_all(dir.join("hooks")).unwrap();
        fs::create_dir_all(dir.join("commands")).unwrap();
        let manifest = format!(
            "name = \"{}\"\nversion = \"1.0.0\"\ndescription = \"Test plugin\"\n",
            name
        );
        fs::write(dir.join("plugin.toml"), manifest).unwrap();
        fs::write(
            dir.join("skills").join("skill.md"),
            "---\nname: skill\ndescription: test\ntriggers: [\"test\"]\n---\nDo stuff.",
        )
        .unwrap();
        dir
    }

    #[test]
    fn test_load_plugin() {
        let tmp = tempfile::tempdir().unwrap();
        let plugin_dir = make_plugin(tmp.path(), "myplugin");
        let loader = PluginLoader { plugins_dir: tmp.path().to_path_buf() };
        let plugin = loader.load_plugin(&plugin_dir).unwrap();
        assert_eq!(plugin.manifest.name, "myplugin");
        assert_eq!(plugin.manifest.version, "1.0.0");
    }

    #[test]
    fn test_install_and_remove() {
        let src_tmp = tempfile::tempdir().unwrap();
        let dst_tmp = tempfile::tempdir().unwrap();

        let src_dir = make_plugin(src_tmp.path(), "coolplugin");
        let loader = PluginLoader { plugins_dir: dst_tmp.path().to_path_buf() };

        let installed = loader.install_from_path(&src_dir).unwrap();
        assert_eq!(installed.manifest.name, "coolplugin");

        let list = loader.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0, "coolplugin");

        loader.remove("coolplugin").unwrap();
        assert!(loader.list().is_empty());
    }

    #[test]
    fn test_install_duplicate_fails() {
        let src_tmp = tempfile::tempdir().unwrap();
        let dst_tmp = tempfile::tempdir().unwrap();
        let src_dir = make_plugin(src_tmp.path(), "dupe");
        let loader = PluginLoader { plugins_dir: dst_tmp.path().to_path_buf() };
        loader.install_from_path(&src_dir).unwrap();
        assert!(loader.install_from_path(&src_dir).is_err());
    }

    #[test]
    fn test_skill_paths() {
        let tmp = tempfile::tempdir().unwrap();
        make_plugin(tmp.path(), "p1");
        make_plugin(tmp.path(), "p2");
        let loader = PluginLoader { plugins_dir: tmp.path().to_path_buf() };
        let skills = loader.all_skill_paths();
        assert_eq!(skills.len(), 2);
    }
}
