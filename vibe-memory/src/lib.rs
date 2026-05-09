//! # VibeMemory — Local SQLite Vector Memory

pub mod error;
pub mod extension;
pub mod global_store;
pub mod hub;
pub mod project_store;
pub mod schema;

use serde::{Deserialize, Serialize};

pub use error::{MemoryError, Result};
pub use extension::{ExtensionManager, VectorExtension};
pub use global_store::GlobalMemStore;
pub use hub::MemoryContextHub;
pub use project_store::ProjectMemStore;
pub use schema::initialize_store;

/// Memory sector classification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemorySector {
    Episodic,
    Semantic,
    Procedural,
    Emotional,
    Reflective,
}

impl MemorySector {
    pub fn decay_rate(&self) -> f64 {
        match self {
            Self::Episodic => 0.015,
            Self::Semantic => 0.005,
            Self::Procedural => 0.008,
            Self::Emotional => 0.020,
            Self::Reflective => 0.001,
        }
    }

    pub fn weight(&self) -> f64 {
        match self {
            Self::Episodic => 1.2,
            Self::Semantic => 1.0,
            Self::Procedural => 1.1,
            Self::Emotional => 1.3,
            Self::Reflective => 0.8,
        }
    }

    pub fn keyword_signals(&self) -> &'static [&'static str] {
        match self {
            Self::Episodic => &[
                "yesterday", "today", "remember", "happened", "when i", "last time",
                "session", "just now", "earlier", "event", "experience", "meeting",
            ],
            Self::Semantic => &[
                "means", "defined", "always", "fact", "is a", "known as",
                "definition", "concept", "api", "protocol", "standard",
            ],
            Self::Procedural => &[
                "step", "how to", "command", "recipe", "process", "workflow",
                "first", "then", "next", "run", "execute", "build", "install",
            ],
            Self::Emotional => &[
                "frustrated", "happy", "love", "hate", "annoying", "great",
                "terrible", "excited", "worried", "confused", "delighted", "prefers", "prefer",
            ],
            Self::Reflective => &[
                "realize", "insight", "pattern", "lesson", "learned", "principle",
                "takeaway", "reflection", "observation", "noticed", "strategy",
            ],
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Episodic => "episodic",
            Self::Semantic => "semantic",
            Self::Procedural => "procedural",
            Self::Emotional => "emotional",
            Self::Reflective => "reflective",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "episodic" => Some(Self::Episodic),
            "semantic" => Some(Self::Semantic),
            "procedural" => Some(Self::Procedural),
            "emotional" => Some(Self::Emotional),
            "reflective" => Some(Self::Reflective),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Episodic,
            Self::Semantic,
            Self::Procedural,
            Self::Emotional,
            Self::Reflective,
        ]
    }
}

impl Default for MemorySector {
    fn default() -> Self {
        Self::Episodic
    }
}

/// Metadata for a memory entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryMeta {
    pub tags: Vec<String>,
    pub pinned: bool,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub ttl_seconds: Option<u64>,
}

impl MemoryMeta {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_pinned(mut self, pinned: bool) -> Self {
        self.pinned = pinned;
        self
    }

    pub fn with_project(mut self, project: String) -> Self {
        self.project_id = Some(project);
        self
    }

    pub fn with_ttl(mut self, seconds: u64) -> Self {
        self.ttl_seconds = Some(seconds);
        self
    }
}

/// A memory entry stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub sector: String,
    pub salience: f64,
    pub decay_lambda: f64,
    pub embedding: Vec<f32>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_seen_at: i64,
    pub version: u32,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub ttl_expires_at: Option<i64>,
}

impl MemoryEntry {
    pub fn current_salience(&self, now: i64) -> f64 {
        if self.pinned {
            return self.salience;
        }
        let elapsed_days = (now - self.created_at) as f64 / (24.0 * 3600.0);
        self.salience * (-self.decay_lambda * elapsed_days).exp()
    }

    pub fn token_count(&self) -> usize {
        self.content.split_whitespace().count() * 13 / 10
    }
}

/// Result from a search query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub sector: String,
    pub score: f64,
    pub salience: f64,
    pub tags: Vec<String>,
    pub project_id: Option<String>,
    pub store: StoreKind,
}

/// Which store a result came from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StoreKind {
    Project,
    Global,
}

impl StoreKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Global => "global",
        }
    }
}

/// Waypoint (associative link) between memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Waypoint {
    pub id: String,
    pub src_id: String,
    pub dst_id: String,
    pub weight: f64,
    pub cross_project: bool,
    pub created_at: i64,
}

/// Purge report from consolidation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurgeReport {
    pub entries_purged: usize,
    pub entries_decayed: usize,
    pub project_store: String,
    pub global_store: String,
}

/// Hub statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubStats {
    pub project_count: usize,
    pub global_count: usize,
    pub project_db_size: u64,
    pub global_db_size: u64,
}

/// Sector weight configuration.
pub type SectorWeights = std::collections::HashMap<String, f64>;

/// Generate a unique memory ID.
pub fn generate_id() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let random: u64 = rand::random();
    format!("{:x}-{:x}", timestamp, random)
}

/// Get current epoch seconds.
pub fn epoch_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Hash a workspace path to derive storage location.
pub fn workspace_hash(workspace: &std::path::Path) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(workspace.to_string_lossy().as_bytes());
    hex::encode(&hasher.finalize()[..8])
}

/// Classify sector from content using keyword matching.
pub fn classify_sector(content: &str) -> MemorySector {
    let content_lower = content.to_lowercase();
    let mut scores: Vec<(MemorySector, f64)> = MemorySector::all()
        .iter()
        .map(|s| {
            let score = s.keyword_signals()
                .iter()
                .filter(|kw| content_lower.contains(*kw))
                .count() as f64;
            (s.clone(), score)
        })
        .collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    scores.first()
        .map(|(s, _)| s.clone())
        .unwrap_or(MemorySector::Episodic)
}

/// Get default vector dimensions from environment or config.
pub fn default_dimensions() -> usize {
    std::env::var("VIBE_MEMORY_DIM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(768)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sector_classification() {
        let episodic = classify_sector("Yesterday we had a meeting about the new feature");
        assert_eq!(episodic, MemorySector::Episodic);

        let semantic = classify_sector("The definition of an API is a contract between services");
        assert_eq!(semantic, MemorySector::Semantic);

        let procedural = classify_sector("Step 1: Run cargo build, Step 2: Run cargo test");
        assert_eq!(procedural, MemorySector::Procedural);

        let emotional = classify_sector("I really love how Rust handles ownership");
        assert_eq!(emotional, MemorySector::Emotional);
    }

    #[test]
    fn test_workspace_hash() {
        use std::path::PathBuf;
        let path = PathBuf::from("/tmp/myproject");
        let hash = workspace_hash(&path);
        assert_eq!(hash.len(), 16);
    }

    #[test]
    fn test_memory_entry_salience_decay() {
        let now = epoch_secs();
        let entry = MemoryEntry {
            id: "test".to_string(),
            content: "test".to_string(),
            sector: "episodic".to_string(),
            salience: 1.0,
            decay_lambda: 0.01,
            embedding: vec![],
            created_at: now - (7 * 24 * 3600),
            updated_at: now,
            last_seen_at: now,
            version: 1,
            pinned: false,
            tags: vec![],
            metadata: serde_json::Value::Null,
            project_id: None,
            session_id: None,
            ttl_expires_at: None,
        };

        let current = entry.current_salience(now);
        assert!(current < 1.0);
        assert!(current > 0.5);
    }

    #[test]
    fn test_pinned_salience_unchanged() {
        let now = epoch_secs();
        let entry = MemoryEntry {
            id: "test".to_string(),
            content: "test".to_string(),
            sector: "episodic".to_string(),
            salience: 0.5,
            decay_lambda: 0.01,
            embedding: vec![],
            created_at: now - (30 * 24 * 3600),
            updated_at: now,
            last_seen_at: now,
            version: 1,
            pinned: true,
            tags: vec![],
            metadata: serde_json::Value::Null,
            project_id: None,
            session_id: None,
            ttl_expires_at: None,
        };

        let current = entry.current_salience(now);
        assert_eq!(current, 0.5);
    }
}
