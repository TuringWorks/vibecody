//! Plugin Lifecycle Manager — install, update, uninstall, enable, disable, dev mode.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Plugin installation state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginState {
    Installed,
    Enabled,
    Disabled,
    Outdated,    // newer version available
    DevMode,     // linked locally for development
    Errored(String),
}

/// Installed plugin record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub name: String,
    pub version: String,
    pub state: PluginState,
    pub install_dir: PathBuf,
    pub installed_at: String,
    pub updated_at: Option<String>,
    pub config: HashMap<String, String>,
    pub checksum: Option<String>,
}

/// Plugin state store (persisted as JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStateStore {
    pub plugins: Vec<InstalledPlugin>,
    pub version: String,
}

impl PluginStateStore {
    pub fn new() -> Self {
        Self { plugins: vec![], version: "1.0.0".to_string() }
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if path.exists() {
            let data = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&data)?)
        } else {
            Ok(Self::new())
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }

    pub fn find(&self, name: &str) -> Option<&InstalledPlugin> {
        self.plugins.iter().find(|p| p.name == name)
    }

    pub fn find_mut(&mut self, name: &str) -> Option<&mut InstalledPlugin> {
        self.plugins.iter_mut().find(|p| p.name == name)
    }

    pub fn is_installed(&self, name: &str) -> bool {
        self.plugins.iter().any(|p| p.name == name)
    }

    pub fn enabled_plugins(&self) -> Vec<&InstalledPlugin> {
        self.plugins.iter()
            .filter(|p| matches!(p.state, PluginState::Enabled | PluginState::DevMode))
            .collect()
    }
}

/// Plugin Lifecycle Manager
pub struct PluginLifecycle {
    pub plugins_dir: PathBuf,
    pub state_path: PathBuf,
    pub store: PluginStateStore,
}

impl PluginLifecycle {
    pub fn new() -> anyhow::Result<Self> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let plugins_dir = home.join(".vibecli").join("plugins");
        let state_path = home.join(".vibecli").join("plugin-state.json");

        std::fs::create_dir_all(&plugins_dir)?;
        let store = PluginStateStore::load(&state_path)?;

        Ok(Self { plugins_dir, state_path, store })
    }

    #[cfg(test)]
    pub fn with_dir(dir: PathBuf) -> anyhow::Result<Self> {
        let plugins_dir = dir.join("plugins");
        let state_path = dir.join("plugin-state.json");
        std::fs::create_dir_all(&plugins_dir)?;
        let store = PluginStateStore::load(&state_path)?;
        Ok(Self { plugins_dir, state_path, store })
    }

    /// Install a plugin from a git repository URL
    pub fn install_from_repo(&mut self, name: &str, repo_url: &str) -> anyhow::Result<InstalledPlugin> {
        if self.store.is_installed(name) {
            anyhow::bail!("Plugin '{}' is already installed. Use 'update' instead.", name);
        }

        let plugin_dir = self.plugins_dir.join(name);
        if plugin_dir.exists() {
            std::fs::remove_dir_all(&plugin_dir)?;
        }

        // Clone the repository
        let status = std::process::Command::new("git")
            .args(["clone", "--depth", "1", repo_url, plugin_dir.to_str().unwrap_or("")])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .status()?;

        if !status.success() {
            anyhow::bail!("Failed to clone plugin repository: {}", repo_url);
        }

        // Verify plugin.toml exists
        if !plugin_dir.join("plugin.toml").exists() {
            std::fs::remove_dir_all(&plugin_dir)?;
            anyhow::bail!("Repository does not contain a valid plugin.toml manifest");
        }

        // Read version from manifest
        let manifest_str = std::fs::read_to_string(plugin_dir.join("plugin.toml"))?;
        let version = Self::extract_version(&manifest_str);

        let plugin = InstalledPlugin {
            name: name.to_string(),
            version,
            state: PluginState::Enabled,
            install_dir: plugin_dir,
            installed_at: chrono_now(),
            updated_at: None,
            config: HashMap::new(),
            checksum: None,
        };

        self.store.plugins.push(plugin.clone());
        self.store.save(&self.state_path)?;

        Ok(plugin)
    }

    /// Install from a local directory (dev mode / symlink)
    pub fn install_dev(&mut self, name: &str, source_dir: &Path) -> anyhow::Result<InstalledPlugin> {
        if !source_dir.join("plugin.toml").exists() {
            anyhow::bail!("Directory does not contain a plugin.toml manifest");
        }

        let plugin_dir = self.plugins_dir.join(name);

        // Create symlink for dev mode
        #[cfg(unix)]
        std::os::unix::fs::symlink(source_dir, &plugin_dir)?;
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(source_dir, &plugin_dir)?;

        let manifest_str = std::fs::read_to_string(source_dir.join("plugin.toml"))?;
        let version = Self::extract_version(&manifest_str);

        let plugin = InstalledPlugin {
            name: name.to_string(),
            version,
            state: PluginState::DevMode,
            install_dir: plugin_dir,
            installed_at: chrono_now(),
            updated_at: None,
            config: HashMap::new(),
            checksum: None,
        };

        // Remove existing if present
        self.store.plugins.retain(|p| p.name != name);
        self.store.plugins.push(plugin.clone());
        self.store.save(&self.state_path)?;

        Ok(plugin)
    }

    /// Uninstall a plugin
    pub fn uninstall(&mut self, name: &str) -> anyhow::Result<()> {
        if !self.store.is_installed(name) {
            anyhow::bail!("Plugin '{}' is not installed", name);
        }

        let plugin_dir = self.plugins_dir.join(name);
        if plugin_dir.exists() {
            // Handle both symlinks (dev mode) and real directories
            if plugin_dir.is_symlink() {
                std::fs::remove_file(&plugin_dir)?;
            } else {
                std::fs::remove_dir_all(&plugin_dir)?;
            }
        }

        self.store.plugins.retain(|p| p.name != name);
        self.store.save(&self.state_path)?;

        Ok(())
    }

    /// Enable a disabled plugin
    pub fn enable(&mut self, name: &str) -> anyhow::Result<()> {
        let plugin = self.store.find_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;

        match plugin.state {
            PluginState::Enabled | PluginState::DevMode => {
                anyhow::bail!("Plugin '{}' is already enabled", name);
            }
            _ => {
                plugin.state = PluginState::Enabled;
                self.store.save(&self.state_path)?;
                Ok(())
            }
        }
    }

    /// Disable a plugin without uninstalling
    pub fn disable(&mut self, name: &str) -> anyhow::Result<()> {
        let plugin = self.store.find_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;

        plugin.state = PluginState::Disabled;
        self.store.save(&self.state_path)?;
        Ok(())
    }

    /// Update a plugin to latest version
    pub fn update(&mut self, name: &str) -> anyhow::Result<String> {
        let plugin = self.store.find(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;

        if plugin.state == PluginState::DevMode {
            anyhow::bail!("Cannot update a dev-mode plugin. Use git pull in the source directory.");
        }

        let plugin_dir = self.plugins_dir.join(name);

        // Git pull to update
        let output = std::process::Command::new("git")
            .args(["pull", "--ff-only"])
            .current_dir(&plugin_dir)
            .output()?;

        if !output.status.success() {
            anyhow::bail!("Failed to update plugin: {}", String::from_utf8_lossy(&output.stderr));
        }

        // Re-read version
        let manifest_str = std::fs::read_to_string(plugin_dir.join("plugin.toml"))?;
        let new_version = Self::extract_version(&manifest_str);

        let plugin = self.store.find_mut(name).unwrap();
        let old_version = plugin.version.clone();
        plugin.version = new_version.clone();
        plugin.updated_at = Some(chrono_now());
        if plugin.state == PluginState::Outdated {
            plugin.state = PluginState::Enabled;
        }
        self.store.save(&self.state_path)?;

        Ok(format!("{} → {}", old_version, new_version))
    }

    /// Update all installed plugins
    pub fn update_all(&mut self) -> anyhow::Result<Vec<(String, String)>> {
        let names: Vec<String> = self.store.plugins.iter()
            .filter(|p| p.state != PluginState::DevMode)
            .map(|p| p.name.clone())
            .collect();

        let mut results = vec![];
        for name in names {
            match self.update(&name) {
                Ok(version_change) => results.push((name, version_change)),
                Err(e) => results.push((name, format!("error: {}", e))),
            }
        }
        Ok(results)
    }

    /// List all installed plugins
    pub fn list(&self) -> &[InstalledPlugin] {
        &self.store.plugins
    }

    /// Configure a plugin setting
    pub fn set_config(&mut self, name: &str, key: &str, value: &str) -> anyhow::Result<()> {
        let plugin = self.store.find_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;
        plugin.config.insert(key.to_string(), value.to_string());
        self.store.save(&self.state_path)?;
        Ok(())
    }

    /// Get a plugin's configuration
    pub fn get_config(&self, name: &str) -> anyhow::Result<&HashMap<String, String>> {
        let plugin = self.store.find(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;
        Ok(&plugin.config)
    }

    /// Get plugin info
    pub fn info(&self, name: &str) -> anyhow::Result<PluginInfo> {
        let plugin = self.store.find(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin '{}' not found", name))?;

        let manifest_path = plugin.install_dir.join("plugin.toml");
        let manifest_str = if manifest_path.exists() {
            std::fs::read_to_string(&manifest_path).ok()
        } else {
            None
        };

        let skills_count = plugin.install_dir.join("skills")
            .read_dir()
            .map(|d| d.filter_map(|e| e.ok()).filter(|e| {
                e.path().extension().map(|ext| ext == "md").unwrap_or(false)
            }).count())
            .unwrap_or(0);

        let hooks_count = plugin.install_dir.join("hooks")
            .read_dir()
            .map(|d| d.filter_map(|e| e.ok()).count())
            .unwrap_or(0);

        let commands_count = plugin.install_dir.join("commands")
            .read_dir()
            .map(|d| d.filter_map(|e| e.ok()).count())
            .unwrap_or(0);

        Ok(PluginInfo {
            plugin: plugin.clone(),
            manifest_raw: manifest_str,
            skills_count,
            hooks_count,
            commands_count,
        })
    }

    fn extract_version(toml_str: &str) -> String {
        // Simple TOML version extraction without full parse
        for line in toml_str.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("version") {
                if let Some(val) = trimmed.split('=').nth(1) {
                    return val.trim().trim_matches('"').trim_matches('\'').to_string();
                }
            }
        }
        "0.0.0".to_string()
    }
}

/// Detailed plugin information
#[derive(Debug)]
pub struct PluginInfo {
    pub plugin: InstalledPlugin,
    pub manifest_raw: Option<String>,
    pub skills_count: usize,
    pub hooks_count: usize,
    pub commands_count: usize,
}

/// Simple timestamp (avoids chrono dependency)
fn chrono_now() -> String {
    // Use system time formatted as ISO 8601
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple epoch-based timestamp — plugins can format as needed
    format!("{}", now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vibecli-plugin-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_plugin_state_store_new() {
        let store = PluginStateStore::new();
        assert!(store.plugins.is_empty());
        assert_eq!(store.version, "1.0.0");
    }

    #[test]
    fn test_plugin_state_store_save_load() {
        let dir = temp_dir();
        let path = dir.join("state.json");

        let mut store = PluginStateStore::new();
        store.plugins.push(InstalledPlugin {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            state: PluginState::Enabled,
            install_dir: dir.join("plugins/test-plugin"),
            installed_at: "12345".to_string(),
            updated_at: None,
            config: HashMap::new(),
            checksum: None,
        });
        store.save(&path).unwrap();

        let loaded = PluginStateStore::load(&path).unwrap();
        assert_eq!(loaded.plugins.len(), 1);
        assert_eq!(loaded.plugins[0].name, "test-plugin");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_plugin_state_store_find() {
        let mut store = PluginStateStore::new();
        store.plugins.push(InstalledPlugin {
            name: "alpha".to_string(),
            version: "1.0.0".to_string(),
            state: PluginState::Enabled,
            install_dir: PathBuf::from("/tmp/alpha"),
            installed_at: "0".to_string(),
            updated_at: None,
            config: HashMap::new(),
            checksum: None,
        });

        assert!(store.find("alpha").is_some());
        assert!(store.find("beta").is_none());
        assert!(store.is_installed("alpha"));
        assert!(!store.is_installed("beta"));
    }

    #[test]
    fn test_enabled_plugins_filter() {
        let mut store = PluginStateStore::new();
        for (name, state) in [
            ("a", PluginState::Enabled),
            ("b", PluginState::Disabled),
            ("c", PluginState::DevMode),
            ("d", PluginState::Errored("bad".into())),
        ] {
            store.plugins.push(InstalledPlugin {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                state,
                install_dir: PathBuf::from(format!("/tmp/{}", name)),
                installed_at: "0".to_string(),
                updated_at: None,
                config: HashMap::new(),
                checksum: None,
            });
        }

        let enabled = store.enabled_plugins();
        assert_eq!(enabled.len(), 2); // a (Enabled) and c (DevMode)
    }

    #[test]
    fn test_lifecycle_enable_disable() {
        let dir = temp_dir();
        let mut lc = PluginLifecycle::with_dir(dir.clone()).unwrap();

        // Add a plugin manually
        lc.store.plugins.push(InstalledPlugin {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            state: PluginState::Disabled,
            install_dir: dir.join("plugins/test"),
            installed_at: "0".to_string(),
            updated_at: None,
            config: HashMap::new(),
            checksum: None,
        });

        lc.enable("test").unwrap();
        assert_eq!(lc.store.find("test").unwrap().state, PluginState::Enabled);

        lc.disable("test").unwrap();
        assert_eq!(lc.store.find("test").unwrap().state, PluginState::Disabled);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_lifecycle_enable_already_enabled() {
        let dir = temp_dir();
        let mut lc = PluginLifecycle::with_dir(dir.clone()).unwrap();

        lc.store.plugins.push(InstalledPlugin {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            state: PluginState::Enabled,
            install_dir: dir.join("plugins/test"),
            installed_at: "0".to_string(),
            updated_at: None,
            config: HashMap::new(),
            checksum: None,
        });

        let result = lc.enable("test");
        assert!(result.is_err());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_lifecycle_uninstall_not_installed() {
        let dir = temp_dir();
        let mut lc = PluginLifecycle::with_dir(dir.clone()).unwrap();

        let result = lc.uninstall("nonexistent");
        assert!(result.is_err());

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_lifecycle_set_get_config() {
        let dir = temp_dir();
        let mut lc = PluginLifecycle::with_dir(dir.clone()).unwrap();

        lc.store.plugins.push(InstalledPlugin {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            state: PluginState::Enabled,
            install_dir: dir.join("plugins/test"),
            installed_at: "0".to_string(),
            updated_at: None,
            config: HashMap::new(),
            checksum: None,
        });

        lc.set_config("test", "api_key", "secret123").unwrap();
        let config = lc.get_config("test").unwrap();
        assert_eq!(config.get("api_key").unwrap(), "secret123");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extract_version() {
        assert_eq!(
            PluginLifecycle::extract_version("name = \"test\"\nversion = \"1.2.3\"\n"),
            "1.2.3"
        );
        assert_eq!(
            PluginLifecycle::extract_version("no version here"),
            "0.0.0"
        );
    }

    #[test]
    fn test_plugin_state_serialization() {
        let state = PluginState::Errored("failed to load".to_string());
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("errored"));
        let back: PluginState = serde_json::from_str(&json).unwrap();
        assert_eq!(back, state);
    }

    #[test]
    fn test_list_plugins() {
        let dir = temp_dir();
        let mut lc = PluginLifecycle::with_dir(dir.clone()).unwrap();

        assert!(lc.list().is_empty());

        lc.store.plugins.push(InstalledPlugin {
            name: "p1".to_string(),
            version: "1.0.0".to_string(),
            state: PluginState::Enabled,
            install_dir: dir.join("plugins/p1"),
            installed_at: "0".to_string(),
            updated_at: None,
            config: HashMap::new(),
            checksum: None,
        });

        assert_eq!(lc.list().len(), 1);

        fs::remove_dir_all(&dir).ok();
    }

    // ── Additional tests ──────────────────────────────────────────────────

    #[test]
    fn test_plugin_state_store_find_mut() {
        let mut store = PluginStateStore::new();
        store.plugins.push(InstalledPlugin {
            name: "mutable".to_string(),
            version: "1.0.0".to_string(),
            state: PluginState::Enabled,
            install_dir: PathBuf::from("/tmp/mutable"),
            installed_at: "0".to_string(),
            updated_at: None,
            config: HashMap::new(),
            checksum: None,
        });

        let p = store.find_mut("mutable").unwrap();
        p.version = "2.0.0".to_string();
        assert_eq!(store.find("mutable").unwrap().version, "2.0.0");
    }

    #[test]
    fn test_plugin_state_store_load_nonexistent() {
        let path = PathBuf::from("/tmp/nonexistent_vibecli_plugin_state.json");
        let store = PluginStateStore::load(&path).unwrap();
        assert!(store.plugins.is_empty());
        assert_eq!(store.version, "1.0.0");
    }

    #[test]
    fn test_extract_version_with_single_quotes() {
        assert_eq!(
            PluginLifecycle::extract_version("version = '2.5.1'\n"),
            "2.5.1"
        );
    }

    #[test]
    fn test_extract_version_with_spaces() {
        assert_eq!(
            PluginLifecycle::extract_version("version  =  \"3.0.0\"  \n"),
            "3.0.0"
        );
    }

    #[test]
    fn test_enable_nonexistent_plugin() {
        let dir = temp_dir();
        let mut lc = PluginLifecycle::with_dir(dir.clone()).unwrap();
        let result = lc.enable("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_disable_nonexistent_plugin() {
        let dir = temp_dir();
        let mut lc = PluginLifecycle::with_dir(dir.clone()).unwrap();
        let result = lc.disable("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_set_config_nonexistent_plugin() {
        let dir = temp_dir();
        let mut lc = PluginLifecycle::with_dir(dir.clone()).unwrap();
        let result = lc.set_config("nonexistent", "key", "value");
        assert!(result.is_err());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_get_config_nonexistent_plugin() {
        let dir = temp_dir();
        let lc = PluginLifecycle::with_dir(dir.clone()).unwrap();
        let result = lc.get_config("nonexistent");
        assert!(result.is_err());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_plugin_state_all_variants_serde() {
        let variants = vec![
            PluginState::Installed,
            PluginState::Enabled,
            PluginState::Disabled,
            PluginState::Outdated,
            PluginState::DevMode,
            PluginState::Errored("boom".into()),
        ];
        for state in variants {
            let json = serde_json::to_string(&state).unwrap();
            let back: PluginState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, state);
        }
    }

    #[test]
    fn test_installed_plugin_serde_roundtrip() {
        let plugin = InstalledPlugin {
            name: "serde-test".to_string(),
            version: "1.2.3".to_string(),
            state: PluginState::DevMode,
            install_dir: PathBuf::from("/tmp/serde-test"),
            installed_at: "12345".to_string(),
            updated_at: Some("67890".to_string()),
            config: HashMap::from([("key".to_string(), "val".to_string())]),
            checksum: Some("abc123".to_string()),
        };
        let json = serde_json::to_string(&plugin).unwrap();
        let back: InstalledPlugin = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "serde-test");
        assert_eq!(back.state, PluginState::DevMode);
        assert_eq!(back.updated_at, Some("67890".to_string()));
        assert_eq!(back.config.get("key").unwrap(), "val");
        assert_eq!(back.checksum, Some("abc123".to_string()));
    }

    #[test]
    fn test_enabled_plugins_empty_store() {
        let store = PluginStateStore::new();
        assert!(store.enabled_plugins().is_empty());
    }

    #[test]
    fn test_install_from_repo_already_installed() {
        let dir = temp_dir();
        let mut lc = PluginLifecycle::with_dir(dir.clone()).unwrap();
        lc.store.plugins.push(InstalledPlugin {
            name: "existing".to_string(),
            version: "1.0.0".to_string(),
            state: PluginState::Enabled,
            install_dir: dir.join("plugins/existing"),
            installed_at: "0".to_string(),
            updated_at: None,
            config: HashMap::new(),
            checksum: None,
        });
        let result = lc.install_from_repo("existing", "https://example.com/repo.git");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already installed"));
        fs::remove_dir_all(&dir).ok();
    }
}
