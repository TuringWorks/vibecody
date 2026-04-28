//! Secure externalized resource management for VibeCody.
//!
//! Stores configuration data (CVE databases, SAST rules, MCP catalog, secret patterns)
//! in `~/.vibecli/resources/` with:
//! - SHA-256 integrity verification (manifest.json)
//! - File permissions 0600 (owner-only read/write)
//! - Embedded compiled-in defaults as fallback
//! - Atomic writes (write to .tmp, rename)
//! - Update and verify commands

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ─── Resource Types ──────────────────────────────────────────────────────────

/// Known resource file identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceId {
    McpCatalog,
    VulnDb,
    SastRules,
    SecretPatterns,
}

impl ResourceId {
    pub fn filename(&self) -> &'static str {
        match self {
            Self::McpCatalog    => "mcp-catalog.json",
            Self::VulnDb        => "vuln-db.json",
            Self::SastRules     => "sast-rules.json",
            Self::SecretPatterns => "secret-patterns.json",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::McpCatalog    => "MCP Plugin Directory",
            Self::VulnDb        => "CVE Vulnerability Database",
            Self::SastRules     => "SAST Security Rules",
            Self::SecretPatterns => "Secret Detection Patterns",
        }
    }

    pub fn all() -> &'static [ResourceId] {
        &[Self::McpCatalog, Self::VulnDb, Self::SastRules, Self::SecretPatterns]
    }
}

// ─── Manifest ────────────────────────────────────────────────────────────────

/// Integrity manifest with SHA-256 checksums for each resource file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceManifest {
    /// Schema version.
    pub version: u32,
    /// When this manifest was last updated (epoch secs).
    pub updated_at: u64,
    /// SHA-256 checksums keyed by filename.
    pub checksums: HashMap<String, String>,
    /// File sizes in bytes keyed by filename.
    pub sizes: HashMap<String, u64>,
    /// Source of the data ("embedded", "exported", "updated").
    pub source: String,
}

impl ResourceManifest {
    pub fn new(source: &str) -> Self {
        Self {
            version: 1,
            updated_at: epoch_secs(),
            checksums: HashMap::new(),
            sizes: HashMap::new(),
            source: source.to_string(),
        }
    }
}

// ─── Verification Result ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct VerifyResult {
    pub resource: String,
    pub status: VerifyStatus,
    pub expected_hash: Option<String>,
    pub actual_hash: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum VerifyStatus {
    Ok,
    Missing,
    Corrupted,
    NoManifest,
}

impl std::fmt::Display for VerifyStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok         => write!(f, "OK"),
            Self::Missing    => write!(f, "MISSING"),
            Self::Corrupted  => write!(f, "CORRUPTED"),
            Self::NoManifest => write!(f, "NO MANIFEST"),
        }
    }
}

// ─── Resource Manager ────────────────────────────────────────────────────────

/// Manages externalized resource files with integrity verification.
pub struct ResourceManager {
    resources_dir: PathBuf,
}

impl ResourceManager {
    /// Default resources directory: ~/.vibecli/resources/
    pub fn default_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vibecli")
            .join("resources")
    }

    pub fn new(resources_dir: PathBuf) -> Self {
        Self { resources_dir }
    }

    pub fn default_manager() -> Self {
        Self::new(Self::default_dir())
    }

    /// Get the path to a resource file.
    pub fn resource_path(&self, id: &ResourceId) -> PathBuf {
        self.resources_dir.join(id.filename())
    }

    /// Get the manifest path.
    pub fn manifest_path(&self) -> PathBuf {
        self.resources_dir.join("manifest.json")
    }

    /// Check if the resources directory exists and has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.manifest_path().exists()
    }

    // ── Read ─────────────────────────────────────────────────────────

    /// Load a resource file as a string. Returns None if missing.
    pub fn load_raw(&self, id: &ResourceId) -> Option<String> {
        let path = self.resource_path(id);
        std::fs::read_to_string(&path).ok()
    }

    /// Load a resource as parsed JSON.
    pub fn load_json(&self, id: &ResourceId) -> Option<serde_json::Value> {
        let raw = self.load_raw(id)?;
        serde_json::from_str(&raw).ok()
    }

    /// Load a resource with type deserialization.
    pub fn load_typed<T: serde::de::DeserializeOwned>(&self, id: &ResourceId) -> Option<T> {
        let raw = self.load_raw(id)?;
        serde_json::from_str(&raw).ok()
    }

    /// Load the manifest.
    pub fn load_manifest(&self) -> Option<ResourceManifest> {
        let path = self.manifest_path();
        let raw = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&raw).ok()
    }

    // ── Write ────────────────────────────────────────────────────────

    /// Initialize the resources directory with embedded defaults.
    pub fn initialize(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.resources_dir)?;
        set_secure_permissions(&self.resources_dir);
        Ok(())
    }

    /// Write a resource file atomically (write .tmp, rename, update manifest).
    pub fn write_resource(&self, id: &ResourceId, content: &str) -> anyhow::Result<()> {
        self.initialize()?;

        let path = self.resource_path(id);
        let tmp_path = path.with_extension("json.tmp");

        // Atomic write: write to temp, then rename
        std::fs::write(&tmp_path, content)?;
        std::fs::rename(&tmp_path, &path)?;
        set_secure_permissions(&path);

        // Update manifest with new checksum
        self.update_manifest_entry(id, content)?;

        Ok(())
    }

    /// Write a resource from a serializable value.
    pub fn write_json<T: Serialize>(&self, id: &ResourceId, value: &T) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(value)?;
        self.write_resource(id, &json)
    }

    /// Update the manifest entry for a single resource.
    fn update_manifest_entry(&self, id: &ResourceId, content: &str) -> anyhow::Result<()> {
        let mut manifest = self.load_manifest().unwrap_or_else(|| ResourceManifest::new("exported"));
        let hash = sha256_hex(content.as_bytes());
        manifest.checksums.insert(id.filename().to_string(), hash);
        manifest.sizes.insert(id.filename().to_string(), content.len() as u64);
        manifest.updated_at = epoch_secs();

        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        let manifest_path = self.manifest_path();
        std::fs::write(&manifest_path, manifest_json)?;
        set_secure_permissions(&manifest_path);

        Ok(())
    }

    /// Export all embedded defaults to disk.
    pub fn export_defaults(&self) -> anyhow::Result<ExportResult> {
        self.initialize()?;

        let mut result = ExportResult {
            resources_dir: self.resources_dir.clone(),
            files_written: Vec::new(),
            manifest_written: false,
        };

        // Export each resource type with its embedded default
        let defaults = embedded_defaults();
        let mut manifest = ResourceManifest::new("embedded");

        for (id, content) in &defaults {
            let path = self.resource_path(id);
            std::fs::write(&path, content)?;
            set_secure_permissions(&path);

            let hash = sha256_hex(content.as_bytes());
            manifest.checksums.insert(id.filename().to_string(), hash);
            manifest.sizes.insert(id.filename().to_string(), content.len() as u64);
            result.files_written.push(id.filename().to_string());
        }

        // Write manifest
        let manifest_json = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(self.manifest_path(), manifest_json)?;
        set_secure_permissions(&self.manifest_path());
        result.manifest_written = true;

        Ok(result)
    }

    // ── Verify ───────────────────────────────────────────────────────

    /// Verify integrity of all resource files against the manifest.
    pub fn verify_all(&self) -> Vec<VerifyResult> {
        let manifest = match self.load_manifest() {
            Some(m) => m,
            None => {
                return ResourceId::all().iter().map(|id| VerifyResult {
                    resource: id.filename().to_string(),
                    status: VerifyStatus::NoManifest,
                    expected_hash: None,
                    actual_hash: None,
                    size: None,
                }).collect();
            }
        };

        ResourceId::all().iter().map(|id| {
            let filename = id.filename().to_string();
            let expected_hash = manifest.checksums.get(&filename).cloned();

            let path = self.resource_path(id);
            if !path.exists() {
                return VerifyResult {
                    resource: filename,
                    status: VerifyStatus::Missing,
                    expected_hash,
                    actual_hash: None,
                    size: None,
                };
            }

            match std::fs::read(&path) {
                Ok(bytes) => {
                    let actual_hash = sha256_hex(&bytes);
                    let size = bytes.len() as u64;
                    let status = match &expected_hash {
                        Some(expected) if *expected == actual_hash => VerifyStatus::Ok,
                        Some(_) => VerifyStatus::Corrupted,
                        None => VerifyStatus::NoManifest,
                    };
                    VerifyResult {
                        resource: filename,
                        status,
                        expected_hash,
                        actual_hash: Some(actual_hash),
                        size: Some(size),
                    }
                }
                Err(_) => VerifyResult {
                    resource: filename,
                    status: VerifyStatus::Missing,
                    expected_hash,
                    actual_hash: None,
                    size: None,
                },
            }
        }).collect()
    }

    /// Verify a single resource.
    pub fn verify(&self, id: &ResourceId) -> VerifyResult {
        self.verify_all().into_iter()
            .find(|r| r.resource == id.filename())
            .unwrap_or(VerifyResult {
                resource: id.filename().to_string(),
                status: VerifyStatus::Missing,
                expected_hash: None,
                actual_hash: None,
                size: None,
            })
    }

    // ── Status ───────────────────────────────────────────────────────

    /// Get overall status of the resource system.
    pub fn status(&self) -> ResourceStatus {
        let manifest = self.load_manifest();
        let verifications = self.verify_all();

        let total = verifications.len();
        let ok = verifications.iter().filter(|v| v.status == VerifyStatus::Ok).count();
        let missing = verifications.iter().filter(|v| v.status == VerifyStatus::Missing).count();
        let corrupted = verifications.iter().filter(|v| v.status == VerifyStatus::Corrupted).count();

        let total_size: u64 = verifications.iter()
            .filter_map(|v| v.size)
            .sum();

        ResourceStatus {
            initialized: self.is_initialized(),
            resources_dir: self.resources_dir.clone(),
            total_resources: total,
            ok_count: ok,
            missing_count: missing,
            corrupted_count: corrupted,
            total_size_bytes: total_size,
            manifest_updated_at: manifest.map(|m| m.updated_at),
            verifications,
        }
    }

    // ── Load with fallback ───────────────────────────────────────────

    /// Load a resource with fallback to embedded default if file is missing or corrupted.
    pub fn load_or_default(&self, id: &ResourceId) -> String {
        // Try disk first
        if let Some(content) = self.load_raw(id) {
            // Verify integrity if manifest exists
            let verification = self.verify(id);
            if verification.status == VerifyStatus::Ok || verification.status == VerifyStatus::NoManifest {
                return content;
            }
            // Corrupted — fall through to default
        }
        // Fall back to embedded default
        embedded_default_for(id)
    }

    /// Load and parse with fallback.
    pub fn load_or_default_typed<T: serde::de::DeserializeOwned>(&self, id: &ResourceId) -> Option<T> {
        let raw = self.load_or_default(id);
        serde_json::from_str(&raw).ok()
    }
}

// ─── Export Result ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ExportResult {
    pub resources_dir: PathBuf,
    pub files_written: Vec<String>,
    pub manifest_written: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceStatus {
    pub initialized: bool,
    pub resources_dir: PathBuf,
    pub total_resources: usize,
    pub ok_count: usize,
    pub missing_count: usize,
    pub corrupted_count: usize,
    pub total_size_bytes: u64,
    pub manifest_updated_at: Option<u64>,
    pub verifications: Vec<VerifyResult>,
}

// ─── SHA-256 ─────────────────────────────────────────────────────────────────

/// Compute SHA-256 hex digest of bytes (no external crate — pure Rust).
pub fn sha256_hex(data: &[u8]) -> String {
    // SHA-256 constants
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    // Pre-processing: pad message
    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    // Process each 512-bit (64-byte) block
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i*4], chunk[i*4+1], chunk[i*4+2], chunk[i*4+3]]);
        }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g; g = f; f = e;
            e = d.wrapping_add(temp1);
            d = c; c = b; b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a); h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c); h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e); h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g); h[7] = h[7].wrapping_add(hh);
    }

    h.iter().map(|v| format!("{:08x}", v)).collect()
}

// ─── Secure Permissions ──────────────────────────────────────────────────────

/// Set file/directory permissions to 0600 (owner-only read/write).
fn set_secure_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = if path.is_dir() {
            std::fs::Permissions::from_mode(0o700)
        } else {
            std::fs::Permissions::from_mode(0o600)
        };
        let _ = std::fs::set_permissions(path, perms);
    }
    #[cfg(not(unix))]
    let _ = path;
}

// ─── Embedded Defaults ───────────────────────────────────────────────────────

/// Get all embedded default resources (compiled into the binary).
fn embedded_defaults() -> Vec<(ResourceId, String)> {
    vec![
        (ResourceId::McpCatalog, embedded_default_for(&ResourceId::McpCatalog)),
        (ResourceId::VulnDb, embedded_default_for(&ResourceId::VulnDb)),
        (ResourceId::SastRules, embedded_default_for(&ResourceId::SastRules)),
        (ResourceId::SecretPatterns, embedded_default_for(&ResourceId::SecretPatterns)),
    ]
}

/// Get the embedded default content for a specific resource.
/// These are minimal JSON representations compiled into the binary.
fn embedded_default_for(id: &ResourceId) -> String {
    match id {
        ResourceId::McpCatalog => {
            // Minimal embedded catalog — full list loaded from disk
            serde_json::json!({
                "version": 1,
                "source": "embedded",
                "plugins": [
                    {"id": "mcp-filesystem", "name": "filesystem", "author": "modelcontextprotocol", "category": "File Systems"},
                    {"id": "mcp-github", "name": "github", "author": "modelcontextprotocol", "category": "Git"},
                    {"id": "mcp-git", "name": "git", "author": "modelcontextprotocol", "category": "Git"},
                    {"id": "mcp-fetch", "name": "fetch", "author": "modelcontextprotocol", "category": "Cloud"},
                    {"id": "mcp-memory", "name": "memory", "author": "modelcontextprotocol", "category": "AI/ML"},
                ]
            }).to_string()
        }
        ResourceId::VulnDb => {
            serde_json::json!({
                "version": 1,
                "source": "embedded",
                "note": "Minimal embedded DB. Run `vibecli resources export` to create full external DB, or use live OSV.dev API.",
                "count": 0,
                "vulnerabilities": []
            }).to_string()
        }
        ResourceId::SastRules => {
            serde_json::json!({
                "version": 1,
                "source": "embedded",
                "count": 0,
                "rules": []
            }).to_string()
        }
        ResourceId::SecretPatterns => {
            serde_json::json!({
                "version": 1,
                "source": "embedded",
                "patterns": [
                    {"name": "AWS Access Key", "pattern": "AKIA", "severity": "Critical"},
                    {"name": "GitHub Token", "pattern": "ghp_", "severity": "Critical"},
                    {"name": "Private Key", "pattern": "-----BEGIN PRIVATE KEY-----", "severity": "Critical"},
                    {"name": "JWT Token", "pattern": "eyJ", "severity": "High"},
                ]
            }).to_string()
        }
    }
}

// ─── Utilities ───────────────────────────────────────────────────────────────

fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir() -> PathBuf {
        let ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("vibecody-resources-test-{}", ns))
    }

    // ── SHA-256 ──────────────────────────────────────────────────────

    #[test]
    fn sha256_empty() {
        let hash = sha256_hex(b"");
        assert_eq!(hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn sha256_hello() {
        let hash = sha256_hex(b"hello");
        assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }

    #[test]
    fn sha256_abc() {
        let hash = sha256_hex(b"abc");
        assert_eq!(hash, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn sha256_long_string() {
        let data = "a]".repeat(1000);
        let hash = sha256_hex(data.as_bytes());
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // 256 bits = 64 hex chars
    }

    // ── ResourceId ───────────────────────────────────────────────────

    #[test]
    fn resource_id_filenames() {
        assert_eq!(ResourceId::McpCatalog.filename(), "mcp-catalog.json");
        assert_eq!(ResourceId::VulnDb.filename(), "vuln-db.json");
        assert_eq!(ResourceId::SastRules.filename(), "sast-rules.json");
        assert_eq!(ResourceId::SecretPatterns.filename(), "secret-patterns.json");
    }

    #[test]
    fn resource_id_all() {
        assert_eq!(ResourceId::all().len(), 4);
    }

    // ── ResourceManager ──────────────────────────────────────────────

    #[test]
    fn manager_initialize() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        mgr.initialize().expect("init");
        assert!(dir.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manager_export_defaults() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        let result = mgr.export_defaults().expect("export");
        assert_eq!(result.files_written.len(), 4);
        assert!(result.manifest_written);
        assert!(mgr.is_initialized());
        // All files should exist
        for id in ResourceId::all() {
            assert!(mgr.resource_path(id).exists(), "Missing: {}", id.filename());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manager_write_and_read() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        let content = r#"{"test": true}"#;
        mgr.write_resource(&ResourceId::McpCatalog, content).expect("write");

        let loaded = mgr.load_raw(&ResourceId::McpCatalog).expect("should load");
        assert_eq!(loaded, content);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manager_write_json() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        let data = serde_json::json!({"plugins": [1, 2, 3]});
        mgr.write_json(&ResourceId::McpCatalog, &data).expect("write");

        let loaded: serde_json::Value = mgr.load_typed(&ResourceId::McpCatalog).expect("load");
        assert_eq!(loaded["plugins"].as_array().unwrap().len(), 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manager_load_missing_returns_none() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        assert!(mgr.load_raw(&ResourceId::VulnDb).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Verification ─────────────────────────────────────────────────

    #[test]
    fn verify_after_export() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        mgr.export_defaults().expect("export");

        let results = mgr.verify_all();
        assert_eq!(results.len(), 4);
        for r in &results {
            assert_eq!(r.status, VerifyStatus::Ok, "Failed for: {}", r.resource);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verify_detects_corruption() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        mgr.export_defaults().expect("export");

        // Corrupt a file
        let path = mgr.resource_path(&ResourceId::McpCatalog);
        std::fs::write(&path, "CORRUPTED DATA").expect("corrupt");

        let result = mgr.verify(&ResourceId::McpCatalog);
        assert_eq!(result.status, VerifyStatus::Corrupted);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verify_detects_missing() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        mgr.export_defaults().expect("export");

        // Delete a file
        std::fs::remove_file(mgr.resource_path(&ResourceId::SastRules)).expect("delete");

        let result = mgr.verify(&ResourceId::SastRules);
        assert_eq!(result.status, VerifyStatus::Missing);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verify_no_manifest() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        // No export, no manifest
        let results = mgr.verify_all();
        for r in &results {
            assert_eq!(r.status, VerifyStatus::NoManifest);
        }
    }

    // ── Status ───────────────────────────────────────────────────────

    #[test]
    fn status_after_export() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        mgr.export_defaults().expect("export");

        let status = mgr.status();
        assert!(status.initialized);
        assert_eq!(status.total_resources, 4);
        assert_eq!(status.ok_count, 4);
        assert_eq!(status.missing_count, 0);
        assert_eq!(status.corrupted_count, 0);
        assert!(status.total_size_bytes > 0);
        assert!(status.manifest_updated_at.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn status_uninitialized() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        let status = mgr.status();
        assert!(!status.initialized);
        assert_eq!(status.ok_count, 0);
    }

    // ── Load with fallback ───────────────────────────────────────────

    #[test]
    fn load_or_default_uses_disk() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        let custom = r#"{"custom": true}"#;
        mgr.write_resource(&ResourceId::McpCatalog, custom).expect("write");

        let loaded = mgr.load_or_default(&ResourceId::McpCatalog);
        assert!(loaded.contains("custom"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_or_default_falls_back() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        // No files on disk — should return embedded default
        let loaded = mgr.load_or_default(&ResourceId::McpCatalog);
        assert!(loaded.contains("embedded"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_or_default_fallback_on_corruption() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        mgr.export_defaults().expect("export");

        // Corrupt the file
        std::fs::write(mgr.resource_path(&ResourceId::VulnDb), "NOT JSON").expect("corrupt");

        // Should detect corruption via manifest and fall back
        let loaded = mgr.load_or_default(&ResourceId::VulnDb);
        assert!(loaded.contains("embedded"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Manifest ─────────────────────────────────────────────────────

    #[test]
    fn manifest_roundtrip() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        mgr.write_resource(&ResourceId::McpCatalog, "test data").expect("write");

        let manifest = mgr.load_manifest().expect("manifest");
        assert_eq!(manifest.version, 1);
        assert!(manifest.checksums.contains_key("mcp-catalog.json"));
        assert!(manifest.sizes.contains_key("mcp-catalog.json"));
        assert_eq!(*manifest.sizes.get("mcp-catalog.json").unwrap(), 9); // "test data" = 9 bytes
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifest_updates_on_each_write() {
        let dir = test_dir();
        let mgr = ResourceManager::new(dir.clone());
        mgr.write_resource(&ResourceId::McpCatalog, "v1").expect("write");
        let hash1 = mgr.load_manifest().unwrap().checksums.get("mcp-catalog.json").cloned();

        mgr.write_resource(&ResourceId::McpCatalog, "v2").expect("write");
        let hash2 = mgr.load_manifest().unwrap().checksums.get("mcp-catalog.json").cloned();

        assert_ne!(hash1, hash2, "Hash should change when content changes");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Embedded defaults ────────────────────────────────────────────

    #[test]
    fn embedded_defaults_all_valid_json() {
        for id in ResourceId::all() {
            let content = embedded_default_for(id);
            let parsed: Result<serde_json::Value, _> = serde_json::from_str(&content);
            assert!(parsed.is_ok(), "Embedded default for {} is not valid JSON", id.filename());
        }
    }

    #[test]
    fn embedded_defaults_have_version() {
        for id in ResourceId::all() {
            let content = embedded_default_for(id);
            let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
            assert!(parsed.get("version").is_some(), "Embedded default for {} missing version", id.filename());
        }
    }

    // ── VerifyStatus display ─────────────────────────────────────────

    #[test]
    fn verify_status_display() {
        assert_eq!(format!("{}", VerifyStatus::Ok), "OK");
        assert_eq!(format!("{}", VerifyStatus::Missing), "MISSING");
        assert_eq!(format!("{}", VerifyStatus::Corrupted), "CORRUPTED");
    }
}
