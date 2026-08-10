//! The one place VibeCLI resolves an embedding model and opens a code index.
//!
//! Same doctrine as [`daemon_bootstrap`](crate::daemon_bootstrap): every
//! surface that needs a semantic index — the `/index` and `/qa` REPL
//! commands, the daemon's `/index/*` routes, the startup banner, `/health` —
//! goes through here. A second copy of "which model, which file, is it
//! stale?" is how panels end up disagreeing about whether a workspace is
//! indexed.
//!
//! # Layout on disk
//!
//! ```text
//! <workspace>/.vibecli/index/
//!   index__ollama__nomic-embed-text.json       ← vectors
//!   index__ollama__nomic-embed-text.meta.json  ← header (model, dimension, counts)
//!   index__voyage__voyage-code-3.json
//!   index__voyage__voyage-code-3.meta.json
//! ```
//!
//! Indexes for different models coexist. Switching models is instant if the
//! target index already exists, and switching back never re-embeds.
//!
//! # Credentials
//!
//! API keys come from the encrypted [`ProfileStore`] under the provider name
//! (`openai`, `voyage`, `cohere`, `gemini`) — the same entries the chat
//! providers use, so a user who has already added an OpenAI key gets OpenAI
//! embeddings with no extra setup. Nothing here reads an environment variable
//! or writes a key to disk.

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use vibe_core::index::embeddings::{self, EmbeddingIndex, IndexHeader};
use vibe_embed::{EmbeddingSettings, ProviderKind, SharedEmbedder};

/// Directory holding this workspace's per-model indexes.
pub fn index_dir(workspace: &Path) -> PathBuf {
    workspace.join(".vibecli").join("index")
}

/// The pre-multi-model index location, kept only so an existing one can be
/// migrated rather than silently ignored.
fn legacy_index_path(workspace: &Path) -> PathBuf {
    workspace.join(".vibecli").join("index.json")
}

/// Read an API key for `provider` from the encrypted ProfileStore.
///
/// Returns `None` when the store is unavailable or the key is unset — the
/// caller turns that into [`EmbeddingError::MissingApiKey`], which names the
/// provider and points at Settings.
///
/// [`EmbeddingError::MissingApiKey`]: vibe_embed::EmbeddingError::MissingApiKey
pub fn api_key_for(provider: ProviderKind) -> Option<String> {
    let store = crate::profile_store::ProfileStore::new().ok()?;
    store
        .get_api_key("default", provider.as_str())
        .ok()
        .flatten()
        .filter(|k| !k.trim().is_empty())
}

/// Resolve the configured model into a live embedder.
pub fn build_embedder(settings: &EmbeddingSettings) -> Result<SharedEmbedder> {
    settings
        .build(api_key_for)
        .map_err(|e| anyhow!("{e}"))
        .with_context(|| format!("Cannot embed with {}", settings.describe()))
}

/// Move a pre-multi-model `.vibecli/index.json` into the per-model directory.
///
/// The legacy file records which model built it, so it lands under that
/// model's name and stays usable. Returns the new path when a migration
/// happened. Failure is reported, never silent: a migration that quietly did
/// nothing would look identical to "you never indexed this workspace".
pub fn migrate_legacy_index(workspace: &Path) -> Result<Option<PathBuf>> {
    let legacy = legacy_index_path(workspace);
    if !legacy.exists() {
        return Ok(None);
    }
    let mut index = EmbeddingIndex::load(&legacy)
        .with_context(|| format!("Cannot read the legacy index at {}", legacy.display()))?;
    let dir = index_dir(workspace);
    let moved = index
        .save_in(&dir)
        .with_context(|| format!("Cannot write the migrated index into {}", dir.display()))?;
    std::fs::remove_file(&legacy).with_context(|| {
        format!(
            "Migrated the index to {} but could not remove {}",
            moved.display(),
            legacy.display()
        )
    })?;
    tracing::info!(
        "Migrated legacy index {} → {}",
        legacy.display(),
        moved.display()
    );
    Ok(Some(moved))
}

/// Load the index for `settings`, with `embedder` attached and ready to
/// search. `Ok(None)` means this model has no index yet — a normal state, not
/// an error.
pub fn open(
    workspace: &Path,
    settings: &EmbeddingSettings,
    embedder: SharedEmbedder,
) -> Result<Option<EmbeddingIndex>> {
    // A legacy index is worth exactly one migration attempt per open.
    if let Err(e) = migrate_legacy_index(workspace) {
        tracing::warn!("Legacy index migration failed: {e:#}");
    }
    let dir = index_dir(workspace);
    match EmbeddingIndex::load_for(&dir, &settings.model_ref()) {
        None => Ok(None),
        Some(loaded) => loaded
            .and_then(|i| i.with_embedder(embedder))
            .map(Some)
            .with_context(|| format!("Cannot open the index for {}", settings.describe())),
    }
}

/// Build (or rebuild) the index for `settings` and save it.
pub async fn rebuild(
    workspace: &Path,
    settings: &EmbeddingSettings,
    embedder: SharedEmbedder,
) -> Result<(EmbeddingIndex, PathBuf)> {
    let mut index = EmbeddingIndex::build(workspace, embedder)
        .await
        .with_context(|| format!("Building the index with {}", settings.describe()))?;
    let path = index.save_in(&index_dir(workspace))?;
    Ok((index, path))
}

// ── Status ────────────────────────────────────────────────────────────────────

/// What the banner, `/health` and the desktop panels report about a
/// workspace's semantic index.
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexStatus {
    /// Model currently configured.
    pub selected: EmbeddingSettings,
    /// Human-readable form of `selected`.
    pub description: String,
    /// Whether an index exists for the configured model.
    pub built: bool,
    /// Header of the configured model's index, when it exists.
    pub current: Option<IndexHeader>,
    /// Every index on disk for this workspace, including other models. This
    /// is what makes "switch model" honest in the UI — the user can see which
    /// alternatives are already built and therefore free to switch to.
    pub available: Vec<IndexHeader>,
}

/// Describe the semantic-index state of `workspace`.
///
/// Reads only the small `.meta.json` sidecars, so it is safe on a startup
/// path even when an index holds hundreds of megabytes of vectors.
pub fn status(workspace: &Path, settings: &EmbeddingSettings) -> IndexStatus {
    let available: Vec<IndexHeader> = embeddings::list_indexes(&index_dir(workspace))
        .into_iter()
        .map(|(_, header)| header)
        .collect();
    let wanted = settings.model_ref();
    let current = available.iter().find(|h| h.model == wanted).cloned();
    IndexStatus {
        description: settings.describe(),
        selected: settings.clone(),
        built: current.is_some(),
        current,
        available,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_embed::ModelRef;

    fn settings(model: &str) -> EmbeddingSettings {
        EmbeddingSettings::new(ProviderKind::Ollama, model)
    }

    #[test]
    fn index_dir_is_workspace_scoped() {
        let ws = Path::new("/tmp/proj");
        assert_eq!(index_dir(ws), Path::new("/tmp/proj/.vibecli/index"));
    }

    #[test]
    fn status_of_an_unindexed_workspace_is_empty_not_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let s = status(dir.path(), &settings("nomic-embed-text"));
        assert!(!s.built);
        assert!(s.current.is_none());
        assert!(s.available.is_empty());
        assert!(s.description.contains("nomic-embed-text"));
    }

    #[test]
    fn no_legacy_index_is_not_a_migration() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(migrate_legacy_index(dir.path()).expect("ok").is_none());
    }

    /// A v1 `.vibecli/index.json` must land under its own model's name and
    /// leave no copy behind.
    #[test]
    fn legacy_index_migrates_into_the_per_model_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let legacy = legacy_index_path(dir.path());
        std::fs::create_dir_all(legacy.parent().expect("parent")).expect("mkdir");
        std::fs::write(
            &legacy,
            serde_json::json!({
                "provider": {"type": "ollama", "model": "nomic-embed-text", "api_url": "http://127.0.0.1:11434"},
                "vectors": [[1.0, 2.0, 3.0]],
                "docs": [{"file": "a.rs", "chunk_start": 0, "chunk_end": 1, "text": "fn a() {}"}]
            })
            .to_string(),
        )
        .expect("write");

        let moved = migrate_legacy_index(dir.path())
            .expect("migrates")
            .expect("a migration happened");
        assert!(moved.exists());
        assert!(!legacy.exists(), "the legacy file must not be left behind");
        assert!(moved.to_string_lossy().contains("ollama__nomic-embed-text"));

        // And it is discoverable as the index for that model.
        let s = status(dir.path(), &settings("nomic-embed-text"));
        assert!(s.built);
        assert_eq!(s.current.expect("header").dimension, Some(3));
    }

    #[test]
    fn migrating_twice_is_a_no_op_the_second_time() {
        let dir = tempfile::tempdir().expect("tempdir");
        let legacy = legacy_index_path(dir.path());
        std::fs::create_dir_all(legacy.parent().expect("parent")).expect("mkdir");
        std::fs::write(
            &legacy,
            serde_json::json!({
                "provider": {"type": "ollama", "model": "bge-m3", "api_url": "x"},
                "vectors": [], "docs": []
            })
            .to_string(),
        )
        .expect("write");
        assert!(migrate_legacy_index(dir.path()).expect("ok").is_some());
        assert!(migrate_legacy_index(dir.path()).expect("ok").is_none());
    }

    /// An unreadable legacy file must report, not pretend nothing was there.
    #[test]
    fn a_corrupt_legacy_index_is_an_error_not_a_shrug() {
        let dir = tempfile::tempdir().expect("tempdir");
        let legacy = legacy_index_path(dir.path());
        std::fs::create_dir_all(legacy.parent().expect("parent")).expect("mkdir");
        std::fs::write(&legacy, "not json").expect("write");
        assert!(migrate_legacy_index(dir.path()).is_err());
    }

    #[test]
    fn status_lists_other_models_as_switchable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = index_dir(dir.path());
        for (provider, model, dim) in [
            (ProviderKind::Ollama, "nomic-embed-text", 3),
            (ProviderKind::Voyage, "voyage-code-3", 4),
        ] {
            let mut idx = EmbeddingIndex::empty(ModelRef::new(provider, model));
            idx.save_in(&store).expect("saves");
            let _ = dim;
        }
        let s = status(dir.path(), &settings("nomic-embed-text"));
        assert!(s.built, "the configured model's index is present");
        assert_eq!(s.available.len(), 2, "both models are listed as available");
        assert!(s
            .available
            .iter()
            .any(|h| h.model.provider == ProviderKind::Voyage));
    }

    /// Selecting a model with no index must not report another model's index
    /// as if it were the current one.
    #[test]
    fn status_does_not_borrow_another_models_index() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut idx = EmbeddingIndex::empty(ModelRef::new(ProviderKind::Ollama, "bge-m3"));
        idx.save_in(&index_dir(dir.path())).expect("saves");

        let s = status(dir.path(), &settings("nomic-embed-text"));
        assert!(!s.built);
        assert!(s.current.is_none());
        assert_eq!(s.available.len(), 1);
    }
}
