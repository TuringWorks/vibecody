//! Embedding-based semantic search index for codebase files.
//!
//! Chunks source files, embeds each chunk through [`vibe_embed`], and answers
//! cosine-similarity queries. Any provider and model `vibe-embed` supports
//! works here — Ollama, OpenAI (and OpenAI-compatible servers), Voyage,
//! Cohere, Gemini, or an in-process candle model.
//!
//! # Indexes are per-model, side by side
//!
//! Vectors from two different models are not comparable, so each model gets
//! its own file named after [`ModelRef::slug`]:
//!
//! ```text
//! .vibecli/index/
//!   index__ollama__nomic-embed-text.json        + .meta.json
//!   index__voyage__voyage-code-3.json           + .meta.json
//! ```
//!
//! Switching models is therefore instant and reversible: the old index is
//! still on disk, still valid, and still the one used if the user switches
//! back. [`index_path`] computes the path; [`list_indexes`] enumerates what
//! has been built, reading only the small `.meta.json` sidecars.
//!
//! # The header is the contract
//!
//! Every index carries an [`IndexHeader`]: format version, the model that
//! built it, and the **observed** dimension — the length of the vectors
//! actually stored, not a number from a lookup table. [`load`](EmbeddingIndex::load)
//! rejects a future format version, and [`attach`](EmbeddingIndex::attach)
//! rejects an embedder that does not match the header. Neither check existed
//! before, and without them a model change silently returned nonsense scores.
//!
//! # Credentials are never persisted
//!
//! The previous version of this file serialised the whole provider — API key
//! included — into a plaintext `index.json`. The header stores a [`ModelRef`]
//! only. The embedder, and therefore the key, is supplied at runtime by the
//! caller from the encrypted ProfileStore and is `#[serde(skip)]`.
//!
//! # Quick start
//! ```no_run
//! use vibe_core::index::embeddings::EmbeddingIndex;
//! use vibe_embed::{EmbeddingConfig, ModelRef, ProviderKind};
//! # async fn example() -> anyhow::Result<()> {
//! let embedder = EmbeddingConfig::new(
//!     ModelRef::new(ProviderKind::Ollama, "nomic-embed-text"),
//! ).build()?;
//! let index = EmbeddingIndex::build(std::path::Path::new("."), embedder).await?;
//! let hits = index.search("authenticate user", 5).await?;
//! # let _ = hits;
//! # Ok(()) }
//! ```

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use vibe_embed::{EmbedKind, ModelRef, ProviderKind, SharedEmbedder};

// ── Format version ────────────────────────────────────────────────────────────

/// Bumped whenever the on-disk shape changes incompatibly.
///
/// v1 (unversioned) stored the provider — API key and all — and no dimension.
/// v2 stores a header with the model identity and observed dimension.
pub const INDEX_FORMAT_VERSION: u32 = 2;

// ── IndexHeader ───────────────────────────────────────────────────────────────

/// Identity and shape of an index. Written both inside the index file and to
/// a `.meta.json` sidecar so listing does not require parsing every vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexHeader {
    pub format_version: u32,
    /// Model that produced every vector in this index.
    pub model: ModelRef,
    /// Length of the stored vectors, as measured when they were produced.
    /// `None` only for an index with no vectors yet.
    #[serde(default)]
    pub dimension: Option<usize>,
    pub chunk_count: usize,
    pub file_count: usize,
    /// Unix seconds when the index was last written. `None` if the system
    /// clock was unreadable — absent rather than a stand-in value.
    #[serde(default)]
    pub built_at: Option<u64>,
}

impl IndexHeader {
    fn new(model: ModelRef) -> Self {
        Self {
            format_version: INDEX_FORMAT_VERSION,
            model,
            dimension: None,
            chunk_count: 0,
            file_count: 0,
            built_at: None,
        }
    }

    /// Whether an embedder can serve queries against this index.
    ///
    /// Requires the same model *and*, when the embedder declares one, the same
    /// dimension. An explicitly-truncated Matryoshka variant is a different
    /// model for this purpose — its slug and `ModelRef` differ.
    pub fn accepts(&self, embedder: &dyn vibe_embed::Embedder) -> bool {
        if embedder.model() != &self.model {
            return false;
        }
        match (self.dimension, embedder.dim()) {
            (Some(stored), Some(declared)) => stored == declared,
            _ => true,
        }
    }
}

fn now_unix() -> Option<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

// ── Paths ─────────────────────────────────────────────────────────────────────

/// Path of the index for `model` inside `dir`.
pub fn index_path(dir: &Path, model: &ModelRef) -> PathBuf {
    dir.join(format!("index__{}.json", model.slug()))
}

fn meta_path(index_path: &Path) -> PathBuf {
    index_path.with_extension("meta.json")
}

/// Headers of every index already built in `dir`, newest first.
///
/// Reads only the sidecars, so this stays fast with multi-hundred-megabyte
/// indexes on disk. A sidecar that fails to parse is skipped rather than
/// failing the whole listing — one corrupt file should not hide the others.
pub fn list_indexes(dir: &Path) -> Vec<(PathBuf, IndexHeader)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<(PathBuf, IndexHeader)> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("index__") && n.ends_with(".meta.json"))
        })
        .filter_map(|meta| {
            let text = std::fs::read_to_string(&meta).ok()?;
            let header: IndexHeader = serde_json::from_str(&text).ok()?;
            // index__x.meta.json → index__x.json
            let index = meta.with_extension("").with_extension("json");
            Some((index, header))
        })
        .collect();
    found.sort_by_key(|(_, h)| std::cmp::Reverse(h.built_at.unwrap_or(0)));
    found
}

// ── EmbeddingDoc / SearchHit ──────────────────────────────────────────────────

/// A chunk of source text with its origin location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingDoc {
    pub file: PathBuf,
    pub chunk_start: usize, // start line (0-indexed)
    pub chunk_end: usize,   // end line (exclusive)
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub file: PathBuf,
    pub chunk_start: usize,
    pub chunk_end: usize,
    pub text: String,
    /// Cosine similarity in [-1, 1].
    pub score: f32,
}

// ── EmbeddingIndex ────────────────────────────────────────────────────────────

/// In-memory vector index over source-code chunks.
///
/// Backed by parallel `(vector, doc)` arrays with linear cosine search.
/// Convert to a [`TurboQuantIndex`](super::turboquant::TurboQuantIndex) via
/// [`to_turboquant`](Self::to_turboquant) for a ~10× smaller footprint.
#[derive(Serialize, Deserialize)]
pub struct EmbeddingIndex {
    pub header: IndexHeader,
    /// Parallel arrays: vectors[i] ↔ docs[i].
    vectors: Vec<Vec<f32>>,
    docs: Vec<EmbeddingDoc>,
    /// Supplied at runtime, never persisted — it holds the API key.
    #[serde(skip)]
    embedder: Option<SharedEmbedder>,
}

/// Prints the header only. Formatting the vectors would dump hundreds of
/// megabytes into a log line, and the embedder holds an API key.
impl std::fmt::Debug for EmbeddingIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddingIndex")
            .field("model", &self.header.model.to_string())
            .field("dimension", &self.header.dimension)
            .field("chunks", &self.docs.len())
            .field("embedder_attached", &self.embedder.is_some())
            .finish()
    }
}

impl EmbeddingIndex {
    /// An empty index for `model`, with no embedder attached yet.
    pub fn empty(model: ModelRef) -> Self {
        Self {
            header: IndexHeader::new(model),
            vectors: Vec::new(),
            docs: Vec::new(),
            embedder: None,
        }
    }

    /// Number of chunks in the index.
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// Returns `true` if the index contains no chunks.
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// The model this index was built with.
    pub fn model(&self) -> &ModelRef {
        &self.header.model
    }

    /// Observed vector length, or `None` for an empty index.
    pub fn dimension(&self) -> Option<usize> {
        self.header.dimension
    }

    /// Attach the embedder used to vectorise queries.
    ///
    /// Fails if it is a different model, or the same model at a different
    /// dimension. Answering a query with a mismatched embedder does not error
    /// at the maths level — `cosine_similarity` just returns `0.0` for every
    /// mismatched pair — so the index would look empty rather than broken.
    pub fn attach(&mut self, embedder: SharedEmbedder) -> Result<()> {
        if !self.header.accepts(embedder.as_ref()) {
            return Err(anyhow!(
                "index was built with {} ({} dimensions) but the supplied embedder is {} ({}); \
                 build an index for this model instead — existing indexes are kept side by side",
                self.header.model,
                self.header
                    .dimension
                    .map_or_else(|| "unknown".to_string(), |d| d.to_string()),
                embedder.model(),
                embedder
                    .dim()
                    .map_or_else(|| "unknown".to_string(), |d| d.to_string()),
            ));
        }
        self.embedder = Some(embedder);
        Ok(())
    }

    /// Builder form of [`attach`](Self::attach).
    pub fn with_embedder(mut self, embedder: SharedEmbedder) -> Result<Self> {
        self.attach(embedder)?;
        Ok(self)
    }

    fn embedder(&self) -> Result<&SharedEmbedder> {
        self.embedder.as_ref().ok_or_else(|| {
            anyhow!(
                "no embedder attached to this index — call attach() with an embedder for {}",
                self.header.model
            )
        })
    }

    // ── Build / update ────────────────────────────────────────────────────────

    /// Walk `workspace`, chunk source files, embed every chunk, and build the
    /// index from scratch.
    pub async fn build(workspace: &Path, embedder: SharedEmbedder) -> Result<Self> {
        Self::build_filtered(workspace, embedder, |_| true).await
    }

    /// [`build`](Self::build) with a caller-supplied file filter.
    pub async fn build_filtered<F>(
        workspace: &Path,
        embedder: SharedEmbedder,
        accept: F,
    ) -> Result<Self>
    where
        F: Fn(&Path) -> bool,
    {
        let mut index = Self {
            header: IndexHeader::new(embedder.model().clone()),
            vectors: Vec::new(),
            docs: Vec::new(),
            embedder: Some(embedder),
        };
        let files: Vec<PathBuf> = collect_source_files(workspace)
            .into_iter()
            .filter(|p| accept(p))
            .collect();
        tracing::info!(
            "EmbeddingIndex: embedding {} source files with {}",
            files.len(),
            index.header.model
        );
        index.embed_files(&files).await?;
        tracing::info!(
            "EmbeddingIndex: {} chunks, {} dimensions",
            index.docs.len(),
            index
                .header
                .dimension
                .map_or_else(|| "?".to_string(), |d| d.to_string())
        );
        Ok(index)
    }

    /// Re-embed changed files, removing their old chunks first.
    pub async fn update(&mut self, changed_files: &[PathBuf]) -> Result<()> {
        if changed_files.is_empty() {
            return Ok(());
        }
        let remove_set: std::collections::HashSet<&PathBuf> = changed_files.iter().collect();

        // Single O(n) pass: drain both parallel vecs and keep only the entries
        // whose file is NOT in the removal set.
        let (kept_docs, kept_vecs): (Vec<_>, Vec<_>) = self
            .docs
            .drain(..)
            .zip(self.vectors.drain(..))
            .filter(|(doc, _)| !remove_set.contains(&doc.file))
            .unzip();
        self.docs = kept_docs;
        self.vectors = kept_vecs;

        let existing: Vec<PathBuf> = changed_files
            .iter()
            .filter(|p| p.exists())
            .cloned()
            .collect();
        self.embed_files(&existing).await
    }

    /// Semantic search: embed `query` and return the top-k most similar chunks.
    pub async fn search(&self, query: &str, k: usize) -> Result<Vec<SearchHit>> {
        if self.vectors.is_empty() || k == 0 {
            return Ok(vec![]);
        }
        // EmbedKind::Query is the whole point of the asymmetric models: the
        // stored chunks went in as Document, and querying as Document too
        // measurably costs recall on nomic / mxbai / Voyage / Cohere / Gemini.
        let query_vec = self
            .embedder()?
            .embed(query, EmbedKind::Query)
            .await
            .context("Failed to embed search query")?;

        if let Some(dim) = self.header.dimension {
            if query_vec.len() != dim {
                return Err(anyhow!(
                    "query embedding is {} dimensions but the index stores {dim}; \
                     the model behind {} changed shape — rebuild this index",
                    query_vec.len(),
                    self.header.model,
                ));
            }
        }

        let mut scored: Vec<(f32, usize)> = self
            .vectors
            .iter()
            .enumerate()
            .map(|(i, v)| (cosine_similarity(&query_vec, v), i))
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let hits = scored
            .into_iter()
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

    /// Save the index and its `.meta.json` sidecar.
    pub fn save(&mut self, path: &Path) -> Result<()> {
        self.refresh_header();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string(self)?)
            .with_context(|| format!("Cannot write embedding index to {}", path.display()))?;
        std::fs::write(meta_path(path), serde_json::to_string_pretty(&self.header)?)
            .with_context(|| format!("Cannot write index metadata beside {}", path.display()))?;
        Ok(())
    }

    /// Save to the per-model path inside `dir`, returning where it went.
    pub fn save_in(&mut self, dir: &Path) -> Result<PathBuf> {
        let path = index_path(dir, &self.header.model);
        self.save(&path)?;
        Ok(path)
    }

    /// Load an index. No embedder is attached — call
    /// [`attach`](Self::attach) before searching.
    ///
    /// Accepts the unversioned v1 layout written by earlier builds and
    /// migrates it in memory (dropping the API key v1 stored in plaintext).
    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)
            .with_context(|| format!("Cannot read embedding index from {}", path.display()))?;

        match serde_json::from_str::<Self>(&json) {
            Ok(index) => {
                if index.header.format_version > INDEX_FORMAT_VERSION {
                    return Err(anyhow!(
                        "index at {} is format v{} but this build understands up to v{}; \
                         upgrade VibeCody or rebuild the index",
                        path.display(),
                        index.header.format_version,
                        INDEX_FORMAT_VERSION,
                    ));
                }
                Ok(index)
            }
            Err(new_err) => legacy::migrate(&json).map_err(|legacy_err| {
                anyhow!(
                    "Cannot parse embedding index at {}: {new_err} \
                     (also tried the legacy format: {legacy_err})",
                    path.display()
                )
            }),
        }
    }

    /// Load the index for `model` from `dir`, if one has been built.
    pub fn load_for(dir: &Path, model: &ModelRef) -> Option<Result<Self>> {
        let path = index_path(dir, model);
        path.exists().then(|| Self::load(&path))
    }

    /// Number of indexed chunks.
    pub fn chunk_count(&self) -> usize {
        self.docs.len()
    }

    /// Convert this index into a TurboQuant compressed index.
    ///
    /// Returns `None` for an empty index. Errors if a vector does not match
    /// the index dimension — TurboQuant silently dropped such vectors before,
    /// which turned a model mix-up into a quietly incomplete index.
    pub fn to_turboquant(&self, seed: u64) -> Option<Result<super::turboquant::TurboQuantIndex>> {
        let dim = self.header.dimension?;
        if self.vectors.is_empty() {
            return None;
        }
        let config = super::turboquant::TurboQuantConfig {
            dimension: dim,
            seed,
            qjl_proj_dim: None,
        };
        let mut tq = super::turboquant::TurboQuantIndex::new(config);
        let inserted = self.vectors.iter().enumerate().try_for_each(|(i, vec)| {
            let doc = &self.docs[i];
            let meta = std::collections::HashMap::from([
                ("file".to_string(), doc.file.to_string_lossy().to_string()),
                ("chunk_start".to_string(), doc.chunk_start.to_string()),
                ("chunk_end".to_string(), doc.chunk_end.to_string()),
            ]);
            tq.insert(format!("chunk_{i}"), vec, meta)
                .map_err(|e| anyhow!("chunk {i} ({}): {e}", doc.file.display()))
        });
        Some(inserted.map(|()| tq))
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
        self.docs
            .iter()
            .map(|d| &d.file)
            .collect::<std::collections::HashSet<_>>()
            .len()
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn refresh_header(&mut self) {
        self.header.format_version = INDEX_FORMAT_VERSION;
        self.header.chunk_count = self.docs.len();
        self.header.file_count = self.file_count();
        self.header.dimension = self.vectors.first().map(|v| v.len());
        self.header.built_at = now_unix();
    }

    /// Chunk `files`, embed every chunk in provider-sized batches, and append.
    ///
    /// Batching is what makes a full-workspace index practical: the previous
    /// implementation issued one HTTP request per chunk, so a 5 000-chunk
    /// repository meant 5 000 sequential round-trips.
    async fn embed_files(&mut self, files: &[PathBuf]) -> Result<()> {
        let embedder = self.embedder()?.clone();

        let pending: Vec<(PathBuf, TextChunk)> = files
            .iter()
            .filter_map(|path| match read_indexable(path) {
                Ok(Some(content)) => Some((path.clone(), content)),
                Ok(None) => None,
                Err(e) => {
                    tracing::warn!("Skipping {}: {e}", path.display());
                    None
                }
            })
            .flat_map(|(path, content)| {
                chunk_text(&content)
                    .into_iter()
                    .map(move |c| (path.clone(), c))
            })
            .collect();

        if pending.is_empty() {
            return Ok(());
        }

        let texts: Vec<String> = pending.iter().map(|(_, c)| c.text.clone()).collect();
        let vectors = embedder
            .embed_all(&texts, EmbedKind::Document)
            .await
            .with_context(|| format!("Embedding failed via {}", self.header.model))?;

        // The index dimension is whatever the model actually produced. Every
        // vector must agree with it — a ragged index scores 0.0 on the odd
        // ones out and looks like a relevance problem, not a data problem.
        let expected = self
            .header
            .dimension
            .or_else(|| vectors.first().map(|v| v.len()))
            .ok_or_else(|| anyhow!("embedder returned no vectors"))?;

        for ((path, chunk), vector) in pending.into_iter().zip(vectors) {
            if vector.len() != expected {
                return Err(anyhow!(
                    "{} produced a {}-dimension vector for {} but {expected} for earlier chunks",
                    self.header.model,
                    vector.len(),
                    path.display(),
                ));
            }
            self.vectors.push(vector);
            self.docs.push(EmbeddingDoc {
                file: path,
                chunk_start: chunk.start,
                chunk_end: chunk.end,
                text: chunk.text,
            });
        }

        self.header.dimension = Some(expected);
        Ok(())
    }
}

// ── Legacy (v1) migration ─────────────────────────────────────────────────────

mod legacy {
    use super::{EmbeddingDoc, EmbeddingIndex, IndexHeader, INDEX_FORMAT_VERSION};
    use anyhow::Result;
    use serde::Deserialize;
    use vibe_embed::{ModelRef, ProviderKind};

    /// The pre-versioning on-disk shape. `api_key` was persisted in plaintext;
    /// migration reads the model name and discards the credential.
    #[derive(Deserialize)]
    #[serde(tag = "type", rename_all = "snake_case")]
    enum V1Provider {
        Ollama {
            model: String,
        },
        #[serde(rename = "open_ai", alias = "openai")]
        OpenAI {
            model: String,
        },
    }

    #[derive(Deserialize)]
    struct V1Index {
        provider: V1Provider,
        vectors: Vec<Vec<f32>>,
        docs: Vec<EmbeddingDoc>,
    }

    pub(super) fn migrate(json: &str) -> Result<EmbeddingIndex> {
        let v1: V1Index = serde_json::from_str(json)?;
        let model = match v1.provider {
            V1Provider::Ollama { model } => ModelRef::new(ProviderKind::Ollama, model),
            V1Provider::OpenAI { model } => ModelRef::new(ProviderKind::OpenAI, model),
        };
        tracing::info!(
            "Migrating a v1 embedding index built with {model} to v{INDEX_FORMAT_VERSION}"
        );
        let dimension = v1.vectors.first().map(|v| v.len());
        let file_count = v1
            .docs
            .iter()
            .map(|d| &d.file)
            .collect::<std::collections::HashSet<_>>()
            .len();
        Ok(EmbeddingIndex {
            header: IndexHeader {
                format_version: INDEX_FORMAT_VERSION,
                model,
                dimension,
                chunk_count: v1.docs.len(),
                file_count,
                built_at: None,
            },
            vectors: v1.vectors,
            docs: v1.docs,
            embedder: None,
        })
    }
}

// ── Constants ─────────────────────────────────────────────────────────────────

const MAX_FILE_SIZE_BYTES: u64 = 500 * 1024; // 500 KB
const CHUNK_LINES: usize = 60; // ~512 tokens at typical density
const CHUNK_OVERLAP: usize = 8; // overlap between consecutive chunks

// ── Chunking ──────────────────────────────────────────────────────────────────

struct TextChunk {
    start: usize,
    end: usize,
    text: String,
}

/// Read a file if it is small enough and valid UTF-8. `Ok(None)` means
/// "deliberately skipped", distinct from `Err` meaning "could not read".
fn read_indexable(path: &Path) -> Result<Option<String>> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_FILE_SIZE_BYTES {
        tracing::debug!("Skipping oversized file: {}", path.display());
        return Ok(None);
    }
    Ok(Some(std::fs::read_to_string(path)?))
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
        ".git",
        "node_modules",
        "target",
        "dist",
        "build",
        "__pycache__",
        ".venv",
        "venv",
        ".tox",
        ".cargo",
    ];

    const SOURCE_EXTENSIONS: &[&str] = &[
        "rs", "py", "ts", "tsx", "js", "jsx", "go", "java", "c", "cpp", "h", "cs", "rb", "swift",
        "kt", "scala", "ml", "hs", "ex", "exs", "lua", "sh", "bash", "zsh", "fish", "toml", "yaml",
        "yml", "json", "md",
    ];

    WalkDir::new(workspace)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            let path_str = e.path().to_string_lossy();
            !SKIP_DIRS.iter().any(|d| {
                path_str.contains(&format!("/{}/", d)) || path_str.contains(&format!("\\{}\\", d))
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
///
/// Returns `0.0` for mismatched lengths. That is a safe arithmetic answer but
/// a terrible diagnostic, which is why callers check dimensions up front
/// rather than relying on it.
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

/// Convenience: the providers whose keys the app must supply before their
/// embedding models can be offered.
pub fn providers_needing_keys() -> impl Iterator<Item = ProviderKind> {
    ProviderKind::ALL
        .iter()
        .copied()
        .filter(|p| p.requires_api_key())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Arc;
    use vibe_embed::Embedder;

    /// Deterministic embedder: no network, fixed dimension, distinct vectors
    /// per input so ranking is meaningful.
    struct FakeEmbedder {
        model: ModelRef,
        dim: usize,
    }

    impl FakeEmbedder {
        fn shared(provider: ProviderKind, name: &str, dim: usize) -> SharedEmbedder {
            Arc::new(Self {
                model: ModelRef::new(provider, name),
                dim,
            })
        }
    }

    #[async_trait]
    impl Embedder for FakeEmbedder {
        fn model(&self) -> &ModelRef {
            &self.model
        }
        fn dim(&self) -> Option<usize> {
            Some(self.dim)
        }
        async fn embed_batch(
            &self,
            texts: &[String],
            _kind: EmbedKind,
        ) -> vibe_embed::Result<Vec<Vec<f32>>> {
            Ok(texts
                .iter()
                .map(|t| {
                    (0..self.dim)
                        .map(|i| ((t.len() + i) % 7) as f32 + 0.5)
                        .collect()
                })
                .collect())
        }
    }

    fn workspace_with_files() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("a.rs"), "fn alpha() {}\nfn beta() {}").expect("write");
        std::fs::write(dir.path().join("b.py"), "def gamma():\n    pass").expect("write");
        dir
    }

    // ── Header / identity ────────────────────────────────────────────────────

    #[tokio::test]
    async fn build_records_the_observed_dimension() {
        let ws = workspace_with_files();
        let index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "m", 16),
        )
        .await
        .expect("builds");
        assert_eq!(index.dimension(), Some(16));
        assert!(index.len() >= 2);
    }

    /// The core safety property: an index built with one model refuses an
    /// embedder from another, instead of silently scoring everything 0.0.
    #[tokio::test]
    async fn attaching_a_different_model_is_rejected() {
        let ws = workspace_with_files();
        let mut index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "nomic", 8),
        )
        .await
        .expect("builds");
        let err = index
            .attach(FakeEmbedder::shared(
                ProviderKind::OpenAI,
                "text-embedding-3-small",
                8,
            ))
            .expect_err("must reject a different model");
        assert!(err.to_string().contains("built with"));
    }

    /// Same model name, different dimension (a re-pulled Ollama model, or a
    /// Matryoshka change) must also be caught.
    #[tokio::test]
    async fn attaching_the_same_model_at_a_new_dimension_is_rejected() {
        let ws = workspace_with_files();
        let mut index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "m", 8),
        )
        .await
        .expect("builds");
        assert!(index
            .attach(FakeEmbedder::shared(ProviderKind::Ollama, "m", 16))
            .is_err());
    }

    #[tokio::test]
    async fn attaching_the_matching_model_succeeds() {
        let ws = workspace_with_files();
        let mut index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "m", 8),
        )
        .await
        .expect("builds");
        assert!(index
            .attach(FakeEmbedder::shared(ProviderKind::Ollama, "m", 8))
            .is_ok());
    }

    #[tokio::test]
    async fn search_without_an_embedder_says_so() {
        let ws = workspace_with_files();
        let mut index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "m", 8),
        )
        .await
        .expect("builds");
        let path = ws.path().join("idx.json");
        index.save(&path).expect("saves");

        let reloaded = EmbeddingIndex::load(&path).expect("loads");
        let err = reloaded
            .search("anything", 3)
            .await
            .expect_err("no embedder");
        assert!(err.to_string().contains("no embedder attached"));
    }

    // ── Per-model paths ──────────────────────────────────────────────────────

    #[test]
    fn two_models_get_two_paths() {
        let dir = Path::new("/tmp/idx");
        let a = index_path(
            dir,
            &ModelRef::new(ProviderKind::Ollama, "nomic-embed-text"),
        );
        let b = index_path(dir, &ModelRef::new(ProviderKind::Voyage, "voyage-code-3"));
        assert_ne!(a, b);
        assert!(a.to_string_lossy().contains("ollama__nomic-embed-text"));
    }

    #[tokio::test]
    async fn indexes_for_two_models_coexist() {
        let ws = workspace_with_files();
        let store = ws.path().join("indexes");

        let mut a = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "nomic", 8),
        )
        .await
        .expect("builds a");
        let mut b = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Voyage, "voyage-code-3", 16),
        )
        .await
        .expect("builds b");

        let pa = a.save_in(&store).expect("saves a");
        let pb = b.save_in(&store).expect("saves b");
        assert_ne!(pa, pb);
        assert!(pa.exists() && pb.exists());

        let listed = list_indexes(&store);
        assert_eq!(listed.len(), 2, "both indexes must be listed");
        assert!(listed.iter().any(|(_, h)| h.dimension == Some(8)));
        assert!(listed.iter().any(|(_, h)| h.dimension == Some(16)));
    }

    #[tokio::test]
    async fn load_for_finds_only_the_requested_model() {
        let ws = workspace_with_files();
        let store = ws.path().join("indexes");
        let model = ModelRef::new(ProviderKind::Ollama, "nomic");
        let mut index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "nomic", 8),
        )
        .await
        .expect("builds");
        index.save_in(&store).expect("saves");

        assert!(EmbeddingIndex::load_for(&store, &model).is_some());
        assert!(EmbeddingIndex::load_for(
            &store,
            &ModelRef::new(ProviderKind::Cohere, "embed-v4.0")
        )
        .is_none());
    }

    /// Several real catalog models have dots in their id (`voyage-3.5`,
    /// `embed-v4.0`), and the sidecar path is computed with `with_extension`,
    /// which is dot-sensitive. Pin the round-trip.
    #[tokio::test]
    async fn dotted_model_names_round_trip_through_the_sidecar_path() {
        let ws = workspace_with_files();
        let store = ws.path().join("indexes");
        for (provider, model) in [
            (ProviderKind::Voyage, "voyage-3.5"),
            (ProviderKind::Cohere, "embed-v4.0"),
        ] {
            let mut index =
                EmbeddingIndex::build(ws.path(), FakeEmbedder::shared(provider, model, 8))
                    .await
                    .expect("builds");
            let path = index.save_in(&store).expect("saves");
            assert!(path.exists(), "{model}: index missing");
            assert!(
                meta_path(&path).exists(),
                "{model}: sidecar missing at {}",
                meta_path(&path).display()
            );
        }
        let listed = list_indexes(&store);
        assert_eq!(listed.len(), 2, "both dotted models must be listed");
        for (index_file, _) in &listed {
            assert!(
                index_file.exists(),
                "listed path must resolve: {index_file:?}"
            );
        }
    }

    #[test]
    fn listing_an_absent_directory_is_empty_not_an_error() {
        assert!(list_indexes(Path::new("/tmp/definitely-not-here-vibe")).is_empty());
    }

    // ── Persistence ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn save_and_load_roundtrip_preserves_identity() {
        let ws = workspace_with_files();
        let path = ws.path().join("idx.json");
        let mut index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Cohere, "embed-v4.0", 12),
        )
        .await
        .expect("builds");
        let chunks = index.len();
        index.save(&path).expect("saves");

        let loaded = EmbeddingIndex::load(&path).expect("loads");
        assert_eq!(loaded.len(), chunks);
        assert_eq!(loaded.dimension(), Some(12));
        assert_eq!(loaded.model().model, "embed-v4.0");
        assert_eq!(loaded.model().provider, ProviderKind::Cohere);
    }

    /// The saved file must not contain an API key. v1 serialised the whole
    /// provider struct, key included, into plaintext JSON.
    #[tokio::test]
    async fn saved_index_contains_no_credentials() {
        let ws = workspace_with_files();
        let path = ws.path().join("idx.json");
        let mut index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::OpenAI, "text-embedding-3-small", 8),
        )
        .await
        .expect("builds");
        index.save(&path).expect("saves");
        let raw = std::fs::read_to_string(&path).expect("reads");
        assert!(
            !raw.contains("api_key"),
            "index must not persist credentials"
        );
        assert!(!raw.contains("sk-"));
    }

    #[tokio::test]
    async fn sidecar_matches_the_index() {
        let ws = workspace_with_files();
        let path = ws.path().join("idx.json");
        let mut index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "m", 8),
        )
        .await
        .expect("builds");
        index.save(&path).expect("saves");
        let meta = path.with_extension("meta.json");
        let header: IndexHeader =
            serde_json::from_str(&std::fs::read_to_string(&meta).expect("reads")).expect("parses");
        assert_eq!(header.chunk_count, index.len());
        assert_eq!(header.dimension, index.dimension());
        assert_eq!(header.format_version, INDEX_FORMAT_VERSION);
    }

    #[test]
    fn a_future_format_version_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("idx.json");
        let json = serde_json::json!({
            "header": {
                "format_version": INDEX_FORMAT_VERSION + 1,
                "model": {"provider": "ollama", "model": "m"},
                "dimension": 8, "chunk_count": 0, "file_count": 0
            },
            "vectors": [], "docs": []
        });
        std::fs::write(&path, json.to_string()).expect("writes");
        let err = EmbeddingIndex::load(&path).expect_err("must refuse");
        assert!(err.to_string().contains("format v"));
    }

    #[test]
    fn load_nonexistent_file_fails() {
        assert!(EmbeddingIndex::load(Path::new("/tmp/nonexistent_vibe_idx_test.json")).is_err());
    }

    // ── Legacy migration ─────────────────────────────────────────────────────

    #[test]
    fn v1_index_migrates_and_drops_the_persisted_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("index.json");
        let v1 = serde_json::json!({
            "provider": {"type": "open_ai", "api_key": "sk-leaked", "model": "text-embedding-3-small"},
            "vectors": [[1.0, 0.0, 0.0]],
            "docs": [{"file": "src/main.rs", "chunk_start": 0, "chunk_end": 1, "text": "fn main() {}"}]
        });
        std::fs::write(&path, v1.to_string()).expect("writes");

        let migrated = EmbeddingIndex::load(&path).expect("migrates");
        assert_eq!(migrated.header.format_version, INDEX_FORMAT_VERSION);
        assert_eq!(migrated.model().provider, ProviderKind::OpenAI);
        assert_eq!(migrated.model().model, "text-embedding-3-small");
        assert_eq!(migrated.dimension(), Some(3));
        assert_eq!(migrated.len(), 1);
        // The leaked key must not survive migration into the new file.
        let mut migrated = migrated;
        let out = dir.path().join("v2.json");
        migrated.save(&out).expect("saves");
        assert!(!std::fs::read_to_string(&out)
            .expect("reads")
            .contains("sk-leaked"));
    }

    #[test]
    fn v1_ollama_index_migrates() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("index.json");
        let v1 = serde_json::json!({
            "provider": {"type": "ollama", "model": "nomic-embed-text", "api_url": "http://127.0.0.1:11434"},
            "vectors": [[1.0, 2.0]],
            "docs": [{"file": "a.rs", "chunk_start": 0, "chunk_end": 1, "text": "x"}]
        });
        std::fs::write(&path, v1.to_string()).expect("writes");
        let migrated = EmbeddingIndex::load(&path).expect("migrates");
        assert_eq!(migrated.model().provider, ProviderKind::Ollama);
        assert_eq!(migrated.model().model, "nomic-embed-text");
    }

    #[test]
    fn unparseable_index_reports_both_attempts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("index.json");
        std::fs::write(&path, "{\"nonsense\": true}").expect("writes");
        let err = EmbeddingIndex::load(&path).expect_err("must fail");
        assert!(err.to_string().contains("legacy format"));
    }

    // ── Update ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn update_reembeds_only_changed_files() {
        let ws = workspace_with_files();
        let mut index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "m", 8),
        )
        .await
        .expect("builds");
        let before = index.len();

        let changed = ws.path().join("a.rs");
        std::fs::write(&changed, "fn alpha() {}\nfn beta() {}\nfn delta() {}").expect("write");
        index
            .update(std::slice::from_ref(&changed))
            .await
            .expect("updates");

        assert!(index.docs().iter().any(|d| d.file == changed));
        assert_eq!(index.vectors().len(), index.docs().len());
        assert!(index.len() >= before);
    }

    #[tokio::test]
    async fn update_drops_chunks_for_a_deleted_file() {
        let ws = workspace_with_files();
        let mut index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "m", 8),
        )
        .await
        .expect("builds");
        let gone = ws.path().join("a.rs");
        std::fs::remove_file(&gone).expect("removes");
        index
            .update(std::slice::from_ref(&gone))
            .await
            .expect("updates");
        assert!(!index.docs().iter().any(|d| d.file == gone));
        assert_eq!(index.vectors().len(), index.docs().len());
    }

    #[tokio::test]
    async fn empty_update_is_a_no_op() {
        let ws = workspace_with_files();
        let mut index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "m", 8),
        )
        .await
        .expect("builds");
        let before = index.len();
        index.update(&[]).await.expect("no-op");
        assert_eq!(index.len(), before);
    }

    // ── Search ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn search_returns_ranked_hits() {
        let ws = workspace_with_files();
        let index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "m", 8),
        )
        .await
        .expect("builds");
        let hits = index.search("alpha", 2).await.expect("searches");
        assert!(hits.len() <= 2);
        assert!(hits.windows(2).all(|w| w[0].score >= w[1].score));
    }

    #[tokio::test]
    async fn search_on_an_empty_index_returns_nothing() {
        let index = EmbeddingIndex::empty(ModelRef::new(ProviderKind::Ollama, "m"));
        assert!(index.search("anything", 5).await.expect("ok").is_empty());
    }

    #[tokio::test]
    async fn search_with_k_zero_returns_nothing() {
        let ws = workspace_with_files();
        let index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "m", 8),
        )
        .await
        .expect("builds");
        assert!(index.search("alpha", 0).await.expect("ok").is_empty());
    }

    // ── TurboQuant ───────────────────────────────────────────────────────────

    #[test]
    fn to_turboquant_on_an_empty_index_is_none() {
        let index = EmbeddingIndex::empty(ModelRef::new(ProviderKind::Ollama, "m"));
        assert!(index.to_turboquant(42).is_none());
    }

    #[tokio::test]
    async fn to_turboquant_succeeds_for_a_uniform_index() {
        let ws = workspace_with_files();
        let index = EmbeddingIndex::build(
            ws.path(),
            FakeEmbedder::shared(ProviderKind::Ollama, "m", 8),
        )
        .await
        .expect("builds");
        assert!(index.to_turboquant(42).expect("some").is_ok());
    }

    /// A ragged index used to lose vectors silently — `insert`'s error was
    /// discarded. It must surface instead.
    #[test]
    fn to_turboquant_reports_a_ragged_vector_instead_of_dropping_it() {
        let mut index = EmbeddingIndex::empty(ModelRef::new(ProviderKind::Ollama, "m"));
        index.vectors = vec![vec![1.0; 8], vec![1.0; 4]];
        index.docs = (0..2)
            .map(|i| EmbeddingDoc {
                file: PathBuf::from(format!("f{i}.rs")),
                chunk_start: 0,
                chunk_end: 1,
                text: "x".into(),
            })
            .collect();
        index.header.dimension = Some(8);
        let err = index
            .to_turboquant(42)
            .expect("some")
            .expect_err("must error");
        assert!(err.to_string().contains("chunk 1"));
    }

    // ── Chunking / collection (unchanged behaviour, still pinned) ────────────

    #[test]
    fn chunk_text_small_file() {
        let chunks = chunk_text("line 1\nline 2\nline 3");
        assert_eq!(chunks.len(), 1);
        assert_eq!((chunks[0].start, chunks[0].end), (0, 3));
    }

    #[test]
    fn chunk_text_respects_overlap() {
        let content: String = (0..130).map(|i| format!("line {}\n", i)).collect();
        let chunks = chunk_text(&content);
        assert!(chunks.len() >= 2);
        assert!(chunks[1].start < chunks[0].end);
    }

    #[test]
    fn chunk_text_empty_content() {
        assert!(chunk_text("").is_empty());
    }

    #[test]
    fn chunk_text_exact_chunk_size() {
        let content: String = (0..60)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let chunks = chunk_text(&content);
        assert_eq!(chunks.len(), 1);
        assert_eq!((chunks[0].start, chunks[0].end), (0, 60));
    }

    #[test]
    fn collect_source_files_skips_target_and_node_modules() {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("src");
        std::fs::create_dir(&src).expect("mkdir");
        std::fs::write(src.join("main.rs"), "fn main() {}").expect("write");
        for skipped in ["target", "node_modules"] {
            let d = dir.path().join(skipped);
            std::fs::create_dir(&d).expect("mkdir");
            std::fs::write(d.join("lib.rs"), "// generated").expect("write");
        }
        let files = collect_source_files(dir.path());
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("main.rs"));
    }

    #[test]
    fn collect_source_files_filters_by_extension() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("main.rs"), "fn main() {}").expect("write");
        std::fs::write(dir.path().join("app.py"), "print('hi')").expect("write");
        std::fs::write(dir.path().join("photo.png"), "binary").expect("write");
        assert_eq!(collect_source_files(dir.path()).len(), 2);
    }

    #[test]
    fn oversized_files_are_skipped_not_errors() {
        let dir = tempfile::tempdir().expect("tempdir");
        let big = dir.path().join("big.rs");
        std::fs::write(&big, "x".repeat((MAX_FILE_SIZE_BYTES + 1) as usize)).expect("write");
        assert!(matches!(read_indexable(&big), Ok(None)));
    }

    // ── Cosine ───────────────────────────────────────────────────────────────

    #[test]
    fn cosine_identical_vectors() {
        let v = vec![1.0f32, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        assert!(cosine_similarity(&[1.0, 0.0, 0.0], &[0.0, 1.0, 0.0]).abs() < 1e-6);
    }

    #[test]
    fn cosine_opposite_vectors() {
        assert!((cosine_similarity(&[1.0, 0.0], &[-1.0, 0.0]) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_edge_cases_return_zero() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[0.0, 0.0]), 0.0);
    }

    #[test]
    fn cosine_clamped_for_near_parallel() {
        let sim = cosine_similarity(&[1.0, 1e-7], &[1.0, 1e-7]);
        assert!((-1.0..=1.0).contains(&sim), "cosine out of range: {sim}");
    }

    #[test]
    fn cosine_known_angle() {
        let sim = cosine_similarity(&[1.0, 0.0], &[1.0, 1.0]);
        assert!((sim - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5);
    }
}
