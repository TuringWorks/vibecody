//! Memory Context Hub — orchestrates project and global stores.

use crate::{
    GlobalMemStore, HubStats, MemoryError, ProjectMemStore, PurgeReport, SearchResult, StoreKind,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

pub type SectorWeights = HashMap<String, f64>;

#[derive(Clone)]
pub struct MemoryContextHub {
    project_store: Arc<RwLock<Option<ProjectMemStore>>>,
    global_store: Option<Arc<GlobalMemStore>>,
    sector_weights: Arc<RwLock<SectorWeights>>,
    project_weight: f64,
    global_weight: f64,
}

fn default_sector_weights() -> SectorWeights {
    let mut weights = HashMap::new();
    weights.insert("episodic".to_string(), 1.2);
    weights.insert("semantic".to_string(), 1.0);
    weights.insert("procedural".to_string(), 1.1);
    weights.insert("emotional".to_string(), 1.3);
    weights.insert("reflective".to_string(), 0.8);
    weights
}

impl MemoryContextHub {
    /// Create a new memory context hub with optional global store.
    /// Global store is optional to allow sandbox testing.
    pub fn new() -> Self {
        let global_store = GlobalMemStore::open().ok().map(Arc::new);

        Self {
            project_store: Arc::new(RwLock::new(None)),
            global_store,
            sector_weights: Arc::new(RwLock::new(default_sector_weights())),
            project_weight: 1.5,
            global_weight: 1.0,
        }
    }

    /// Create with a custom global store path (for testing).
    pub fn with_global_at(path: &Path) -> Self {
        let store = GlobalMemStore::open_at(path).ok().map(Arc::new);

        Self {
            project_store: Arc::new(RwLock::new(None)),
            global_store: store,
            sector_weights: Arc::new(RwLock::new(default_sector_weights())),
            project_weight: 1.5,
            global_weight: 1.0,
        }
    }

    pub fn with_weights(project_weight: f64, global_weight: f64) -> Self {
        let mut hub = Self::new();
        hub.project_weight = project_weight;
        hub.global_weight = global_weight;
        hub
    }

    pub async fn sector_weights(&self) -> SectorWeights {
        self.sector_weights.read().await.clone()
    }

    pub async fn set_sector_weights(&self, weights: SectorWeights) {
        let mut w = self.sector_weights.write().await;
        *w = weights;
    }

    pub async fn set_project(&self, workspace: &Path) -> Result<(), MemoryError> {
        let store = ProjectMemStore::open(workspace)?;
        let mut guard = self.project_store.write().await;
        *guard = Some(store);
        Ok(())
    }

    pub async fn clear_project(&self) {
        let mut guard = self.project_store.write().await;
        if let Some(store) = guard.take() {
            store.clear().await.ok();
        }
    }

    pub async fn store_to_project(
        &self,
        workspace: std::path::PathBuf,
        content: &str,
    ) -> Result<crate::MemoryEntry, MemoryError> {
        let mut guard = self.project_store.write().await;
        if guard.is_none() {
            *guard = Some(ProjectMemStore::open(&workspace)?);
        }
        guard.as_ref().unwrap().store(content, None).await
    }

    pub async fn store_global(&self, content: &str) -> Result<crate::MemoryEntry, MemoryError> {
        if let Some(ref store) = self.global_store {
            store.store(content, None).await
        } else {
            Err(MemoryError::StoreNotFound(
                "Global store not available".to_string(),
            ))
        }
    }

    pub async fn search_context(
        &self,
        workspace: &Path,
        query: &str,
        top_k: usize,
        min_score: Option<f64>,
    ) -> Result<Vec<SearchResult>, MemoryError> {
        {
            let mut guard = self.project_store.write().await;
            if guard.is_none() {
                *guard = Some(ProjectMemStore::open(workspace)?);
            }
        }

        let project_store = self.project_store.read().await;
        let project_store = project_store.as_ref().expect("project store not set");

        let project_results = project_store.search(query, top_k * 2, min_score).await?;

        // Query global store if available
        let global_results = if let Some(ref store) = self.global_store {
            store
                .search(query, top_k * 2, min_score)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let mut merged: Vec<SearchResult> = Vec::new();

        for mut r in project_results {
            let sector_weight = self.get_sector_weight(&r.sector).await;
            r.score = r.score * self.project_weight * sector_weight;
            r.store = StoreKind::Project;
            merged.push(r);
        }

        for mut r in global_results {
            let sector_weight = self.get_sector_weight(&r.sector).await;
            r.score = r.score * self.global_weight * sector_weight;
            r.store = StoreKind::Global;
            merged.push(r);
        }

        merged.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        merged.truncate(top_k);

        debug!(
            "Search '{}' returned {} results ({} project, {} global)",
            query,
            merged.len(),
            merged
                .iter()
                .filter(|r| r.store == StoreKind::Project)
                .count(),
            merged
                .iter()
                .filter(|r| r.store == StoreKind::Global)
                .count(),
        );

        Ok(merged)
    }

    pub async fn assemble_context(
        &self,
        workspace: &Path,
        query: &str,
        budget_tokens: usize,
    ) -> Result<String, MemoryError> {
        let results = self.search_context(workspace, query, 50, None).await?;

        let mut context_parts = Vec::new();
        let mut total_tokens = 0;

        for result in results {
            let tokens = result.content.split_whitespace().count() * 13 / 10;
            if total_tokens + tokens > budget_tokens {
                break;
            }
            total_tokens += tokens;
            context_parts.push(format!(
                "- [{}] {}\n  (score: {:.3}, sector: {})",
                result.store.as_str().to_uppercase(),
                result.content,
                result.score,
                result.sector
            ));
        }

        if context_parts.is_empty() {
            return Ok("<vibe-memory>\n</vibe-memory>".to_string());
        }

        Ok(format!(
            "<vibe-memory>\n{}\n</vibe-memory>",
            context_parts.join("\n")
        ))
    }

    pub async fn consolidate(&self, workspace: &Path) -> Result<PurgeReport, MemoryError> {
        {
            let mut guard = self.project_store.write().await;
            if guard.is_none() {
                *guard = Some(ProjectMemStore::open(workspace)?);
            }
        }

        let project_store = self.project_store.read().await;
        let project_store = project_store.as_ref().expect("project store not set");

        let proj_decayed = project_store.apply_decay().await?;

        let global_decayed = if let Some(ref store) = self.global_store {
            store.apply_decay().await.unwrap_or(0)
        } else {
            0
        };

        let proj_purged = project_store.purge(0.1).await?;
        let global_purged = if let Some(ref store) = self.global_store {
            store.purge(0.1).await.unwrap_or(0)
        } else {
            0
        };

        Ok(PurgeReport {
            entries_purged: proj_purged + global_purged,
            entries_decayed: proj_decayed + global_decayed,
            project_store: format!("{:?}", workspace),
            global_store: "~/.vibecli/memory/global.db".to_string(),
        })
    }

    pub async fn get_stats(&self, workspace: &Path) -> Result<HubStats, MemoryError> {
        {
            let mut guard = self.project_store.write().await;
            if guard.is_none() {
                *guard = Some(ProjectMemStore::open(workspace)?);
            }
        }

        let project_store = self.project_store.read().await;
        let project_store = project_store.as_ref().expect("project store not set");

        let proj_entries = project_store.list(None, None).await?;
        let proj_db_size = std::fs::metadata(project_store.path())
            .map(|m| m.len())
            .unwrap_or(0);

        let global_count = if let Some(ref store) = self.global_store {
            store.list(None, None).await.unwrap_or_default().len()
        } else {
            0
        };

        let global_db_size = if let Some(ref store) = self.global_store {
            std::fs::metadata(store.path())
                .map(|m| m.len())
                .unwrap_or(0)
        } else {
            0
        };

        Ok(HubStats {
            project_count: proj_entries.len(),
            global_count,
            project_db_size: proj_db_size,
            global_db_size,
        })
    }

    pub async fn clear_project_memories(&self, workspace: &Path) -> Result<usize, MemoryError> {
        let mut guard = self.project_store.write().await;
        if guard.is_none() {
            *guard = Some(ProjectMemStore::open(workspace)?);
        }
        guard.as_ref().unwrap().clear().await
    }

    async fn get_sector_weight(&self, sector: &str) -> f64 {
        let weights = self.sector_weights.read().await;
        weights.get(sector).copied().unwrap_or(1.0)
    }
}

impl Default for MemoryContextHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_hub_basic() {
        let workspace = TempDir::new().unwrap();
        let hub = MemoryContextHub::with_global_at(workspace.path());

        hub.store_to_project(workspace.path().to_path_buf(), "Project knowledge")
            .await
            .expect("store");
        hub.store_global("Global preference").await.ok(); // May fail in sandbox

        let results = hub
            .search_context(workspace.path(), "knowledge", 5, None)
            .await
            .expect("search");
        assert!(!results.is_empty());

        let stats = hub.get_stats(workspace.path()).await.expect("stats");
        assert!(stats.project_count >= 1);

        hub.clear_project().await;
    }

    #[tokio::test]
    async fn test_context_assembly() {
        let workspace = TempDir::new().unwrap();
        let hub = MemoryContextHub::with_global_at(workspace.path());

        hub.store_to_project(workspace.path().to_path_buf(), "Rust ownership model")
            .await
            .expect("store");

        let context = hub
            .assemble_context(workspace.path(), "rust", 2000)
            .await
            .expect("assemble");

        assert!(context.contains("<vibe-memory>"));
        assert!(context.contains("</vibe-memory>"));
        assert!(context.contains("Rust") || context.contains("rust"));
    }

    #[tokio::test]
    async fn test_sector_weights() {
        let workspace = TempDir::new().unwrap();
        let hub = MemoryContextHub::with_global_at(workspace.path());

        let mut weights = hub.sector_weights().await;
        assert_eq!(weights["emotional"], 1.3);
        assert_eq!(weights["episodic"], 1.2);

        weights.insert("emotional".to_string(), 2.0);
        hub.set_sector_weights(weights).await;

        let updated = hub.sector_weights().await;
        assert_eq!(updated["emotional"], 2.0);
    }

    #[tokio::test]
    async fn test_hub_without_global() {
        // Test hub works even without global store
        let workspace = TempDir::new().unwrap();
        let hub = MemoryContextHub::new(); // No global store

        hub.store_to_project(workspace.path().to_path_buf(), "Project only")
            .await
            .expect("store");

        let results = hub
            .search_context(workspace.path(), "project", 5, None)
            .await
            .expect("search");
        assert!(!results.is_empty());

        // Global should have no results
        assert!(results.iter().all(|r| r.store == StoreKind::Project));
    }
}
