//! VibeDesk settings commands — provider config, API keys, UI prefs, and OAuth
//! client config, all backed by the SAME encrypted `ProfileStore`
//! (`~/.vibecli/profile_settings.db`) that VibeCoder uses. This is how settings
//! carry over between the two apps: they read/write one shared store.
//!
//! Secrets (API keys, OAuth client secrets) live encrypted in ProfileStore.
//! Non-secret UI prefs (theme, default provider) use the generic global/panel
//! settings tables. We reuse VibeCLI's `ProfileStore` directly rather than
//! re-implementing crypto.

use vibecli_cli::profile_store::ProfileStore;

const DEFAULT_PROFILE: &str = "default";

/// ProfileStore namespace for VibeDesk's global UI prefs.
const SETTINGS_NS: &str = "__vibedesk__";
/// Pre-rename namespace (the app was VibeX) — source for the one-time migration.
const LEGACY_SETTINGS_NS: &str = "__vibex__";

fn store() -> Result<ProfileStore, String> {
    ProfileStore::new()
}

/// One-time migration of global UI settings from the pre-rename `__vibex__`
/// namespace to `__vibedesk__`, so an upgraded install keeps its theme and
/// default provider/model instead of resetting.
///
/// Idempotent and non-destructive: it only copies when VibeDesk has no settings
/// of its own yet (fresh `__vibedesk__`) and legacy VibeX settings exist, and it
/// leaves the old namespace untouched. Provider API keys are stored per-provider
/// (shared across VibeCoder/VibeApp/VibeDesk), so they are unaffected and not
/// touched here. Returns the number of settings copied.
pub fn migrate_legacy_settings() -> Result<usize, String> {
    migrate_legacy_settings_in(&store()?)
}

/// Core migration, parameterised over the store so tests can drive it against a
/// throwaway DB (`ProfileStore::open_with`) instead of the production one.
fn migrate_legacy_settings_in(s: &ProfileStore) -> Result<usize, String> {
    // If VibeDesk already has settings, it has either migrated before or the
    // user has written fresh prefs — either way, don't clobber them.
    let current = s.get_all_provider_config(DEFAULT_PROFILE, SETTINGS_NS)?;
    if current.as_object().is_some_and(|o| !o.is_empty()) {
        return Ok(0);
    }

    let legacy = s.get_all_provider_config(DEFAULT_PROFILE, LEGACY_SETTINGS_NS)?;
    let Some(entries) = legacy.as_object() else {
        return Ok(0);
    };

    let mut copied = 0usize;
    for (key, value) in entries {
        if let Some(v) = value.as_str() {
            s.set_provider_config(DEFAULT_PROFILE, SETTINGS_NS, key, v)?;
            copied += 1;
        }
    }
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_store() -> ProfileStore {
        // Unique path per call so parallel tests never share a DB; key is fixed.
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "vibedesk-migrate-{}-{n}.db",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        ProfileStore::open_with(&path, [42u8; 32]).expect("open test store")
    }

    #[test]
    fn copies_legacy_settings_when_vibedesk_is_empty() {
        let s = tmp_store();
        s.set_provider_config(DEFAULT_PROFILE, LEGACY_SETTINGS_NS, "theme-id", "midnight")
            .unwrap();
        s.set_provider_config(DEFAULT_PROFILE, LEGACY_SETTINGS_NS, "provider", "claude")
            .unwrap();

        let copied = migrate_legacy_settings_in(&s).unwrap();
        assert_eq!(copied, 2);

        let now = s
            .get_all_provider_config(DEFAULT_PROFILE, SETTINGS_NS)
            .unwrap();
        assert_eq!(now["theme-id"], "midnight");
        assert_eq!(now["provider"], "claude");
    }

    #[test]
    fn is_idempotent_and_non_destructive() {
        let s = tmp_store();
        s.set_provider_config(DEFAULT_PROFILE, LEGACY_SETTINGS_NS, "theme-id", "midnight")
            .unwrap();

        assert_eq!(migrate_legacy_settings_in(&s).unwrap(), 1);
        // Second run copies nothing (VibeDesk already has settings) …
        assert_eq!(migrate_legacy_settings_in(&s).unwrap(), 0);
        // … and the legacy namespace is left intact.
        let legacy = s
            .get_all_provider_config(DEFAULT_PROFILE, LEGACY_SETTINGS_NS)
            .unwrap();
        assert_eq!(legacy["theme-id"], "midnight");
    }

    #[test]
    fn does_not_clobber_existing_vibedesk_settings() {
        let s = tmp_store();
        s.set_provider_config(DEFAULT_PROFILE, SETTINGS_NS, "theme-id", "daylight")
            .unwrap();
        s.set_provider_config(DEFAULT_PROFILE, LEGACY_SETTINGS_NS, "theme-id", "midnight")
            .unwrap();

        assert_eq!(migrate_legacy_settings_in(&s).unwrap(), 0);
        let now = s
            .get_all_provider_config(DEFAULT_PROFILE, SETTINGS_NS)
            .unwrap();
        assert_eq!(now["theme-id"], "daylight"); // untouched
    }

    #[test]
    fn no_legacy_settings_is_a_noop() {
        let s = tmp_store();
        assert_eq!(migrate_legacy_settings_in(&s).unwrap(), 0);
    }
}

// ── Provider API keys ───────────────────────────────────────────────────────

/// Set (or overwrite) the API key for a provider.
#[tauri::command]
pub async fn provider_key_set(provider: String, api_key: String) -> Result<(), String> {
    store()?.set_api_key(DEFAULT_PROFILE, &provider, &api_key)
}

/// Whether a provider has a key stored (never returns the key itself to the UI).
#[tauri::command]
pub async fn provider_key_has(provider: String) -> Result<bool, String> {
    Ok(store()?.get_api_key(DEFAULT_PROFILE, &provider)?.is_some())
}

/// List providers that have a key configured.
#[tauri::command]
pub async fn provider_key_list() -> Result<Vec<String>, String> {
    store()?.list_api_key_providers(DEFAULT_PROFILE)
}

/// Remove a provider's API key.
#[tauri::command]
pub async fn provider_key_delete(provider: String) -> Result<(), String> {
    store()?.delete_api_key(DEFAULT_PROFILE, &provider)
}

// ── Per-provider config (endpoint URL, default model, etc.) ─────────────────

/// Set a per-provider config value (e.g. `api_url`, `model`).
#[tauri::command]
pub async fn provider_config_set(
    provider: String,
    key: String,
    value: String,
) -> Result<(), String> {
    store()?.set_provider_config(DEFAULT_PROFILE, &provider, &key, &value)
}

/// Get all config values for a provider as a JSON object.
#[tauri::command]
pub async fn provider_config_get_all(provider: String) -> Result<serde_json::Value, String> {
    store()?.get_all_provider_config(DEFAULT_PROFILE, &provider)
}

// ── Global UI prefs (theme, default provider/model, identity) ───────────────

/// Set a global setting (non-secret UI prefs like theme id, default provider).
#[tauri::command]
pub async fn setting_set(key: String, value: String) -> Result<(), String> {
    store()?.set_provider_config(DEFAULT_PROFILE, SETTINGS_NS, &key, &value)
}

/// Get a single global setting (null if unset).
#[tauri::command]
pub async fn setting_get(key: String) -> Result<serde_json::Value, String> {
    match store()?.get_provider_config(DEFAULT_PROFILE, SETTINGS_NS, &key)? {
        Some(v) => Ok(serde_json::Value::String(v)),
        None => Ok(serde_json::Value::Null),
    }
}

/// Get all VibeDesk global settings as a JSON object.
#[tauri::command]
pub async fn setting_get_all() -> Result<serde_json::Value, String> {
    store()?.get_all_provider_config(DEFAULT_PROFILE, SETTINGS_NS)
}

// ── Identity provider / OAuth client config ─────────────────────────────────
//
// OAuth client credentials per identity provider, stored encrypted under a
// dedicated `__oauth__:<provider>` pseudo-provider namespace in ProfileStore.
// The client_secret is a secret, so it lives in the encrypted store; we never
// hand it back to the UI (only whether config exists).

/// Store OAuth client credentials for an identity provider.
#[tauri::command]
pub async fn oauth_client_set(
    provider: String,
    client_id: String,
    client_secret: String,
) -> Result<(), String> {
    let s = store()?;
    let ns = format!("__oauth__:{provider}");
    s.set_provider_config(DEFAULT_PROFILE, &ns, "client_id", &client_id)?;
    s.set_provider_config(DEFAULT_PROFILE, &ns, "client_secret", &client_secret)?;
    Ok(())
}

/// Whether an identity provider has a client_id configured.
#[tauri::command]
pub async fn oauth_client_has(provider: String) -> Result<bool, String> {
    let ns = format!("__oauth__:{provider}");
    Ok(store()?
        .get_provider_config(DEFAULT_PROFILE, &ns, "client_id")?
        .is_some())
}
