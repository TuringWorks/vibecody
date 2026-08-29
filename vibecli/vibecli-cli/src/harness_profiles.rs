//! Persistence for per-(provider, model) harness profiles.
//!
//! [`vibe_ai::harness`] resolves a profile from four layers and deliberately
//! does not know where the top two come from: it stays a pure library, and the
//! storage rules live in one place. This module is that place — it reads the
//! user's overrides out of the encrypted [`ProfileStore`] and installs them
//! into the resolver, and writes them back when a settings surface changes one.
//!
//! # What is stored
//!
//! A *patch*, never a resolved profile. Storing the resolved shape would freeze
//! today's defaults into the user's settings, so improving a default would
//! silently never reach anyone who had opened the panel once. A patch records
//! only what the user actually changed.
//!
//! Keys are `"<provider>/<model>"`, or `"<provider>/*"` for a provider-wide
//! patch, built by [`vibe_ai::harness::override_key`] and
//! [`vibe_ai::harness::provider_wide_key`] so the store and the resolver cannot
//! disagree about the shape — a mismatch there is an override that saves
//! successfully and never applies.
//!
//! Per [AGENTS.md → Zero-Config First], settings live in the encrypted store
//! and never in a `*.toml`, a `*.json`, or an environment variable.

use std::collections::HashMap;
use vibe_ai::harness::{ModelProfile, ProfileOverride, ResolvedProfile};

use crate::profile_store::ProfileStore;

/// The `panel_settings` panel name these overrides live under.
pub const PANEL: &str = "harness";

/// Read every stored override for `profile_id`.
///
/// A row that does not parse is skipped with a warning rather than failing the
/// whole load: one malformed entry — a hand-edited row, a value written by a
/// newer build — must not cost the user every other override they set.
pub fn load(store: &ProfileStore, profile_id: &str) -> HashMap<String, ProfileOverride> {
    let rows = match store.get_all(profile_id, PANEL) {
        Ok(serde_json::Value::Object(map)) => map,
        Ok(_) => return HashMap::new(),
        Err(e) => {
            tracing::warn!(error = %e, "Could not read harness overrides; using built-in defaults");
            return HashMap::new();
        }
    };
    rows.into_iter()
        .filter_map(|(key, value)| {
            let raw = value.as_str()?;
            match serde_json::from_str::<ProfileOverride>(raw) {
                Ok(patch) => Some((key, patch)),
                Err(e) => {
                    tracing::warn!(key = %key, error = %e, "Skipping unreadable harness override");
                    None
                }
            }
        })
        .collect()
}

/// Load the stored overrides and install them into the resolver.
///
/// Called at daemon start and again after every write, because
/// `harness::set_overrides` replaces the whole set: re-reading the store is
/// what makes a delete actually take effect rather than lingering in memory
/// until the next restart.
pub fn install(store: &ProfileStore, profile_id: &str) {
    let overrides = load(store, profile_id);
    let count = overrides.len();
    vibe_ai::harness::set_overrides(overrides);
    tracing::info!(count, "Harness overrides installed");
}

/// Load overrides from the default profile, if the store can be opened.
///
/// Best-effort by design: a machine with no store yet is the common first-run
/// case, and it means "no overrides", not an error. The built-in defaults are
/// a complete, working configuration on their own.
pub fn install_from_default_profile() {
    let Ok(store) = ProfileStore::new() else {
        tracing::debug!("No profile store; harness uses built-in defaults");
        return;
    };
    let profile_id = store
        .get_default_profile_id()
        .unwrap_or_else(|_| "default".to_string());
    install(&store, &profile_id);
}

/// Write one override and re-install the set.
///
/// An empty patch is a delete, not a stored row of nothing: "reset to default"
/// has to remove the entry so that a later improvement to the built-in default
/// reaches this pair.
pub fn save(
    store: &ProfileStore,
    profile_id: &str,
    key: &str,
    patch: &ProfileOverride,
) -> Result<(), String> {
    match patch.is_empty() {
        true => store.delete(profile_id, PANEL, key)?,
        false => {
            let json = serde_json::to_string(patch).map_err(|e| e.to_string())?;
            store.set(profile_id, PANEL, key, &json)?;
        }
    }
    install(store, profile_id);
    Ok(())
}

/// Remove one override and re-install the set.
pub fn delete(store: &ProfileStore, profile_id: &str, key: &str) -> Result<(), String> {
    store.delete(profile_id, PANEL, key)?;
    install(store, profile_id);
    Ok(())
}

/// The resolved profile for one pair, with the provenance a settings panel
/// needs to tell "you chose this" from "we ship this".
pub fn resolve(provider: &str, model: &str) -> ResolvedProfile {
    vibe_ai::harness::resolve(provider, model)
}

/// What this pair will actually be sent.
pub fn effective(provider: &str, model: &str) -> ModelProfile {
    vibe_ai::harness::profile_for(provider, model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_ai::harness::{override_key, PromptDialect, ToolTransport};

    /// `save`, `delete` and `install` all write the resolver's **process-wide**
    /// override map, so every test that calls one of them races every other.
    ///
    /// Found the hard way: the resolution test kept reading the built-in
    /// profile because a neighbouring `save` test had already replaced the map
    /// with its own contents. Poison-tolerant, so one failure does not cascade.
    static GLOBAL_OVERRIDES: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_global_overrides<T>(f: impl FnOnce() -> T) -> T {
        let _guard = GLOBAL_OVERRIDES
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let out = f();
        vibe_ai::harness::set_overrides(HashMap::new());
        out
    }

    /// Never `ProfileStore::new()` — that opens the developer's real
    /// `~/.vibecli` database. See AGENTS.md → Test Isolation.
    fn temp_store() -> (ProfileStore, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let store = ProfileStore::open_with(&dir.path().join("test.db"), [42u8; 32])
            .expect("open test store");
        (store, dir)
    }

    fn patch() -> ProfileOverride {
        ProfileOverride {
            tool_transport: Some(ToolTransport::Prose),
            max_output_tokens: Some(32_000),
            ..Default::default()
        }
    }

    #[test]
    fn a_saved_override_round_trips() {
        let (store, _dir) = temp_store();
        let key = override_key("claude", "claude-opus-5");
        let json = serde_json::to_string(&patch()).unwrap();
        store.set("default", PANEL, &key, &json).unwrap();

        let loaded = load(&store, "default");
        assert_eq!(loaded.get(&key), Some(&patch()));
    }

    #[test]
    fn an_empty_store_yields_no_overrides() {
        let (store, _dir) = temp_store();
        assert!(load(&store, "default").is_empty());
    }

    /// One bad row must not cost the user every other override they set.
    #[test]
    fn an_unreadable_row_is_skipped_not_fatal() {
        let (store, _dir) = temp_store();
        let good = override_key("claude", "claude-opus-5");
        store
            .set("default", PANEL, &good, &serde_json::to_string(&patch()).unwrap())
            .unwrap();
        store
            .set("default", PANEL, "openai/gpt-5.5", "{not json at all")
            .unwrap();

        let loaded = load(&store, "default");
        assert_eq!(loaded.len(), 1, "the good row survives");
        assert!(loaded.contains_key(&good));
    }

    /// "Reset to default" has to *remove* the row. A stored patch of nothing
    /// would keep this pair pinned to today's defaults forever, so a later
    /// improvement to the built-in would never reach it.
    #[test]
    fn saving_an_empty_patch_deletes_the_row() {
        with_global_overrides(|| {
            let (store, _dir) = temp_store();
            let key = override_key("claude", "claude-opus-5");
            store
                .set("default", PANEL, &key, &serde_json::to_string(&patch()).unwrap())
                .unwrap();
            assert_eq!(load(&store, "default").len(), 1);

            save(&store, "default", &key, &ProfileOverride::default()).unwrap();
            assert!(load(&store, "default").is_empty());
        });
    }

    #[test]
    fn a_deleted_override_is_gone() {
        with_global_overrides(|| {
            let (store, _dir) = temp_store();
            let key = override_key("claude", "claude-opus-5");
            save(&store, "default", &key, &patch()).unwrap();
            assert_eq!(load(&store, "default").len(), 1);

            delete(&store, "default", &key).unwrap();
            assert!(load(&store, "default").is_empty());
        });
    }

    /// The key the store writes and the key the resolver reads have to be the
    /// same string, or an override saves successfully and never applies.
    #[test]
    fn a_saved_override_actually_changes_the_resolved_profile() {
        let (store, _dir) = temp_store();
        let key = override_key("claude", "claude-opus-5");
        let json = serde_json::to_string(&patch()).unwrap();
        store.set("default", PANEL, &key, &json).unwrap();

        // `install` writes the process-global override map, so this runs under
        // the same guard as every other test that touches it.
        let resolved = with_global_overrides(|| {
            install(&store, "default");
            vibe_ai::harness::profile_for("claude", "claude-opus-5")
        });
        assert_eq!(resolved.tool_transport, ToolTransport::Prose);
        assert_eq!(resolved.max_output_tokens, Some(32_000));
        // A field the patch did not set keeps the built-in value.
        assert_eq!(resolved.prompt_dialect, PromptDialect::Compact);
    }
}
