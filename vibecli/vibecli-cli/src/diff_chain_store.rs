//! D1.1 — Encrypted persistence for diffcomplete chains.
//!
//! Co-located with `WorkspaceStore` on `<workspace>/.vibecli/workspace.db`,
//! using the same machine-bound + workspace-bound ChaCha20-Poly1305 key
//! derived from `vibecli-workspace-store-v1:HOME:USER:workspace_path`.
//! No new key derivation scheme.
//!
//! ## Schema
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS diff_chains (
//!     id                TEXT PRIMARY KEY,        -- ULID-style id
//!     file_path         TEXT NOT NULL,
//!     language          TEXT NOT NULL,
//!     selection_start   INTEGER NOT NULL,
//!     selection_end     INTEGER NOT NULL,
//!     original_text_enc BLOB NOT NULL,           -- ChaCha20-Poly1305
//!     steps_enc         BLOB NOT NULL,           -- ChaCha20-Poly1305(JSON Vec<DiffChainStep>)
//!     final_state       TEXT NOT NULL,           -- 'applied' | 'cancelled' | 'open'
//!     final_meta_json   TEXT,                    -- plaintext (no PII): step idx + reason
//!     parent_chain_id   TEXT,
//!     created_at        TEXT NOT NULL,           -- RFC3339
//!     updated_at        TEXT NOT NULL,
//!     schema_version    INTEGER NOT NULL DEFAULT 1
//! );
//! ```
//!
//! ## Patent posture
//!
//! Writes happen on discrete user-driven events only:
//!  * `upsert_step` — caller posts after a regenerate succeeds,
//!  * `set_final_state` — caller posts on Apply / Cancel / modal-close.
//!
//! Idempotency is on `(chain_id, max(step.index))` — the same regenerate
//! POSTed twice doesn't create a duplicate. There is no timer, no
//! polling, no idle scanner.
//!
//! Patent re-audit: PASS (elements 1–5 unchanged).

#![allow(dead_code)]

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use chrono::Utc;
use rand::Rng;
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::diff_chain::{DiffChain, DiffChainFinal, DiffChainStep};

// ── Key derivation (mirrors WorkspaceStore::derive_key) ─────────────────────

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

// ── AEAD helpers ────────────────────────────────────────────────────────────

fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let mut nonce_bytes = [0u8; 12];
    rand::rng().fill(&mut nonce_bytes);
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut ct = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("encrypt: {e}"))?;
    let mut blob = Vec::with_capacity(12 + ct.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.append(&mut ct);
    Ok(blob)
}

fn decrypt(key: &[u8; 32], blob: &[u8]) -> Result<Vec<u8>, String> {
    if blob.len() < 13 {
        return Err("blob too short".into());
    }
    let (nonce_bytes, ct) = blob.split_at(12);
    let cipher = ChaCha20Poly1305::new(key.into());
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ct)
        .map_err(|e| format!("decrypt: {e}"))
}

// ── Schema ──────────────────────────────────────────────────────────────────

/// Co-located with `WorkspaceStore::db_path`. Same DB file, additive
/// migrations only — `CREATE TABLE IF NOT EXISTS`.
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

         CREATE TABLE IF NOT EXISTS diff_chains (
             id                TEXT PRIMARY KEY,
             file_path         TEXT NOT NULL,
             language          TEXT NOT NULL,
             selection_start   INTEGER NOT NULL,
             selection_end     INTEGER NOT NULL,
             original_text_enc BLOB NOT NULL,
             steps_enc         BLOB NOT NULL,
             final_state       TEXT NOT NULL,
             final_meta_json   TEXT,
             parent_chain_id   TEXT,
             created_at        TEXT NOT NULL,
             updated_at        TEXT NOT NULL,
             schema_version    INTEGER NOT NULL DEFAULT 1
         );
         CREATE INDEX IF NOT EXISTS idx_chains_file ON diff_chains(file_path);
         CREATE INDEX IF NOT EXISTS idx_chains_updated ON diff_chains(updated_at);
         CREATE INDEX IF NOT EXISTS idx_chains_parent ON diff_chains(parent_chain_id);",
    )
    .map_err(|e| e.to_string())?;
    Ok(conn)
}

// ── Store ───────────────────────────────────────────────────────────────────

pub struct DiffChainStore {
    conn: Connection,
    key: [u8; 32],
    workspace_path: PathBuf,
}

impl DiffChainStore {
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

    /// Tests open against an explicit DB path with a fixed key so they
    /// don't touch real workspace databases.
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

    /// D1.1 — Append-or-update a chain step. Idempotent: re-posting
    /// the same `(chain_id, step.index)` returns the existing row
    /// untouched (matches the design doc's "POSTed twice" rule).
    /// When `chain_id` does not yet exist, the chain row is created
    /// with `final_state = "open"`.
    pub fn upsert_step(
        &self,
        chain_id: &str,
        file_path: &str,
        language: &str,
        selection_start: u32,
        selection_end: u32,
        original_text: &str,
        step: &DiffChainStep,
        parent_chain_id: Option<&str>,
    ) -> Result<DiffChain, String> {
        let now = Utc::now().to_rfc3339();
        let existing = self.get(chain_id)?;
        let chain = match existing {
            Some(mut c) => {
                // Idempotency: identical step-index + identical body is a no-op.
                if let Some(present) = c.steps.iter().find(|s| s.index == step.index) {
                    if present == step {
                        return Ok(c);
                    }
                    return Err(format!(
                        "step {} already present for chain {chain_id} with different body — \
                         use a fresh chain_id (forking) instead of rewriting",
                        step.index
                    ));
                }
                c.steps.push(step.clone());
                c.updated_at = Utc::now();
                c
            }
            None => DiffChain {
                id: chain_id.to_string(),
                workspace: self.workspace_path.clone(),
                file_path: file_path.to_string(),
                language: language.to_string(),
                selection_start,
                selection_end,
                original_text: original_text.to_string(),
                steps: vec![step.clone()],
                final_state: DiffChainFinal::Open,
                parent_chain_id: parent_chain_id.map(|s| s.to_string()),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                schema_version: 1,
            },
        };
        let original_text_enc = encrypt(&self.key, chain.original_text.as_bytes())?;
        let steps_json = serde_json::to_vec(&chain.steps).map_err(|e| e.to_string())?;
        let steps_enc = encrypt(&self.key, &steps_json)?;
        let created_at_str = chain.created_at.to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO diff_chains (
                     id, file_path, language, selection_start, selection_end,
                     original_text_enc, steps_enc, final_state, final_meta_json,
                     parent_chain_id, created_at, updated_at, schema_version
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
                 ON CONFLICT(id) DO UPDATE SET
                     steps_enc=excluded.steps_enc,
                     updated_at=excluded.updated_at",
                params![
                    chain.id,
                    chain.file_path,
                    chain.language,
                    chain.selection_start,
                    chain.selection_end,
                    original_text_enc,
                    steps_enc,
                    chain.final_state.as_db_str(),
                    serde_json::Value::Null.to_string(),
                    chain.parent_chain_id,
                    created_at_str,
                    now,
                    chain.schema_version as i64,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(chain)
    }

    /// D1.1 — Update the chain's final state. Discrete events only:
    /// Apply, Cancel, modal-closed. The autosave caller posts here
    /// once per chain at most (the modal is one-shot).
    pub fn set_final_state(
        &self,
        chain_id: &str,
        final_state: &DiffChainFinal,
    ) -> Result<(), String> {
        let meta_json = serde_json::to_string(final_state).map_err(|e| e.to_string())?;
        let now = Utc::now().to_rfc3339();
        let n = self
            .conn
            .execute(
                "UPDATE diff_chains
                 SET final_state=?1, final_meta_json=?2, updated_at=?3
                 WHERE id=?4",
                params![final_state.as_db_str(), meta_json, now, chain_id],
            )
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err(format!("no chain with id {chain_id}"));
        }
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Option<DiffChain>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, file_path, language, selection_start, selection_end,
                        original_text_enc, steps_enc, final_state, final_meta_json,
                        parent_chain_id, created_at, updated_at, schema_version
                 FROM diff_chains WHERE id=?1",
            )
            .map_err(|e| e.to_string())?;
        let row = stmt.query_row(params![id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, Vec<u8>>(5)?,
                r.get::<_, Vec<u8>>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, Option<String>>(8)?,
                r.get::<_, Option<String>>(9)?,
                r.get::<_, String>(10)?,
                r.get::<_, String>(11)?,
                r.get::<_, i64>(12)?,
            ))
        });
        match row {
            Ok(t) => Ok(Some(self.row_to_chain(t)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn list_for_file(&self, file_path: &str, limit: usize) -> Result<Vec<DiffChain>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, file_path, language, selection_start, selection_end,
                        original_text_enc, steps_enc, final_state, final_meta_json,
                        parent_chain_id, created_at, updated_at, schema_version
                 FROM diff_chains
                 WHERE file_path=?1
                 ORDER BY updated_at DESC
                 LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![file_path, limit as i64], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, Vec<u8>>(5)?,
                    r.get::<_, Vec<u8>>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, Option<String>>(8)?,
                    r.get::<_, Option<String>>(9)?,
                    r.get::<_, String>(10)?,
                    r.get::<_, String>(11)?,
                    r.get::<_, i64>(12)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            let t = r.map_err(|e| e.to_string())?;
            out.push(self.row_to_chain(t)?);
        }
        Ok(out)
    }

    fn row_to_chain(
        &self,
        t: (
            String,
            String,
            String,
            i64,
            i64,
            Vec<u8>,
            Vec<u8>,
            String,
            Option<String>,
            Option<String>,
            String,
            String,
            i64,
        ),
    ) -> Result<DiffChain, String> {
        let (
            id,
            file_path,
            language,
            selection_start,
            selection_end,
            original_text_enc,
            steps_enc,
            final_state_str,
            final_meta_json,
            parent_chain_id,
            created_at,
            updated_at,
            schema_version,
        ) = t;
        let original_text = String::from_utf8(decrypt(&self.key, &original_text_enc)?)
            .map_err(|e| format!("utf8: {e}"))?;
        let steps_bytes = decrypt(&self.key, &steps_enc)?;
        let steps: Vec<DiffChainStep> =
            serde_json::from_slice(&steps_bytes).map_err(|e| e.to_string())?;
        let final_state = match (final_state_str.as_str(), final_meta_json) {
            ("open", _) => DiffChainFinal::Open,
            (_, Some(s)) => serde_json::from_str(&s).map_err(|e| e.to_string())?,
            (other, None) => {
                return Err(format!(
                    "row {id} has final_state={other:?} but no final_meta_json"
                ))
            }
        };
        Ok(DiffChain {
            id,
            workspace: self.workspace_path.clone(),
            file_path,
            language,
            selection_start: selection_start as u32,
            selection_end: selection_end as u32,
            original_text,
            steps,
            final_state,
            parent_chain_id,
            created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                .map_err(|e| e.to_string())?
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                .map_err(|e| e.to_string())?
                .with_timezone(&chrono::Utc),
            schema_version: schema_version as u16,
        })
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff_chain::{CancellationReason, DiffChainStep};
    use chrono::Utc;
    use tempfile::TempDir;

    fn step(index: u32, instr: &str) -> DiffChainStep {
        DiffChainStep {
            index,
            instruction: instr.to_string(),
            refinement: None,
            additional_files: vec![],
            diff: format!("diff-{index}"),
            provider: "anthropic".into(),
            model: "claude-opus-4-7".into(),
            tokens_input: 100,
            tokens_output: 20,
            generated_at: Utc::now(),
        }
    }

    fn open_store(tmp: &TempDir) -> DiffChainStore {
        DiffChainStore::open_with(&tmp.path().join("ws.db"), [42u8; 32]).unwrap()
    }

    #[test]
    fn upsert_first_step_creates_chain() {
        let tmp = TempDir::new().unwrap();
        let store = open_store(&tmp);
        let chain = store
            .upsert_step(
                "c1",
                "src/auth.rs",
                "rust",
                10,
                20,
                "fn validate() {}",
                &step(0, "tighten error path"),
                None,
            )
            .unwrap();
        assert_eq!(chain.id, "c1");
        assert_eq!(chain.steps.len(), 1);
        assert_eq!(chain.final_state, DiffChainFinal::Open);
    }

    #[test]
    fn upsert_idempotent_on_same_step_index_and_body() {
        let tmp = TempDir::new().unwrap();
        let store = open_store(&tmp);
        let s = step(0, "first");
        store
            .upsert_step("c1", "f.rs", "rust", 0, 0, "orig", &s, None)
            .unwrap();
        let again = store
            .upsert_step("c1", "f.rs", "rust", 0, 0, "orig", &s, None)
            .unwrap();
        assert_eq!(again.steps.len(), 1, "duplicate post must not double-write");
    }

    #[test]
    fn upsert_rejects_rewriting_an_existing_step_with_different_body() {
        let tmp = TempDir::new().unwrap();
        let store = open_store(&tmp);
        store
            .upsert_step("c1", "f.rs", "rust", 0, 0, "orig", &step(0, "first"), None)
            .unwrap();
        let mut conflict = step(0, "DIFFERENT");
        conflict.diff = "different diff".into();
        let err = store
            .upsert_step("c1", "f.rs", "rust", 0, 0, "orig", &conflict, None)
            .unwrap_err();
        assert!(err.contains("already present"), "got: {err}");
    }

    #[test]
    fn appending_multiple_steps_keeps_them_in_order() {
        let tmp = TempDir::new().unwrap();
        let store = open_store(&tmp);
        for i in 0..3u32 {
            store
                .upsert_step(
                    "c1",
                    "f.rs",
                    "rust",
                    0,
                    0,
                    "orig",
                    &step(i, &format!("step{i}")),
                    None,
                )
                .unwrap();
        }
        let got = store.get("c1").unwrap().unwrap();
        assert_eq!(got.steps.len(), 3);
        assert_eq!(got.steps[0].index, 0);
        assert_eq!(got.steps[2].index, 2);
    }

    #[test]
    fn set_final_state_persists_apply_and_cancel() {
        let tmp = TempDir::new().unwrap();
        let store = open_store(&tmp);
        store
            .upsert_step("c1", "f.rs", "rust", 0, 0, "orig", &step(0, "x"), None)
            .unwrap();

        store
            .set_final_state(
                "c1",
                &DiffChainFinal::Applied {
                    applied_step: 0,
                    applied_at: Utc::now(),
                },
            )
            .unwrap();
        let got = store.get("c1").unwrap().unwrap();
        match got.final_state {
            DiffChainFinal::Applied { applied_step, .. } => assert_eq!(applied_step, 0),
            other => panic!("expected applied, got {other:?}"),
        }

        store
            .set_final_state(
                "c1",
                &DiffChainFinal::Cancelled {
                    reason: CancellationReason::UserCancel,
                    cancelled_at: Utc::now(),
                },
            )
            .unwrap();
        let got = store.get("c1").unwrap().unwrap();
        assert!(matches!(got.final_state, DiffChainFinal::Cancelled { .. }));
    }

    #[test]
    fn original_text_round_trips_through_encryption() {
        let tmp = TempDir::new().unwrap();
        let store = open_store(&tmp);
        let secret = "fn validate(token: &str) -> bool { token == \"hunter2\" }";
        store
            .upsert_step("c1", "f.rs", "rust", 0, 0, secret, &step(0, "x"), None)
            .unwrap();
        let got = store.get("c1").unwrap().unwrap();
        assert_eq!(got.original_text, secret);
    }

    #[test]
    fn list_for_file_orders_newest_first() {
        let tmp = TempDir::new().unwrap();
        let store = open_store(&tmp);
        for id in ["c1", "c2", "c3"] {
            store
                .upsert_step(id, "f.rs", "rust", 0, 0, "orig", &step(0, "x"), None)
                .unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let list = store.list_for_file("f.rs", 10).unwrap();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].id, "c3", "newest first");
        assert_eq!(list[2].id, "c1");
    }

    #[test]
    fn set_final_state_on_unknown_chain_is_an_error() {
        let tmp = TempDir::new().unwrap();
        let store = open_store(&tmp);
        let err = store
            .set_final_state(
                "does-not-exist",
                &DiffChainFinal::Applied {
                    applied_step: 0,
                    applied_at: Utc::now(),
                },
            )
            .unwrap_err();
        assert!(err.contains("no chain"), "got: {err}");
    }

    #[test]
    fn raw_blob_in_db_is_not_plaintext() {
        let tmp = TempDir::new().unwrap();
        let store = open_store(&tmp);
        let secret = "MARKER_DO_NOT_LEAK_TO_DISK";
        store
            .upsert_step("c1", "f.rs", "rust", 0, 0, secret, &step(0, "x"), None)
            .unwrap();
        // Reopen the raw connection — the blob bytes must not contain
        // the plaintext marker.
        let conn = Connection::open(tmp.path().join("ws.db")).unwrap();
        let blob: Vec<u8> = conn
            .query_row(
                "SELECT original_text_enc FROM diff_chains WHERE id=?1",
                params!["c1"],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            !blob.windows(secret.len()).any(|w| w == secret.as_bytes()),
            "plaintext leaked into encrypted blob"
        );
    }
}
