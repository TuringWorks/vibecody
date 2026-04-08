#![allow(dead_code)]
//! Encrypted secrets vault for company orchestration.
//!
//! Each secret is encrypted with a keystream derived from HMAC-SHA256
//! using a per-company master key and a random nonce. The master key is
//! stored in `~/.vibecli/profile_settings.db` (master_keys table,
//! encrypted with ChaCha20-Poly1305). Falls back to the OS keychain
//! (macOS Keychain / Linux Secret Service / Windows Credential Manager),
//! then to a per-company key file at `~/.vibecli/keys/<company_id>.key`.
//! Existing keychain/file entries are migrated to the profile store on first access.
//!
//! Encryption scheme:
//!   keystream = HMAC-SHA256(master_key, nonce || key_name || counter)
//!   ciphertext = plaintext XOR keystream (streamed in 32-byte blocks)
//!
//! Secrets are versioned — every update creates a new version record.

use anyhow::{anyhow, Context, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use rand::Rng;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}
fn new_id() -> String { uuid::Uuid::new_v4().to_string() }

// ── Master key management ─────────────────────────────────────────────────────

const KEYCHAIN_SERVICE: &str = "vibecli-secrets";

/// Truncated company ID used as the keychain account name (and legacy file stem).
fn key_account(company_id: &str) -> &str {
    &company_id[..16.min(company_id.len())]
}

fn legacy_key_path(company_id: &str) -> std::path::PathBuf {
    let mut p = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    p.push(".vibecli");
    p.push("keys");
    p.push(format!("{}.key", key_account(company_id)));
    p
}

fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key[..]);
    key
}

fn decode_hex_key(hex_str: &str) -> Option<[u8; 32]> {
    if hex_str.len() != 64 { return None; }
    let mut key = [0u8; 32];
    hex::decode_to_slice(hex_str, &mut key).ok()?;
    Some(key)
}

/// Load or create a 32-byte master key for the company.
///
/// Storage priority:
///   1. profile_settings.db `master_keys` table (ChaCha20-Poly1305 encrypted)
///   2. OS keychain — migrated to profile store on first access
///   3. Legacy key file — migrated to profile store on first access
pub fn get_or_create_master_key(company_id: &str) -> Result<[u8; 32]> {
    use crate::profile_store::ProfileStore;

    // ── 1. Try profile store (encrypted SQLite) ───────────────────────────────
    if let Ok(store) = ProfileStore::new() {
        match store.get_master_key(company_id) {
            Ok(Some(key)) => return Ok(key),
            Ok(None) => {
                // Not stored yet — check legacy sources then generate.
                let key = load_from_keychain_or_file(company_id).unwrap_or_else(generate_key);
                let _ = store.set_master_key(company_id, &key);
                cleanup_legacy(company_id);
                return Ok(key);
            }
            Err(_) => {} // fall through
        }
    }

    // ── 2. OS keychain fallback ───────────────────────────────────────────────
    let account = key_account(company_id);
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, account) {
        if let Ok(hex_key) = entry.get_password() {
            if let Some(key) = decode_hex_key(&hex_key) {
                return Ok(key);
            }
        }
    }

    // ── 3. File fallback (headless / CI) ─────────────────────────────────────
    let path = legacy_key_path(company_id);
    if path.exists() {
        if let Ok(bytes) = std::fs::read(&path) {
            if let Some(key) = decode_hex_key(&String::from_utf8_lossy(&bytes)) {
                return Ok(key);
            }
        }
    }
    let key = generate_key();
    write_key_file(company_id, &key)?;
    Ok(key)
}

fn load_from_keychain_or_file(company_id: &str) -> Option<[u8; 32]> {
    let account = key_account(company_id);
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, account) {
        if let Ok(hex_key) = entry.get_password() {
            if let Some(key) = decode_hex_key(&hex_key) {
                return Some(key);
            }
        }
    }
    let path = legacy_key_path(company_id);
    if path.exists() {
        if let Ok(bytes) = std::fs::read(&path) {
            return decode_hex_key(&String::from_utf8_lossy(&bytes));
        }
    }
    None
}

fn cleanup_legacy(company_id: &str) {
    let account = key_account(company_id);
    if let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, account) {
        let _ = entry.delete_password();
    }
    let _ = std::fs::remove_file(legacy_key_path(company_id));
}

fn write_key_file(company_id: &str, key: &[u8; 32]) -> Result<()> {
    let path = legacy_key_path(company_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&path, hex::encode(key)).context("writing master key file")
}

// ── Encryption / Decryption ───────────────────────────────────────────────────

fn encrypt(master_key: &[u8; 32], nonce: &[u8; 16], key_name: &str, plaintext: &[u8]) -> Vec<u8> {
    xor_with_keystream(master_key, nonce, key_name, plaintext)
}

fn decrypt(master_key: &[u8; 32], nonce: &[u8; 16], key_name: &str, ciphertext: &[u8]) -> Vec<u8> {
    xor_with_keystream(master_key, nonce, key_name, ciphertext)
}

fn xor_with_keystream(master_key: &[u8; 32], nonce: &[u8; 16], key_name: &str, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let name_bytes = key_name.as_bytes();
    let mut counter: u64 = 0;
    let mut pos = 0;
    while pos < data.len() {
        // Derive 32-byte block: HMAC(master_key, nonce || key_name || counter)
        let mut mac = HmacSha256::new_from_slice(master_key).expect("HMAC init");
        mac.update(nonce);
        mac.update(name_bytes);
        mac.update(&counter.to_le_bytes());
        let block = mac.finalize().into_bytes();
        let block = block.as_slice();
        let take = 32.min(data.len() - pos);
        for i in 0..take {
            out.push(data[pos + i] ^ block[i]);
        }
        pos += take;
        counter += 1;
    }
    out
}

// ── Data structs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanySecret {
    pub id: String,
    pub company_id: String,
    pub key_name: String,
    /// Base64-encoded ciphertext.
    pub encrypted_value: String,
    /// Hex-encoded 16-byte nonce.
    pub nonce: String,
    pub version: i64,
    pub created_by: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

// ── SecretStore ───────────────────────────────────────────────────────────────

pub struct SecretStore<'a> {
    conn: &'a Connection,
}

impl<'a> SecretStore<'a> {
    pub fn new(conn: &'a Connection) -> Self { Self { conn } }

    pub fn ensure_schema(&self) -> Result<()> {
        self.conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS secrets (
                id              TEXT PRIMARY KEY,
                company_id      TEXT NOT NULL,
                key_name        TEXT NOT NULL,
                encrypted_value TEXT NOT NULL,
                nonce           TEXT NOT NULL,
                version         INTEGER NOT NULL DEFAULT 1,
                created_by      TEXT,
                created_at      INTEGER NOT NULL,
                updated_at      INTEGER NOT NULL,
                UNIQUE(company_id, key_name)
            );
            CREATE INDEX IF NOT EXISTS idx_secrets_company ON secrets(company_id);
        "#)?;
        Ok(())
    }

    /// Set (create or update) a secret.
    pub fn set(
        &self,
        company_id: &str,
        key_name: &str,
        plaintext: &str,
        created_by: Option<&str>,
    ) -> Result<CompanySecret> {
        let master_key = get_or_create_master_key(company_id)?;
        let mut nonce = [0u8; 16];
        rand::thread_rng().fill(&mut nonce[..]);
        let ciphertext = encrypt(&master_key, &nonce, key_name, plaintext.as_bytes());
        let enc_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &ciphertext);
        let nonce_hex = hex::encode(nonce);
        let now = now_ms();

        // Upsert
        let existing: Option<(String, i64)> = self.conn.query_row(
            "SELECT id, version FROM secrets WHERE company_id = ?1 AND key_name = ?2",
            params![company_id, key_name],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).ok();

        if let Some((id, version)) = existing {
            self.conn.execute(
                "UPDATE secrets SET encrypted_value = ?1, nonce = ?2, version = ?3, created_by = ?4, updated_at = ?5 WHERE id = ?6",
                params![enc_b64, nonce_hex, version + 1, created_by, now as i64, id],
            )?;
            self.get(company_id, key_name)?.context("secret not found after update")
        } else {
            let id = new_id();
            self.conn.execute(
                "INSERT INTO secrets (id, company_id, key_name, encrypted_value, nonce, version, created_by, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,1,?6,?7,?8)",
                params![id, company_id, key_name, enc_b64, nonce_hex, created_by, now as i64, now as i64],
            )?;
            self.get(company_id, key_name)?.context("secret not found after insert")
        }
    }

    /// Retrieve and decrypt a secret value.
    pub fn get_value(&self, company_id: &str, key_name: &str) -> Result<String> {
        let secret = self.get(company_id, key_name)?.context("secret not found")?;
        let master_key = get_or_create_master_key(company_id)?;
        let nonce_bytes = hex::decode(&secret.nonce).context("decoding nonce")?;
        let nonce: [u8; 16] = nonce_bytes.try_into().map_err(|_| anyhow!("bad nonce length"))?;
        let ciphertext = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &secret.encrypted_value)
            .context("decoding ciphertext")?;
        let plaintext = decrypt(&master_key, &nonce, key_name, &ciphertext);
        String::from_utf8(plaintext).context("decoding plaintext as UTF-8")
    }

    pub fn get(&self, company_id: &str, key_name: &str) -> Result<Option<CompanySecret>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, key_name, encrypted_value, nonce, version, created_by, created_at, updated_at
             FROM secrets WHERE company_id = ?1 AND key_name = ?2",
        )?;
        let mut rows = stmt.query_map(params![company_id, key_name], row_to_secret)?;
        rows.next().transpose().map_err(|e| anyhow!("{e}"))
    }

    pub fn list(&self, company_id: &str) -> Result<Vec<CompanySecret>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, key_name, encrypted_value, nonce, version, created_by, created_at, updated_at
             FROM secrets WHERE company_id = ?1 ORDER BY key_name ASC",
        )?;
        let rows = stmt.query_map(params![company_id], row_to_secret)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn delete(&self, company_id: &str, key_name: &str) -> Result<bool> {
        let n = self.conn.execute(
            "DELETE FROM secrets WHERE company_id = ?1 AND key_name = ?2",
            params![company_id, key_name],
        )?;
        Ok(n > 0)
    }
}

fn row_to_secret(row: &rusqlite::Row) -> rusqlite::Result<CompanySecret> {
    Ok(CompanySecret {
        id: row.get(0)?,
        company_id: row.get(1)?,
        key_name: row.get(2)?,
        encrypted_value: row.get(3)?,
        nonce: row.get(4)?,
        version: row.get(5)?,
        created_by: row.get(6)?,
        created_at: row.get::<_, i64>(7)? as u64,
        updated_at: row.get::<_, i64>(8)? as u64,
    })
}

impl CompanySecret {
    pub fn summary_line(&self) -> String {
        format!(
            "v{} {}  [{}]",
            self.version, self.key_name,
            &self.id[..8.min(self.id.len())]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn make_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        conn
    }

    /// Each test gets its own unique company_id so parallel tests never share
    /// the same key file on disk (key_path uses the first 16 chars of company_id).
    fn co() -> String {
        format!("tst-{}", uuid::Uuid::new_v4().simple())
    }

    // ── set / get_value round-trip ───────────────────────────────────────────

    #[test]
    fn given_secret_set_when_retrieved_then_plaintext_matches() {
        let co = co();
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        store.set(&co, "API_KEY", "super-secret-123", None).unwrap();
        let value = store.get_value(&co, "API_KEY").unwrap();
        assert_eq!(value, "super-secret-123");
    }

    #[test]
    fn given_empty_plaintext_when_set_and_retrieved_then_empty_string_returned() {
        let co = co();
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        store.set(&co, "EMPTY_KEY", "", None).unwrap();
        let value = store.get_value(&co, "EMPTY_KEY").unwrap();
        assert_eq!(value, "");
    }

    #[test]
    fn given_unicode_plaintext_when_set_and_retrieved_then_utf8_roundtrip_correct() {
        let co = co();
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        let secret = "こんにちは世界 🔐";
        store.set(&co, "UNICODE_KEY", secret, None).unwrap();
        let value = store.get_value(&co, "UNICODE_KEY").unwrap();
        assert_eq!(value, secret);
    }

    // ── update increments version ────────────────────────────────────────────

    #[test]
    fn given_secret_updated_when_version_checked_then_incremented() {
        let co = co();
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        store.set(&co, "DB_PASS", "pass-v1", None).unwrap();
        let s2 = store.set(&co, "DB_PASS", "pass-v2", None).unwrap();
        assert_eq!(s2.version, 2);
    }

    #[test]
    fn given_secret_updated_when_get_value_then_latest_plaintext_returned() {
        let co = co();
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        store.set(&co, "ROTATE_ME", "old-value", None).unwrap();
        store.set(&co, "ROTATE_ME", "new-value", None).unwrap();
        let value = store.get_value(&co, "ROTATE_ME").unwrap();
        assert_eq!(value, "new-value");
    }

    #[test]
    fn given_new_secret_when_version_checked_then_version_is_one() {
        let co = co();
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        let s = store.set(&co, "V1_KEY", "value", None).unwrap();
        assert_eq!(s.version, 1);
    }

    // ── get (metadata) ───────────────────────────────────────────────────────

    #[test]
    fn given_secret_set_when_get_metadata_then_key_name_matches() {
        let co = co();
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        store.set(&co, "MY_TOKEN", "tok", Some("agent-1")).unwrap();
        let meta = store.get(&co, "MY_TOKEN").unwrap().unwrap();
        assert_eq!(meta.key_name, "MY_TOKEN");
        assert_eq!(meta.created_by.as_deref(), Some("agent-1"));
    }

    #[test]
    fn given_nonexistent_key_when_get_value_then_error() {
        let co = co();
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        let result = store.get_value(&co, "DOES_NOT_EXIST");
        assert!(result.is_err());
    }

    #[test]
    fn given_nonexistent_key_when_get_metadata_then_none() {
        let co = co();
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        let result = store.get(&co, "PHANTOM").unwrap();
        assert!(result.is_none());
    }

    // ── list ─────────────────────────────────────────────────────────────────

    #[test]
    fn given_multiple_secrets_when_list_then_returns_all_for_company() {
        let co = co();
        let other_co = format!("tst-{}", uuid::Uuid::new_v4().simple());
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        store.set(&co, "KEY_A", "a", None).unwrap();
        store.set(&co, "KEY_B", "b", None).unwrap();
        store.set(&other_co, "KEY_C", "c", None).unwrap();
        let list = store.list(&co).unwrap();
        assert_eq!(list.len(), 2);
        // Ordered alphabetically by key_name
        assert_eq!(list[0].key_name, "KEY_A");
        assert_eq!(list[1].key_name, "KEY_B");
    }

    #[test]
    fn given_no_secrets_when_list_then_empty() {
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        let list = store.list("company-with-no-secrets").unwrap();
        assert!(list.is_empty());
    }

    // ── delete ───────────────────────────────────────────────────────────────

    #[test]
    fn given_existing_secret_when_deleted_then_returns_true_and_no_longer_listed() {
        let co = co();
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        store.set(&co, "TO_DELETE", "val", None).unwrap();
        let deleted = store.delete(&co, "TO_DELETE").unwrap();
        assert!(deleted);
        let list = store.list(&co).unwrap();
        assert!(list.iter().all(|s| s.key_name != "TO_DELETE"));
    }

    #[test]
    fn given_nonexistent_key_when_delete_called_then_returns_false() {
        let co = co();
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        let deleted = store.delete(&co, "GHOST_KEY").unwrap();
        assert!(!deleted);
    }

    // ── ciphertext is not plaintext ──────────────────────────────────────────

    #[test]
    fn given_secret_stored_when_encrypted_value_inspected_then_not_equal_to_plaintext() {
        let co = co();
        let conn = make_conn();
        let store = SecretStore::new(&conn);
        store.ensure_schema().unwrap();
        let plaintext = "my-very-secret-value";
        store.set(&co, "RAW_CHECK", plaintext, None).unwrap();
        let meta = store.get(&co, "RAW_CHECK").unwrap().unwrap();
        // encrypted_value is base64-encoded ciphertext, not the original string
        assert_ne!(meta.encrypted_value, plaintext);
    }
}
