//! Incremental cache — SHA256 content hashing so re-runs only re-parse changed files.
//!
//! `FileHashes` maps `file_path -> sha256(content)`. The builder consults it before
//! parsing: if a file's hash is unchanged AND it's already in the graph, it's
//! skipped; if changed, the old file's nodes/edges are removed (`CodeGraph::remove_file`)
//! and the file is re-parsed and re-inserted. This is the same SHA256-cache pattern
//! Graphify uses, and it makes `--update` re-runs cheap.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// SHA256 content hash of a file.
pub fn hash_content(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Map of `file_path -> content hash` persisted between runs.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FileHashes {
    map: HashMap<String, String>,
}

impl FileHashes {
    /// Construct an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a file's hash. Returns the previous hash if any.
    pub fn set(&mut self, path: impl Into<String>, hash: impl Into<String>) -> Option<String> {
        self.map.insert(path.into(), hash.into())
    }

    /// Look up the stored hash for a path.
    pub fn get(&self, path: &str) -> Option<&str> {
        self.map.get(path).map(|s| s.as_str())
    }

    /// Remove a path from the cache.
    pub fn remove(&mut self, path: &str) {
        self.map.remove(path);
    }

    /// True if `path` is recorded with the given `hash` (i.e. unchanged since last run).
    pub fn is_unchanged(&self, path: &str, hash: &str) -> bool {
        self.map.get(path).map_or(false, |h| h == hash)
    }

    /// Number of tracked files.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Serialize to JSON (for persistence alongside the graph store).
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON.
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }
}

/// Compute the hash of a file on disk. Returns `None` if the file cannot be read.
pub fn hash_file(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(hash_content(&content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(hash_content("abc"), hash_content("abc"));
        assert_ne!(hash_content("abc"), hash_content("abd"));
    }

    #[test]
    fn cache_detects_change() {
        let mut c = FileHashes::new();
        let h1 = hash_content("fn main() {}");
        c.set("a.rs", h1.clone());
        assert!(c.is_unchanged("a.rs", &h1));
        let h2 = hash_content("fn main() { let x = 1; }");
        assert!(!c.is_unchanged("a.rs", &h2));
    }

    #[test]
    fn roundtrip_json() {
        let mut c = FileHashes::new();
        c.set("a.rs", hash_content("x"));
        let json = c.to_json().unwrap();
        let c2 = FileHashes::from_json(&json).unwrap();
        assert_eq!(c2.get("a.rs"), Some(c.get("a.rs").unwrap()));
    }
}