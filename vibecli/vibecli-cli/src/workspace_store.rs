#![allow(dead_code)]
//! Project-level encrypted settings and secrets store.
//!
//! Database: `<workspace>/.vibecli/workspace.db`
//! Encryption: ChaCha20-Poly1305 (AEAD) — random 12-byte nonce prepended
//!             to ciphertext in every BLOB column.
//! Key: SHA-256("vibecli-workspace-store-v1:" + $HOME + ":" + $USER + ":" + workspace_path)
//!      — machine-bound AND workspace-bound, so secrets from one project
//!        cannot be decrypted in another.
//!
//! Tables:
//!   workspace_settings — encrypted project-level key/value settings
//!                        (default provider, model, theme overrides, etc.)
//!   workspace_secrets  — encrypted per-project secrets with versioning
//!                        (project API keys, .env values, tokens, etc.)

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::RngExt;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

// ── Key derivation ────────────────────────────────────────────────────────────

/// Derives the workspace encryption key from machine identity + workspace path.
/// Each (machine, workspace) pair produces a unique key.
fn derive_key(workspace_path: &str) -> [u8; 32] {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    let mut h = Sha256::new();
    h.update(b"vibecli-workspace-store-v1:");
    h.update(home.as_bytes());
    h.update(b":");
    h.update(user.as_bytes());
    h.update(b":");
    h.update(workspace_path.as_bytes());
    h.finalize().into()
}

// ── Encryption / Decryption ───────────────────────────────────────────────────

fn encrypt(key: &[u8; 32], plaintext: &str) -> Result<Vec<u8>, String> {
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill(&mut nonce_bytes);
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut ct = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("encrypt: {e}"))?;
    let mut blob = Vec::with_capacity(12 + ct.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.append(&mut ct);
    Ok(blob)
}

fn decrypt(key: &[u8; 32], blob: &[u8]) -> Result<String, String> {
    if blob.len() < 13 {
        return Err("blob too short".into());
    }
    let (nonce_bytes, ct) = blob.split_at(12);
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce = Nonce::from_slice(nonce_bytes);
    let pt = cipher
        .decrypt(nonce, ct)
        .map_err(|e| format!("decrypt: {e}"))?;
    String::from_utf8(pt).map_err(|e| format!("utf8: {e}"))
}

// ── Database path & schema ────────────────────────────────────────────────────

pub fn db_path(workspace_path: &Path) -> PathBuf {
    workspace_path.join(".vibecli").join("workspace.db")
}

fn open_conn(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;

         CREATE TABLE IF NOT EXISTS workspace_settings (
             key             TEXT    PRIMARY KEY,
             encrypted_value BLOB    NOT NULL,
             updated_at      INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS workspace_secrets (
             id              TEXT    PRIMARY KEY,
             key_name        TEXT    NOT NULL UNIQUE,
             encrypted_value BLOB    NOT NULL,
             version         INTEGER NOT NULL DEFAULT 1,
             created_by      TEXT,
             created_at      INTEGER NOT NULL,
             updated_at      INTEGER NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_ws_secrets_key ON workspace_secrets(key_name);

         -- B2.3: per-plugin install policy.
         -- Policies aren't secrets — they're admin-set workspace-policy
         -- decisions, so no encryption. `set_by` lets us enforce that a
         -- `required` policy can only be lowered by an admin (fit-gap
         -- §18 principle #2: client-side, admin-authored).
         CREATE TABLE IF NOT EXISTS plugin_policies (
             plugin_name TEXT    PRIMARY KEY,
             policy      TEXT    NOT NULL,
             set_by      TEXT    NOT NULL,
             updated_at  INTEGER NOT NULL
         );",
    )
    .map_err(|e| e.to_string())?;
    Ok(conn)
}

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSecretMeta {
    pub id: String,
    pub key_name: String,
    pub version: i64,
    pub created_by: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl WorkspaceSecretMeta {
    pub fn summary_line(&self) -> String {
        format!(
            "v{}  {}  [{}]",
            self.version,
            self.key_name,
            &self.id[..8.min(self.id.len())]
        )
    }
}

// ── Plugin policy types (B2.3) ────────────────────────────────────────────────

/// Runtime plugin policy. Same shape as `plugin_manifest::DefaultPolicy`
/// (kept independent so `workspace_store` doesn't depend on the plugin
/// stack — invertible direction). Wire form: lowercase strings
/// `"off" | "on" | "required"`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginPolicy {
    /// Components disabled at runtime.
    Off,
    /// Components enabled at runtime in this workspace.
    On,
    /// Admin-pinned — components enabled and cannot be lowered to
    /// `Off` except by `PolicySetter::Admin`.
    Required,
}

/// Who set the policy. Used by `set_plugin_policy` to enforce the
/// "Required cannot be lowered to Off without admin" rule.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PolicySetter {
    /// Policy authored by the workspace admin. Can do anything.
    Admin,
    /// Policy set automatically at install time, e.g. from the
    /// manifest's `default_policy`.
    Install,
    /// Policy set interactively by the workspace user.
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginPolicyEntry {
    pub plugin_name: String,
    pub policy: PluginPolicy,
    pub set_by: PolicySetter,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyError {
    /// Tried to lower a `Required` policy without admin authority.
    RequiredCannotBeLoweredByNonAdmin {
        plugin_name: String,
        attempted: PluginPolicy,
        attempted_by: PolicySetter,
    },
    /// Database error (forwarded as a string for ergonomic `?`
    /// chaining without importing rusqlite errors at every call site).
    Db(String),
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequiredCannotBeLoweredByNonAdmin {
                plugin_name,
                attempted,
                attempted_by,
            } => write!(
                f,
                "plugin `{}` is policy=Required; cannot be set to {:?} by {:?} \
                 (only PolicySetter::Admin can lower a Required pin)",
                plugin_name, attempted, attempted_by
            ),
            Self::Db(s) => write!(f, "db: {s}"),
        }
    }
}

impl std::error::Error for PolicyError {}

impl From<String> for PolicyError {
    fn from(s: String) -> Self {
        Self::Db(s)
    }
}

fn policy_to_str(p: PluginPolicy) -> &'static str {
    match p {
        PluginPolicy::Off => "off",
        PluginPolicy::On => "on",
        PluginPolicy::Required => "required",
    }
}

fn setter_to_str(s: PolicySetter) -> &'static str {
    match s {
        PolicySetter::Admin => "admin",
        PolicySetter::Install => "install",
        PolicySetter::User => "user",
    }
}

fn policy_from_str(s: &str) -> Result<PluginPolicy, String> {
    match s {
        "off" => Ok(PluginPolicy::Off),
        "on" => Ok(PluginPolicy::On),
        "required" => Ok(PluginPolicy::Required),
        other => Err(format!("unknown plugin policy `{other}`")),
    }
}

fn setter_from_str(s: &str) -> Result<PolicySetter, String> {
    match s {
        "admin" => Ok(PolicySetter::Admin),
        "install" => Ok(PolicySetter::Install),
        "user" => Ok(PolicySetter::User),
        other => Err(format!("unknown policy setter `{other}`")),
    }
}

// ── WorkspaceStore ────────────────────────────────────────────────────────────

pub struct WorkspaceStore {
    conn: Connection,
    key: [u8; 32],
    workspace_path: PathBuf,
}

impl WorkspaceStore {
    /// Open (or create) the workspace store for the given workspace directory.
    pub fn open(workspace_path: &Path) -> Result<Self, String> {
        let canonical = workspace_path
            .canonicalize()
            .unwrap_or_else(|_| workspace_path.to_path_buf());
        let path = db_path(&canonical);
        let key = derive_key(&canonical.to_string_lossy());
        let conn = open_conn(&path)?;
        Ok(Self {
            conn,
            key,
            workspace_path: canonical,
        })
    }

    /// For tests: open against an arbitrary path with a custom key.
    pub fn open_with(db_path: &Path, key: [u8; 32]) -> Result<Self, String> {
        let conn = open_conn(db_path)?;
        Ok(Self {
            conn,
            key,
            workspace_path: db_path.parent().unwrap_or(Path::new(".")).to_path_buf(),
        })
    }

    pub fn workspace_path(&self) -> &Path {
        &self.workspace_path
    }

    // ── Workspace settings ────────────────────────────────────────────────────

    pub fn setting_get(&self, key: &str) -> Result<Option<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT encrypted_value FROM workspace_settings WHERE key=?1")
            .map_err(|e| e.to_string())?;
        let result: rusqlite::Result<Vec<u8>> = stmt.query_row(params![key], |r| r.get(0));
        match result {
            Ok(blob) => Ok(Some(decrypt(&self.key, &blob)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn setting_set(&self, key: &str, value: &str) -> Result<(), String> {
        let blob = encrypt(&self.key, value)?;
        self.conn
            .execute(
                "INSERT INTO workspace_settings (key, encrypted_value, updated_at)
                 VALUES (?1,?2,?3)
                 ON CONFLICT(key) DO UPDATE SET
                     encrypted_value=excluded.encrypted_value,
                     updated_at=excluded.updated_at",
                params![key, blob, now_ms()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn setting_delete(&self, key: &str) -> Result<bool, String> {
        let n = self
            .conn
            .execute("DELETE FROM workspace_settings WHERE key=?1", params![key])
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    /// List all setting keys with their `updated_at` timestamps (values not returned).
    pub fn setting_list(&self) -> Result<Vec<serde_json::Value>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT key, updated_at FROM workspace_settings ORDER BY key")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(serde_json::json!({
                    "key": r.get::<_, String>(0)?,
                    "updated_at": r.get::<_, i64>(1)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    // ── Workspace secrets ─────────────────────────────────────────────────────

    /// Retrieve and decrypt a project secret.
    pub fn secret_get(&self, key_name: &str) -> Result<Option<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT encrypted_value FROM workspace_secrets WHERE key_name=?1")
            .map_err(|e| e.to_string())?;
        let result: rusqlite::Result<Vec<u8>> = stmt.query_row(params![key_name], |r| r.get(0));
        match result {
            Ok(blob) => Ok(Some(decrypt(&self.key, &blob)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Store or update a project secret (increments version on update).
    pub fn secret_set(
        &self,
        key_name: &str,
        value: &str,
        created_by: Option<&str>,
    ) -> Result<WorkspaceSecretMeta, String> {
        let blob = encrypt(&self.key, value)?;
        let now = now_ms();

        let existing: Option<(String, i64)> = self
            .conn
            .query_row(
                "SELECT id, version FROM workspace_secrets WHERE key_name=?1",
                params![key_name],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();

        if let Some((id, version)) = existing {
            self.conn
                .execute(
                    "UPDATE workspace_secrets
                     SET encrypted_value=?1, version=?2, updated_at=?3
                     WHERE id=?4",
                    params![blob, version + 1, now, id],
                )
                .map_err(|e| e.to_string())?;
            self.secret_meta(key_name)?
                .ok_or_else(|| "secret not found after update".into())
        } else {
            let id = new_id();
            self.conn
                .execute(
                    "INSERT INTO workspace_secrets
                         (id, key_name, encrypted_value, version, created_by, created_at, updated_at)
                     VALUES (?1,?2,?3,1,?4,?5,?5)",
                    params![id, key_name, blob, created_by, now],
                )
                .map_err(|e| e.to_string())?;
            self.secret_meta(key_name)?
                .ok_or_else(|| "secret not found after insert".into())
        }
    }

    pub fn secret_delete(&self, key_name: &str) -> Result<bool, String> {
        let n = self
            .conn
            .execute(
                "DELETE FROM workspace_secrets WHERE key_name=?1",
                params![key_name],
            )
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    /// List secret metadata (no values) ordered by key name.
    pub fn secret_list(&self) -> Result<Vec<WorkspaceSecretMeta>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, key_name, version, created_by, created_at, updated_at
                 FROM workspace_secrets ORDER BY key_name",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], row_to_meta)
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    fn secret_meta(&self, key_name: &str) -> Result<Option<WorkspaceSecretMeta>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, key_name, version, created_by, created_at, updated_at
                 FROM workspace_secrets WHERE key_name=?1",
            )
            .map_err(|e| e.to_string())?;
        let result = stmt.query_row(params![key_name], row_to_meta);
        match result {
            Ok(m) => Ok(Some(m)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    // ── Plugin policies (B2.3) ────────────────────────────────────────────────

    /// Fetch the current policy entry for a plugin. Returns `None`
    /// when no row exists; callers should treat that as `Off` (the
    /// safe default — components from an unknown plugin never run).
    pub fn get_plugin_policy(
        &self,
        plugin_name: &str,
    ) -> Result<Option<PluginPolicyEntry>, PolicyError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT plugin_name, policy, set_by, updated_at
                 FROM plugin_policies WHERE plugin_name=?1",
            )
            .map_err(|e| PolicyError::Db(e.to_string()))?;
        let row = stmt.query_row(params![plugin_name], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        });
        match row {
            Ok((name, p, s, ts)) => Ok(Some(PluginPolicyEntry {
                plugin_name: name,
                policy: policy_from_str(&p)?,
                set_by: setter_from_str(&s)?,
                updated_at: ts,
            })),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(PolicyError::Db(e.to_string())),
        }
    }

    /// Resolve a plugin's effective policy. Unknown plugin → `Off`.
    /// Convenience wrapper for the runtime hot path where callers
    /// don't care who set the policy or when.
    pub fn effective_plugin_policy(&self, plugin_name: &str) -> Result<PluginPolicy, PolicyError> {
        Ok(self
            .get_plugin_policy(plugin_name)?
            .map(|e| e.policy)
            .unwrap_or(PluginPolicy::Off))
    }

    /// Set (or replace) a plugin's policy.
    ///
    /// Enforces fit-gap §18 principle #2: a `Required` pin can only
    /// be lowered to `Off` by `PolicySetter::Admin`. Anyone may raise
    /// a policy (Off → On → Required); only admin may lower a
    /// Required pin. Setting `Required` from `On` is also admin-only
    /// because it changes the lower-bound the user can pick. Same-
    /// policy no-op writes always succeed.
    pub fn set_plugin_policy(
        &self,
        plugin_name: &str,
        policy: PluginPolicy,
        set_by: PolicySetter,
    ) -> Result<PluginPolicyEntry, PolicyError> {
        let existing = self.get_plugin_policy(plugin_name)?;
        if let Some(existing) = &existing {
            let admin = matches!(set_by, PolicySetter::Admin);
            let same = existing.policy == policy;
            let lowering_required =
                existing.policy == PluginPolicy::Required && policy != PluginPolicy::Required;
            let raising_to_required =
                existing.policy != PluginPolicy::Required && policy == PluginPolicy::Required;
            if !same && !admin && (lowering_required || raising_to_required) {
                return Err(PolicyError::RequiredCannotBeLoweredByNonAdmin {
                    plugin_name: plugin_name.to_string(),
                    attempted: policy,
                    attempted_by: set_by,
                });
            }
        }
        let now = now_ms();
        self.conn
            .execute(
                "INSERT INTO plugin_policies (plugin_name, policy, set_by, updated_at)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(plugin_name) DO UPDATE SET
                     policy = excluded.policy,
                     set_by = excluded.set_by,
                     updated_at = excluded.updated_at",
                params![
                    plugin_name,
                    policy_to_str(policy),
                    setter_to_str(set_by),
                    now
                ],
            )
            .map_err(|e| PolicyError::Db(e.to_string()))?;
        Ok(PluginPolicyEntry {
            plugin_name: plugin_name.to_string(),
            policy,
            set_by,
            updated_at: now,
        })
    }

    /// Delete a plugin's policy row. Admin-only because deletion is
    /// observationally equivalent to lowering to `Off` (the
    /// "no row → Off" fallback in `effective_plugin_policy`).
    pub fn delete_plugin_policy(
        &self,
        plugin_name: &str,
        set_by: PolicySetter,
    ) -> Result<bool, PolicyError> {
        if !matches!(set_by, PolicySetter::Admin) {
            // Defer to the same check as `set_plugin_policy(Off, …)`
            // so the error shape matches.
            if let Some(existing) = self.get_plugin_policy(plugin_name)? {
                if existing.policy == PluginPolicy::Required {
                    return Err(PolicyError::RequiredCannotBeLoweredByNonAdmin {
                        plugin_name: plugin_name.to_string(),
                        attempted: PluginPolicy::Off,
                        attempted_by: set_by,
                    });
                }
            }
        }
        let n = self
            .conn
            .execute(
                "DELETE FROM plugin_policies WHERE plugin_name=?1",
                params![plugin_name],
            )
            .map_err(|e| PolicyError::Db(e.to_string()))?;
        Ok(n > 0)
    }

    /// List all plugin policy entries, ordered by plugin name. Used by
    /// the B2.6 governance panel and the `vibecli plugin list` command.
    pub fn list_plugin_policies(&self) -> Result<Vec<PluginPolicyEntry>, PolicyError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT plugin_name, policy, set_by, updated_at
                 FROM plugin_policies ORDER BY plugin_name",
            )
            .map_err(|e| PolicyError::Db(e.to_string()))?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            })
            .map_err(|e| PolicyError::Db(e.to_string()))?;
        let mut out = Vec::new();
        for r in rows {
            let (name, p, s, ts) = r.map_err(|e| PolicyError::Db(e.to_string()))?;
            out.push(PluginPolicyEntry {
                plugin_name: name,
                policy: policy_from_str(&p)?,
                set_by: setter_from_str(&s)?,
                updated_at: ts,
            });
        }
        Ok(out)
    }
}

fn row_to_meta(r: &rusqlite::Row) -> rusqlite::Result<WorkspaceSecretMeta> {
    Ok(WorkspaceSecretMeta {
        id: r.get(0)?,
        key_name: r.get(1)?,
        version: r.get(2)?,
        created_by: r.get(3)?,
        created_at: r.get(4)?,
        updated_at: r.get(5)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> WorkspaceStore {
        let dir = std::env::temp_dir().join(format!(
            "vibe_ws_test_{}_{}",
            std::process::id(),
            rand::random::<u32>()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let db = dir.join("workspace.db");
        WorkspaceStore::open_with(&db, [42u8; 32]).unwrap()
    }

    // ── derive_key ────────────────────────────────────────────────────────────

    #[test]
    fn derive_key_differs_by_path() {
        let k1 = derive_key("/home/user/project-a");
        let k2 = derive_key("/home/user/project-b");
        assert_ne!(k1, k2);
    }

    #[test]
    fn derive_key_is_deterministic() {
        assert_eq!(derive_key("/some/path"), derive_key("/some/path"));
    }

    // ── settings ──────────────────────────────────────────────────────────────

    #[test]
    fn setting_set_and_get() {
        let s = temp_store();
        s.setting_set("provider", "claude").unwrap();
        assert_eq!(s.setting_get("provider").unwrap(), Some("claude".into()));
    }

    #[test]
    fn setting_get_missing_returns_none() {
        let s = temp_store();
        assert_eq!(s.setting_get("nope").unwrap(), None);
    }

    #[test]
    fn setting_overwrites() {
        let s = temp_store();
        s.setting_set("model", "claude-3").unwrap();
        s.setting_set("model", "claude-4").unwrap();
        assert_eq!(s.setting_get("model").unwrap(), Some("claude-4".into()));
    }

    #[test]
    fn setting_delete() {
        let s = temp_store();
        s.setting_set("k", "v").unwrap();
        assert!(s.setting_delete("k").unwrap());
        assert_eq!(s.setting_get("k").unwrap(), None);
    }

    #[test]
    fn setting_delete_nonexistent_returns_false() {
        let s = temp_store();
        assert!(!s.setting_delete("ghost").unwrap());
    }

    #[test]
    fn setting_list_returns_keys() {
        let s = temp_store();
        s.setting_set("a", "1").unwrap();
        s.setting_set("b", "2").unwrap();
        let list = s.setting_list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0]["key"], "a");
        assert_eq!(list[1]["key"], "b");
    }

    // ── secrets ───────────────────────────────────────────────────────────────

    #[test]
    fn secret_set_and_get() {
        let s = temp_store();
        s.secret_set("DB_URL", "postgres://localhost/mydb", None)
            .unwrap();
        assert_eq!(
            s.secret_get("DB_URL").unwrap(),
            Some("postgres://localhost/mydb".into())
        );
    }

    #[test]
    fn secret_get_missing_returns_none() {
        let s = temp_store();
        assert_eq!(s.secret_get("GHOST").unwrap(), None);
    }

    #[test]
    fn secret_set_increments_version() {
        let s = temp_store();
        let m1 = s.secret_set("TOKEN", "v1", None).unwrap();
        let m2 = s.secret_set("TOKEN", "v2", None).unwrap();
        assert_eq!(m1.version, 1);
        assert_eq!(m2.version, 2);
    }

    #[test]
    fn secret_get_returns_latest_value() {
        let s = temp_store();
        s.secret_set("KEY", "old", None).unwrap();
        s.secret_set("KEY", "new", None).unwrap();
        assert_eq!(s.secret_get("KEY").unwrap(), Some("new".into()));
    }

    #[test]
    fn secret_delete() {
        let s = temp_store();
        s.secret_set("K", "v", None).unwrap();
        assert!(s.secret_delete("K").unwrap());
        assert_eq!(s.secret_get("K").unwrap(), None);
    }

    #[test]
    fn secret_delete_nonexistent_returns_false() {
        let s = temp_store();
        assert!(!s.secret_delete("GHOST").unwrap());
    }

    #[test]
    fn secret_list_shows_metadata_only() {
        let s = temp_store();
        s.secret_set("DB_URL", "postgres://...", Some("agent-1"))
            .unwrap();
        s.secret_set("API_KEY", "sk-...", None).unwrap();
        let list = s.secret_list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].key_name, "API_KEY");
        assert_eq!(list[0].created_by, None);
        assert_eq!(list[1].key_name, "DB_URL");
        assert_eq!(list[1].created_by.as_deref(), Some("agent-1"));
    }

    #[test]
    fn secret_ciphertext_is_not_plaintext() {
        let s = temp_store();
        s.secret_set("RAW", "my-secret-value", None).unwrap();
        // Read raw blob from DB
        let blob: Vec<u8> = s
            .conn
            .query_row(
                "SELECT encrypted_value FROM workspace_secrets WHERE key_name='RAW'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_ne!(blob, b"my-secret-value");
    }

    #[test]
    fn keys_from_different_paths_cannot_decrypt_each_other() {
        let dir = std::env::temp_dir().join(format!(
            "vibe_ws_cross_{}_{}",
            std::process::id(),
            rand::random::<u32>()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let k1 = derive_key("/project-alpha");
        let k2 = derive_key("/project-beta");
        let ct = encrypt(&k1, "secret").unwrap();
        assert!(decrypt(&k2, &ct).is_err());
    }

    // ── plugin policies (B2.3) ────────────────────────────────────────────────

    #[test]
    fn plugin_policy_default_is_none_resolves_to_off() {
        let s = temp_store();
        assert_eq!(s.get_plugin_policy("ghost").unwrap(), None);
        assert_eq!(
            s.effective_plugin_policy("ghost").unwrap(),
            PluginPolicy::Off
        );
    }

    #[test]
    fn plugin_policy_install_can_set_on() {
        let s = temp_store();
        let entry = s
            .set_plugin_policy("p1", PluginPolicy::On, PolicySetter::Install)
            .unwrap();
        assert_eq!(entry.policy, PluginPolicy::On);
        assert_eq!(entry.set_by, PolicySetter::Install);
        assert_eq!(s.effective_plugin_policy("p1").unwrap(), PluginPolicy::On);
    }

    #[test]
    fn plugin_policy_user_can_raise_from_off_to_on() {
        let s = temp_store();
        s.set_plugin_policy("p1", PluginPolicy::Off, PolicySetter::User)
            .unwrap();
        s.set_plugin_policy("p1", PluginPolicy::On, PolicySetter::User)
            .unwrap();
        assert_eq!(s.effective_plugin_policy("p1").unwrap(), PluginPolicy::On);
    }

    #[test]
    fn plugin_policy_user_cannot_raise_to_required() {
        let s = temp_store();
        s.set_plugin_policy("p1", PluginPolicy::On, PolicySetter::User)
            .unwrap();
        let err = s
            .set_plugin_policy("p1", PluginPolicy::Required, PolicySetter::User)
            .unwrap_err();
        assert!(matches!(
            err,
            PolicyError::RequiredCannotBeLoweredByNonAdmin { .. }
        ));
    }

    #[test]
    fn plugin_policy_admin_can_pin_required() {
        let s = temp_store();
        s.set_plugin_policy("p1", PluginPolicy::On, PolicySetter::User)
            .unwrap();
        s.set_plugin_policy("p1", PluginPolicy::Required, PolicySetter::Admin)
            .unwrap();
        assert_eq!(
            s.effective_plugin_policy("p1").unwrap(),
            PluginPolicy::Required
        );
    }

    #[test]
    fn plugin_policy_user_cannot_lower_required_to_off() {
        let s = temp_store();
        s.set_plugin_policy("p1", PluginPolicy::Required, PolicySetter::Admin)
            .unwrap();
        let err = s
            .set_plugin_policy("p1", PluginPolicy::Off, PolicySetter::User)
            .unwrap_err();
        assert!(matches!(
            err,
            PolicyError::RequiredCannotBeLoweredByNonAdmin {
                attempted: PluginPolicy::Off,
                ..
            }
        ));
    }

    #[test]
    fn plugin_policy_admin_can_lower_required_to_off() {
        let s = temp_store();
        s.set_plugin_policy("p1", PluginPolicy::Required, PolicySetter::Admin)
            .unwrap();
        s.set_plugin_policy("p1", PluginPolicy::Off, PolicySetter::Admin)
            .unwrap();
        assert_eq!(s.effective_plugin_policy("p1").unwrap(), PluginPolicy::Off);
    }

    #[test]
    fn plugin_policy_no_op_same_policy_succeeds_for_anyone() {
        let s = temp_store();
        s.set_plugin_policy("p1", PluginPolicy::Required, PolicySetter::Admin)
            .unwrap();
        // User re-setting same Required value must succeed (no
        // state change to gate).
        s.set_plugin_policy("p1", PluginPolicy::Required, PolicySetter::User)
            .unwrap();
        assert_eq!(
            s.effective_plugin_policy("p1").unwrap(),
            PluginPolicy::Required
        );
    }

    #[test]
    fn plugin_policy_user_cannot_delete_required() {
        let s = temp_store();
        s.set_plugin_policy("p1", PluginPolicy::Required, PolicySetter::Admin)
            .unwrap();
        let err = s
            .delete_plugin_policy("p1", PolicySetter::User)
            .unwrap_err();
        assert!(matches!(
            err,
            PolicyError::RequiredCannotBeLoweredByNonAdmin { .. }
        ));
    }

    #[test]
    fn plugin_policy_admin_can_delete_required() {
        let s = temp_store();
        s.set_plugin_policy("p1", PluginPolicy::Required, PolicySetter::Admin)
            .unwrap();
        assert!(s.delete_plugin_policy("p1", PolicySetter::Admin).unwrap());
        assert_eq!(s.effective_plugin_policy("p1").unwrap(), PluginPolicy::Off);
    }

    #[test]
    fn plugin_policy_list_returns_sorted_entries() {
        let s = temp_store();
        s.set_plugin_policy("zeta", PluginPolicy::On, PolicySetter::User)
            .unwrap();
        s.set_plugin_policy("alpha", PluginPolicy::Off, PolicySetter::User)
            .unwrap();
        s.set_plugin_policy("mid", PluginPolicy::Required, PolicySetter::Admin)
            .unwrap();
        let list = s.list_plugin_policies().unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].plugin_name, "alpha");
        assert_eq!(list[1].plugin_name, "mid");
        assert_eq!(list[2].plugin_name, "zeta");
        assert_eq!(list[1].policy, PluginPolicy::Required);
        assert_eq!(list[1].set_by, PolicySetter::Admin);
    }
}
