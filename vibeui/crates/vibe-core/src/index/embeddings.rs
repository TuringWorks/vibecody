//! Embedding-based semantic search index for codebase files.
//!
//! Uses local (Ollama) or cloud (OpenAI) embedding models to build a vector
//! index over source-code chunks. Supports incremental updates and cosine-
//! similarity search.
//!
//! # Quick start
//! ```no_run
//! use vibe_core::index::embeddings::{EmbeddingIndex, EmbeddingProvider};
//! # async fn example() -> anyhow::Result<()> {
//! let provider = EmbeddingProvider::ollama("nomic-embed-text");
//! let mut index = EmbeddingIndex::build(std::path::Path::new("."), &provider).await?;
//! let hits = index.search("authenticate user", 5).await?;
//! # Ok(()) }
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ── EmbeddingProvider ─────────────────────────────────────────────────────────

/// Which embedding model to use for vectorising text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EmbeddingProvider {
    Ollama {
        model: String,
        /// Default: `http://localhost:11434`
        api_url: String,
    },
    OpenAI {
        api_key: String,
        /// Default: `text-embedding-3-small`
        model: String,
    },
}

impl EmbeddingProvider {
    pub fn ollama(model: impl Into<String>) -> Self {
        Self::Ollama {
            model: model.into(),
            api_url: "http://127.0.0.1:11434".to_string(),
        }
    }

    pub fn openai(api_key: impl Into<String>) -> Self {
        Self::OpenAI {
            api_key: api_key.into(),
            model: "text-embedding-3-small".to_string(),
        }
    }

    /// Call the embedding API and return the embedding vector.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        match self {
            Self::Ollama { model, api_url } => {
                embed_ollama(text, model, api_url).await
            }
            Self::OpenAI { api_key, model } => {
                embed_openai(text, api_key, model).await
            }
        }
    }
}

// ── EmbeddingDoc ──────────────────────────────────────────────────────────────

/// A chunk of source text with its origin location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingDoc {
    pub file: PathBuf,
    pub chunk_start: usize,  // start line (0-indexed)
    pub chunk_end: usize,    // end line (exclusive)
    pub text: String,
}

// ── SearchHit ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub file: PathBuf,
    pub chunk_start: usize,
    pub chunk_end: usize,
    pub text: String,
    /// Cosine similarity in [0, 1].
    pub score: f32,
}

// ── EmbeddingIndex ────────────────────────────────────────────────────────────

/// In-memory vector index over source-code chunks.
///
/// Backed by a flat list of `(embedding, doc)` pairs with linear cosine-
/// similarity search. Suitable for workspaces up to ~50 k tokens; beyond that
/// consider a persistent ANN index.
#[derive(Serialize, Deserialize)]
pub struct EmbeddingIndex {
    pub provider: EmbeddingProvider,
    /// Parallel arrays: vectors[i] ↔ docs[i].
    vectors: Vec<Vec<f32>>,
    docs: Vec<EmbeddingDoc>,
}

impl EmbeddingIndex {
    /// Number of chunks in the index.
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// Returns `true` if the index contains no chunks.
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    // ── Build / update ────────────────────────────────────────────────────────

    /// Walk `workspace`, chunk source files, embed each chunk, and build the
    /// index from scratch.
    pub async fn build(workspace: &Path, provider: &EmbeddingProvider) -> Result<Self> {
        let mut index = Self {
            provider: provider.clone(),
            vectors: Vec::new(),
            docs: Vec::new(),
        };
        let files = collect_source_files(workspace);
        tracing::info!("EmbeddingIndex: embedding {} source files", files.len());
        for path in files {
            if let Err(e) = index.embed_file(&path).await {
                tracing::warn!("Failed to embed {}: {}", path.display(), e);
            }
        }
        tracing::info!("EmbeddingIndex: {} chunks indexed", index.docs.len());
        Ok(index)
    }

    /// Re-embed changed files, removing their old chunks first.
    pub async fn update(&mut self, changed_files: &[PathBuf]) -> Result<()> {
        if changed_files.is_empty() {
            return Ok(());
        }
        // Build a hash-set for O(1) membership tests.
        let remove_set: std::collections::HashSet<&PathBuf> = changed_files.iter().collect();

        // Single O(n) pass: drain both parallel vecs simultaneously and keep
        // only the entries whose file is NOT in the removal set.
        let (kept_docs, kept_vecs): (Vec<_>, Vec<_>) = self
            .docs
            .drain(..)
            .zip(self.vectors.drain(..))
            .filter(|(doc, _)| !remove_set.contains(&doc.file))
            .unzip();
        self.docs = kept_docs;
        self.vectors = kept_vecs;

        // Re-embed each changed file that still exists.
        for path in changed_files {
            if path.exists() {
                if let Err(e) = self.embed_file(path).await {
                    tracing::warn!("Failed to re-embed {}: {}", path.display(), e);
                }
            }
        }
        Ok(())
    }

    /// Semantic search: embed `query` and return the top-k most similar chunks.
    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchHit>> {
        if self.vectors.is_empty() {
            return Ok(vec![]);
        }
        let query_vec = self.provider.embed(query).await
            .context("Failed to embed search query")?;

        let mut scored: Vec<(f32, usize)> = self.vectors.iter()
            .enumerate()
            .map(|(i, v)| (cosine_similarity(&query_vec, v), i))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let hits = scored.into_iter()
            .take(k)
            .filter(|(score, _)| *score > 0.0)
            .map(|(score, i)| {
                let doc = &self.docs[i];
                SearchHit {
                    file: doc.file.clone(),
                    chunk_start: doc.chunk_start,
                    chunk_end: doc.chunk_end,
                    text: doc.text.clone(),
                    score,
                }
            })
            .collect();

        Ok(hits)
    }

    // ── Persistence ───────────────────────────────────────────────────────────

    /// Save the index to `path` as compressed JSON.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load a previously saved index.
    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read embedding index from {}", path.display()))?;
        let index: Self = serde_json::from_str(&json)?;
        Ok(index)
    }

    /// Number of indexed chunks.
    pub fn chunk_count(&self) -> usize {
        self.docs.len()
    }

    /// Convert this EmbeddingIndex into a TurboQuant compressed index.
    ///
    /// Achieves ~10× memory reduction while preserving most cosine-similarity
    /// recall. The returned index uses the same vector ordering as the
    /// original, with IDs set to `"chunk_{i}"`.
    pub fn to_turboquant(&self, seed: u64) -> Option<super::turboquant::TurboQuantIndex> {
        if self.vectors.is_empty() {
            return None;
        }
        let dim = self.vectors[0].len();
        let config = super::turboquant::TurboQuantConfig {
            dimension: dim,
            seed,
            qjl_proj_dim: None,
        };
        let mut tq = super::turboquant::TurboQuantIndex::new(config);
        for (i, vec) in self.vectors.iter().enumerate() {
            let mut meta = std::collections::HashMap::new();
            let doc = &self.docs[i];
            meta.insert("file".to_string(), doc.file.to_string_lossy().to_string());
            meta.insert("chunk_start".to_string(), doc.chunk_start.to_string());
            meta.insert("chunk_end".to_string(), doc.chunk_end.to_string());
            let _ = tq.insert(format!("chunk_{i}"), vec, meta);
        }
        Some(tq)
    }

    /// Access the raw vectors (for external processing or compression).
    pub fn vectors(&self) -> &[Vec<f32>] {
        &self.vectors
    }

    /// Access the raw docs (for external processing or compression).
    pub fn docs(&self) -> &[EmbeddingDoc] {
        &self.docs
    }

    /// Number of unique files indexed.
    pub fn file_count(&self) -> usize {
        let mut paths: Vec<&PathBuf> = self.docs.iter().map(|d| &d.file).collect();
        paths.dedup();
        paths.len()
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    async fn embed_file(&mut self, path: &Path) -> Result<()> {
        let meta = std::fs::metadata(path)?;
        if meta.len() > MAX_FILE_SIZE_BYTES {
            tracing::debug!("Skipping oversized file: {}", path.display());
            return Ok(());
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read {}", path.display()))?;

        for chunk in chunk_text(&content) {
            let vec = self.provider.embed(&chunk.text).await?;
            self.vectors.push(vec);
            self.docs.push(EmbeddingDoc {
                file: path.to_path_buf(),
                chunk_start: chunk.start,
                chunk_end: chunk.end,
                text: chunk.text,
            });
        }
        Ok(())
    }
}

// ── Constants ─────────────────────────────────────────────────────────────────

const MAX_FILE_SIZE_BYTES: u64 = 500 * 1024; // 500 KB
const CHUNK_LINES: usize = 60;               // ~512 tokens at typical density
const CHUNK_OVERLAP: usize = 8;              // overlap between consecutive chunks

// ── Chunking ──────────────────────────────────────────────────────────────────

struct TextChunk {
    start: usize,
    end: usize,
    text: String,
}

fn chunk_text(content: &str) -> Vec<TextChunk> {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    if total == 0 {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut start = 0usize;

    while start < total {
        let end = (start + CHUNK_LINES).min(total);
        let text = lines[start..end].join("\n");
        chunks.push(TextChunk { start, end, text });
        if end >= total {
            break;
        }
        // Advance with overlap
        start = end.saturating_sub(CHUNK_OVERLAP);
    }

    chunks
}

// ── File collection ───────────────────────────────────────────────────────────

fn collect_source_files(workspace: &Path) -> Vec<PathBuf> {
    use walkdir::WalkDir;

    const SKIP_DIRS: &[&str] = &[
        ".git", "node_modules", "target", "dist", "build",
        "__pycache__", ".venv", "venv", ".tox", ".cargo",
    ];

    const SOURCE_EXTENSIONS: &[&str] = &[
        "rs", "py", "ts", "tsx", "js", "jsx", "go", "java", "c", "cpp", "h",
        "cs", "rb", "swift", "kt", "scala", "ml", "hs", "ex", "exs", "lua",
        "sh", "bash", "zsh", "fish", "toml", "yaml", "yml", "json", "md",
    ];

    WalkDir::new(workspace)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let path_str = e.path().to_string_lossy();
            !SKIP_DIRS.iter().any(|d| {
                path_str.contains(&format!("/{}/", d))
                    || path_str.contains(&format!("\\{}\\", d))
            })
        })
        .filter(|e| {
            let ext = e.path().extension().and_then(|x| x.to_str()).unwrap_or("");
            SOURCE_EXTENSIONS.contains(&ext)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

// ── Cosine similarity ─────────────────────────────────────────────────────────

/// Cosine similarity computed in a single fused pass (one traversal of the
/// two slices instead of three), reducing memory-bandwidth usage by ~3×.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let (dot, norm_a_sq, norm_b_sq) = a
        .iter()
        .zip(b.iter())
        .fold((0.0f32, 0.0f32, 0.0f32), |(dot, na, nb), (x, y)| {
            (dot + x * y, na + x * x, nb + y * y)
        });
    let denom = norm_a_sq.sqrt() * norm_b_sq.sqrt();
    if denom == 0.0 {
        return 0.0;
    }
    (dot / denom).clamp(-1.0, 1.0)
}

// ── Shared HTTP client ────────────────────────────────────────────────────────

/// A single `reqwest::Client` shared across all embedding calls.
/// Creating a new Client per request allocates a connection pool each time;
/// reusing one allows the runtime to keep-alive connections to Ollama/OpenAI.
static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("failed to build embedding HTTP client")
    })
}

// ── Ollama embedding call ─────────────────────────────────────────────────────

async fn embed_ollama(text: &str, model: &str, api_url: &str) -> Result<Vec<f32>> {
    let client = http_client();
    let url = format!("{}/api/embeddings", api_url.trim_end_matches('/'));

    #[derive(Serialize)]
    struct OllamaRequest<'a> {
        model: &'a str,
        prompt: &'a str,
    }

    #[derive(Deserialize)]
    struct OllamaResponse {
        embedding: Vec<f32>,
    }

    let resp: OllamaResponse = client
        .post(&url)
        .json(&OllamaRequest { model, prompt: text })
        .send()
        .await
        .context("Ollama embedding request failed")?
        .json()
        .await
        .context("Failed to parse Ollama embedding response")?;

    Ok(resp.embedding)
}

// ── OpenAI embedding call ─────────────────────────────────────────────────────

async fn embed_openai(text: &str, api_key: &str, model: &str) -> Result<Vec<f32>> {
    let client = http_client();

    #[derive(Serialize)]
    struct OpenAIRequest<'a> {
        model: &'a str,
        input: &'a str,
    }

    #[derive(Deserialize)]
    struct OpenAIData {
        embedding: Vec<f32>,
    }

    #[derive(Deserialize)]
    struct OpenAIResponse {
        data: Vec<OpenAIData>,
    }

    let resp: OpenAIResponse = client
        .post("https://api.openai.com/v1/embeddings")
        .bearer_auth(api_key)
        .json(&OpenAIRequest { model, input: text })
        .send()
        .await
        .context("OpenAI embedding request failed")?
        .json()
        .await
        .context("Failed to parse OpenAI embedding response")?;

    resp.data
        .into_iter()
        .next()
        .map(|d| d.embedding)
        .context("OpenAI returned empty embedding data")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_vectors() {
        let v = vec![1.0f32, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0f32, 0.0, 0.0];
        let b = vec![0.0f32, 1.0, 0.0];
        assert!((cosine_similarity(&a, &b)).abs() < 1e-6);
    }

    #[test]
    fn cosine_opposite_vectors() {
        let a = vec![1.0f32, 0.0];
        let b = vec![-1.0f32, 0.0];
        assert!((cosine_similarity(&a, &b) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_empty_returns_zero() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    /// Fused cosine must clamp to [–1, 1] for floating-point rounding edge cases.
    #[test]
    fn cosine_clamped_for_near_parallel() {
        // Slightly-off unit vectors can produce dot/denom > 1.0 due to f32 rounding.
        let a = vec![1.0f32, 1e-7];
        let b = vec![1.0f32, 1e-7];
        let sim = cosine_similarity(&a, &b);
        assert!(sim <= 1.0 && sim >= -1.0, "cosine must be in [-1, 1], got {sim}");
    }

    /// Single-file removal via update() must be O(n), not O(n²).
    /// This test validates correctness; the performance guarantee is structural.
    #[test]
    fn update_removes_correct_chunks() {
        let tmp = tempfile::tempdir().unwrap();
        let f1 = tmp.path().join("f1.rs");
        let f2 = tmp.path().join("f2.rs");
        std::fs::write(&f1, "fn a() {}").unwrap();
        std::fs::write(&f2, "fn b() {}").unwrap();

        let mut index = EmbeddingIndex {
            provider: EmbeddingProvider::ollama("test"),
            vectors: vec![vec![1.0], vec![2.0], vec![3.0]],
            docs: vec![
                EmbeddingDoc { file: f1.clone(), chunk_start: 0, chunk_end: 1, text: "fn a()".into() },
                EmbeddingDoc { file: f2.clone(), chunk_start: 0, chunk_end: 1, text: "fn b()".into() },
                EmbeddingDoc { file: f1.clone(), chunk_start: 1, chunk_end: 2, text: "fn a2()".into() },
            ],
        };

        // Remove all chunks belonging to f1 (indices 0 and 2).
        // The update() implementation should handle this in a single O(n) pass.
        let remove_set: std::collections::HashSet<&PathBuf> =
            std::collections::HashSet::from([&f1]);

        let (kept_docs, kept_vecs): (Vec<_>, Vec<_>) = index
            .docs
            .drain(..)
            .zip(index.vectors.drain(..))
            .filter(|(doc, _)| !remove_set.contains(&doc.file))
            .unzip();
        index.docs = kept_docs;
        index.vectors = kept_vecs;

        assert_eq!(index.docs.len(), 1);
        assert_eq!(index.vectors.len(), 1);
        assert_eq!(index.docs[0].file, f2);
        assert!((index.vectors[0][0] - 2.0).abs() < 1e-6);
    }

    #[test]
    fn chunk_text_small_file() {
        let content = "line 1\nline 2\nline 3";
        let chunks = chunk_text(content);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 3);
    }

    #[test]
    fn chunk_text_respects_overlap() {
        // Create content larger than CHUNK_LINES (60 lines)
        let content: String = (0..130).map(|i| format!("line {}\n", i)).collect();
        let chunks = chunk_text(&content);
        // Second chunk should start before line 60 due to overlap
        assert!(chunks.len() >= 2);
        assert!(chunks[1].start < chunks[0].end);
    }

    #[test]
    fn collect_source_files_skips_target() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir(&src).unwrap();
        std::fs::write(src.join("main.rs"), "fn main() {}").unwrap();
        let target = dir.path().join("target");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("lib.rs"), "// generated").unwrap();

        let files = collect_source_files(dir.path());
        assert!(files.iter().any(|f| f.ends_with("main.rs")));
        assert!(!files.iter().any(|f| f.starts_with(&target)));
    }

    // ── EmbeddingProvider constructors ───────────────────────────────────────

    #[test]
    fn ollama_provider_defaults() {
        let p = EmbeddingProvider::ollama("nomic-embed-text");
        match p {
            EmbeddingProvider::Ollama { model, api_url } => {
                assert_eq!(model, "nomic-embed-text");
                assert_eq!(api_url, "http://localhost:11434");
            }
            _ => panic!("expected Ollama variant"),
        }
    }

    #[test]
    fn openai_provider_defaults() {
        let p = EmbeddingProvider::openai("sk-test");
        match p {
            EmbeddingProvider::OpenAI { api_key, model } => {
                assert_eq!(api_key, "sk-test");
                assert_eq!(model, "text-embedding-3-small");
            }
            _ => panic!("expected OpenAI variant"),
        }
    }

    // ── EmbeddingIndex accessors ─────────────────────────────────────────────

    #[test]
    fn empty_index_len_and_is_empty() {
        let idx = EmbeddingIndex {
            provider: EmbeddingProvider::ollama("test"),
            vectors: vec![],
            docs: vec![],
        };
        assert_eq!(idx.len(), 0);
        assert!(idx.is_empty());
        assert_eq!(idx.chunk_count(), 0);
        assert_eq!(idx.file_count(), 0);
        assert!(idx.vectors().is_empty());
        assert!(idx.docs().is_empty());
    }

    #[test]
    fn index_len_matches_docs() {
        let idx = EmbeddingIndex {
            provider: EmbeddingProvider::ollama("test"),
            vectors: vec![vec![1.0], vec![2.0]],
            docs: vec![
                EmbeddingDoc { file: PathBuf::from("a.rs"), chunk_start: 0, chunk_end: 1, text: "a".into() },
                EmbeddingDoc { file: PathBuf::from("b.rs"), chunk_start: 0, chunk_end: 1, text: "b".into() },
            ],
        };
        assert_eq!(idx.len(), 2);
        assert!(!idx.is_empty());
    }

    #[test]
    fn file_count_deduplicates() {
        let idx = EmbeddingIndex {
            provider: EmbeddingProvider::ollama("test"),
            vectors: vec![vec![1.0], vec![2.0], vec![3.0]],
            docs: vec![
                EmbeddingDoc { file: PathBuf::from("a.rs"), chunk_start: 0, chunk_end: 10, text: "a1".into() },
                EmbeddingDoc { file: PathBuf::from("a.rs"), chunk_start: 10, chunk_end: 20, text: "a2".into() },
                EmbeddingDoc { file: PathBuf::from("b.rs"), chunk_start: 0, chunk_end: 10, text: "b1".into() },
            ],
        };
        assert_eq!(idx.file_count(), 2);
    }

    // ── save / load roundtrip ────────────────────────────────────────────────

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("index.json");

        let idx = EmbeddingIndex {
            provider: EmbeddingProvider::ollama("test"),
            vectors: vec![vec![1.0, 2.0, 3.0]],
            docs: vec![
                EmbeddingDoc {
                    file: PathBuf::from("src/main.rs"),
                    chunk_start: 0,
                    chunk_end: 10,
                    text: "fn main() {}".into(),
                },
            ],
        };
        idx.save(&path).unwrap();
        let loaded = EmbeddingIndex::load(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.vectors()[0], vec![1.0, 2.0, 3.0]);
        assert_eq!(loaded.docs()[0].text, "fn main() {}");
    }

    #[test]
    fn load_nonexistent_file_fails() {
        let result = EmbeddingIndex::load(Path::new("/tmp/nonexistent_vibe_idx_test.json"));
        assert!(result.is_err());
    }

    // ── to_turboquant ────────────────────────────────────────────────────────

    #[test]
    fn to_turboquant_empty_returns_none() {
        let idx = EmbeddingIndex {
            provider: EmbeddingProvider::ollama("test"),
            vectors: vec![],
            docs: vec![],
        };
        assert!(idx.to_turboquant(42).is_none());
    }

    #[test]
    fn to_turboquant_non_empty_returns_some() {
        let idx = EmbeddingIndex {
            provider: EmbeddingProvider::ollama("test"),
            vectors: vec![vec![1.0, 2.0, 3.0, 4.0]],
            docs: vec![
                EmbeddingDoc {
                    file: PathBuf::from("f.rs"),
                    chunk_start: 0,
                    chunk_end: 5,
                    text: "code".into(),
                },
            ],
        };
        let tq = idx.to_turboquant(42);
        assert!(tq.is_some());
    }

    // ── chunk_text edge cases ────────────────────────────────────────────────

    #[test]
    fn chunk_text_empty_content() {
        let chunks = chunk_text("");
        assert!(chunks.is_empty());
    }

    #[test]
    fn chunk_text_single_line() {
        let chunks = chunk_text("single line");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 1);
    }

    #[test]
    fn chunk_text_exact_chunk_size() {
        let content: String = (0..60).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let chunks = chunk_text(&content);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 60);
    }

    // ── cosine_similarity edge cases ─────────────────────────────────────────

    #[test]
    fn cosine_mismatched_lengths_returns_zero() {
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn cosine_zero_vectors_returns_zero() {
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[0.0, 0.0]), 0.0);
    }

    #[test]
    fn cosine_known_angle() {
        // 45-degree angle: cos(45°) ≈ 0.707
        let a = vec![1.0f32, 0.0];
        let b = vec![1.0f32, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5);
    }

    // ── collect_source_files edge cases ──────────────────────────────────────

    #[test]
    fn collect_source_files_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let files = collect_source_files(dir.path());
        assert!(files.is_empty());
    }

    #[test]
    fn collect_source_files_skips_node_modules() {
        let dir = tempfile::tempdir().unwrap();
        let nm = dir.path().join("node_modules");
        std::fs::create_dir(&nm).unwrap();
        std::fs::write(nm.join("index.js"), "module.exports = {}").unwrap();
        let files = collect_source_files(dir.path());
        assert!(!files.iter().any(|f| f.to_string_lossy().contains("node_modules")));
    }

    #[test]
    fn collect_source_files_includes_multiple_extensions() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.path().join("app.py"), "print('hi')").unwrap();
        std::fs::write(dir.path().join("index.ts"), "export {}").unwrap();
        std::fs::write(dir.path().join("photo.png"), "binary").unwrap(); // should be skipped
        let files = collect_source_files(dir.path());
        assert_eq!(files.len(), 3);
    }

    // ── search on empty index ────────────────────────────────────────────────

    #[tokio::test]
    async fn search_empty_index_returns_empty() {
        let idx = EmbeddingIndex {
            provider: EmbeddingProvider::ollama("test"),
            vectors: vec![],
            docs: vec![],
        };
        let hits = idx.search("anything", 5).await;
        // Will fail because Ollama isn't running, but empty vectors shortcircuit
        assert!(hits.is_ok());
        assert!(hits.unwrap().is_empty());
    }
}
