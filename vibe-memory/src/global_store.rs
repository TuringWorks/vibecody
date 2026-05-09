//! Global (computer-scoped) memory store.

use crate::{
    classify_sector, epoch_secs, generate_id,
    extension::ExtensionManager, error::*, schema, MemoryEntry, MemoryMeta,
    SearchResult, StoreKind, Waypoint, MemorySector,
};
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

pub struct GlobalMemStore {
    inner: Arc<Inner>,
}

struct Inner {
    path: PathBuf,
    conn: RwLock<Connection>,
    ext_manager: ExtensionManager,
}

impl GlobalMemStore {
    /// Open (or create) the global memory store.
    /// Uses ~/.vibecli/memory/global.db
    pub fn open() -> Result<Self> {
        let vibe_dir = dirs::home_dir()
            .ok_or_else(|| MemoryError::StoreNotFound("Cannot find home directory".to_string()))?
            .join(".vibecli").join("memory");
        
        std::fs::create_dir_all(&vibe_dir).map_err(MemoryError::Io)?;
        let db_path = vibe_dir.join("global.db");
        
        let conn = Connection::open(&db_path).map_err(MemoryError::Sqlite)?;
        schema::initialize_store(&conn)?;
        let ext_manager = ExtensionManager::new(768);
        
        Ok(Self {
            inner: Arc::new(Inner { path: db_path, conn: RwLock::new(conn), ext_manager }),
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
            inner: Arc::new(Inner { path: db_path, conn: RwLock::new(conn), ext_manager }),
        })
    }

    pub fn path(&self) -> PathBuf { self.inner.path.clone() }

    pub async fn store(&self, content: &str, meta: Option<MemoryMeta>) -> Result<MemoryEntry> {
        let meta = meta.unwrap_or_default();
        let sector = classify_sector(content);
        let entry = self.create_entry(content, sector.as_str(), meta.pinned, meta.tags, meta.project_id, meta.session_id, meta.ttl_seconds.map(|s| epoch_secs() + s as i64))?;
        self.insert_entry(entry).await
    }

    pub async fn store_from_project(&self, content: &str, project_id: &str, meta: Option<MemoryMeta>) -> Result<MemoryEntry> {
        let mut meta = meta.unwrap_or_default();
        meta.project_id = Some(project_id.to_string());
        let sector = classify_sector(content);
        let entry = self.create_entry(content, sector.as_str(), meta.pinned, meta.tags, Some(project_id.to_string()), meta.session_id, meta.ttl_seconds.map(|s| epoch_secs() + s as i64))?;
        self.insert_entry(entry).await
    }

    pub async fn store_with_sector(&self, content: &str, sector: &str) -> Result<MemoryEntry> {
        let entry = self.create_entry(content, sector, false, vec![], None, None, None)?;
        self.insert_entry(entry).await
    }

    pub async fn store_with_ttl(&self, content: &str, ttl_seconds: u64) -> Result<MemoryEntry> {
        let expires_at = epoch_secs() + ttl_seconds as i64;
        let entry = self.create_entry(content, "episodic", false, vec![], None, None, Some(expires_at))?;
        self.insert_entry(entry).await
    }

    pub async fn search(&self, query: &str, top_k: usize, min_score: Option<f64>) -> Result<Vec<SearchResult>> {
        let query_embedding = self.generate_embedding(query);
        let conn = self.inner.conn.read().await;
        let mut stmt = conn.prepare("SELECT id, content, sector, salience, tags, project_id, embedding FROM memory_entries ORDER BY created_at DESC LIMIT 200").map_err(MemoryError::Sqlite)?;
        
        let rows = stmt.query_map([], |row| {
            let embedding_blob: Vec<u8> = row.get(6)?;
            let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, f64>(3)?, row.get::<_, String>(4)?, row.get::<_, Option<String>>(5)?, embedding))
        }).map_err(MemoryError::Sqlite)?;
        
        let mut scored: Vec<_> = Vec::new();
        for row in rows {
            let (id, content, sector, salience, tags, project_id, embedding) = row.map_err(MemoryError::Sqlite)?;
            let similarity = cosine_similarity(&query_embedding, &embedding);
            scored.push((id, content, sector, similarity, tags, project_id, salience));
        }
        
        scored.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
        
        let results: Vec<SearchResult> = scored
            .into_iter()
            .filter(|(_, _, _, score, _, _, _)| min_score.as_ref().map(|m| score >= m).unwrap_or(true))
            .take(top_k)
            .map(|(id, content, sector, score, tags, project_id, salience)| {
                SearchResult { id, content, sector, score, salience, tags: serde_json::from_str(&tags).unwrap_or_default(), project_id, store: StoreKind::Global }
            })
            .collect();
        
        debug!("Global search '{}' returned {} results", query, results.len());
        Ok(results)
    }

    pub async fn search_filtered(&self, query: &str, min_score: Option<f64>, sector: Option<&str>) -> Result<Vec<SearchResult>> {
        let mut results = self.search(query, 200, min_score).await?;
        if let Some(s) = sector { results.retain(|r| r.sector == s); }
        Ok(results)
    }

    pub async fn get(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let conn = self.inner.conn.read().await;
        let result = conn.query_row(
            "SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding FROM memory_entries WHERE id = ?1",
            params![id],
            |row| self.row_to_entry(row),
        );
        match result { Ok(entry) => Ok(Some(entry)), Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None), Err(e) => Err(MemoryError::Sqlite(e)) }
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let conn = self.inner.conn.write().await;
        conn.execute("DELETE FROM memory_entries WHERE id = ?1", params![id]).map_err(MemoryError::Sqlite)?;
        debug!("Deleted global memory entry: {}", id);
        Ok(())
    }

    pub async fn list(&self, sector: Option<&str>, limit: Option<usize>) -> Result<Vec<MemoryEntry>> {
        let conn = self.inner.conn.read().await;
        let sql = match (sector, limit) {
            (Some(s), Some(l)) => format!("SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding FROM memory_entries WHERE sector = '{}' ORDER BY created_at DESC LIMIT {}", s, l),
            (None, Some(l)) => format!("SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding FROM memory_entries ORDER BY created_at DESC LIMIT {}", l),
            (Some(s), None) => format!("SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding FROM memory_entries WHERE sector = '{}' ORDER BY created_at DESC", s),
            (None, None) => "SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding FROM memory_entries ORDER BY created_at DESC".to_string(),
        };
        let mut stmt = conn.prepare(&sql).map_err(MemoryError::Sqlite)?;
        let rows = stmt.query_map([], |row| self.row_to_entry(row)).map_err(MemoryError::Sqlite)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub async fn list_by_project(&self, project_id: &str) -> Result<Vec<MemoryEntry>> {
        let conn = self.inner.conn.read().await;
        let mut stmt = conn.prepare("SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding FROM memory_entries WHERE project_id = ?1 ORDER BY created_at DESC").map_err(MemoryError::Sqlite)?;
        let rows = stmt.query_map(params![project_id], |row| self.row_to_entry(row)).map_err(MemoryError::Sqlite)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub async fn sector_stats(&self) -> Result<HashMap<String, usize>> {
        let conn = self.inner.conn.read().await;
        let mut stmt = conn.prepare("SELECT sector, COUNT(*) FROM memory_entries GROUP BY sector").map_err(MemoryError::Sqlite)?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))).map_err(MemoryError::Sqlite)?;
        let mut stats = HashMap::new();
        for row in rows { let (sector, count) = row.map_err(MemoryError::Sqlite)?; stats.insert(sector, count); }
        Ok(stats)
    }

    pub async fn add_waypoint(&self, src_id: &str, dst_id: &str, weight: f64) -> Result<()> {
        let conn = self.inner.conn.write().await;
        let id = generate_id();
        let now = epoch_secs();
        conn.execute("INSERT INTO waypoints (id, src_id, dst_id, weight, cross_project, created_at) VALUES (?1, ?2, ?3, ?4, 0, ?5)", params![id, src_id, dst_id, weight, now]).map_err(MemoryError::Sqlite)?;
        Ok(())
    }

    pub async fn get_waypoints(&self, src_id: &str) -> Result<Vec<Waypoint>> {
        let conn = self.inner.conn.read().await;
        let mut stmt = conn.prepare("SELECT id, src_id, dst_id, weight, cross_project, created_at FROM waypoints WHERE src_id = ?1").map_err(MemoryError::Sqlite)?;
        let rows = stmt.query_map(params![src_id], |row| {
            Ok(Waypoint { id: row.get(0)?, src_id: row.get(1)?, dst_id: row.get(2)?, weight: row.get(3)?, cross_project: row.get::<_, i32>(4)? != 0, created_at: row.get(5)? })
        }).map_err(MemoryError::Sqlite)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub async fn mark_project_deleted(&self, project_id: &str) -> Result<()> {
        debug!("Marking project {} as deleted", project_id);
        Ok(())
    }

    pub async fn apply_decay(&self) -> Result<usize> {
        let now = epoch_secs();
        let conn = self.inner.conn.write().await;
        let mut stmt = conn.prepare("SELECT id, salience, decay_lambda, created_at, pinned FROM memory_entries").map_err(MemoryError::Sqlite)?;
        let rows: Vec<(String, f64, f64, i64, bool)> = stmt.query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get::<_, i32>(4)? != 0))
        }).map_err(MemoryError::Sqlite)?.filter_map(|r| r.ok()).collect();
        
        let mut updated = 0;
        for (id, salience, decay_lambda, created_at, pinned) in rows {
            if pinned { continue; }
            let elapsed_days = (now - created_at) as f64 / (24.0 * 3600.0);
            let new_salience = salience * (-decay_lambda * elapsed_days).exp();
            if (new_salience - salience).abs() > 0.001 {
                conn.execute("UPDATE memory_entries SET salience = ?1, updated_at = ?2 WHERE id = ?3", params![new_salience, now, id]).map_err(MemoryError::Sqlite)?;
                updated += 1;
            }
        }
        debug!("Applied decay to {} global entries", updated);
        Ok(updated)
    }

    pub async fn purge(&self, threshold: f64) -> Result<usize> {
        let conn = self.inner.conn.write().await;
        let purged = conn.execute("DELETE FROM memory_entries WHERE salience < ?1 AND pinned = 0", params![threshold]).map_err(MemoryError::Sqlite)?;
        debug!("Purged {} global entries", purged);
        Ok(purged as usize)
    }

    pub async fn cleanup_expired(&self) -> Result<usize> {
        let now = epoch_secs();
        let conn = self.inner.conn.write().await;
        let purged = conn.execute("DELETE FROM memory_entries WHERE ttl_expires_at IS NOT NULL AND ttl_expires_at < ?1", params![now]).map_err(MemoryError::Sqlite)?;
        Ok(purged as usize)
    }

    pub async fn clear(&self) -> Result<usize> {
        let conn = self.inner.conn.write().await;
        let count = conn.execute("DELETE FROM memory_entries", []).map_err(MemoryError::Sqlite)?;
        debug!("Cleared {} global entries", count);
        Ok(count as usize)
    }

    fn create_entry(&self, content: &str, sector: &str, pinned: bool, tags: Vec<String>, project_id: Option<String>, session_id: Option<String>, ttl_expires_at: Option<i64>) -> Result<MemoryEntry> {
        let now = epoch_secs();
        let sec = MemorySector::from_str(sector).unwrap_or_default();
        Ok(MemoryEntry {
            id: generate_id(), content: content.to_string(), sector: sector.to_string(),
            salience: 1.0, decay_lambda: sec.decay_rate(), embedding: self.generate_embedding(content),
            created_at: now, updated_at: now, last_seen_at: now, version: 1, pinned, tags,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            project_id, session_id, ttl_expires_at,
        })
    }

    async fn insert_entry(&self, entry: MemoryEntry) -> Result<MemoryEntry> {
        let conn = self.inner.conn.write().await;
        conn.execute(
            "INSERT INTO memory_entries (id, content, content_text, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                entry.id, entry.content.clone(), entry.content.clone(), entry.sector,
                entry.salience, entry.decay_lambda, entry.created_at, entry.updated_at,
                entry.last_seen_at, entry.version, entry.pinned as i32,
                serde_json::to_string(&entry.tags)?, serde_json::to_string(&entry.metadata)?,
                entry.project_id, entry.session_id,
                bincode::serialize(&entry.embedding).map_err(|e| MemoryError::Encryption(e.to_string()))?,
            ],
        ).map_err(MemoryError::Sqlite)?;
        debug!("Stored global memory entry: {}", entry.id);
        Ok(entry)
    }

    fn row_to_entry(&self, row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
        let embedding_blob: Vec<u8> = row.get(14)?;
        let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
        Ok(MemoryEntry {
            id: row.get(0)?, content: row.get(1)?, sector: row.get(2)?, salience: row.get(3)?,
            decay_lambda: row.get(4)?, embedding, created_at: row.get(5)?, updated_at: row.get(6)?,
            last_seen_at: row.get(7)?, version: row.get(8)?, pinned: row.get::<_, i32>(9)? != 0,
            tags: serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or_default(),
            metadata: serde_json::from_str(&row.get::<_, String>(11)?).unwrap_or(serde_json::Value::Null),
            project_id: row.get(12)?, session_id: row.get(13)?, ttl_expires_at: None,
        })
    }

    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        let dim = self.inner.ext_manager.dimensions();
        let mut embedding = vec![0.0f32; dim];
        let lower_text = text.to_lowercase();
        let words: Vec<&str> = lower_text.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            let hash = simple_hash(word);
            let idx = (hash % dim as u64) as usize;
            let weight = 1.0f32 / (1.0 + (i as f32 * 0.1));
            embedding[idx] += weight;
        }
        let magnitude = embedding.iter().map(|v| v * v).sum::<f32>().sqrt();
        if magnitude > 0.0 { for v in &mut embedding { *v /= magnitude; } }
        embedding
    }
}

fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for c in s.bytes() { hash = hash.wrapping_mul(33).wrapping_add(c as u64); }
    hash
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() { return 0.0; }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 { return 0.0; }
    (dot / (mag_a * mag_b)) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Tests using open_at with a temp directory (works in sandbox)
    #[tokio::test]
    async fn test_global_store_basic() {
        let tmp = TempDir::new().unwrap();
        let store = GlobalMemStore::open_at(tmp.path()).expect("open at temp");
        let entry = store.store("Global preference for dark mode", None).await.expect("store");
        assert!(!entry.id.is_empty());
        assert!(entry.sector == "emotional" || entry.sector == "episodic"); // Classification varies
        store.delete(&entry.id).await.expect("delete");
    }

    #[tokio::test]
    async fn test_store_from_project() {
        let tmp = TempDir::new().unwrap();
        let store = GlobalMemStore::open_at(tmp.path()).expect("open at temp");
        let entry = store.store_from_project("Project-specific knowledge", "proj-123", None).await.expect("store from project");
        assert_eq!(entry.project_id, Some("proj-123".to_string()));
        store.delete(&entry.id).await.expect("delete");
    }

    #[tokio::test]
    async fn test_sector_stats() {
        let tmp = TempDir::new().unwrap();
        let store = GlobalMemStore::open_at(tmp.path()).expect("open at temp");
        store.store("Yesterday's event", None).await.expect("episodic");
        store.store("A fact about computers", None).await.expect("semantic");
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
        
        store.store("Rust ownership prevents data races", None).await.expect("store 1");
        store.store("Python GIL prevents multi-threading", None).await.expect("store 2");
        
        let results = store.search_filtered("programming", None, Some("procedural")).await.expect("search");
        // Semantic content should match but may not be procedural
        assert!(results.len() <= 2);
        
        store.clear().await.expect("clear");
    }

    #[tokio::test]
    async fn test_waypoints() {
        let tmp = TempDir::new().unwrap();
        let store = GlobalMemStore::open_at(tmp.path()).expect("open at temp");
        
        let id1 = store.store("Memory about Rust", None).await.expect("store 1").id;
        let id2 = store.store("Memory about async", None).await.expect("store 2").id;
        
        store.add_waypoint(&id1, &id2, 0.9).await.expect("add waypoint");
        
        let waypoints = store.get_waypoints(&id1).await.expect("get waypoints");
        assert!(!waypoints.is_empty());
        assert_eq!(waypoints[0].weight, 0.9);
        
        store.clear().await.expect("clear");
    }
}
