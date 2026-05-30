//! Project-scoped memory store (per-workspace).

use crate::{
    classify_sector, default_dimensions, epoch_secs, error::*, extension::ExtensionManager,
    generate_id, schema, MemoryEntry, MemoryMeta, SearchResult, StoreKind, Waypoint,
};
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info};

pub struct ProjectMemStore {
    inner: Arc<Inner>,
}

struct Inner {
    path: PathBuf,
    conn: Mutex<Connection>,
    ext_manager: ExtensionManager,
}

impl ProjectMemStore {
    pub fn open(workspace: &Path) -> Result<Self> {
        let vibe_dir = workspace.join(".vibecli").join("memory");
        std::fs::create_dir_all(&vibe_dir)
            .map_err(|e| MemoryError::InvalidWorkspace(e.to_string()))?;
        let db_path = vibe_dir.join("memory.db");
        info!("Opening project memory store at: {:?}", db_path);

        let conn = Connection::open(&db_path).map_err(MemoryError::Sqlite)?;
        schema::initialize_store(&conn)?;
        let ext_manager = ExtensionManager::new(default_dimensions());

        Ok(Self {
            inner: Arc::new(Inner {
                path: db_path,
                conn: Mutex::new(conn),
                ext_manager,
            }),
        })
    }

    pub fn path(&self) -> PathBuf {
        self.inner.path.clone()
    }

    pub async fn store(&self, content: &str, meta: Option<MemoryMeta>) -> Result<MemoryEntry> {
        let meta = meta.unwrap_or_default();
        let sector = classify_sector(content);
        let entry = self.create_entry(
            content,
            sector.as_str(),
            meta.pinned,
            meta.tags,
            meta.project_id,
            meta.session_id,
            None,
        )?;

        let conn = self.inner.conn.lock().await;
        conn.execute(
            r#"INSERT INTO memory_entries (id, content, content_text, sector, salience, decay_lambda, 
               created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"#,
            params![
                entry.id, entry.content.clone(), entry.content.clone(), entry.sector,
                entry.salience, entry.decay_lambda, entry.created_at, entry.updated_at,
                entry.last_seen_at, entry.version, entry.pinned as i32,
                serde_json::to_string(&entry.tags)?, serde_json::to_string(&entry.metadata)?,
                entry.project_id, entry.session_id,
                bincode::serialize(&entry.embedding).map_err(|e| MemoryError::Encryption(e.to_string()))?,
            ],
        ).map_err(MemoryError::Sqlite)?;

        debug!("Stored memory entry: {}", entry.id);
        Ok(entry)
    }

    pub async fn store_with_sector(&self, content: &str, sector: &str) -> Result<MemoryEntry> {
        let entry = self.create_entry(content, sector, false, vec![], None, None, None)?;

        let conn = self.inner.conn.lock().await;
        conn.execute(
            r#"INSERT INTO memory_entries (id, content, content_text, sector, salience, decay_lambda, 
               created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"#,
            params![
                entry.id, entry.content.clone(), entry.content.clone(), entry.sector,
                entry.salience, entry.decay_lambda, entry.created_at, entry.updated_at,
                entry.last_seen_at, entry.version, entry.pinned as i32,
                serde_json::to_string(&entry.tags)?, serde_json::to_string(&entry.metadata)?,
                entry.project_id, entry.session_id,
                bincode::serialize(&entry.embedding).map_err(|e| MemoryError::Encryption(e.to_string()))?,
            ],
        ).map_err(MemoryError::Sqlite)?;

        Ok(entry)
    }

    pub async fn store_with_ttl(&self, content: &str, ttl_seconds: u64) -> Result<MemoryEntry> {
        let expires_at = epoch_secs() + ttl_seconds as i64;
        let entry = self.create_entry(
            content,
            "episodic",
            false,
            vec![],
            None,
            None,
            Some(expires_at),
        )?;

        let conn = self.inner.conn.lock().await;
        conn.execute(
            r#"INSERT INTO memory_entries (id, content, content_text, sector, salience, decay_lambda, 
               created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"#,
            params![
                entry.id, entry.content.clone(), entry.content.clone(), entry.sector,
                entry.salience, entry.decay_lambda, entry.created_at, entry.updated_at,
                entry.last_seen_at, entry.version, entry.pinned as i32,
                serde_json::to_string(&entry.tags)?, serde_json::to_string(&entry.metadata)?,
                entry.project_id, entry.session_id,
                bincode::serialize(&entry.embedding).map_err(|e| MemoryError::Encryption(e.to_string()))?,
            ],
        ).map_err(MemoryError::Sqlite)?;

        Ok(entry)
    }

    pub async fn search(
        &self,
        query: &str,
        top_k: usize,
        min_score: Option<f64>,
    ) -> Result<Vec<SearchResult>> {
        let query_embedding = self.generate_embedding(query);

        let conn = self.inner.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, content, sector, salience, tags, embedding FROM memory_entries ORDER BY created_at DESC LIMIT 100"
        ).map_err(MemoryError::Sqlite)?;

        let rows = stmt
            .query_map([], |row| {
                let embedding_blob: Vec<u8> = row.get(5)?;
                let embedding: Vec<f32> = bincode::deserialize(&embedding_blob).unwrap_or_default();
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, String>(4)?,
                    embedding,
                ))
            })
            .map_err(MemoryError::Sqlite)?;

        let mut scored: Vec<_> = Vec::new();
        for row in rows {
            let (id, content, sector, salience, tags, embedding) =
                row.map_err(MemoryError::Sqlite)?;
            let similarity = cosine_similarity(&query_embedding, &embedding);
            scored.push((id, content, sector, similarity, tags, salience));
        }

        scored.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));

        let results: Vec<SearchResult> = scored
            .into_iter()
            .filter(|(_, _, _, score, _, _)| min_score.as_ref().map(|m| score >= m).unwrap_or(true))
            .take(top_k)
            .map(
                |(id, content, sector, score, tags, salience)| SearchResult {
                    id,
                    content,
                    sector,
                    score,
                    salience,
                    tags: serde_json::from_str(&tags).unwrap_or_default(),
                    project_id: None,
                    store: StoreKind::Project,
                },
            )
            .collect();

        debug!("Search '{}' returned {} results", query, results.len());
        Ok(results)
    }

    pub async fn search_with_budget(
        &self,
        query: &str,
        top_k: usize,
        budget_tokens: usize,
    ) -> Result<Vec<SearchResult>> {
        let results = self.search(query, top_k * 2, None).await?;

        let mut total_tokens = 0;
        let trimmed: Vec<SearchResult> = results
            .into_iter()
            .filter(|r| {
                let tokens = r.content.split_whitespace().count() * 13 / 10;
                if total_tokens + tokens <= budget_tokens {
                    total_tokens += tokens;
                    true
                } else {
                    false
                }
            })
            .take(top_k)
            .collect();

        Ok(trimmed)
    }

    pub async fn search_filtered(
        &self,
        query: &str,
        min_score: Option<f64>,
        sector: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let mut results = self.search(query, 100, min_score).await?;
        if let Some(s) = sector {
            results.retain(|r| r.sector == s);
        }
        Ok(results)
    }

    pub async fn get(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let conn = self.inner.conn.lock().await;
        let result = conn.query_row(
            "SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding FROM memory_entries WHERE id = ?1",
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
        debug!("Deleted memory entry: {}", id);
        Ok(())
    }

    pub async fn list(
        &self,
        sector: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryEntry>> {
        let conn = self.inner.conn.lock().await;

        let sql = match (sector, limit) {
            (Some(s), Some(l)) => format!("SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding FROM memory_entries WHERE sector = '{}' ORDER BY created_at DESC LIMIT {}", s, l),
            (Some(s), None) => format!("SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding FROM memory_entries WHERE sector = '{}' ORDER BY created_at DESC", s),
            (None, Some(l)) => format!("SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding FROM memory_entries ORDER BY created_at DESC LIMIT {}", l),
            (None, None) => "SELECT id, content, sector, salience, decay_lambda, created_at, updated_at, last_seen_at, version, pinned, tags, metadata, project_id, session_id, embedding FROM memory_entries ORDER BY created_at DESC".to_string(),
        };

        let mut stmt = conn.prepare(&sql).map_err(MemoryError::Sqlite)?;
        let rows = stmt
            .query_map([], |row| self.row_to_entry(row))
            .map_err(MemoryError::Sqlite)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub async fn add_waypoint(&self, src_id: &str, dst_id: &str, weight: f64) -> Result<()> {
        let conn = self.inner.conn.lock().await;
        let id = generate_id();
        let now = epoch_secs();
        conn.execute("INSERT INTO waypoints (id, src_id, dst_id, weight, cross_project, created_at) VALUES (?1, ?2, ?3, ?4, 0, ?5)", params![id, src_id, dst_id, weight, now]).map_err(MemoryError::Sqlite)?;
        Ok(())
    }

    pub async fn add_waypoint_cross_project(
        &self,
        src_id: &str,
        dst_id: &str,
        weight: f64,
    ) -> Result<()> {
        let conn = self.inner.conn.lock().await;
        let id = generate_id();
        let now = epoch_secs();
        conn.execute("INSERT INTO waypoints (id, src_id, dst_id, weight, cross_project, created_at) VALUES (?1, ?2, ?3, ?4, 1, ?5)", params![id, src_id, dst_id, weight, now]).map_err(MemoryError::Sqlite)?;
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
        debug!("Applied decay to {} entries", updated);
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
        debug!("Purged {} entries below threshold {}", purged, threshold);
        Ok(purged as usize)
    }

    pub async fn backdate(&self, id: &str, timestamp: i64) -> Result<()> {
        let conn = self.inner.conn.lock().await;
        conn.execute(
            "UPDATE memory_entries SET created_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![timestamp, id],
        )
        .map_err(MemoryError::Sqlite)?;
        Ok(())
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
        debug!("Cleared {} entries", count);
        Ok(count as usize)
    }

    fn create_entry(
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
        let sec = crate::MemorySector::from_str(sector).unwrap_or_default();

        Ok(MemoryEntry {
            id: generate_id(),
            content: content.to_string(),
            sector: sector.to_string(),
            salience: 1.0,
            decay_lambda: sec.decay_rate(),
            embedding: self.generate_embedding(content),
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
            ttl_expires_at: None,
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
        if magnitude > 0.0 {
            for v in &mut embedding {
                *v /= magnitude;
            }
        }
        embedding
    }
}

fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for c in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(c as u64);
    }
    hash
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

    #[tokio::test]
    async fn test_project_store_basic() {
        let dir = TempDir::new().unwrap();
        let store = ProjectMemStore::open(dir.path()).expect("open");
        let entry = store.store("Test memory", None).await.expect("store");
        assert!(!entry.id.is_empty());
        assert_eq!(entry.salience, 1.0);
        let retrieved = store.get(&entry.id).await.expect("get");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test memory");
    }

    #[tokio::test]
    async fn test_search() {
        let dir = TempDir::new().unwrap();
        let store = ProjectMemStore::open(dir.path()).expect("open");
        store
            .store("Rust programming language", None)
            .await
            .expect("store 1");
        store
            .store("Python web development", None)
            .await
            .expect("store 2");
        let results = store.search("programming", 5, None).await.expect("search");
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_delete() {
        let dir = TempDir::new().unwrap();
        let store = ProjectMemStore::open(dir.path()).expect("open");
        let entry = store.store("To be deleted", None).await.expect("store");
        store.delete(&entry.id).await.expect("delete");
        let retrieved = store.get(&entry.id).await.expect("get");
        assert!(retrieved.is_none());
    }
}
