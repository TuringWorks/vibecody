//! VibeDesk settings commands — provider config, API keys, UI prefs, and OAuth
//! client config, all backed by the SAME encrypted `ProfileStore`
//! (`~/.vibecli/profile_settings.db`) that VibeUI uses. This is how settings
//! carry over between the two apps: they read/write one shared store.
//!
//! Secrets (API keys, OAuth client secrets) live encrypted in ProfileStore.
//! Non-secret UI prefs (theme, default provider) use the generic global/panel
//! settings tables. We reuse VibeCLI's `ProfileStore` directly rather than
//! re-implementing crypto.

use vibecli_cli::profile_store::ProfileStore;

const DEFAULT_PROFILE: &str = "default";

fn store() -> Result<ProfileStore, String> {
    ProfileStore::new()
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
    store()?.set_provider_config(DEFAULT_PROFILE, "__vibedesk__", &key, &value)
}

/// Get a single global setting (null if unset).
#[tauri::command]
pub async fn setting_get(key: String) -> Result<serde_json::Value, String> {
    match store()?.get_provider_config(DEFAULT_PROFILE, "__vibedesk__", &key)? {
        Some(v) => Ok(serde_json::Value::String(v)),
        None => Ok(serde_json::Value::Null),
    }
}

/// Get all VibeDesk global settings as a JSON object.
#[tauri::command]
pub async fn setting_get_all() -> Result<serde_json::Value, String> {
    store()?.get_all_provider_config(DEFAULT_PROFILE, "__vibedesk__")
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
