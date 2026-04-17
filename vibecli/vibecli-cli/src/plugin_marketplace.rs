#![allow(dead_code)]
//! Plugin marketplace — discovery, metadata, and installation management for
//! WASM-based VibeUI extensions. Extends the existing `vibe-extensions` system.
//!
//! Features:
//! - `PluginManifest` with semver, capabilities, permissions
//! - `PluginRegistry` — in-memory catalogue browsable by category / keyword
//! - `InstallManager` — tracks installed plugins, detect updates
//! - One-click install: validate, download (stub), verify hash, register

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Semantic version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SemVer {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self { Self { major, minor, patch } }

    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.trim_start_matches('v').split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid semver: {}", s));
        }
        let parse = |p: &str| p.parse::<u32>().map_err(|_| format!("Invalid number: {}", p));
        Ok(Self::new(parse(parts[0])?, parse(parts[1])?, parse(parts[2])?))
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Plugin category for browsing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PluginCategory {
    LanguageSupport,
    Linting,
    Formatting,
    Debugging,
    Git,
    AI,
    Theme,
    Productivity,
    Testing,
    Other(String),
}

impl std::fmt::Display for PluginCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginCategory::LanguageSupport => write!(f, "language-support"),
            PluginCategory::Linting => write!(f, "linting"),
            PluginCategory::Formatting => write!(f, "formatting"),
            PluginCategory::Debugging => write!(f, "debugging"),
            PluginCategory::Git => write!(f, "git"),
            PluginCategory::AI => write!(f, "ai"),
            PluginCategory::Theme => write!(f, "theme"),
            PluginCategory::Productivity => write!(f, "productivity"),
            PluginCategory::Testing => write!(f, "testing"),
            PluginCategory::Other(s) => write!(f, "{}", s),
        }
    }
}

/// Permissions a plugin may request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    ReadFiles,
    WriteFiles,
    NetworkAccess,
    ProcessSpawn,
    ClipboardAccess,
    NotificationSend,
}

/// Full manifest for a plugin (what the registry stores).
#[derive(Debug, Clone)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: SemVer,
    pub description: String,
    pub author: String,
    pub category: PluginCategory,
    pub tags: Vec<String>,
    pub permissions: Vec<Permission>,
    pub wasm_url: String,
    pub sha256: String,
    pub downloads: u64,
    pub rating: f32,
    pub vibe_min_version: SemVer,
}

impl PluginManifest {
    pub fn rating_label(&self) -> &'static str {
        if self.rating >= 4.5 { "★★★★★" }
        else if self.rating >= 3.5 { "★★★★☆" }
        else if self.rating >= 2.5 { "★★★☆☆" }
        else { "★★☆☆☆" }
    }

    pub fn is_high_privilege(&self) -> bool {
        self.permissions.iter().any(|p| matches!(p, Permission::ProcessSpawn | Permission::NetworkAccess))
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// In-memory catalogue of available plugins.
pub struct PluginRegistry {
    plugins: HashMap<String, PluginManifest>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        let mut reg = Self::empty();
        reg.seed_demo_plugins();
        reg
    }
}

impl PluginRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn empty() -> Self { Self { plugins: HashMap::new() } }

    pub fn register(&mut self, manifest: PluginManifest) {
        self.plugins.insert(manifest.id.clone(), manifest);
    }

    pub fn get(&self, id: &str) -> Option<&PluginManifest> {
        self.plugins.get(id)
    }

    pub fn all(&self) -> Vec<&PluginManifest> {
        let mut v: Vec<&PluginManifest> = self.plugins.values().collect();
        v.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        v
    }

    pub fn by_category(&self, category: &PluginCategory) -> Vec<&PluginManifest> {
        self.all().into_iter()
            .filter(|p| &p.category == category)
            .collect()
    }

    pub fn search(&self, query: &str) -> Vec<&PluginManifest> {
        let q = query.to_lowercase();
        self.all().into_iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&q)
                || p.description.to_lowercase().contains(&q)
                || p.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    pub fn total_count(&self) -> usize { self.plugins.len() }

    fn seed_demo_plugins(&mut self) {
        self.register(PluginManifest {
            id: "vibe-rust-extras".into(),
            name: "Rust Extras".into(),
            version: SemVer::new(1, 2, 0),
            description: "Enhanced Rust code analysis, macro expansion previews, and clippy integration".into(),
            author: "VibeTeam".into(),
            category: PluginCategory::LanguageSupport,
            tags: vec!["rust".into(), "analysis".into(), "macros".into()],
            permissions: vec![Permission::ReadFiles],
            wasm_url: "https://plugins.vibecody.dev/rust-extras-1.2.0.wasm".into(),
            sha256: "abc123".into(),
            downloads: 142_000,
            rating: 4.8,
            vibe_min_version: SemVer::new(1, 0, 0),
        });

        self.register(PluginManifest {
            id: "vibe-prettier".into(),
            name: "Prettier Integration".into(),
            version: SemVer::new(3, 0, 1),
            description: "Format TypeScript, JavaScript, JSON, and CSS with Prettier".into(),
            author: "CommunityPlugins".into(),
            category: PluginCategory::Formatting,
            tags: vec!["prettier".into(), "format".into(), "typescript".into()],
            permissions: vec![Permission::ReadFiles, Permission::WriteFiles, Permission::ProcessSpawn],
            wasm_url: "https://plugins.vibecody.dev/prettier-3.0.1.wasm".into(),
            sha256: "def456".into(),
            downloads: 95_000,
            rating: 4.5,
            vibe_min_version: SemVer::new(1, 0, 0),
        });

        self.register(PluginManifest {
            id: "vibe-gitlens".into(),
            name: "GitLens".into(),
            version: SemVer::new(2, 1, 0),
            description: "Inline blame, commit history, and branch comparisons".into(),
            author: "VibeTeam".into(),
            category: PluginCategory::Git,
            tags: vec!["git".into(), "blame".into(), "history".into()],
            permissions: vec![Permission::ReadFiles, Permission::ProcessSpawn],
            wasm_url: "https://plugins.vibecody.dev/gitlens-2.1.0.wasm".into(),
            sha256: "ghi789".into(),
            downloads: 211_000,
            rating: 4.9,
            vibe_min_version: SemVer::new(1, 0, 0),
        });

        self.register(PluginManifest {
            id: "vibe-ai-docgen".into(),
            name: "AI Doc Generator".into(),
            version: SemVer::new(0, 8, 2),
            description: "Auto-generate documentation from code using AI".into(),
            author: "AICommunity".into(),
            category: PluginCategory::AI,
            tags: vec!["ai".into(), "docs".into(), "documentation".into()],
            permissions: vec![Permission::ReadFiles, Permission::NetworkAccess],
            wasm_url: "https://plugins.vibecody.dev/ai-docgen-0.8.2.wasm".into(),
            sha256: "jkl012".into(),
            downloads: 34_000,
            rating: 4.1,
            vibe_min_version: SemVer::new(1, 1, 0),
        });
    }
}

// ---------------------------------------------------------------------------
// Install Manager
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallStatus {
    Installed,
    Pending,
    Failed(String),
    UpdateAvailable,
}

#[derive(Debug, Clone)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub installed_at_ms: u64,
    pub status: InstallStatus,
    pub enabled: bool,
}

#[derive(Default)]
pub struct InstallManager {
    installed: HashMap<String, InstalledPlugin>,
}

impl InstallManager {
    pub fn new() -> Self { Self::default() }

    /// Simulate one-click install: validates, (stub) downloads, registers.
    pub fn install(&mut self, manifest: PluginManifest, clock_ms: u64) -> Result<(), String> {
        // Validate version requirement (stub: always satisfied)
        if manifest.id.is_empty() {
            return Err("Plugin ID cannot be empty".into());
        }
        if self.installed.contains_key(&manifest.id) {
            return Err(format!("Plugin `{}` is already installed", manifest.id));
        }

        // Stub: verify sha256 (in production, would hash the downloaded WASM)
        if manifest.sha256.is_empty() {
            return Err("Plugin has no checksum — refusing to install".into());
        }

        self.installed.insert(manifest.id.clone(), InstalledPlugin {
            manifest,
            installed_at_ms: clock_ms,
            status: InstallStatus::Installed,
            enabled: true,
        });

        Ok(())
    }

    pub fn uninstall(&mut self, id: &str) -> Result<(), String> {
        self.installed.remove(id)
            .ok_or_else(|| format!("Plugin `{}` is not installed", id))?;
        Ok(())
    }

    pub fn enable(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        let plugin = self.installed.get_mut(id)
            .ok_or_else(|| format!("Plugin `{}` not found", id))?;
        plugin.enabled = enabled;
        Ok(())
    }

    pub fn is_installed(&self, id: &str) -> bool { self.installed.contains_key(id) }

    pub fn list_installed(&self) -> Vec<&InstalledPlugin> {
        let mut v: Vec<&InstalledPlugin> = self.installed.values().collect();
        v.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
        v
    }

    /// Check for updates: returns plugins where `available` has higher version.
    pub fn check_updates(&mut self, registry: &PluginRegistry) -> Vec<String> {
        let mut outdated = Vec::new();
        for installed in self.installed.values_mut() {
            if let Some(latest) = registry.get(&installed.manifest.id) {
                if latest.version > installed.manifest.version {
                    installed.status = InstallStatus::UpdateAvailable;
                    outdated.push(installed.manifest.id.clone());
                }
            }
        }
        outdated
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_manifest(id: &str) -> PluginManifest {
        PluginManifest {
            id: id.to_string(),
            name: format!("Plugin {}", id),
            version: SemVer::new(1, 0, 0),
            description: "A test plugin".into(),
            author: "Test".into(),
            category: PluginCategory::Productivity,
            tags: vec!["test".into()],
            permissions: vec![Permission::ReadFiles],
            wasm_url: "https://example.com/plugin.wasm".into(),
            sha256: "abc123def456".into(),
            downloads: 1000,
            rating: 4.0,
            vibe_min_version: SemVer::new(1, 0, 0),
        }
    }

    #[test]
    fn test_semver_parse() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(v, SemVer::new(1, 2, 3));
    }

    #[test]
    fn test_semver_ordering() {
        assert!(SemVer::new(2, 0, 0) > SemVer::new(1, 9, 9));
        assert!(SemVer::new(1, 1, 0) > SemVer::new(1, 0, 9));
        assert!(SemVer::new(1, 0, 1) > SemVer::new(1, 0, 0));
    }

    #[test]
    fn test_semver_display() {
        assert_eq!(SemVer::new(3, 0, 1).to_string(), "3.0.1");
    }

    #[test]
    fn test_semver_parse_invalid() {
        assert!(SemVer::parse("not-semver").is_err());
    }

    #[test]
    fn test_registry_seed() {
        let reg = PluginRegistry::new();
        assert!(reg.total_count() >= 4);
    }

    #[test]
    fn test_registry_search() {
        let reg = PluginRegistry::new();
        let results = reg.search("git");
        assert!(results.iter().any(|p| p.id == "vibe-gitlens"));
    }

    #[test]
    fn test_registry_by_category() {
        let reg = PluginRegistry::new();
        let ai = reg.by_category(&PluginCategory::AI);
        assert!(!ai.is_empty());
        assert!(ai.iter().all(|p| p.category == PluginCategory::AI));
    }

    #[test]
    fn test_rating_label() {
        let mut m = demo_manifest("x");
        m.rating = 4.8;
        assert_eq!(m.rating_label(), "★★★★★");
        m.rating = 3.8;
        assert_eq!(m.rating_label(), "★★★★☆");
    }

    #[test]
    fn test_high_privilege_detection() {
        let mut m = demo_manifest("x");
        m.permissions = vec![Permission::ProcessSpawn];
        assert!(m.is_high_privilege());
        m.permissions = vec![Permission::ReadFiles];
        assert!(!m.is_high_privilege());
    }

    #[test]
    fn test_install_and_check() {
        let mut mgr = InstallManager::new();
        let m = demo_manifest("test-plugin");
        mgr.install(m, 0).unwrap();
        assert!(mgr.is_installed("test-plugin"));
    }

    #[test]
    fn test_install_duplicate_fails() {
        let mut mgr = InstallManager::new();
        mgr.install(demo_manifest("p"), 0).unwrap();
        assert!(mgr.install(demo_manifest("p"), 0).is_err());
    }

    #[test]
    fn test_uninstall() {
        let mut mgr = InstallManager::new();
        mgr.install(demo_manifest("p"), 0).unwrap();
        mgr.uninstall("p").unwrap();
        assert!(!mgr.is_installed("p"));
    }

    #[test]
    fn test_enable_disable() {
        let mut mgr = InstallManager::new();
        mgr.install(demo_manifest("p"), 0).unwrap();
        mgr.enable("p", false).unwrap();
        assert!(!mgr.installed["p"].enabled);
        mgr.enable("p", true).unwrap();
        assert!(mgr.installed["p"].enabled);
    }

    #[test]
    fn test_check_updates() {
        let mut mgr = InstallManager::new();
        let mut m = demo_manifest("vibe-gitlens");
        m.version = SemVer::new(1, 0, 0); // older than registry's 2.1.0
        mgr.install(m, 0).unwrap();

        let reg = PluginRegistry::new();
        let outdated = mgr.check_updates(&reg);
        assert!(outdated.contains(&"vibe-gitlens".to_string()));
    }

    #[test]
    fn test_install_empty_checksum_fails() {
        let mut mgr = InstallManager::new();
        let mut m = demo_manifest("p");
        m.sha256 = "".into();
        assert!(mgr.install(m, 0).is_err());
    }
}
