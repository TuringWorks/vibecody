//! Global (computer-scoped) memory store.

use crate::{
    classify_sector, epoch_secs, error::*, extension::ExtensionManager, generate_id, schema,
    MemoryEntry, MemoryMeta, MemorySector, SearchResult, StoreKind, Waypoint,
};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

pub struct GlobalMemStore {
    inner: Arc<Inner>,
}

struct Inner {
    path: PathBuf,
    conn: Mutex<Connection>,
    ext_manager: ExtensionManager,
    /// Produces the vectors for this store. Defaults to the built-in hash
    /// engine; swap it for any real model with `with_embedder`.
    embedder: vibe_embed::SharedEmbedder,
}

impl GlobalMemStore {
    /// Open (or create) the global memory store.
    /// Uses ~/.vibecli/memory/global.db
    pub fn open() -> Result<Self> {
        let vibe_dir = dirs::home_dir()
            .ok_or_else(|| MemoryError::StoreNotFound("Cannot find home directory".to_string()))?
            .join(".vibecli")
            .join("memory");

        std::fs::create_dir_all(&vibe_dir).map_err(MemoryError::Io)?;
        let db_path = vibe_dir.join("global.db");

        let conn = Connection::open(&db_path).map_err(MemoryError::Sqlite)?;
        schema::initialize_store(&conn)?;
        let ext_manager = ExtensionManager::new(768);

        Ok(Self {
            inner: Arc::new(Inner {
                path: db_path,
                embedder: crate::embedding::HashEmbedder::shared(ext_manager.dimensions()),
                conn: Mutex::new(conn),
                ext_manager,
            }),
        })
    }

    /// Open at a custom path (for testing).
    pub fn open_at(path: &std::path::Path) -> Result<Self> {
        std::fs::create_dir_all(path).map_err(MemoryError::Io)?;
        let db_path = path.join("memory.db");

        let conn = Connection::open(&db_path).map_err(MemoryError::Sqlite)?;
        schema::initialize_store(&conn)?;
        let ext_manager = ExtensionManager::new(768);

        Ok(Self {
            inner: Arc::new(Inner {
                path: db_path,
                embedder: crate::embedding::HashEmbedder::shared(ext_manager.dimensions()),
                conn: Mutex::new(conn),
                ext_manager,
            }),
        })
    }

    /// Replace the embedding model this store uses.
    ///
    /// The default is the built-in [`HashEmbedder`](crate::HashEmbedder) —
    /// free, offline, and lexical rather than semantic. Pass any real model
    /// (Ollama, OpenAI, Voyage, Cohere, Gemini, in-process candle) for actual
    /// semantic recall:
    ///
    /// ```no_run
    /// # use vibe_memory::GlobalMemStore;
    /// use vibe_embed::{EmbeddingConfig, ModelRef, ProviderKind};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let embedder = EmbeddingConfig::new(
    ///     ModelRef::new(ProviderKind::Ollama, "nomic-embed-text"),
    /// ).build()?;
    /// let store = GlobalMemStore::open()?.with_embedder(embedder)?;
    /// # let _ = store;
    /// # Ok(()) }
    /// ```
    ///
    /// Memories already stored under a different model are **not** re-embedded
    /// and **not** deleted. They stop matching searches under the new model —
    /// which `search` reports rather than hides — and start matching again if
    /// the old model is restored.
    pub fn with_embedder(self, embedder: vibe_embed::SharedEmbedder) -> Result<Self> {
        // A second handle to the same database file. WAL mode (set by
        // `initialize_store`) is what makes concurrent handles safe here.
        let conn = Connection::open(&self.inner.path).map_err(MemoryError::Sqlite)?;
        let dimensions = embedder
            .dim()
            .unwrap_or_else(|| self.inner.ext_manager.dimensions());
        Ok(Self {
            inner: Arc::new(Inner {
                path: self.inner.path.clone(),
                conn: Mutex::new(conn),
                ext_manager: crate::extension::ExtensionManager::new(dimensions),
                embedder,
            }),
        })
    }

    /// A second handle to the same store, sharing its connection and model.
    pub fn clone_handle(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }

    /// The model this store embeds with.
    pub fn embedding_model(&self) -> &vibe_embed::ModelRef {
        self.inner.embedder.model()
    }

    pub fn path(&self) -> PathBuf {
        self.inner.path.clone()
    }

    pub async fn store(&self, content: &str, meta: Option<MemoryMeta>) -> Result<MemoryEntry> {
        let meta = meta.unwrap_or_default();
        let sector = classify_sector(content);
        let entry = self
            .create_entry(
                content,
                sector.as_str(),
                meta.pinned,
                meta.tags,
                meta.project_id,
                meta.session_id,
                meta.ttl_seconds.map(|s| epoch_secs() + s as i64),
            )
            .await?;
        self.insert_entry(entry).await
    }

    pub async fn store_from_project(
        &self,
        content: &str,
        project_id: &str,
        meta: Option<MemoryMeta>,
    ) -> Result<MemoryEntry> {
        let mut meta = meta.unwrap_or_default();
        meta.project_id = Some(project_id.to_string());
        let sector = classify_sector(content);
        let entry = self
            .create_entry(
                content,
                sector.as_str(),
                meta.pinned,
                meta.tags,
                Some(project_id.to_string()),
                meta.session_id,
                meta.ttl_seconds.map(|s| epoch_secs() + s as i64),
            )
            .await?;
        self.insert_entry(entry).await
    }

    pub async fn store_with_sector(&self, content: &str, sector: &str) -> Result<MemoryEntry> {
        let entry = self
            .create_entry(content, sector, false, vec![], None, None, None)
            .await?;
        self.insert_entry(entry).await
    }

    pub async fn store_with_ttl(&self, content: &str, ttl_seconds: u64) -> Result<MemoryEntry> {
        let expires_at = epoch_secs() + ttl_seconds as i64;
        let entry = self
            .create_entry(
                content,
                "episodic",
                false,
                vec![],
                None,
                None,
                Some(expires_at),
            )
            .await?;
        self.insert_entry(entry).await
    }

    pub async fn search(
        &self,
        query: &str,
        top_k: usize,
        min_score: Option<f64>,
    ) -> Result<Vec<SearchResult>> {
        let query_embedding = self
            .generate_embedding(query, vibe_embed::EmbedKind::Query)
            .await?;
        // Only rows this model produced are comparable. Everything else is
        // counted and reported, not silently scored 0.0 and dropped.
        let tag = crate::VectorTag::of(self.inner.embedder.model(), query_embedding.len());
        let conn = self.inner.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, content, sector, salience, tags, project_id, embedding, embedding_model FROM memory_entries ORDER BY created_at DESC LIMIT 200").map_err(MemoryError::Sqlite)?;

        let rows = stmt
            .query_map([], |row| {
                let embedding_blob: Vec<u8> = row.get(6)?;
                // A blob that will not decode is a broken row, not an empty
                // vector: `unwrap_or_default` here would turn corruption into
                // a silently unsearchable memory.
                let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
                let model_slug: Option<String> = row.get(7)?;
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    embedding,
                    model_slug,
                ))
            })
            .map_err(MemoryError::Sqlite)?;

        let mut scored: Vec<_> = Vec::new();
        let mut diagnostics = crate::SearchDiagnostics::default();
        for row in rows {
            let (id, content, sector, salience, tags, project_id, embedding, model_slug) =
                row.map_err(MemoryError::Sqlite)?;
            if embedding.is_empty() {
                diagnostics.skipped_no_vector += 1;
                continue;
            }
            if !tag.accepts(model_slug.as_deref(), embedding.len()) {
                diagnostics.skipped_other_model += 1;
                continue;
            }
            diagnostics.compared += 1;
            let similarity = cosine_similarity(&query_embedding, &embedding);
            scored.push((id, content, sector, similarity, tags, project_id, salience));
        }

        if !diagnostics.is_complete() {
            // Loud, because the alternative is a result set that looks
            // complete but silently omits every memory written by another
            // model. Re-embedding is the fix; knowing is the prerequisite.
            tracing::warn!(
                compared = diagnostics.compared,
                skipped_other_model = diagnostics.skipped_other_model,
                skipped_no_vector = diagnostics.skipped_no_vector,
                model = %self.inner.embedder.model(),
                "memory search skipped entries embedded with a different model"
            );
        }

        scored.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

        let results: Vec<SearchResult> = scored
            .into_iter()
            .filter(|(_, _, _, score, _, _, _)| {
                min_score.as_ref().map(|m| score >= m).unwrap_or(true)
            })
            .take(top_k)
            .map(
                |(id, content, sector, score, tags, project_id, salience)| SearchResult {
                    id,
                    content,
                    sector,
                    score,
                    salience,
                    tags: serde_json::from_str(&tags).unwrap_or_default(),
                    project_id,
                    store: StoreKind::Global,
                },
            )
            .collect();

        debug!(
            "Global search '{}' returned {} results",
            query,
            results.len()
        );
        Ok(results)
    }

    pub async fn search_filtered(
        &self,
        query: &str,
        min_score: Option<f64>,
        sector: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let mut results = self.search(query, 200, min_score).await?;
        if let Some(s) = sector {
            results.retain(|r| r.sector == s);
        }
        Ok(results)
    }

    pub async fn get(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let conn = self.inner.conn.lock().await;
        let result = conn.query_row(
            "SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding, ttl_expires_at FROM memory_entries WHERE id = ?1",
            params![id],
            |row| self.row_to_entry(row),
        );
        match result {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(MemoryError::Sqlite(e)),
        }
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let conn = self.inner.conn.lock().await;
        conn.execute("DELETE FROM memory_entries WHERE id = ?1", params![id])
            .map_err(MemoryError::Sqlite)?;
        debug!("Deleted global memory entry: {}", id);
        Ok(())
    }

    pub async fn list(
        &self,
        sector: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryEntry>> {
        let conn = self.inner.conn.lock().await;
        let sql = match (sector, limit) {
            (Some(s), Some(l)) => format!("SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding, ttl_expires_at FROM memory_entries WHERE sector = '{}' ORDER BY created_at DESC LIMIT {}", s, l),
            (None, Some(l)) => format!("SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding, ttl_expires_at FROM memory_entries ORDER BY created_at DESC LIMIT {}", l),
            (Some(s), None) => format!("SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding, ttl_expires_at FROM memory_entries WHERE sector = '{}' ORDER BY created_at DESC", s),
            (None, None) => "SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding, ttl_expires_at FROM memory_entries ORDER BY created_at DESC".to_string(),
        };
        let mut stmt = conn.prepare(&sql).map_err(MemoryError::Sqlite)?;
        let rows = stmt
            .query_map([], |row| self.row_to_entry(row))
            .map_err(MemoryError::Sqlite)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub async fn list_by_project(&self, project_id: &str) -> Result<Vec<MemoryEntry>> {
        let conn = self.inner.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding, ttl_expires_at FROM memory_entries WHERE project_id = ?1 ORDER BY created_at DESC").map_err(MemoryError::Sqlite)?;
        let rows = stmt
            .query_map(params![project_id], |row| self.row_to_entry(row))
            .map_err(MemoryError::Sqlite)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub async fn sector_stats(&self) -> Result<HashMap<String, usize>> {
        let conn = self.inner.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT sector, COUNT(*) FROM memory_entries GROUP BY sector")
            .map_err(MemoryError::Sqlite)?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
            })
            .map_err(MemoryError::Sqlite)?;
        let mut stats = HashMap::new();
        for row in rows {
            let (sector, count) = row.map_err(MemoryError::Sqlite)?;
            stats.insert(sector, count);
        }
        Ok(stats)
    }

    pub async fn add_waypoint(&self, src_id: &str, dst_id: &str, weight: f64) -> Result<()> {
        let conn = self.inner.conn.lock().await;
        let id = generate_id();
        let now = epoch_secs();
        conn.execute("INSERT INTO waypoints (id, src_id, dst_id, weight, cross_project, created_at) VALUES (?1, ?2, ?3, ?4, 0, ?5)", params![id, src_id, dst_id, weight, now]).map_err(MemoryError::Sqlite)?;
        Ok(())
    }

    pub async fn get_waypoints(&self, src_id: &str) -> Result<Vec<Waypoint>> {
        let conn = self.inner.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, src_id, dst_id, weight, cross_project, created_at FROM waypoints WHERE src_id = ?1").map_err(MemoryError::Sqlite)?;
        let rows = stmt
            .query_map(params![src_id], |row| {
                Ok(Waypoint {
                    id: row.get(0)?,
                    src_id: row.get(1)?,
                    dst_id: row.get(2)?,
                    weight: row.get(3)?,
                    cross_project: row.get::<_, i32>(4)? != 0,
                    created_at: row.get(5)?,
                })
            })
            .map_err(MemoryError::Sqlite)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub async fn mark_project_deleted(&self, project_id: &str) -> Result<()> {
        debug!("Marking project {} as deleted", project_id);
        Ok(())
    }

    pub async fn apply_decay(&self) -> Result<usize> {
        let now = epoch_secs();
        let conn = self.inner.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT id, salience, decay_lambda, created_at, pinned FROM memory_entries")
            .map_err(MemoryError::Sqlite)?;
        let rows: Vec<(String, f64, f64, i64, bool)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get::<_, i32>(4)? != 0,
                ))
            })
            .map_err(MemoryError::Sqlite)?
            .filter_map(|r| r.ok())
            .collect();

        let mut updated = 0;
        for (id, salience, decay_lambda, created_at, pinned) in rows {
            if pinned {
                continue;
            }
            let elapsed_days = (now - created_at) as f64 / (24.0 * 3600.0);
            let new_salience = salience * (-decay_lambda * elapsed_days).exp();
            if (new_salience - salience).abs() > 0.001 {
                conn.execute(
                    "UPDATE memory_entries SET salience = ?1, updated_at = ?2 WHERE id = ?3",
                    params![new_salience, now, id],
                )
                .map_err(MemoryError::Sqlite)?;
                updated += 1;
            }
        }
        debug!("Applied decay to {} global entries", updated);
        Ok(updated)
    }

    pub async fn purge(&self, threshold: f64) -> Result<usize> {
        let conn = self.inner.conn.lock().await;
        let purged = conn
            .execute(
                "DELETE FROM memory_entries WHERE salience < ?1 AND pinned = 0",
                params![threshold],
            )
            .map_err(MemoryError::Sqlite)?;
        debug!("Purged {} global entries", purged);
        Ok(purged as usize)
    }

    pub async fn cleanup_expired(&self) -> Result<usize> {
        let now = epoch_secs();
        let conn = self.inner.conn.lock().await;
        let purged = conn.execute("DELETE FROM memory_entries WHERE ttl_expires_at IS NOT NULL AND ttl_expires_at < ?1", params![now]).map_err(MemoryError::Sqlite)?;
        Ok(purged as usize)
    }

    pub async fn clear(&self) -> Result<usize> {
        let conn = self.inner.conn.lock().await;
        let count = conn
            .execute("DELETE FROM memory_entries", [])
            .map_err(MemoryError::Sqlite)?;
        debug!("Cleared {} global entries", count);
        Ok(count as usize)
    }

    async fn create_entry(
        &self,
        content: &str,
        sector: &str,
        pinned: bool,
        tags: Vec<String>,
        project_id: Option<String>,
        session_id: Option<String>,
        ttl_expires_at: Option<i64>,
    ) -> Result<MemoryEntry> {
        let now = epoch_secs();
        let sec = MemorySector::from_str(sector).unwrap_or_default();
        Ok(MemoryEntry {
            id: generate_id(),
            content: content.to_string(),
            sector: sector.to_string(),
            salience: 1.0,
            decay_lambda: sec.decay_rate(),
            embedding: self
                .generate_embedding(content, vibe_embed::EmbedKind::Document)
                .await?,
            created_at: now,
            updated_at: now,
            last_seen_at: now,
            version: 1,
            pinned,
            tags,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            project_id,
            session_id,
            ttl_expires_at,
        })
    }

    async fn insert_entry(&self, entry: MemoryEntry) -> Result<MemoryEntry> {
        let conn = self.inner.conn.lock().await;
        conn.execute(
            // `ttl_expires_at` must be written here: `store_with_ttl` computes
            // it, but it used to be omitted from this statement, so every TTL
            // was silently discarded on insert.
            "INSERT INTO memory_entries (id, content, content_text, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding, ttl_expires_at, embedding_model, embedding_dim) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
            params![
                entry.id, entry.content.clone(), entry.content.clone(), entry.sector,
                entry.salience, entry.decay_lambda, entry.created_at, entry.updated_at,
                entry.last_seen_at, entry.version, entry.pinned as i32,
                serde_json::to_string(&entry.tags)?, serde_json::to_string(&entry.metadata)?,
                entry.project_id, entry.session_id,
                bincode::serialize(&entry.embedding).map_err(|e| MemoryError::Encryption(e.to_string()))?,
                entry.ttl_expires_at,
                // Which model produced the vector, and how long it is.
                self.inner.embedder.model().slug(),
                entry.embedding.len() as i64,
            ],
        ).map_err(MemoryError::Sqlite)?;
        debug!("Stored global memory entry: {}", entry.id);
        Ok(entry)
    }

    fn row_to_entry(&self, row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
        let embedding_blob: Vec<u8> = row.get(14)?;
        let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
        Ok(MemoryEntry {
            id: row.get(0)?,
            content: row.get(1)?,
            sector: row.get(2)?,
            salience: row.get(3)?,
            decay_lambda: row.get(4)?,
            embedding,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
            last_seen_at: row.get(7)?,
            version: row.get(8)?,
            pinned: row.get::<_, i32>(9)? != 0,
            tags: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
            metadata: serde_json::from_str(&row.get::<_, String>(11)?)
                .unwrap_or(serde_json::Value::Null),
            project_id: row.get(12)?,
            session_id: row.get(13)?,
            // Column 15 — was hard-coded to `None`, so even a persisted expiry
            // never made it back out of the store.
            ttl_expires_at: row.get(15)?,
        })
    }

    /// Embed `text` with this store's model.
    ///
    /// An embedding failure is not silently swallowed into a zero vector: a
    /// memory stored with an all-zero vector is unreachable by every future
    /// search, and nothing would ever say why.
    async fn generate_embedding(
        &self,
        text: &str,
        kind: vibe_embed::EmbedKind,
    ) -> Result<Vec<f32>> {
        self.inner
            .embedder
            .embed(text, kind)
            .await
            .map_err(|e| MemoryError::Embedding(e.to_string()))
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    (dot / (mag_a * mag_b)) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A different embedding model must not silently return another model's
    /// memories, and must not silently hide them either. Before per-row model
    /// tagging, a dimension change made every existing memory score 0.0 and
    /// disappear from every search with no signal at all.
    #[tokio::test]
    async fn memories_from_another_model_are_excluded_not_mis_scored() {
        let tmp = TempDir::new().expect("tempdir");
        let store = GlobalMemStore::open_at(tmp.path()).expect("open at temp");
        store
            .store("the deployment runbook lives in docs/deploy.md", None)
            .await
            .expect("store");

        // Same store, a different model of the same family.
        let other = store
            .clone_handle()
            .with_embedder(crate::embedding::HashEmbedder::shared(256))
            .expect("swap embedder");
        assert_ne!(
            other.embedding_model().slug(),
            store.embedding_model().slug(),
            "a different bucket count is a different model"
        );

        let hits = other
            .search("deployment runbook", 5, None)
            .await
            .expect("search");
        assert!(
            hits.is_empty(),
            "a memory embedded by another model must not be scored as if comparable"
        );

        // And the original model still finds it — the row was never lost.
        let original = store
            .search("deployment runbook", 5, None)
            .await
            .expect("search");
        assert_eq!(original.len(), 1);
    }

    /// Rows written by this store must carry the model that wrote them.
    #[tokio::test]
    async fn stored_rows_record_their_model_and_dimension() {
        let tmp = TempDir::new().expect("tempdir");
        let store = GlobalMemStore::open_at(tmp.path()).expect("open at temp");
        store
            .store("a fact worth keeping", None)
            .await
            .expect("store");

        let conn = store.inner.conn.lock().await;
        let (slug, dim): (Option<String>, Option<i64>) = conn
            .query_row(
                "SELECT embedding_model, embedding_dim FROM memory_entries LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("row");
        assert_eq!(
            slug.as_deref(),
            Some(store.embedding_model().slug().as_str())
        );
        assert_eq!(dim, Some(768));
    }

    // Tests using open_at with a temp directory (works in sandbox)
    #[tokio::test]
    async fn test_global_store_basic() {
        let tmp = TempDir::new().unwrap();
        let store = GlobalMemStore::open_at(tmp.path()).expect("open at temp");
        let entry = store
            .store("Global preference for dark mode", None)
            .await
            .expect("store");
        assert!(!entry.id.is_empty());
        assert!(entry.sector == "emotional" || entry.sector == "episodic"); // Classification varies
        store.delete(&entry.id).await.expect("delete");
    }

    #[tokio::test]
    async fn test_store_from_project() {
        let tmp = TempDir::new().unwrap();
        let store = GlobalMemStore::open_at(tmp.path()).expect("open at temp");
        let entry = store
            .store_from_project("Project-specific knowledge", "proj-123", None)
            .await
            .expect("store from project");
        assert_eq!(entry.project_id, Some("proj-123".to_string()));
        store.delete(&entry.id).await.expect("delete");
    }

    #[tokio::test]
    async fn test_sector_stats() {
        let tmp = TempDir::new().unwrap();
        let store = GlobalMemStore::open_at(tmp.path()).expect("open at temp");
        store
            .store("Yesterday's event", None)
            .await
            .expect("episodic");
        store
            .store("A fact about computers", None)
            .await
            .expect("semantic");
        let stats = store.sector_stats().await.expect("stats");
        assert!(stats.contains_key("episodic"));
        assert!(stats.contains_key("semantic"));
        store.clear().await.expect("clear");
    }

    // Test with production path (skipped in sandbox)
    #[tokio::test]
    #[ignore = "Requires write access to ~/.vibecli (production only)"]
    async fn test_global_store_production() {
        let store = GlobalMemStore::open().expect("open");
        let entry = store.store("Test memory", None).await.expect("store");
        assert!(!entry.id.is_empty());
        store.delete(&entry.id).await.expect("delete");
    }

    #[tokio::test]
    async fn test_search_filtered() {
        let tmp = TempDir::new().unwrap();
        let store = GlobalMemStore::open_at(tmp.path()).expect("open at temp");

        store
            .store("Rust ownership prevents data races", None)
            .await
            .expect("store 1");
        store
            .store("Python GIL prevents multi-threading", None)
            .await
            .expect("store 2");

        let results = store
            .search_filtered("programming", None, Some("procedural"))
            .await
            .expect("search");
        // Semantic content should match but may not be procedural
        assert!(results.len() <= 2);

        store.clear().await.expect("clear");
    }

    #[tokio::test]
    async fn test_waypoints() {
        let tmp = TempDir::new().unwrap();
        let store = GlobalMemStore::open_at(tmp.path()).expect("open at temp");

        let id1 = store
            .store("Memory about Rust", None)
            .await
            .expect("store 1")
            .id;
        let id2 = store
            .store("Memory about async", None)
            .await
            .expect("store 2")
            .id;

        store
            .add_waypoint(&id1, &id2, 0.9)
            .await
            .expect("add waypoint");

        let waypoints = store.get_waypoints(&id1).await.expect("get waypoints");
        assert!(!waypoints.is_empty());
        assert_eq!(waypoints[0].weight, 0.9);

        store.clear().await.expect("clear");
    }
}
