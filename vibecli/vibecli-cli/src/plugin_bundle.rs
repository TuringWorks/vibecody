/*!
 * plugin_bundle.rs — `.vibepkg` plugin bundle format.
 *
 * Manifest validation, install, uninstall, and list for plugin bundles.
 */

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// BundleVersion
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BundleVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl BundleVersion {
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major = parts[0].parse::<u32>().ok()?;
        let minor = parts[1].parse::<u32>().ok()?;
        let patch = parts[2].parse::<u32>().ok()?;
        Some(Self { major, minor, patch })
    }

    /// Returns true if self >= min (i.e., self is compatible with the minimum requirement).
    pub fn is_compatible_with(&self, min: &BundleVersion) -> bool {
        self >= min
    }
}

impl fmt::Display for BundleVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ---------------------------------------------------------------------------
// BundleManifest
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    pub name: String,
    pub version: BundleVersion,
    pub author: String,
    pub description: String,
    pub skills: Vec<String>,
    pub mcp_configs: Vec<String>,
    pub min_vibecli_version: Option<String>,
}

impl BundleManifest {
    pub fn validate(&self) -> Result<(), BundleError> {
        if self.name.is_empty() {
            return Err(BundleError::InvalidManifest("name must not be empty".into()));
        }
        if self.author.is_empty() {
            return Err(BundleError::InvalidManifest("author must not be empty".into()));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// InstalledBundle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InstalledBundle {
    pub manifest: BundleManifest,
    pub install_path: String,
    pub installed_at: u64,
}

// ---------------------------------------------------------------------------
// BundleError
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum BundleError {
    InvalidManifest(String),
    AlreadyInstalled,
    NotFound,
    IoError(String),
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BundleError::InvalidManifest(msg) => write!(f, "Invalid manifest: {}", msg),
            BundleError::AlreadyInstalled => write!(f, "Bundle already installed"),
            BundleError::NotFound => write!(f, "Bundle not found"),
            BundleError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

// ---------------------------------------------------------------------------
// BundleRegistry
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct BundleRegistry {
    pub installed: Vec<InstalledBundle>,
}

impl BundleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn install(
        &mut self,
        manifest: BundleManifest,
        install_path: impl Into<String>,
        now_secs: u64,
    ) -> Result<(), BundleError> {
        manifest.validate()?;
        if self.find(&manifest.name).is_some() {
            return Err(BundleError::AlreadyInstalled);
        }
        self.installed.push(InstalledBundle {
            manifest,
            install_path: install_path.into(),
            installed_at: now_secs,
        });
        Ok(())
    }

    pub fn uninstall(&mut self, name: &str) -> Result<(), BundleError> {
        let len_before = self.installed.len();
        self.installed.retain(|b| b.manifest.name != name);
        if self.installed.len() < len_before {
            Ok(())
        } else {
            Err(BundleError::NotFound)
        }
    }

    pub fn list(&self) -> &[InstalledBundle] {
        &self.installed
    }

    pub fn find(&self, name: &str) -> Option<&InstalledBundle> {
        self.installed.iter().find(|b| b.manifest.name == name)
    }
}

// ---------------------------------------------------------------------------
// Legacy simple bundle API — retained for BDD harness compatibility
// ---------------------------------------------------------------------------

/// A single plugin entry in a simple dependency bundle.
#[derive(Debug, Clone)]
pub struct PluginMeta {
    pub id: String,
    pub version: String,
    pub requires: Vec<String>,
}

impl PluginMeta {
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self { id: id.into(), version: version.into(), requires: vec![] }
    }

    pub fn require(mut self, dep: impl Into<String>) -> Self {
        self.requires.push(dep.into());
        self
    }
}

/// Result of validating a simple bundle.
#[derive(Debug, Clone, Default)]
pub struct BundleReport {
    pub valid: bool,
    pub missing_deps: Vec<String>,
    pub duplicate_ids: Vec<String>,
}

/// A collection of plugins that can be validated together.
#[derive(Debug, Default)]
pub struct PluginBundle {
    pub plugins: Vec<PluginMeta>,
}

impl PluginBundle {
    pub fn new() -> Self { Self::default() }

    pub fn add(&mut self, plugin: PluginMeta) {
        self.plugins.push(plugin);
    }

    pub fn validate(&self) -> BundleReport {
        let ids: std::collections::HashSet<&str> =
            self.plugins.iter().map(|p| p.id.as_str()).collect();
        let mut seen = std::collections::HashSet::new();
        let mut duplicate_ids = vec![];
        for p in &self.plugins {
            if !seen.insert(p.id.as_str()) {
                duplicate_ids.push(p.id.clone());
            }
        }
        let mut missing_deps = vec![];
        for p in &self.plugins {
            for req in &p.requires {
                if !ids.contains(req.as_str()) {
                    missing_deps.push(format!("{} requires missing {}", p.id, req));
                }
            }
        }
        let valid = missing_deps.is_empty() && duplicate_ids.is_empty();
        BundleReport { valid, missing_deps, duplicate_ids }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest(name: &str) -> BundleManifest {
        BundleManifest {
            name: name.to_string(),
            version: BundleVersion::parse("1.0.0").unwrap(),
            author: "Alice".to_string(),
            description: "A test bundle".to_string(),
            skills: vec![],
            mcp_configs: vec![],
            min_vibecli_version: None,
        }
    }

    #[test]
    fn test_version_parse_valid() {
        let v = BundleVersion::parse("2.3.4").unwrap();
        assert_eq!(v.major, 2);
        assert_eq!(v.minor, 3);
        assert_eq!(v.patch, 4);
        assert_eq!(v.to_string(), "2.3.4");
    }

    #[test]
    fn test_version_parse_invalid_returns_none() {
        assert!(BundleVersion::parse("1.2").is_none());
        assert!(BundleVersion::parse("a.b.c").is_none());
        assert!(BundleVersion::parse("").is_none());
        assert!(BundleVersion::parse("1.2.3.4").is_none());
    }

    #[test]
    fn test_version_compatible() {
        let v = BundleVersion::parse("2.0.0").unwrap();
        let min = BundleVersion::parse("1.0.0").unwrap();
        assert!(v.is_compatible_with(&min));
        assert!(v.is_compatible_with(&v));
    }

    #[test]
    fn test_version_incompatible() {
        let v = BundleVersion::parse("0.9.9").unwrap();
        let min = BundleVersion::parse("1.0.0").unwrap();
        assert!(!v.is_compatible_with(&min));
    }

    #[test]
    fn test_manifest_validate_ok() {
        let m = sample_manifest("my-plugin");
        assert!(m.validate().is_ok());
    }

    #[test]
    fn test_manifest_validate_empty_name_fails() {
        let mut m = sample_manifest("my-plugin");
        m.name = "".to_string();
        let err = m.validate().unwrap_err();
        assert!(matches!(err, BundleError::InvalidManifest(_)));
    }

    #[test]
    fn test_registry_install_and_find() {
        let mut reg = BundleRegistry::new();
        reg.install(sample_manifest("plugin-a"), "/plugins/a", 1000).unwrap();
        let found = reg.find("plugin-a").unwrap();
        assert_eq!(found.install_path, "/plugins/a");
        assert_eq!(found.installed_at, 1000);
    }

    #[test]
    fn test_registry_install_duplicate_fails() {
        let mut reg = BundleRegistry::new();
        reg.install(sample_manifest("dup"), "/plugins/dup", 1000).unwrap();
        let err = reg.install(sample_manifest("dup"), "/plugins/dup2", 2000).unwrap_err();
        assert_eq!(err, BundleError::AlreadyInstalled);
    }

    #[test]
    fn test_registry_uninstall() {
        let mut reg = BundleRegistry::new();
        reg.install(sample_manifest("removable"), "/plugins/r", 1000).unwrap();
        assert_eq!(reg.list().len(), 1);
        reg.uninstall("removable").unwrap();
        assert_eq!(reg.list().len(), 0);
        let err = reg.uninstall("removable").unwrap_err();
        assert_eq!(err, BundleError::NotFound);
    }
}
