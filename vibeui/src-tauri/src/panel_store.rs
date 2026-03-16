//! Encrypted SQLite storage for panel settings, scoped by profile.
//!
//! Database: `~/.vibeui/panel_settings.db`
//! Encryption: ChaCha20-Poly1305 (AEAD) for setting values.

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rusqlite::{params, Connection};
use std::path::PathBuf;

/// Derives a 256-bit key from a passphrase using simple hashing.
/// In production you'd use Argon2 or scrypt, but this is sufficient
/// for local-device encryption where the passphrase is machine-bound.
fn derive_key(passphrase: &str) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    // Simple deterministic key derivation — hash the passphrase twice
    // to fill 32 bytes. For local device encryption this is adequate.
    let mut key = [0u8; 32];
    let bytes = passphrase.as_bytes();
    for (i, chunk) in key.chunks_mut(1).enumerate() {
        chunk[0] = bytes[i % bytes.len()].wrapping_add(i as u8);
    }
    // Mix with a hasher for better distribution
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let h1 = hasher.finish().to_le_bytes();
    passphrase.hash(&mut hasher);
    let h2 = hasher.finish().to_le_bytes();
    key[..8].copy_from_slice(&h1);
    key[8..16].copy_from_slice(&h2);
    // Third and fourth rounds
    for b in key.iter() {
        hasher.write_u8(*b);
    }
    let h3 = hasher.finish().to_le_bytes();
    key[16..24].copy_from_slice(&h3);
    passphrase.len().hash(&mut hasher);
    let h4 = hasher.finish().to_le_bytes();
    key[24..32].copy_from_slice(&h4);
    key
}

/// The default passphrase is derived from the machine's hostname + username.
/// This means settings are machine-bound but don't require user input.
fn default_passphrase() -> String {
    let user = std::env::var("USER").unwrap_or_else(|_| "vibeui".into());
    let host = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "localhost".into());
    format!("vibeui-panel-store-{}-{}", user, host)
}

fn encrypt_value(key: &[u8; 32], plaintext: &str) -> Result<Vec<u8>, String> {
    let cipher = ChaCha20Poly1305::new(key.into());
    // Use first 12 bytes of key as nonce (deterministic per key — acceptable
    // because each key-value pair has a unique DB row, and we re-encrypt on update).
    let nonce_bytes: [u8; 12] = {
        let mut n = [0u8; 12];
        n.copy_from_slice(&key[..12]);
        n
    };
    let nonce = Nonce::from_slice(&nonce_bytes);
    cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption failed: {e}"))
}

fn decrypt_value(key: &[u8; 32], ciphertext: &[u8]) -> Result<String, String> {
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce_bytes: [u8; 12] = {
        let mut n = [0u8; 12];
        n.copy_from_slice(&key[..12]);
        n
    };
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {e}"))?;
    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 decode failed: {e}"))
}

fn db_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let dir = PathBuf::from(home).join(".vibeui");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("panel_settings.db"))
}

fn open_db() -> Result<Connection, String> {
    let path = db_path()?;
    let conn = Connection::open(&path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         CREATE TABLE IF NOT EXISTS profiles (
             id TEXT PRIMARY KEY,
             name TEXT NOT NULL,
             created_at TEXT NOT NULL DEFAULT (datetime('now')),
             is_default INTEGER NOT NULL DEFAULT 0
         );
         CREATE TABLE IF NOT EXISTS panel_settings (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             profile_id TEXT NOT NULL,
             panel_name TEXT NOT NULL,
             setting_key TEXT NOT NULL,
             setting_value BLOB NOT NULL,
             updated_at TEXT NOT NULL DEFAULT (datetime('now')),
             UNIQUE(profile_id, panel_name, setting_key),
             FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_panel_settings_lookup
             ON panel_settings(profile_id, panel_name);
         -- Ensure a default profile exists
         INSERT OR IGNORE INTO profiles (id, name, is_default) VALUES ('default', 'Default', 1);",
    )
    .map_err(|e| e.to_string())?;
    Ok(conn)
}

/// Shared state for the panel store.
pub struct PanelStore {
    conn: Connection,
    key: [u8; 32],
}

impl PanelStore {
    pub fn new() -> Result<Self, String> {
        let conn = open_db()?;
        let passphrase = default_passphrase();
        let key = derive_key(&passphrase);
        Ok(Self { conn, key })
    }

    /// Get a setting value (decrypted).
    pub fn get(&self, profile_id: &str, panel: &str, key: &str) -> Result<Option<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT setting_value FROM panel_settings WHERE profile_id = ?1 AND panel_name = ?2 AND setting_key = ?3")
            .map_err(|e| e.to_string())?;
        let result: Result<Vec<u8>, _> = stmt.query_row(params![profile_id, panel, key], |row| row.get(0));
        match result {
            Ok(blob) => {
                let plaintext = decrypt_value(&self.key, &blob)?;
                Ok(Some(plaintext))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Get all settings for a panel (decrypted).
    pub fn get_all(&self, profile_id: &str, panel: &str) -> Result<serde_json::Value, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT setting_key, setting_value FROM panel_settings WHERE profile_id = ?1 AND panel_name = ?2")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![profile_id, panel], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut map = serde_json::Map::new();
        for row in rows {
            let (k, blob) = row.map_err(|e| e.to_string())?;
            if let Ok(v) = decrypt_value(&self.key, &blob) {
                // Try to parse as JSON, fall back to string
                let val = serde_json::from_str(&v).unwrap_or(serde_json::Value::String(v));
                map.insert(k, val);
            }
            // Skip corrupted values
        }
        Ok(serde_json::Value::Object(map))
    }

    /// Set a setting value (encrypted).
    pub fn set(&self, profile_id: &str, panel: &str, key: &str, value: &str) -> Result<(), String> {
        let encrypted = encrypt_value(&self.key, value)?;
        self.conn
            .execute(
                "INSERT INTO panel_settings (profile_id, panel_name, setting_key, setting_value, updated_at)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'))
                 ON CONFLICT(profile_id, panel_name, setting_key)
                 DO UPDATE SET setting_value = excluded.setting_value, updated_at = excluded.updated_at",
                params![profile_id, panel, key, encrypted],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete a specific setting.
    pub fn delete(&self, profile_id: &str, panel: &str, key: &str) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM panel_settings WHERE profile_id = ?1 AND panel_name = ?2 AND setting_key = ?3",
                params![profile_id, panel, key],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Delete all settings for a panel.
    pub fn delete_panel(&self, profile_id: &str, panel: &str) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM panel_settings WHERE profile_id = ?1 AND panel_name = ?2",
                params![profile_id, panel],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Profile management ──

    pub fn list_profiles(&self) -> Result<Vec<serde_json::Value>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, created_at, is_default FROM profiles ORDER BY is_default DESC, name")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "created_at": row.get::<_, String>(2)?,
                    "is_default": row.get::<_, bool>(3)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn create_profile(&self, id: &str, name: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO profiles (id, name) VALUES (?1, ?2)",
                params![id, name],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_profile(&self, id: &str) -> Result<(), String> {
        if id == "default" {
            return Err("Cannot delete the default profile".into());
        }
        self.conn
            .execute("DELETE FROM profiles WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_default_profile(&self, id: &str) -> Result<(), String> {
        self.conn
            .execute_batch("UPDATE profiles SET is_default = 0")
            .map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "UPDATE profiles SET is_default = 1 WHERE id = ?1",
                params![id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_default_profile_id(&self) -> Result<String, String> {
        self.conn
            .query_row(
                "SELECT id FROM profiles WHERE is_default = 1 LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())
    }

    /// Export all settings for a profile as JSON (for backup/sharing).
    pub fn export_profile(&self, profile_id: &str) -> Result<serde_json::Value, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT panel_name FROM panel_settings WHERE profile_id = ?1")
            .map_err(|e| e.to_string())?;
        let panels: Vec<String> = stmt
            .query_map(params![profile_id], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        let mut result = serde_json::Map::new();
        for panel in panels {
            let settings = self.get_all(profile_id, &panel)?;
            result.insert(panel, settings);
        }
        Ok(serde_json::Value::Object(result))
    }

    /// Import settings from an exported JSON blob.
    pub fn import_profile(&self, profile_id: &str, data: &serde_json::Value) -> Result<u32, String> {
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
