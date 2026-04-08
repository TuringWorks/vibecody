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
use rand::Rng;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};
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
    rand::thread_rng().fill(&mut nonce_bytes);
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
         CREATE INDEX IF NOT EXISTS idx_ws_secrets_key ON workspace_secrets(key_name);",
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
        Ok(Self { conn, key, workspace_path: canonical })
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
        let result: rusqlite::Result<Vec<u8>> =
            stmt.query_row(params![key], |r| r.get(0));
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
            .execute(
                "DELETE FROM workspace_settings WHERE key=?1",
                params![key],
            )
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
            .prepare(
                "SELECT encrypted_value FROM workspace_secrets WHERE key_name=?1",
            )
            .map_err(|e| e.to_string())?;
        let result: rusqlite::Result<Vec<u8>> =
            stmt.query_row(params![key_name], |r| r.get(0));
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
            self.secret_meta(key_name)?.ok_or_else(|| "secret not found after update".into())
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
            self.secret_meta(key_name)?.ok_or_else(|| "secret not found after insert".into())
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
        s.secret_set("DB_URL", "postgres://localhost/mydb", None).unwrap();
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
        s.secret_set("DB_URL", "postgres://...", Some("agent-1")).unwrap();
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
}
