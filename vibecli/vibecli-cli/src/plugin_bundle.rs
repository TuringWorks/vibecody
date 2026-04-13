//! plugin_bundle — Manifest-driven plugin packaging and validation.
//! Plugins declare capabilities and dependencies; the bundle validator
//! checks for missing deps, version conflicts, and duplicate IDs.

/// A single plugin entry in the bundle.
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

/// Result of validating a bundle.
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

    /// Validate the bundle: check for duplicates and unsatisfied deps.
    pub fn validate(&self) -> BundleReport {
        let ids: std::collections::HashSet<&str> =
            self.plugins.iter().map(|p| p.id.as_str()).collect();

        // Detect duplicate IDs
        let mut seen = std::collections::HashSet::new();
        let mut duplicate_ids = vec![];
        for p in &self.plugins {
            if !seen.insert(p.id.as_str()) {
                duplicate_ids.push(p.id.clone());
            }
        }

        // Detect missing deps
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_bundle_is_valid() {
        let b = PluginBundle::new();
        let r = b.validate();
        assert!(r.valid);
        assert!(r.missing_deps.is_empty());
        assert!(r.duplicate_ids.is_empty());
    }

    #[test]
    fn test_single_plugin_no_deps_valid() {
        let mut b = PluginBundle::new();
        b.add(PluginMeta::new("core", "1.0"));
        assert!(b.validate().valid);
    }

    #[test]
    fn test_satisfied_dependency_valid() {
        let mut b = PluginBundle::new();
        b.add(PluginMeta::new("core", "1.0"));
        b.add(PluginMeta::new("ext", "1.0").require("core"));
        assert!(b.validate().valid);
    }

    #[test]
    fn test_missing_dependency_invalid() {
        let mut b = PluginBundle::new();
        b.add(PluginMeta::new("ext", "1.0").require("missing-dep"));
        let r = b.validate();
        assert!(!r.valid);
        assert_eq!(r.missing_deps.len(), 1);
        assert!(r.missing_deps[0].contains("missing-dep"));
    }

    #[test]
    fn test_duplicate_id_invalid() {
        let mut b = PluginBundle::new();
        b.add(PluginMeta::new("core", "1.0"));
        b.add(PluginMeta::new("core", "2.0"));
        let r = b.validate();
        assert!(!r.valid);
        assert!(r.duplicate_ids.contains(&"core".to_string()));
    }

    #[test]
    fn test_multiple_missing_deps() {
        let mut b = PluginBundle::new();
        b.add(PluginMeta::new("p", "1.0").require("a").require("b"));
        let r = b.validate();
        assert_eq!(r.missing_deps.len(), 2);
    }

    #[test]
    fn test_plugin_meta_require_chaining() {
        let p = PluginMeta::new("p", "1.0").require("a").require("b");
        assert_eq!(p.requires.len(), 2);
    }

    #[test]
    fn test_both_duplicate_and_missing() {
        let mut b = PluginBundle::new();
        b.add(PluginMeta::new("x", "1.0").require("missing"));
        b.add(PluginMeta::new("x", "2.0"));
        let r = b.validate();
        assert!(!r.valid);
        assert!(!r.missing_deps.is_empty());
        assert!(!r.duplicate_ids.is_empty());
    }
}
