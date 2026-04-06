#![allow(dead_code)]
//! System-level encrypted settings store.
//!
//! Database: `~/.vibecli/profile_settings.db`
//! Encryption: ChaCha20-Poly1305 (AEAD) — random 12-byte nonce prepended
//!             to ciphertext in every BLOB column.
//! Key: SHA-256("vibecli-profile-store-v1:" + $HOME + ":" + $USER)
//!
//! Tables:
//!   profiles         — named setting profiles (default: "default")
//!   panel_settings   — encrypted UI panel settings per profile
//!   api_keys         — encrypted provider API keys per profile
//!   provider_configs — encrypted provider settings (model, endpoint, etc.)
//!   global_settings  — encrypted app-wide settings
//!   master_keys      — encrypted company master encryption keys

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::Rng;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

// ── Key derivation ────────────────────────────────────────────────────────────

/// Derives the store encryption key from machine identity.
/// Machine-bound: SHA-256("vibecli-profile-store-v1:" + HOME + ":" + USER)
pub fn derive_key() -> [u8; 32] {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();
    let mut h = Sha256::new();
    h.update(b"vibecli-profile-store-v1:");
    h.update(home.as_bytes());
    h.update(b":");
    h.update(user.as_bytes());
    h.finalize().into()
}

// ── Encryption / Decryption ───────────────────────────────────────────────────

/// Encrypts `plaintext` → nonce(12) || ciphertext blob.
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

/// Decrypts a nonce(12) || ciphertext blob → plaintext string.
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

pub fn db_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "HOME not set".to_string())?;
    let dir = PathBuf::from(home).join(".vibecli");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("profile_settings.db"))
}

fn open_conn(path: &PathBuf) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;

         CREATE TABLE IF NOT EXISTS profiles (
             id         TEXT PRIMARY KEY,
             name       TEXT NOT NULL,
             is_default INTEGER NOT NULL DEFAULT 0,
             created_at INTEGER NOT NULL
         );

         CREATE TABLE IF NOT EXISTS panel_settings (
             id            INTEGER PRIMARY KEY AUTOINCREMENT,
             profile_id    TEXT    NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
             panel_name    TEXT    NOT NULL,
             setting_key   TEXT    NOT NULL,
             setting_value BLOB    NOT NULL,
             updated_at    INTEGER NOT NULL,
             UNIQUE(profile_id, panel_name, setting_key)
         );
         CREATE INDEX IF NOT EXISTS idx_ps_lookup
             ON panel_settings(profile_id, panel_name);

         CREATE TABLE IF NOT EXISTS api_keys (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             profile_id      TEXT    NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
             provider        TEXT    NOT NULL,
             encrypted_value BLOB    NOT NULL,
             updated_at      INTEGER NOT NULL,
             UNIQUE(profile_id, provider)
         );
         CREATE INDEX IF NOT EXISTS idx_ak_profile ON api_keys(profile_id);

         CREATE TABLE IF NOT EXISTS provider_configs (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             profile_id      TEXT    NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
             provider        TEXT    NOT NULL,
             setting_key     TEXT    NOT NULL,
             setting_value   BLOB    NOT NULL,
             updated_at      INTEGER NOT NULL,
             UNIQUE(profile_id, provider, setting_key)
         );
         CREATE INDEX IF NOT EXISTS idx_pc_profile_provider
             ON provider_configs(profile_id, provider);

         CREATE TABLE IF NOT EXISTS global_settings (
             id              INTEGER PRIMARY KEY AUTOINCREMENT,
             profile_id      TEXT    NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
             setting_key     TEXT    NOT NULL,
             setting_value   BLOB    NOT NULL,
             updated_at      INTEGER NOT NULL,
             UNIQUE(profile_id, setting_key)
         );
         CREATE INDEX IF NOT EXISTS idx_gs_profile ON global_settings(profile_id);

         CREATE TABLE IF NOT EXISTS master_keys (
             company_id      TEXT    PRIMARY KEY,
             encrypted_value BLOB    NOT NULL,
             created_at      INTEGER NOT NULL,
             updated_at      INTEGER NOT NULL
         );

         INSERT OR IGNORE INTO profiles (id, name, is_default, created_at)
             VALUES ('default', 'Default', 1, unixepoch() * 1000);",
    )
    .map_err(|e| e.to_string())?;
    Ok(conn)
}

// ── ProfileStore ──────────────────────────────────────────────────────────────

pub struct ProfileStore {
    conn: Connection,
    key: [u8; 32],
}

impl ProfileStore {
    pub fn new() -> Result<Self, String> {
        let path = db_path()?;
        let conn = open_conn(&path)?;
        Ok(Self { conn, key: derive_key() })
    }

    /// For tests: open against an arbitrary path with a custom key.
    pub fn open_with(path: &PathBuf, key: [u8; 32]) -> Result<Self, String> {
        let conn = open_conn(path)?;
        Ok(Self { conn, key })
    }

    // ── Panel settings (migrated from panel_settings.db) ─────────────────────

    pub fn get(
        &self,
        profile_id: &str,
        panel: &str,
        key: &str,
    ) -> Result<Option<String>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT setting_value FROM panel_settings
                 WHERE profile_id=?1 AND panel_name=?2 AND setting_key=?3",
            )
            .map_err(|e| e.to_string())?;
        let result: rusqlite::Result<Vec<u8>> =
            stmt.query_row(params![profile_id, panel, key], |r| r.get(0));
        match result {
            Ok(blob) => Ok(Some(decrypt(&self.key, &blob)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn get_all(
        &self,
        profile_id: &str,
        panel: &str,
    ) -> Result<serde_json::Value, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT setting_key, setting_value FROM panel_settings
                 WHERE profile_id=?1 AND panel_name=?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![profile_id, panel], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut map = serde_json::Map::new();
        for row in rows {
            let (k, blob) = row.map_err(|e| e.to_string())?;
            if let Ok(v) = decrypt(&self.key, &blob) {
                let val = serde_json::from_str(&v).unwrap_or(serde_json::Value::String(v));
                map.insert(k, val);
            }
        }
        Ok(serde_json::Value::Object(map))
    }

    pub fn set(
        &self,
        profile_id: &str,
        panel: &str,
        key: &str,
        value: &str,
    ) -> Result<(), String> {
        let blob = encrypt(&self.key, value)?;
        self.conn
            .execute(
                "INSERT INTO panel_settings
                     (profile_id, panel_name, setting_key, setting_value, updated_at)
                 VALUES (?1,?2,?3,?4,?5)
                 ON CONFLICT(profile_id, panel_name, setting_key)
                 DO UPDATE SET setting_value=excluded.setting_value,
                               updated_at=excluded.updated_at",
                params![profile_id, panel, key, blob, now_ms()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete(&self, profile_id: &str, panel: &str, key: &str) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM panel_settings
                 WHERE profile_id=?1 AND panel_name=?2 AND setting_key=?3",
                params![profile_id, panel, key],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_panel(&self, profile_id: &str, panel: &str) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM panel_settings WHERE profile_id=?1 AND panel_name=?2",
                params![profile_id, panel],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── API keys ─────────────────────────────────────────────────────────────

    pub fn set_api_key(
        &self,
        profile_id: &str,
        provider: &str,
        api_key: &str,
    ) -> Result<(), String> {
        let blob = encrypt(&self.key, api_key)?;
        self.conn
            .execute(
                "INSERT INTO api_keys (profile_id, provider, encrypted_value, updated_at)
                 VALUES (?1,?2,?3,?4)
                 ON CONFLICT(profile_id, provider)
                 DO UPDATE SET encrypted_value=excluded.encrypted_value,
                               updated_at=excluded.updated_at",
                params![profile_id, provider, blob, now_ms()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_api_key(
        &self,
        profile_id: &str,
        provider: &str,
    ) -> Result<Option<String>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT encrypted_value FROM api_keys
                 WHERE profile_id=?1 AND provider=?2",
            )
            .map_err(|e| e.to_string())?;
        let result: rusqlite::Result<Vec<u8>> =
            stmt.query_row(params![profile_id, provider], |r| r.get(0));
        match result {
            Ok(blob) => Ok(Some(decrypt(&self.key, &blob)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn list_api_key_providers(
        &self,
        profile_id: &str,
    ) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT provider FROM api_keys WHERE profile_id=?1 ORDER BY provider",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![profile_id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    pub fn delete_api_key(
        &self,
        profile_id: &str,
        provider: &str,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM api_keys WHERE profile_id=?1 AND provider=?2",
                params![profile_id, provider],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Provider configs ─────────────────────────────────────────────────────

    pub fn set_provider_config(
        &self,
        profile_id: &str,
        provider: &str,
        key: &str,
        value: &str,
    ) -> Result<(), String> {
        let blob = encrypt(&self.key, value)?;
        self.conn
            .execute(
                "INSERT INTO provider_configs
                     (profile_id, provider, setting_key, setting_value, updated_at)
                 VALUES (?1,?2,?3,?4,?5)
                 ON CONFLICT(profile_id, provider, setting_key)
                 DO UPDATE SET setting_value=excluded.setting_value,
                               updated_at=excluded.updated_at",
                params![profile_id, provider, key, blob, now_ms()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_provider_config(
        &self,
        profile_id: &str,
        provider: &str,
        key: &str,
    ) -> Result<Option<String>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT setting_value FROM provider_configs
                 WHERE profile_id=?1 AND provider=?2 AND setting_key=?3",
            )
            .map_err(|e| e.to_string())?;
        let result: rusqlite::Result<Vec<u8>> =
            stmt.query_row(params![profile_id, provider, key], |r| r.get(0));
        match result {
            Ok(blob) => Ok(Some(decrypt(&self.key, &blob)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn get_all_provider_config(
        &self,
        profile_id: &str,
        provider: &str,
    ) -> Result<serde_json::Value, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT setting_key, setting_value FROM provider_configs
                 WHERE profile_id=?1 AND provider=?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![profile_id, provider], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut map = serde_json::Map::new();
        for row in rows {
            let (k, blob) = row.map_err(|e| e.to_string())?;
            if let Ok(v) = decrypt(&self.key, &blob) {
                map.insert(k, serde_json::Value::String(v));
            }
        }
        Ok(serde_json::Value::Object(map))
    }

    // ── Global settings ───────────────────────────────────────────────────────

    pub fn set_global(
        &self,
        profile_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), String> {
        let blob = encrypt(&self.key, value)?;
        self.conn
            .execute(
                "INSERT INTO global_settings
                     (profile_id, setting_key, setting_value, updated_at)
                 VALUES (?1,?2,?3,?4)
                 ON CONFLICT(profile_id, setting_key)
                 DO UPDATE SET setting_value=excluded.setting_value,
                               updated_at=excluded.updated_at",
                params![profile_id, key, blob, now_ms()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_global(
        &self,
        profile_id: &str,
        key: &str,
    ) -> Result<Option<String>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT setting_value FROM global_settings
                 WHERE profile_id=?1 AND setting_key=?2",
            )
            .map_err(|e| e.to_string())?;
        let result: rusqlite::Result<Vec<u8>> =
            stmt.query_row(params![profile_id, key], |r| r.get(0));
        match result {
            Ok(blob) => Ok(Some(decrypt(&self.key, &blob)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn get_all_global(
        &self,
        profile_id: &str,
    ) -> Result<serde_json::Value, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT setting_key, setting_value FROM global_settings
                 WHERE profile_id=?1 ORDER BY setting_key",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![profile_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut map = serde_json::Map::new();
        for row in rows {
            let (k, blob) = row.map_err(|e| e.to_string())?;
            if let Ok(v) = decrypt(&self.key, &blob) {
                map.insert(k, serde_json::Value::String(v));
            }
        }
        Ok(serde_json::Value::Object(map))
    }

    pub fn delete_global(&self, profile_id: &str, key: &str) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM global_settings WHERE profile_id=?1 AND setting_key=?2",
                params![profile_id, key],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Company master keys ───────────────────────────────────────────────────

    /// Retrieve and decrypt the 32-byte master key for a company.
    pub fn get_master_key(&self, company_id: &str) -> Result<Option<[u8; 32]>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT encrypted_value FROM master_keys WHERE company_id=?1",
            )
            .map_err(|e| e.to_string())?;
        let result: rusqlite::Result<Vec<u8>> =
            stmt.query_row(params![company_id], |r| r.get(0));
        match result {
            Ok(blob) => {
                let hex_key = decrypt(&self.key, &blob)?;
                if hex_key.len() != 64 {
                    return Err("master key corrupt: wrong hex length".into());
                }
                let mut key = [0u8; 32];
                hex::decode_to_slice(&hex_key, &mut key)
                    .map_err(|e| format!("master key hex decode: {e}"))?;
                Ok(Some(key))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Encrypt and store the 32-byte master key for a company.
    pub fn set_master_key(
        &self,
        company_id: &str,
        key: &[u8; 32],
    ) -> Result<(), String> {
        let hex_key = hex::encode(key);
        let blob = encrypt(&self.key, &hex_key)?;
        let now = now_ms();
        self.conn
            .execute(
                "INSERT INTO master_keys (company_id, encrypted_value, created_at, updated_at)
                 VALUES (?1,?2,?3,?3)
                 ON CONFLICT(company_id)
                 DO UPDATE SET encrypted_value=excluded.encrypted_value,
                               updated_at=excluded.updated_at",
                params![company_id, blob, now],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_master_key(&self, company_id: &str) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM master_keys WHERE company_id=?1",
                params![company_id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Profile management ────────────────────────────────────────────────────

    pub fn list_profiles(&self) -> Result<Vec<serde_json::Value>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, created_at, is_default FROM profiles
                 ORDER BY is_default DESC, name",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, String>(0)?,
                    "name": r.get::<_, String>(1)?,
                    "created_at": r.get::<_, i64>(2)?,
                    "is_default": r.get::<_, bool>(3)?,
                }))
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    pub fn create_profile(&self, id: &str, name: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO profiles (id, name, is_default, created_at) VALUES (?1,?2,0,?3)",
                params![id, name, now_ms()],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_profile(&self, id: &str) -> Result<(), String> {
        if id == "default" {
            return Err("Cannot delete the default profile".into());
        }
        self.conn
            .execute("DELETE FROM profiles WHERE id=?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_default_profile(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute_batch("UPDATE profiles SET is_default=0")
            .map_err(|e| e.to_string())?;
        self.conn
            .execute("UPDATE profiles SET is_default=1 WHERE id=?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_default_profile_id(&self) -> Result<String, String> {
        self.conn
            .query_row(
                "SELECT id FROM profiles WHERE is_default=1 LIMIT 1",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }

    pub fn export_profile(&self, profile_id: &str) -> Result<serde_json::Value, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT panel_name FROM panel_settings WHERE profile_id=?1",
            )
            .map_err(|e| e.to_string())?;
        let panels: Vec<String> = stmt
            .query_map(params![profile_id], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let mut result = serde_json::Map::new();
        for panel in panels {
            result.insert(panel.clone(), self.get_all(profile_id, &panel)?);
        }
        Ok(serde_json::Value::Object(result))
    }

    pub fn import_profile(
        &self,
        profile_id: &str,
        data: &serde_json::Value,
    ) -> Result<u32, String> {
        let mut count = 0u32;
        if let Some(obj) = data.as_object() {
            for (panel, settings) in obj {
                if let Some(settings_obj) = settings.as_object() {
                    for (key, value) in settings_obj {
                        let val_str = match value {
                            serde_json::Value::String(s) => s.clone(),
                            other => serde_json::to_string(other).unwrap_or_default(),
                        };
                        self.set(profile_id, panel, key, &val_str)?;
                        count += 1;
                    }
                }
            }
        }
        Ok(count)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> ProfileStore {
        let dir = std::env::temp_dir().join(format!(
            "vibe_ps_test_{}_{}",
            std::process::id(),
            rand::random::<u32>()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("profile_settings.db");
        let key = [42u8; 32];
        ProfileStore::open_with(&path, key).unwrap()
    }

    // ── encryption round-trip ──────────────────────────────────────────────────

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = [1u8; 32];
        let ct = encrypt(&key, "hello world").unwrap();
        let pt = decrypt(&key, &ct).unwrap();
        assert_eq!(pt, "hello world");
    }

    #[test]
    fn encrypt_uses_random_nonce_so_outputs_differ() {
        let key = [1u8; 32];
        let ct1 = encrypt(&key, "same").unwrap();
        let ct2 = encrypt(&key, "same").unwrap();
        assert_ne!(ct1, ct2, "random nonces should produce different ciphertexts");
    }

    #[test]
    fn decrypt_with_wrong_key_fails() {
        let k1 = [1u8; 32];
        let k2 = [2u8; 32];
        let ct = encrypt(&k1, "secret").unwrap();
        assert!(decrypt(&k2, &ct).is_err());
    }

    #[test]
    fn derive_key_is_deterministic() {
        assert_eq!(derive_key(), derive_key());
    }

    #[test]
    fn derive_key_produces_32_bytes() {
        assert_eq!(derive_key().len(), 32);
    }

    // ── panel settings ────────────────────────────────────────────────────────

    #[test]
    fn panel_set_and_get() {
        let s = temp_store();
        s.set("default", "editor", "theme", "dark").unwrap();
        assert_eq!(s.get("default", "editor", "theme").unwrap(), Some("dark".into()));
    }

    #[test]
    fn panel_get_missing_returns_none() {
        let s = temp_store();
        assert_eq!(s.get("default", "editor", "nope").unwrap(), None);
    }

    #[test]
    fn panel_set_overwrites() {
        let s = temp_store();
        s.set("default", "editor", "theme", "dark").unwrap();
        s.set("default", "editor", "theme", "light").unwrap();
        assert_eq!(s.get("default", "editor", "theme").unwrap(), Some("light".into()));
    }

    #[test]
    fn panel_get_all() {
        let s = temp_store();
        s.set("default", "editor", "theme", "dark").unwrap();
        s.set("default", "editor", "font_size", "14").unwrap();
        let all = s.get_all("default", "editor").unwrap();
        let obj = all.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert_eq!(obj["theme"], "dark");
        assert_eq!(obj["font_size"], "14");
    }

    #[test]
    fn panel_delete_setting() {
        let s = temp_store();
        s.set("default", "editor", "theme", "dark").unwrap();
        s.delete("default", "editor", "theme").unwrap();
        assert_eq!(s.get("default", "editor", "theme").unwrap(), None);
    }

    #[test]
    fn panel_delete_panel() {
        let s = temp_store();
        s.set("default", "editor", "theme", "dark").unwrap();
        s.set("default", "editor", "font_size", "14").unwrap();
        s.delete_panel("default", "editor").unwrap();
        assert!(s.get_all("default", "editor").unwrap().as_object().unwrap().is_empty());
    }

    // ── api keys ──────────────────────────────────────────────────────────────

    #[test]
    fn api_key_set_and_get() {
        let s = temp_store();
        s.set_api_key("default", "openai", "sk-test-123").unwrap();
        assert_eq!(
            s.get_api_key("default", "openai").unwrap(),
            Some("sk-test-123".into())
        );
    }

    #[test]
    fn api_key_missing_returns_none() {
        let s = temp_store();
        assert_eq!(s.get_api_key("default", "openai").unwrap(), None);
    }

    #[test]
    fn api_key_overwrites() {
        let s = temp_store();
        s.set_api_key("default", "openai", "sk-old").unwrap();
        s.set_api_key("default", "openai", "sk-new").unwrap();
        assert_eq!(s.get_api_key("default", "openai").unwrap(), Some("sk-new".into()));
    }

    #[test]
    fn api_key_list_providers() {
        let s = temp_store();
        s.set_api_key("default", "openai", "k1").unwrap();
        s.set_api_key("default", "anthropic", "k2").unwrap();
        let providers = s.list_api_key_providers("default").unwrap();
        assert!(providers.contains(&"openai".to_string()));
        assert!(providers.contains(&"anthropic".to_string()));
    }

    #[test]
    fn api_key_delete() {
        let s = temp_store();
        s.set_api_key("default", "openai", "sk-test").unwrap();
        s.delete_api_key("default", "openai").unwrap();
        assert_eq!(s.get_api_key("default", "openai").unwrap(), None);
    }

    // ── provider configs ──────────────────────────────────────────────────────

    #[test]
    fn provider_config_set_and_get() {
        let s = temp_store();
        s.set_provider_config("default", "openai", "model", "gpt-4o").unwrap();
        assert_eq!(
            s.get_provider_config("default", "openai", "model").unwrap(),
            Some("gpt-4o".into())
        );
    }

    #[test]
    fn provider_config_get_all() {
        let s = temp_store();
        s.set_provider_config("default", "openai", "model", "gpt-4o").unwrap();
        s.set_provider_config("default", "openai", "endpoint", "https://api.openai.com").unwrap();
        let all = s.get_all_provider_config("default", "openai").unwrap();
        let obj = all.as_object().unwrap();
        assert_eq!(obj.len(), 2);
    }

    // ── global settings ───────────────────────────────────────────────────────

    #[test]
    fn global_set_and_get() {
        let s = temp_store();
        s.set_global("default", "theme", "dark").unwrap();
        assert_eq!(s.get_global("default", "theme").unwrap(), Some("dark".into()));
    }

    #[test]
    fn global_get_all() {
        let s = temp_store();
        s.set_global("default", "theme", "dark").unwrap();
        s.set_global("default", "telemetry", "false").unwrap();
        let all = s.get_all_global("default").unwrap();
        assert_eq!(all.as_object().unwrap().len(), 2);
    }

    #[test]
    fn global_delete() {
        let s = temp_store();
        s.set_global("default", "theme", "dark").unwrap();
        s.delete_global("default", "theme").unwrap();
        assert_eq!(s.get_global("default", "theme").unwrap(), None);
    }

    // ── master keys ───────────────────────────────────────────────────────────

    #[test]
    fn master_key_set_and_get() {
        let s = temp_store();
        let key = [7u8; 32];
        s.set_master_key("company-abc", &key).unwrap();
        assert_eq!(s.get_master_key("company-abc").unwrap(), Some(key));
    }

    #[test]
    fn master_key_missing_returns_none() {
        let s = temp_store();
        assert_eq!(s.get_master_key("no-such-company").unwrap(), None);
    }

    #[test]
    fn master_key_overwrites() {
        let s = temp_store();
        let k1 = [1u8; 32];
        let k2 = [2u8; 32];
        s.set_master_key("co", &k1).unwrap();
        s.set_master_key("co", &k2).unwrap();
        assert_eq!(s.get_master_key("co").unwrap(), Some(k2));
    }

    #[test]
    fn master_key_delete() {
        let s = temp_store();
        let key = [3u8; 32];
        s.set_master_key("co", &key).unwrap();
        s.delete_master_key("co").unwrap();
        assert_eq!(s.get_master_key("co").unwrap(), None);
    }

    // ── profiles ──────────────────────────────────────────────────────────────

    #[test]
    fn list_profiles_has_default() {
        let s = temp_store();
        let profiles = s.list_profiles().unwrap();
        assert!(profiles.iter().any(|p| p["id"] == "default"));
    }

    #[test]
    fn create_and_list_profile() {
        let s = temp_store();
        s.create_profile("work", "Work").unwrap();
        let profiles = s.list_profiles().unwrap();
        assert!(profiles.iter().any(|p| p["id"] == "work"));
    }

    #[test]
    fn delete_default_profile_fails() {
        let s = temp_store();
        assert!(s.delete_profile("default").is_err());
    }

    #[test]
    fn set_default_profile() {
        let s = temp_store();
        s.create_profile("work", "Work").unwrap();
        s.set_default_profile("work").unwrap();
        assert_eq!(s.get_default_profile_id().unwrap(), "work");
    }

    #[test]
    fn export_and_import_roundtrip() {
        let s = temp_store();
        s.set("default", "editor", "theme", "dark").unwrap();
        s.set("default", "terminal", "shell", "/bin/zsh").unwrap();
        let exported = s.export_profile("default").unwrap();
        s.create_profile("copy", "Copy").unwrap();
        let count = s.import_profile("copy", &exported).unwrap();
        assert_eq!(count, 2);
        assert_eq!(s.get("copy", "editor", "theme").unwrap(), Some("dark".into()));
    }
}
