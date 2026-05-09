//! Error types for vibe-memory

use thiserror::Error;

/// All errors that can occur in vibe-memory operations.
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivation(String),

    #[error("Store not found at path: {0}")]
    StoreNotFound(String),

    #[error("Memory entry not found: {0}")]
    EntryNotFound(String),

    #[error("Invalid vector dimensions: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },

    #[error("Vector extension not available: {0}")]
    ExtensionNotAvailable(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid workspace path: {0}")]
    InvalidWorkspace(String),
}

/// Result type alias for vibe-memory operations.
pub type Result<T> = std::result::Result<T, MemoryError>;
