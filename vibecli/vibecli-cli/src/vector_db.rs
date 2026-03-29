// Unified vector database integration module.
// Provides a common abstraction over multiple vector database backends
// (Qdrant, Pinecone, Pgvector, Milvus, Weaviate, Chroma) plus a fully
// functional in-memory vector store for local development and testing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Supported vector database providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VectorDbProvider {
    Qdrant,
    Pinecone,
    Pgvector,
    Milvus,
    Weaviate,
    Chroma,
    InMemory,
}

/// Distance / similarity metrics used for nearest-neighbor search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistanceMetric {
    Cosine,
    Euclidean,
    DotProduct,
    Manhattan,
}

/// Status of a collection index build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexStatus {
    Ready,
    Building,
    Error(String),
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// Connection and behavioral configuration for a vector database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDbConfig {
    pub provider: VectorDbProvider,
    pub endpoint: String,
    pub api_key: Option<String>,
    pub collection_name: String,
    pub dimension: u32,
    pub metric: DistanceMetric,
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

/// A single vector record stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEntry {
    pub id: String,
    pub vector: Vec<f32>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    pub content: Option<String>,
}

/// A single result returned from a similarity search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f64,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    pub content: Option<String>,
}

/// Parameters for a similarity search query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub vector: Vec<f32>,
    pub top_k: usize,
    pub filter: Option<HashMap<String, serde_json::Value>>,
    pub min_score: Option<f64>,
}

/// Metadata about an existing collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub name: String,
    pub dimension: u32,
    pub count: u64,
    pub metric: DistanceMetric,
}

/// Configuration for creating a new collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    pub name: String,
    pub dimension: u32,
    pub metric: DistanceMetric,
    pub hnsw_config: Option<HnswConfig>,
}

/// HNSW (Hierarchical Navigable Small World) index parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig {
    /// Number of bi-directional links per node.
    pub m: u32,
    /// Size of the dynamic candidate list during index construction.
    pub ef_construction: u32,
    /// Size of the dynamic candidate list during search.
    pub ef_search: u32,
}

// ---------------------------------------------------------------------------
// InMemoryVectorDb
// ---------------------------------------------------------------------------

/// A fully functional in-memory vector store.
///
/// Useful for local development, unit tests, and small-scale semantic search
/// without requiring an external database process.
#[derive(Debug)]
pub struct InMemoryVectorDb {
    dimension: u32,
    metric: DistanceMetric,
    entries: HashMap<String, VectorEntry>,
}

impl InMemoryVectorDb {
    /// Create a new in-memory store for vectors of the given `dimension`,
    /// scored by `metric`.
    pub fn new(dimension: u32, metric: DistanceMetric) -> Self {
        Self {
            dimension,
            metric,
            entries: HashMap::new(),
        }
    }

    /// Insert a single vector entry.
    ///
    /// Returns an error if the entry's vector dimension does not match the
    /// store's configured dimension.
    pub fn insert(&mut self, entry: VectorEntry) -> anyhow::Result<()> {
        if entry.vector.len() != self.dimension as usize {
            anyhow::bail!(
                "dimension mismatch: expected {}, got {}",
                self.dimension,
                entry.vector.len()
            );
        }
        self.entries.insert(entry.id.clone(), entry);
        Ok(())
    }

    /// Insert a batch of entries, skipping any that have mismatched dimensions.
    ///
    /// Returns the number of entries successfully inserted.
    pub fn insert_batch(&mut self, entries: Vec<VectorEntry>) -> anyhow::Result<usize> {
        let mut count = 0usize;
        for entry in entries {
            if entry.vector.len() == self.dimension as usize {
                self.entries.insert(entry.id.clone(), entry);
                count += 1;
            }
        }
        Ok(count)
    }

    /// Search for the nearest neighbours to `query.vector`.
    ///
    /// Results are sorted by descending score (higher = more similar for
    /// Cosine / DotProduct, inverted for Euclidean / Manhattan) and limited
    /// to `query.top_k`. An optional `query.min_score` threshold is applied
    /// after scoring.
    pub fn search(&self, query: &SearchQuery) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .entries
            .values()
            .filter_map(|entry| {
                if entry.vector.len() != query.vector.len() {
                    return None;
                }
                let score = self.compute_distance(&query.vector, &entry.vector);
                if let Some(min) = query.min_score {
                    if score < min {
                        return None;
                    }
                }
                Some(SearchResult {
                    id: entry.id.clone(),
                    score,
                    metadata: entry.metadata.clone(),
                    content: entry.content.clone(),
                })
            })
            .collect();

        // Sort descending by score (higher is better).
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(query.top_k);
        results
    }

    /// Delete the entry with the given `id`. Returns `true` if it existed.
    pub fn delete(&mut self, id: &str) -> bool {
        self.entries.remove(id).is_some()
    }

    /// Retrieve a reference to the entry with the given `id`.
    pub fn get(&self, id: &str) -> Option<&VectorEntry> {
        self.entries.get(id)
    }

    /// Return the number of stored entries.
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Remove all stored entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Convert the entire in-memory store to a TurboQuant compressed index.
    ///
    /// Achieves ~10× memory reduction while preserving high cosine-similarity
    /// recall. Metadata is carried over as string key-value pairs.
    pub fn to_turboquant(
        &self,
        seed: u64,
    ) -> vibe_core::index::turboquant::TurboQuantIndex {
        let config = vibe_core::index::turboquant::TurboQuantConfig {
            dimension: self.dimension as usize,
            seed,
            qjl_proj_dim: None,
        };
        let mut tq = vibe_core::index::turboquant::TurboQuantIndex::new(config);
        for entry in self.entries.values() {
            let meta: std::collections::HashMap<String, String> = entry
                .metadata
                .iter()
                .map(|(k, v)| (k.clone(), v.to_string()))
                .collect();
            let _ = tq.insert(entry.id.clone(), &entry.vector, meta);
        }
        tq
    }

    /// Compute a similarity / distance score between two vectors.
    ///
    /// For **Cosine** and **DotProduct** higher values mean more similar.
    /// For **Euclidean** and **Manhattan** the raw distance is negated so that
    /// the sort-descending convention still places the best matches first.
    fn compute_distance(&self, a: &[f32], b: &[f32]) -> f64 {
        match self.metric {
            DistanceMetric::Cosine => {
                let mut dot = 0.0_f64;
                let mut norm_a = 0.0_f64;
                let mut norm_b = 0.0_f64;
                for (x, y) in a.iter().zip(b.iter()) {
                    let xf = *x as f64;
                    let yf = *y as f64;
                    dot += xf * yf;
                    norm_a += xf * xf;
                    norm_b += yf * yf;
                }
                let denom = norm_a.sqrt() * norm_b.sqrt();
                if denom == 0.0 {
                    0.0
                } else {
                    dot / denom
                }
            }
            DistanceMetric::Euclidean => {
                let sum: f64 = a
                    .iter()
                    .zip(b.iter())
                    .map(|(x, y)| {
                        let d = (*x as f64) - (*y as f64);
                        d * d
                    })
                    .sum();
                -sum.sqrt() // negate so higher = closer
            }
            DistanceMetric::DotProduct => {
                a.iter()
                    .zip(b.iter())
                    .map(|(x, y)| (*x as f64) * (*y as f64))
                    .sum()
            }
            DistanceMetric::Manhattan => {
                let sum: f64 = a
                    .iter()
                    .zip(b.iter())
                    .map(|(x, y)| ((*x as f64) - (*y as f64)).abs())
                    .sum();
                -sum // negate so higher = closer
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// Validate a `VectorDbConfig`, returning a list of human-readable error
/// strings (empty if valid).
pub fn validate_config(config: &VectorDbConfig) -> Vec<String> {
    let mut errors = Vec::new();

    if config.endpoint.trim().is_empty() {
        errors.push("endpoint must not be empty".to_string());
    }

    if config.dimension == 0 {
        errors.push("dimension must be greater than 0".to_string());
    }

    if config.collection_name.trim().is_empty() {
        errors.push("collection_name must not be empty".to_string());
    }

    // Provider-specific validations
    match config.provider {
        VectorDbProvider::Pinecone => {
            if config.api_key.is_none() {
                errors.push("api_key is required for Pinecone".to_string());
            }
        }
        VectorDbProvider::Qdrant
        | VectorDbProvider::Pgvector
        | VectorDbProvider::Milvus
        | VectorDbProvider::Weaviate
        | VectorDbProvider::Chroma => {
            if !config.endpoint.starts_with("http://") && !config.endpoint.starts_with("https://") {
                errors.push("endpoint must start with http:// or https://".to_string());
            }
        }
        VectorDbProvider::InMemory => { /* no remote endpoint needed */ }
    }

    errors
}

/// Generate a JSON request body for the Qdrant REST API to create a
/// collection with the given `config`.
pub fn generate_qdrant_collection_request(config: &CollectionConfig) -> String {
    let distance = match config.metric {
        DistanceMetric::Cosine => "Cosine",
        DistanceMetric::Euclidean => "Euclid",
        DistanceMetric::DotProduct => "Dot",
        DistanceMetric::Manhattan => "Manhattan",
    };

    let mut obj = serde_json::json!({
        "vectors": {
            "size": config.dimension,
            "distance": distance
        }
    });

    if let Some(ref hnsw) = config.hnsw_config {
        obj["hnsw_config"] = serde_json::json!({
            "m": hnsw.m,
            "ef_construct": hnsw.ef_construction,
        });
    }

    serde_json::to_string_pretty(&obj).expect("failed to serialize Qdrant request")
}

/// Generate a JSON request body for the Pinecone REST upsert endpoint.
pub fn generate_pinecone_upsert_request(entries: &[VectorEntry]) -> String {
    let vectors: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            let mut v = serde_json::json!({
                "id": e.id,
                "values": e.vector,
            });
            if !e.metadata.is_empty() {
                v["metadata"] = serde_json::Value::Object(
                    e.metadata
                        .iter()
                        .map(|(k, val)| (k.clone(), val.clone()))
                        .collect(),
                );
            }
            v
        })
        .collect();

    let body = serde_json::json!({ "vectors": vectors });
    serde_json::to_string_pretty(&body).expect("failed to serialize Pinecone request")
}

/// Generate PostgreSQL DDL for a pgvector-backed table and index.
pub fn generate_pgvector_schema(config: &CollectionConfig) -> String {
    let index_method = match config.metric {
        DistanceMetric::Cosine => "vector_cosine_ops",
        DistanceMetric::Euclidean => "vector_l2_ops",
        DistanceMetric::DotProduct => "vector_ip_ops",
        DistanceMetric::Manhattan => "vector_l1_ops",
    };

    let hnsw_params = config
        .hnsw_config
        .as_ref()
        .map(|h| format!(" WITH (m = {}, ef_construction = {})", h.m, h.ef_construction))
        .unwrap_or_default();

    format!(
        "CREATE EXTENSION IF NOT EXISTS vector;\n\n\
         CREATE TABLE IF NOT EXISTS {name} (\n    \
             id TEXT PRIMARY KEY,\n    \
             embedding vector({dim}),\n    \
             metadata JSONB DEFAULT '{{}}',\n    \
             content TEXT\n\
         );\n\n\
         CREATE INDEX IF NOT EXISTS {name}_embedding_idx\n    \
             ON {name} USING hnsw (embedding {ops}){params};\n",
        name = config.name,
        dim = config.dimension,
        ops = index_method,
        params = hnsw_params,
    )
}

/// Split a list of entries into batches of at most `batch_size`.
pub fn chunk_for_batch(entries: Vec<VectorEntry>, batch_size: usize) -> Vec<Vec<VectorEntry>> {
    if batch_size == 0 {
        return vec![entries];
    }
    entries
        .into_iter()
        .collect::<Vec<_>>()
        .chunks(batch_size)
        .map(|c| c.to_vec())
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(id: &str, vector: Vec<f32>) -> VectorEntry {
        VectorEntry {
            id: id.to_string(),
            vector,
            metadata: HashMap::new(),
            content: None,
        }
    }

    fn make_entry_with_content(id: &str, vector: Vec<f32>, content: &str) -> VectorEntry {
        VectorEntry {
            id: id.to_string(),
            vector,
            metadata: HashMap::new(),
            content: Some(content.to_string()),
        }
    }

    // -- InMemoryVectorDb tests --

    #[test]
    fn test_in_memory_insert_and_search() {
        let mut db = InMemoryVectorDb::new(3, DistanceMetric::Cosine);
        db.insert(make_entry("a", vec![1.0, 0.0, 0.0])).unwrap();
        db.insert(make_entry("b", vec![0.0, 1.0, 0.0])).unwrap();
        db.insert(make_entry("c", vec![1.0, 1.0, 0.0])).unwrap();

        let query = SearchQuery {
            vector: vec![1.0, 0.0, 0.0],
            top_k: 2,
            filter: None,
            min_score: None,
        };
        let results = db.search(&query);
        assert_eq!(results.len(), 2);
        // "a" is an exact match and should be first.
        assert_eq!(results[0].id, "a");
        assert!((results[0].score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_in_memory_cosine_similarity() {
        let db = InMemoryVectorDb::new(3, DistanceMetric::Cosine);
        // identical vectors → cosine 1.0
        let score = db.compute_distance(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]);
        assert!((score - 1.0).abs() < 1e-6);
        // orthogonal vectors → cosine 0.0
        let score = db.compute_distance(&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0]);
        assert!(score.abs() < 1e-6);
    }

    #[test]
    fn test_in_memory_euclidean_distance() {
        let db = InMemoryVectorDb::new(2, DistanceMetric::Euclidean);
        // same point → 0.0 (negated = 0.0)
        let score = db.compute_distance(&[0.0, 0.0], &[0.0, 0.0]);
        assert!((score - 0.0).abs() < 1e-6);
        // (0,0) to (3,4) → distance 5, negated = -5
        let score = db.compute_distance(&[0.0, 0.0], &[3.0, 4.0]);
        assert!((score - (-5.0)).abs() < 1e-6);
    }

    #[test]
    fn test_in_memory_dot_product() {
        let db = InMemoryVectorDb::new(3, DistanceMetric::DotProduct);
        let score = db.compute_distance(&[1.0, 2.0, 3.0], &[4.0, 5.0, 6.0]);
        // 1*4 + 2*5 + 3*6 = 32
        assert!((score - 32.0).abs() < 1e-6);
    }

    #[test]
    fn test_in_memory_manhattan_distance() {
        let db = InMemoryVectorDb::new(3, DistanceMetric::Manhattan);
        let score = db.compute_distance(&[1.0, 2.0, 3.0], &[4.0, 6.0, 3.0]);
        // |1-4| + |2-6| + |3-3| = 3+4+0 = 7, negated = -7
        assert!((score - (-7.0)).abs() < 1e-6);
    }

    #[test]
    fn test_in_memory_delete() {
        let mut db = InMemoryVectorDb::new(2, DistanceMetric::Cosine);
        db.insert(make_entry("x", vec![1.0, 0.0])).unwrap();
        assert_eq!(db.count(), 1);
        assert!(db.delete("x"));
        assert_eq!(db.count(), 0);
        assert!(!db.delete("x")); // already gone
    }

    #[test]
    fn test_in_memory_get() {
        let mut db = InMemoryVectorDb::new(2, DistanceMetric::Cosine);
        db.insert(make_entry_with_content("g1", vec![0.5, 0.5], "hello"))
            .unwrap();
        let entry = db.get("g1").unwrap();
        assert_eq!(entry.content.as_deref(), Some("hello"));
        assert!(db.get("nonexistent").is_none());
    }

    #[test]
    fn test_in_memory_clear() {
        let mut db = InMemoryVectorDb::new(2, DistanceMetric::Cosine);
        db.insert(make_entry("a", vec![1.0, 0.0])).unwrap();
        db.insert(make_entry("b", vec![0.0, 1.0])).unwrap();
        assert_eq!(db.count(), 2);
        db.clear();
        assert_eq!(db.count(), 0);
    }

    #[test]
    fn test_in_memory_dimension_mismatch() {
        let mut db = InMemoryVectorDb::new(3, DistanceMetric::Cosine);
        let result = db.insert(make_entry("bad", vec![1.0, 2.0]));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("dimension mismatch"));
    }

    #[test]
    fn test_in_memory_batch_insert() {
        let mut db = InMemoryVectorDb::new(2, DistanceMetric::Cosine);
        let entries = vec![
            make_entry("b1", vec![1.0, 0.0]),
            make_entry("b2", vec![0.0, 1.0]),
            make_entry("bad", vec![1.0]), // wrong dimension — skipped
        ];
        let inserted = db.insert_batch(entries).unwrap();
        assert_eq!(inserted, 2);
        assert_eq!(db.count(), 2);
    }

    #[test]
    fn test_in_memory_min_score_filter() {
        let mut db = InMemoryVectorDb::new(2, DistanceMetric::Cosine);
        db.insert(make_entry("close", vec![1.0, 0.1])).unwrap();
        db.insert(make_entry("far", vec![0.0, 1.0])).unwrap();

        let query = SearchQuery {
            vector: vec![1.0, 0.0],
            top_k: 10,
            filter: None,
            min_score: Some(0.9),
        };
        let results = db.search(&query);
        // Only "close" should pass the 0.9 threshold
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "close");
    }

    // -- validate_config tests --

    #[test]
    fn test_validate_config_valid() {
        let config = VectorDbConfig {
            provider: VectorDbProvider::InMemory,
            endpoint: "local".to_string(),
            api_key: None,
            collection_name: "test".to_string(),
            dimension: 128,
            metric: DistanceMetric::Cosine,
            extra: HashMap::new(),
        };
        let errors = validate_config(&config);
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_validate_config_empty_endpoint() {
        let config = VectorDbConfig {
            provider: VectorDbProvider::InMemory,
            endpoint: "".to_string(),
            api_key: None,
            collection_name: "test".to_string(),
            dimension: 128,
            metric: DistanceMetric::Cosine,
            extra: HashMap::new(),
        };
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("endpoint")));
    }

    #[test]
    fn test_validate_config_zero_dimension() {
        let config = VectorDbConfig {
            provider: VectorDbProvider::InMemory,
            endpoint: "local".to_string(),
            api_key: None,
            collection_name: "test".to_string(),
            dimension: 0,
            metric: DistanceMetric::Cosine,
            extra: HashMap::new(),
        };
        let errors = validate_config(&config);
        assert!(errors.iter().any(|e| e.contains("dimension")));
    }

    // -- Generator function tests --

    #[test]
    fn test_generate_qdrant_collection() {
        let config = CollectionConfig {
            name: "embeddings".to_string(),
            dimension: 384,
            metric: DistanceMetric::Cosine,
            hnsw_config: Some(HnswConfig {
                m: 16,
                ef_construction: 100,
                ef_search: 64,
            }),
        };
        let json = generate_qdrant_collection_request(&config);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["vectors"]["size"], 384);
        assert_eq!(parsed["vectors"]["distance"], "Cosine");
        assert_eq!(parsed["hnsw_config"]["m"], 16);
    }

    #[test]
    fn test_generate_pinecone_upsert() {
        let entries = vec![make_entry("p1", vec![0.1, 0.2, 0.3])];
        let json = generate_pinecone_upsert_request(&entries);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["vectors"][0]["id"], "p1");
        assert_eq!(parsed["vectors"][0]["values"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_generate_pgvector_schema() {
        let config = CollectionConfig {
            name: "docs".to_string(),
            dimension: 768,
            metric: DistanceMetric::Cosine,
            hnsw_config: None,
        };
        let sql = generate_pgvector_schema(&config);
        assert!(sql.contains("CREATE EXTENSION IF NOT EXISTS vector"));
        assert!(sql.contains("embedding vector(768)"));
        assert!(sql.contains("vector_cosine_ops"));
        assert!(sql.contains("docs"));
    }

    // -- chunk_for_batch tests --

    #[test]
    fn test_chunk_for_batch() {
        let entries: Vec<VectorEntry> = (0..7)
            .map(|i| make_entry(&format!("e{}", i), vec![i as f32]))
            .collect();
        let batches = chunk_for_batch(entries, 3);
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].len(), 3);
        assert_eq!(batches[1].len(), 3);
        assert_eq!(batches[2].len(), 1);
    }

    // -- Serialization round-trip --

    #[test]
    fn test_distance_metric_serialization() {
        let metric = DistanceMetric::DotProduct;
        let json = serde_json::to_string(&metric).unwrap();
        assert_eq!(json, "\"dot_product\"");
        let deserialized: DistanceMetric = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, DistanceMetric::DotProduct);
    }

    #[test]
    fn test_vector_db_provider_serialization() {
        let provider = VectorDbProvider::Pgvector;
        let json = serde_json::to_string(&provider).unwrap();
        assert_eq!(json, "\"pgvector\"");
        let deserialized: VectorDbProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, VectorDbProvider::Pgvector);
    }

    #[test]
    fn test_index_status_error_variant() {
        let status = IndexStatus::Error("timeout".to_string());
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: IndexStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, IndexStatus::Error("timeout".to_string()));
    }
}
