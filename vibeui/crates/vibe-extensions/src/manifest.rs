//! Extension manifest, permissions, and registry.
//!
//! Every WASM extension ships with a manifest (`extension.json`) that declares
//! metadata (name, version, author, description), required permissions, and a
//! minimum host version.  The [`ExtensionRegistry`] tracks loaded manifests and
//! provides add / remove / lookup operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ── Permission ───────────────────────────────────────────────────────────────

/// Capabilities an extension may request from the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    /// Read files from the workspace.
    FileRead,
    /// Write / create files in the workspace.
    FileWrite,
    /// Access network (HTTP, WebSocket).
    Network,
    /// Execute shell commands.
    ProcessExec,
    /// Read environment variables.
    EnvRead,
    /// Show notifications to the user.
    Notify,
    /// Access clipboard contents.
    Clipboard,
}

impl Permission {
    /// Returns all known permissions (useful for "grant all" policies).
    pub fn all() -> &'static [Permission] {
        &[
            Permission::FileRead,
            Permission::FileWrite,
            Permission::Network,
            Permission::ProcessExec,
            Permission::EnvRead,
            Permission::Notify,
            Permission::Clipboard,
        ]
    }

    /// Returns permissions considered "safe" and auto-granted by default.
    pub fn safe_defaults() -> &'static [Permission] {
        &[Permission::FileRead, Permission::Notify]
    }

    /// Returns `true` if this permission is considered dangerous.
    pub fn is_dangerous(&self) -> bool {
        matches!(
            self,
            Permission::FileWrite | Permission::Network | Permission::ProcessExec
        )
    }
}

// ── VersionReq ───────────────────────────────────────────────────────────────

/// A simple semver-style version with major.minor.patch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionReq {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl VersionReq {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Parse from a "major.minor.patch" string.
    pub fn parse(s: &str) -> Result<Self, ManifestError> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(ManifestError::InvalidVersion(s.to_string()));
        }
        let major = parts[0]
            .parse::<u32>()
            .map_err(|_| ManifestError::InvalidVersion(s.to_string()))?;
        let minor = parts[1]
            .parse::<u32>()
            .map_err(|_| ManifestError::InvalidVersion(s.to_string()))?;
        let patch = parts[2]
            .parse::<u32>()
            .map_err(|_| ManifestError::InvalidVersion(s.to_string()))?;
        Ok(Self { major, minor, patch })
    }

    /// Returns `true` if `self` is compatible with (i.e., satisfied by) `host`.
    /// Compatible means same major version and host >= self.
    pub fn is_compatible_with(&self, host: &VersionReq) -> bool {
        if self.major != host.major {
            return false;
        }
        if self.minor < host.minor {
            return true;
        }
        if self.minor > host.minor {
            return false;
        }
        self.patch <= host.patch
    }
}

impl std::fmt::Display for VersionReq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

// ── ManifestError ────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("manifest name must not be empty")]
    EmptyName,
    #[error("manifest name contains invalid characters: {0}")]
    InvalidName(String),
    #[error("invalid version string: {0}")]
    InvalidVersion(String),
    #[error("duplicate extension: {0}")]
    Duplicate(String),
    #[error("extension not found: {0}")]
    NotFound(String),
    #[error("permission denied: {0:?}")]
    PermissionDenied(Permission),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ── ExtensionManifest ────────────────────────────────────────────────────────

/// Metadata declared by an extension in its `extension.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    /// Unique extension identifier (e.g., `"my-linter"`).
    pub name: String,
    /// Semver version string.
    pub version: String,
    /// Human-readable display name.
    #[serde(default)]
    pub display_name: String,
    /// Author or publisher.
    #[serde(default)]
    pub author: String,
    /// Short description.
    #[serde(default)]
    pub description: String,
    /// Required host permissions.
    #[serde(default)]
    pub permissions: Vec<Permission>,
    /// Minimum host version required.
    #[serde(default)]
    pub min_host_version: Option<String>,
    /// Path to the WASM binary, relative to manifest.
    #[serde(default = "default_wasm_path")]
    pub wasm_path: String,
}

fn default_wasm_path() -> String {
    "extension.wasm".to_string()
}

impl ExtensionManifest {
    /// Validate the manifest, returning an error for the first invalid field.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.name.is_empty() {
            return Err(ManifestError::EmptyName);
        }
        // Name must be alphanumeric, hyphens, underscores only.
        if !self
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ManifestError::InvalidName(self.name.clone()));
        }
        // Version must parse.
        VersionReq::parse(&self.version)?;
        // min_host_version, if present, must parse.
        if let Some(ref v) = self.min_host_version {
            VersionReq::parse(v)?;
        }
        Ok(())
    }

    /// Parse a manifest from JSON.
    pub fn from_json(json: &str) -> Result<Self, ManifestError> {
        let manifest: Self = serde_json::from_str(json)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Check whether this extension requests a specific permission.
    pub fn has_permission(&self, perm: Permission) -> bool {
        self.permissions.contains(&perm)
    }

    /// Check whether *all* requested permissions are in the granted set.
    pub fn check_permissions(&self, granted: &[Permission]) -> Result<(), ManifestError> {
        for p in &self.permissions {
            if !granted.contains(p) {
                return Err(ManifestError::PermissionDenied(*p));
            }
        }
        Ok(())
    }

    /// Returns true if this extension requests any dangerous permissions.
    pub fn requests_dangerous_permissions(&self) -> bool {
        self.permissions.iter().any(|p| p.is_dangerous())
    }
}

// ── ExtensionRegistry ────────────────────────────────────────────────────────

/// In-memory registry of loaded extension manifests.
#[derive(Debug, Default)]
pub struct ExtensionRegistry {
    extensions: HashMap<String, ExtensionManifest>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an extension. Returns error if name is already registered.
    pub fn add(&mut self, manifest: ExtensionManifest) -> Result<(), ManifestError> {
        manifest.validate()?;
        if self.extensions.contains_key(&manifest.name) {
            return Err(ManifestError::Duplicate(manifest.name.clone()));
        }
        self.extensions.insert(manifest.name.clone(), manifest);
        Ok(())
    }

    /// Remove an extension by name. Returns error if not found.
    pub fn remove(&mut self, name: &str) -> Result<ExtensionManifest, ManifestError> {
        self.extensions
            .remove(name)
            .ok_or_else(|| ManifestError::NotFound(name.to_string()))
    }

    /// Look up an extension by name.
    pub fn get(&self, name: &str) -> Option<&ExtensionManifest> {
        self.extensions.get(name)
    }

    /// Number of registered extensions.
    pub fn len(&self) -> usize {
        self.extensions.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }

    /// List all registered extension names.
    pub fn names(&self) -> Vec<&str> {
        self.extensions.keys().map(|s| s.as_str()).collect()
    }

    /// List extensions that request a given permission.
    pub fn extensions_with_permission(&self, perm: Permission) -> Vec<&ExtensionManifest> {
        self.extensions
            .values()
            .filter(|m| m.has_permission(perm))
            .collect()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_manifest() -> ExtensionManifest {
        ExtensionManifest {
            name: "test-extension".to_string(),
            version: "1.0.0".to_string(),
            display_name: "Test Extension".to_string(),
            author: "Test Author".to_string(),
            description: "A test extension".to_string(),
            permissions: vec![Permission::FileRead, Permission::Notify],
            min_host_version: Some("0.1.0".to_string()),
            wasm_path: "test.wasm".to_string(),
        }
    }

    // ── Manifest validation ──────────────────────────────────────────────

    #[test]
    fn valid_manifest_passes_validation() {
        let m = sample_manifest();
        assert!(m.validate().is_ok());
    }

    #[test]
    fn empty_name_fails_validation() {
        let mut m = sample_manifest();
        m.name = "".to_string();
        let err = m.validate().unwrap_err();
        assert!(matches!(err, ManifestError::EmptyName));
    }

    #[test]
    fn name_with_spaces_fails_validation() {
        let mut m = sample_manifest();
        m.name = "bad name".to_string();
        let err = m.validate().unwrap_err();
        assert!(matches!(err, ManifestError::InvalidName(_)));
    }

    #[test]
    fn name_with_special_chars_fails_validation() {
        let mut m = sample_manifest();
        m.name = "ext@1.0".to_string();
        assert!(m.validate().is_err());
    }

    #[test]
    fn name_with_hyphens_and_underscores_passes() {
        let mut m = sample_manifest();
        m.name = "my_cool-ext_v2".to_string();
        assert!(m.validate().is_ok());
    }

    #[test]
    fn invalid_version_fails_validation() {
        let mut m = sample_manifest();
        m.version = "not-a-version".to_string();
        assert!(m.validate().is_err());
    }

    #[test]
    fn two_part_version_fails() {
        let mut m = sample_manifest();
        m.version = "1.0".to_string();
        assert!(m.validate().is_err());
    }

    #[test]
    fn invalid_min_host_version_fails() {
        let mut m = sample_manifest();
        m.min_host_version = Some("abc".to_string());
        assert!(m.validate().is_err());
    }

    // ── JSON parsing ─────────────────────────────────────────────────────

    #[test]
    fn from_json_parses_valid_manifest() {
        let json = r#"{
            "name": "linter",
            "version": "2.1.0",
            "display_name": "My Linter",
            "author": "dev",
            "description": "Lints stuff",
            "permissions": ["file_read", "file_write"],
            "min_host_version": "1.0.0",
            "wasm_path": "linter.wasm"
        }"#;
        let m = ExtensionManifest::from_json(json).unwrap();
        assert_eq!(m.name, "linter");
        assert_eq!(m.version, "2.1.0");
        assert_eq!(m.author, "dev");
        assert_eq!(m.permissions.len(), 2);
        assert!(m.has_permission(Permission::FileRead));
        assert!(m.has_permission(Permission::FileWrite));
    }

    #[test]
    fn from_json_with_minimal_fields() {
        let json = r#"{"name": "minimal", "version": "0.0.1"}"#;
        let m = ExtensionManifest::from_json(json).unwrap();
        assert_eq!(m.name, "minimal");
        assert!(m.permissions.is_empty());
        assert_eq!(m.wasm_path, "extension.wasm"); // default
    }

    #[test]
    fn from_json_rejects_invalid_json() {
        let result = ExtensionManifest::from_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn from_json_rejects_empty_name() {
        let json = r#"{"name": "", "version": "1.0.0"}"#;
        let result = ExtensionManifest::from_json(json);
        assert!(result.is_err());
    }

    // ── Permission checking ──────────────────────────────────────────────

    #[test]
    fn check_permissions_all_granted() {
        let m = sample_manifest();
        let granted = vec![Permission::FileRead, Permission::Notify, Permission::Network];
        assert!(m.check_permissions(&granted).is_ok());
    }

    #[test]
    fn check_permissions_missing_permission() {
        let m = sample_manifest();
        // Only grant FileRead but manifest also needs Notify
        let granted = vec![Permission::FileRead];
        let err = m.check_permissions(&granted).unwrap_err();
        assert!(matches!(err, ManifestError::PermissionDenied(Permission::Notify)));
    }

    #[test]
    fn dangerous_permissions_detected() {
        let mut m = sample_manifest();
        assert!(!m.requests_dangerous_permissions()); // FileRead + Notify are safe
        m.permissions.push(Permission::ProcessExec);
        assert!(m.requests_dangerous_permissions());
    }

    #[test]
    fn permission_is_dangerous_classification() {
        assert!(!Permission::FileRead.is_dangerous());
        assert!(Permission::FileWrite.is_dangerous());
        assert!(Permission::Network.is_dangerous());
        assert!(Permission::ProcessExec.is_dangerous());
        assert!(!Permission::EnvRead.is_dangerous());
        assert!(!Permission::Notify.is_dangerous());
        assert!(!Permission::Clipboard.is_dangerous());
    }

    #[test]
    fn safe_defaults_are_subset_of_all() {
        let all = Permission::all();
        for p in Permission::safe_defaults() {
            assert!(all.contains(p));
        }
    }

    #[test]
    fn all_permissions_has_seven_entries() {
        assert_eq!(Permission::all().len(), 7);
    }

    // ── VersionReq ───────────────────────────────────────────────────────

    #[test]
    fn version_parse_valid() {
        let v = VersionReq::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn version_parse_invalid() {
        assert!(VersionReq::parse("1.2").is_err());
        assert!(VersionReq::parse("abc").is_err());
        assert!(VersionReq::parse("1.2.x").is_err());
        assert!(VersionReq::parse("").is_err());
    }

    #[test]
    fn version_display() {
        let v = VersionReq::new(3, 14, 159);
        assert_eq!(v.to_string(), "3.14.159");
    }

    #[test]
    fn version_compatibility_same_version() {
        let v = VersionReq::new(1, 0, 0);
        assert!(v.is_compatible_with(&VersionReq::new(1, 0, 0)));
    }

    #[test]
    fn version_compatibility_host_newer_minor() {
        let required = VersionReq::new(1, 2, 0);
        assert!(required.is_compatible_with(&VersionReq::new(1, 3, 0)));
    }

    #[test]
    fn version_compatibility_host_older_minor() {
        let required = VersionReq::new(1, 3, 0);
        assert!(!required.is_compatible_with(&VersionReq::new(1, 2, 0)));
    }

    #[test]
    fn version_compatibility_different_major() {
        let required = VersionReq::new(2, 0, 0);
        assert!(!required.is_compatible_with(&VersionReq::new(1, 9, 9)));
    }

    // ── ExtensionRegistry ────────────────────────────────────────────────

    #[test]
    fn registry_starts_empty() {
        let reg = ExtensionRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn registry_add_and_lookup() {
        let mut reg = ExtensionRegistry::new();
        reg.add(sample_manifest()).unwrap();
        assert_eq!(reg.len(), 1);
        let found = reg.get("test-extension").unwrap();
        assert_eq!(found.author, "Test Author");
    }

    #[test]
    fn registry_duplicate_name_rejected() {
        let mut reg = ExtensionRegistry::new();
        reg.add(sample_manifest()).unwrap();
        let err = reg.add(sample_manifest()).unwrap_err();
        assert!(matches!(err, ManifestError::Duplicate(_)));
    }

    #[test]
    fn registry_remove_existing() {
        let mut reg = ExtensionRegistry::new();
        reg.add(sample_manifest()).unwrap();
        let removed = reg.remove("test-extension").unwrap();
        assert_eq!(removed.name, "test-extension");
        assert!(reg.is_empty());
    }

    #[test]
    fn registry_remove_nonexistent() {
        let mut reg = ExtensionRegistry::new();
        let err = reg.remove("nope").unwrap_err();
        assert!(matches!(err, ManifestError::NotFound(_)));
    }

    #[test]
    fn registry_lookup_missing_returns_none() {
        let reg = ExtensionRegistry::new();
        assert!(reg.get("missing").is_none());
    }

    #[test]
    fn registry_names_lists_all() {
        let mut reg = ExtensionRegistry::new();
        reg.add(sample_manifest()).unwrap();
        let mut m2 = sample_manifest();
        m2.name = "other-ext".to_string();
        reg.add(m2).unwrap();
        let mut names = reg.names();
        names.sort();
        assert_eq!(names, vec!["other-ext", "test-extension"]);
    }

    #[test]
    fn registry_filter_by_permission() {
        let mut reg = ExtensionRegistry::new();
        reg.add(sample_manifest()).unwrap(); // has FileRead, Notify
        let mut m2 = sample_manifest();
        m2.name = "writer-ext".to_string();
        m2.permissions = vec![Permission::FileWrite];
        reg.add(m2).unwrap();

        let readers = reg.extensions_with_permission(Permission::FileRead);
        assert_eq!(readers.len(), 1);
        assert_eq!(readers[0].name, "test-extension");

        let writers = reg.extensions_with_permission(Permission::FileWrite);
        assert_eq!(writers.len(), 1);
        assert_eq!(writers[0].name, "writer-ext");

        let networkers = reg.extensions_with_permission(Permission::Network);
        assert_eq!(networkers.len(), 0);
    }

    // ── Permission serde round-trip ──────────────────────────────────────

    #[test]
    fn permission_serde_round_trip() {
        let perms = vec![Permission::FileRead, Permission::ProcessExec, Permission::Clipboard];
        let json = serde_json::to_string(&perms).unwrap();
        let parsed: Vec<Permission> = serde_json::from_str(&json).unwrap();
        assert_eq!(perms, parsed);
    }

    #[test]
    fn manifest_serde_round_trip() {
        let m = sample_manifest();
        let json = serde_json::to_string(&m).unwrap();
        let parsed: ExtensionManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, m.name);
        assert_eq!(parsed.version, m.version);
        assert_eq!(parsed.permissions, m.permissions);
    }
}
