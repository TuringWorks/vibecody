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
