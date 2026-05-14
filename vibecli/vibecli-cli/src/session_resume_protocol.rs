//! Session resume protocol — cloud-agent remote-control envelope.
//!
//! Phase 53 P1 (A9 from v13 fitgap, Copilot Cloud Agent + VS 2026
//! Integrated Cloud Agent). When a host (mobile / watch / web) wants to
//! resume an in-flight CLI session, it presents a `ResumeHandoff`
//! envelope that the daemon verifies before unlocking the session.
//!
//! Wire shape:
//! ```json
//! {
//!   "v":          1,
//!   "sessionId":  "sess-abc123",
//!   "hostUrl":    "https://localhost:7878",
//!   "issuedAt":   1715000000,
//!   "expiresAt":  1715000300,
//!   "nonce":      "<random 16 bytes base64url>",
//!   "signature":  "<base64url(ECDSA(SHA-256(canonical(envelope without signature))))>"
//! }
//! ```
//!
//! P-256 ECDSA matches `watch_auth.rs` and the A2A signed-card layer
//! shipped under B6 (PR #14). One curve across the daemon keeps key
//! management coherent.
//!
//! Canonical bytes signed = JSON object with the `signature` field
//! omitted, keys sorted lexicographically. The receiver re-derives
//! these bytes deterministically before ECDSA-verify.

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use p256::ecdsa::{
    signature::{Signer, Verifier},
    Signature, SigningKey, VerifyingKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PROTOCOL_VERSION: u32 = 1;
/// Default validity window — 5 minutes from issue. Mobile/watch
/// clients refresh on demand; long-lived handoffs are an explicit
/// security risk.
pub const DEFAULT_TTL_SECS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResumeHandoff {
    pub v: u32,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "hostUrl")]
    pub host_url: String,
    #[serde(rename = "issuedAt")]
    pub issued_at: u64,
    #[serde(rename = "expiresAt")]
    pub expires_at: u64,
    pub nonce: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerifyError {
    UnsupportedVersion(u32),
    Expired { now: u64, expires_at: u64 },
    NotYetValid { now: u64, issued_at: u64 },
    EmptyField(&'static str),
    BadSignatureEncoding(String),
    SignatureMismatch,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerifyError::UnsupportedVersion(v) => write!(f, "unsupported handoff version: {v}"),
            VerifyError::Expired { now, expires_at } => write!(f, "handoff expired: now={now} expiresAt={expires_at}"),
            VerifyError::NotYetValid { now, issued_at } => write!(f, "handoff not yet valid: now={now} issuedAt={issued_at}"),
            VerifyError::EmptyField(name) => write!(f, "{name} must not be empty"),
            VerifyError::BadSignatureEncoding(e) => write!(f, "signature is not base64url: {e}"),
            VerifyError::SignatureMismatch => write!(f, "signature did not verify against expected key"),
        }
    }
}

impl std::error::Error for VerifyError {}

/// Sign a handoff envelope. Mutates the supplied envelope to populate
/// `signature` over the canonical JSON of every other field.
pub fn sign_handoff(envelope: &mut ResumeHandoff, signing_key_bytes: &[u8]) -> Result<()> {
    let signing_key = SigningKey::from_slice(signing_key_bytes)
        .context("sign: signing_key_bytes not a valid P-256 secret")?;
    let canonical = canonical_unsigned_bytes(envelope)?;
    let digest = Sha256::digest(&canonical);
    let sig: Signature = signing_key.sign(&digest);
    envelope.signature = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.to_bytes());
    Ok(())
}

/// Verify the envelope: structural fields, time window, and signature.
/// `now_unix` is passed explicitly so tests don't depend on wall clock.
pub fn verify_handoff(
    envelope: &ResumeHandoff,
    public_key_sec1: &[u8],
    now_unix: u64,
) -> std::result::Result<(), VerifyError> {
    if envelope.v != PROTOCOL_VERSION {
        return Err(VerifyError::UnsupportedVersion(envelope.v));
    }
    if envelope.session_id.trim().is_empty() {
        return Err(VerifyError::EmptyField("sessionId"));
    }
    if envelope.host_url.trim().is_empty() {
        return Err(VerifyError::EmptyField("hostUrl"));
    }
    if envelope.nonce.trim().is_empty() {
        return Err(VerifyError::EmptyField("nonce"));
    }
    if now_unix < envelope.issued_at {
        return Err(VerifyError::NotYetValid {
            now: now_unix,
            issued_at: envelope.issued_at,
        });
    }
    if now_unix > envelope.expires_at {
        return Err(VerifyError::Expired {
            now: now_unix,
            expires_at: envelope.expires_at,
        });
    }
    let sig_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(envelope.signature.as_bytes())
        .map_err(|e| VerifyError::BadSignatureEncoding(e.to_string()))?;
    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|e| VerifyError::BadSignatureEncoding(e.to_string()))?;
    let verifying_key = VerifyingKey::from_sec1_bytes(public_key_sec1)
        .map_err(|e| VerifyError::BadSignatureEncoding(format!("public key: {e}")))?;
    let canonical = canonical_unsigned_bytes(envelope)
        .map_err(|e| VerifyError::BadSignatureEncoding(e.to_string()))?;
    let digest = Sha256::digest(&canonical);
    verifying_key
        .verify(&digest, &signature)
        .map_err(|_| VerifyError::SignatureMismatch)
}

/// Helper for callers that want a fresh handoff with sane defaults.
pub fn build_handoff(
    session_id: &str,
    host_url: &str,
    issued_at_unix: u64,
    ttl_secs: u64,
    nonce_b64url: String,
) -> ResumeHandoff {
    ResumeHandoff {
        v: PROTOCOL_VERSION,
        session_id: session_id.to_string(),
        host_url: host_url.to_string(),
        issued_at: issued_at_unix,
        expires_at: issued_at_unix + ttl_secs,
        nonce: nonce_b64url,
        signature: String::new(),
    }
}

/// Canonical JSON bytes of the envelope with `signature` zeroed out
/// before signing. Sorted-key serialization makes the input
/// deterministic across platforms.
fn canonical_unsigned_bytes(envelope: &ResumeHandoff) -> Result<Vec<u8>> {
    let mut e = envelope.clone();
    e.signature = String::new();
    let v = serde_json::to_value(&e).context("serialize handoff")?;
    let sorted = sort_value(v);
    serde_json::to_vec(&sorted)
        .map_err(|e| anyhow!("emit canonical bytes: {e}"))
}

fn sort_value(v: serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<(String, serde_json::Value)> =
                map.into_iter().map(|(k, v)| (k, sort_value(v))).collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let mut out = serde_json::Map::with_capacity(entries.len());
            for (k, v) in entries {
                out.insert(k, v);
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(sort_value).collect())
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::SigningKey;
    use p256::elliptic_curve::sec1::ToEncodedPoint;
    use rand::rngs::OsRng;

    fn keypair() -> (Vec<u8>, Vec<u8>) {
        let sk = SigningKey::random(&mut OsRng);
        let sk_bytes = sk.to_bytes().to_vec();
        let pk_sec1 = sk.verifying_key().to_encoded_point(false).as_bytes().to_vec();
        (sk_bytes, pk_sec1)
    }

    fn fresh_envelope(issued_at: u64) -> ResumeHandoff {
        build_handoff(
            "sess-abc",
            "https://localhost:7878",
            issued_at,
            DEFAULT_TTL_SECS,
            "nonce-1234567890abcdef".into(),
        )
    }

    #[test]
    fn sign_then_verify_round_trips() {
        let (sk, pk) = keypair();
        let mut env = fresh_envelope(1_000_000);
        sign_handoff(&mut env, &sk).unwrap();
        verify_handoff(&env, &pk, 1_000_000 + 30).unwrap();
    }

    #[test]
    fn verify_rejects_after_expiry() {
        let (sk, pk) = keypair();
        let mut env = fresh_envelope(1_000_000);
        sign_handoff(&mut env, &sk).unwrap();
        let err = verify_handoff(&env, &pk, 1_000_000 + DEFAULT_TTL_SECS + 1).unwrap_err();
        assert!(matches!(err, VerifyError::Expired { .. }));
    }

    #[test]
    fn verify_rejects_tampered_session_id() {
        let (sk, pk) = keypair();
        let mut env = fresh_envelope(1_000_000);
        sign_handoff(&mut env, &sk).unwrap();
        env.session_id = "sess-other".into();
        let err = verify_handoff(&env, &pk, 1_000_000 + 30).unwrap_err();
        assert_eq!(err, VerifyError::SignatureMismatch);
    }

    #[test]
    fn verify_rejects_unsupported_version() {
        let (sk, pk) = keypair();
        let mut env = fresh_envelope(1_000_000);
        env.v = 99;
        sign_handoff(&mut env, &sk).unwrap();
        let err = verify_handoff(&env, &pk, 1_000_000 + 30).unwrap_err();
        assert!(matches!(err, VerifyError::UnsupportedVersion(99)));
    }

    #[test]
    fn verify_rejects_empty_session_id() {
        let (sk, pk) = keypair();
        let mut env = fresh_envelope(1_000_000);
        env.session_id = "".into();
        sign_handoff(&mut env, &sk).unwrap();
        let err = verify_handoff(&env, &pk, 1_000_000 + 30).unwrap_err();
        assert_eq!(err, VerifyError::EmptyField("sessionId"));
    }

    #[test]
    fn verify_rejects_bogus_signature_encoding() {
        let (_sk, pk) = keypair();
        let mut env = fresh_envelope(1_000_000);
        env.signature = "!!! not base64url !!!".into();
        let err = verify_handoff(&env, &pk, 1_000_000 + 30).unwrap_err();
        assert!(matches!(err, VerifyError::BadSignatureEncoding(_)));
    }
}
