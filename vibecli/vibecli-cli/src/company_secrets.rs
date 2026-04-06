#![allow(dead_code)]
//! Encrypted secrets vault for company orchestration.
//!
//! Each secret is encrypted with a keystream derived from HMAC-SHA256
//! using a per-company master key and a random nonce. The master key is
//! stored in a per-company key file at `~/.vibecli/keys/<company_id>.key`.
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

fn key_path(company_id: &str) -> std::path::PathBuf {
    let mut p = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    p.push(".vibecli");
    p.push("keys");
    p.push(format!("{}.key", &company_id[..16.min(company_id.len())]));
    p
}

/// Load or create a 32-byte master key for the company.
pub fn get_or_create_master_key(company_id: &str) -> Result<[u8; 32]> {
    let path = key_path(company_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    if path.exists() {
        let bytes = std::fs::read(&path).context("reading master key")?;
        if bytes.len() == 64 {
            let mut key = [0u8; 32];
            hex::decode_to_slice(&bytes, &mut key).context("decoding master key")?;
            return Ok(key);
        }
    }
    // Generate new key
    let mut key = [0u8; 32];
    rand::thread_rng().fill(&mut key[..]);
    std::fs::write(&path, hex::encode(key)).context("writing master key")?;
    Ok(key)
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
        let mut rows = stmt.query_map(params![company_id, key_name], |row| row_to_secret(row))?;
        rows.next().transpose().map_err(|e| anyhow!("{e}"))
    }

    pub fn list(&self, company_id: &str) -> Result<Vec<CompanySecret>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, company_id, key_name, encrypted_value, nonce, version, created_by, created_at, updated_at
             FROM secrets WHERE company_id = ?1 ORDER BY key_name ASC",
        )?;
        let rows = stmt.query_map(params![company_id], |row| row_to_secret(row))?
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
