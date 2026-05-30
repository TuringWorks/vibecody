//! SQLite vector extension selection and management.
//!
//! Supports three vector extensions with runtime selection:
//! - **sqlite-vec**: Default, most portable, WASM-compatible
//! - **sqlite-vector**: SIMD-accelerated for x86_64
//! - **vectorlite**: HNSW-based for large-scale (>100K entries)

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Vector extension selection strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VectorExtension {
    /// SQLite-vec: pure C, zero deps, WASM-compatible
    Vec,
    /// SQLite-vector: SIMD-accelerated (x86_64 only)
    Vector,
    /// VectorLite: HNSW-based, good for large scale
    Lite,
}

impl Default for VectorExtension {
    fn default() -> Self {
        Self::Vec // Most portable default
    }
}

impl VectorExtension {
    /// Create a vector extension from environment variable or default.
    pub fn from_env() -> Self {
        std::env::var("VIBE_MEMORY_VECTOR_EXT")
            .map(|v| match v.to_lowercase().as_str() {
                "vec" | "sqlite-vec" => Self::Vec,
                "vector" | "sqlite-vector" => Self::Vector,
                "lite" | "vectorlite" => Self::Lite,
                _ => {
                    warn!("Unknown VIBE_MEMORY_VECTOR_EXT '{}', defaulting to vec", v);
                    Self::Vec
                }
            })
            .unwrap_or_default()
    }

    /// Check if this extension is available in the current environment.
    pub fn is_available(&self) -> bool {
        match self {
            Self::Vec => true, // Always available (we implement fallback)
            Self::Vector => {
                #[cfg(target_arch = "x86_64")]
                {
                    // Check for SIMD support
                    is_x86_64_with_simd()
                }
                #[cfg(not(target_arch = "x86_64"))]
                {
                    false
                }
            }
            Self::Lite => true, // Could check for library availability
        }
    }

    /// Get the extension name for display.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Vec => "sqlite-vec",
            Self::Vector => "sqlite-vector",
            Self::Lite => "vectorlite",
        }
    }

    /// Get the recommended dimensions for this extension.
    pub fn recommended_dimensions(&self) -> usize {
        match self {
            Self::Vec => 768,
            Self::Vector => 768,
            Self::Lite => 1536, // HNSW works better with higher dims
        }
    }
}

/// Architecture detection helper.
#[cfg(target_arch = "x86_64")]
fn is_x86_64_with_simd() -> bool {
    // Check for AVX2 (common in modern x86_64)
    #[cfg(target_feature = "avx2")]
    return true;
    #[cfg(not(target_feature = "avx2"))]
    return std::env::var("VIBE_FORCE_VECTOR").is_ok();
}

/// Extension manager for handling vector operations.
#[derive(Debug, Clone)]
pub struct ExtensionManager {
    extension: VectorExtension,
    dimensions: usize,
}

impl ExtensionManager {
    /// Create a new extension manager with detected or configured extension.
    pub fn new(dimensions: usize) -> Self {
        let ext = VectorExtension::from_env();
        Self::with_extension(ext, dimensions)
    }

    /// Create with a specific extension.
    pub fn with_extension(extension: VectorExtension, dimensions: usize) -> Self {
        if !extension.is_available() {
            debug!(
                "Extension {} not available, falling back to vec",
                extension.name()
            );
            return Self {
                extension: VectorExtension::Vec,
                dimensions,
            };
        }

        info!(
            "Using vector extension: {} (dims: {})",
            extension.name(),
            dimensions
        );

        Self {
            extension,
            dimensions,
        }
    }

    /// Get the configured extension.
    pub fn extension(&self) -> VectorExtension {
        self.extension
    }

    /// Get the dimensions.
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Generate SQL for creating a vector index based on extension.
    pub fn create_index_sql(&self, table: &str, column: &str) -> String {
        match self.extension {
            VectorExtension::Vec => {
                // sqlite-vec: virtual table approach
                format!(
                    r#"
                    CREATE VIRTUAL TABLE IF NOT EXISTS {table}_vec 
                    USING vec(
                        id TEXT PRIMARY KEY,
                        embedding REAL[{dimensions}]
                    );
                    "#,
                    table = format!("{}_{}", table, column),
                    dimensions = self.dimensions
                )
            }
            VectorExtension::Vector => {
                // sqlite-vector: SIMD-accelerated indexing
                format!(
                    r#"
                    CREATE VIRTUAL TABLE IF NOT EXISTS {table}_vec 
                    USING vector(
                        embedding REAL[{dimensions}],
                        metric=cosine,
                        k=50
                    );
                    "#,
                    table = format!("{}_{}", table, column),
                    dimensions = self.dimensions
                )
            }
            VectorExtension::Lite => {
                // vectorlite: HNSW-based
                format!(
                    r#"
                    CREATE VIRTUAL TABLE IF NOT EXISTS {table}_vec 
                    USING vectorlite(
                        embedding REAL[{dimensions}],
                        m=16,
                        ef_construction=200
                    );
                    "#,
                    table = format!("{}_{}", table, column),
                    dimensions = self.dimensions
                )
            }
        }
    }

    /// Generate SQL for similarity search based on extension.
    pub fn search_sql(&self, table: &str, k: usize) -> String {
        match self.extension {
            VectorExtension::Vec => {
                format!(
                    r#"
                    SELECT id, distance 
                    FROM {table}_vec 
                    WHERE embedding MATCH '{vector}'
                    ORDER BY distance 
                    LIMIT {k};
                    "#,
                    table = format!("{}_embedding", table),
                    vector = "{}" // Placeholder for vector binding
                )
            }
            VectorExtension::Vector | VectorExtension::Lite => {
                format!(
                    r#"
                    SELECT id, distance 
                    FROM {table}_vec 
                    ORDER BY distance 
                    LIMIT {k};
                    "#,
                    table = format!("{}_embedding", table),
                    k = k
                )
            }
        }
    }

    /// Check if the extension supports a specific feature.
    pub fn supports(&self, feature: &str) -> bool {
        match (self.extension, feature) {
            (VectorExtension::Vec, "binary") => true,
            (VectorExtension::Vector, "binary") => true,
            (VectorExtension::Vector, "simd") => cfg!(target_arch = "x86_64"),
            (VectorExtension::Lite, "hnsw") => true,
            (VectorExtension::Lite, "reindex") => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_from_env_vec() {
        std::env::set_var("VIBE_MEMORY_VECTOR_EXT", "vec");
        assert_eq!(VectorExtension::from_env(), VectorExtension::Vec);
        std::env::remove_var("VIBE_MEMORY_VECTOR_EXT");
    }

    #[test]
    fn test_extension_from_env_vector() {
        std::env::set_var("VIBE_MEMORY_VECTOR_EXT", "sqlite-vector");
        assert_eq!(VectorExtension::from_env(), VectorExtension::Vector);
        std::env::remove_var("VIBE_MEMORY_VECTOR_EXT");
    }

    #[test]
    fn test_extension_default() {
        std::env::remove_var("VIBE_MEMORY_VECTOR_EXT");
        assert_eq!(VectorExtension::from_env(), VectorExtension::Vec);
    }

    #[test]
    fn test_extension_names() {
        assert_eq!(VectorExtension::Vec.name(), "sqlite-vec");
        assert_eq!(VectorExtension::Vector.name(), "sqlite-vector");
        assert_eq!(VectorExtension::Lite.name(), "vectorlite");
    }

    #[test]
    fn test_manager_creation() {
        let manager = ExtensionManager::new(768);
        assert_eq!(manager.dimensions(), 768);
    }
}
