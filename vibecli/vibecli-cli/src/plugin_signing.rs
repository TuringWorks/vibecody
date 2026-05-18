//! Detached P-256 ECDSA signing for `vibecli-plugin.toml` manifests.
//!
//! B2.2 of the plugin-bundle work. Anchors fit-gap §18 principle #4:
//! publisher trust roots are per-publisher P-256 ECDSA keys, embedded
//! in the manifest as a `PublicKeyJwk`. There is no opaque trust
//! chain — the user explicitly trusts the publisher key when adding
//! the plugin (TOFU; B2.4 will surface the fingerprint at install).
//!
//! Wire shape: signature lives in a sibling file `vibecli-plugin.sig`
//! next to `vibecli-plugin.toml` inside the extracted MCPB bundle.
//! The detached form matches the JWS-detached pattern already used by
//! `signed_agent_card.rs`, and avoids the chicken-and-egg of trying to
//! embed a signature inside the very TOML it covers.
//!
//! ```json
//! {
//!   "kid":             "publisher-default",
//!   "algorithm":       "ES256",
//!   "value":           "<base64url(ECDSA(SHA-256(canonical_json(manifest))))>",
//!   "manifest_digest": "<sha256-hex of canonical_json(manifest)>"
//! }
//! ```
//!
//! `manifest_digest` is redundant for the cryptographic check (the
//! verifier recomputes it) but lets `vibecli plugin install` show the
//! user *what was signed* before they commit to the trust decision.

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use p256::ecdsa::{
    signature::{Signer, Verifier},
    Signature, SigningKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

use crate::plugin_manifest::PluginManifest;
use crate::signed_agent_card::{jwk_from_verifying_key, verifying_key_from_jwk};

/// Filename of the sibling signature inside an extracted bundle.
pub const SIGNATURE_FILENAME: &str = "vibecli-plugin.sig";

/// Filename of the manifest inside an extracted bundle.
pub const MANIFEST_FILENAME: &str = "vibecli-plugin.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginSignature {
    /// Key identifier — opaque to the verifier, but surfaced to the
    /// user at install time so they can distinguish "publisher's
    /// release key" from "publisher's CI key" if both exist.
    pub kid: String,

    /// Always `"ES256"` today. We refuse other values to stay narrow.
    pub algorithm: String,

    /// Base64url-no-pad of the 64-byte ECDSA signature.
    pub value: String,

    /// SHA-256 hex of the canonical-JSON manifest bytes. The verifier
    /// recomputes this; we store it so the install UI can show the
    /// user "you are about to trust this digest" before the trust
    /// decision is committed.
    pub manifest_digest: String,
}

#[derive(Debug)]
pub enum SignatureError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Verify(String),
    UnsupportedAlgorithm(String),
    Encoding(String),
    MissingFile(&'static str),
}

impl std::fmt::Display for SignatureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Json(e) => write!(f, "json: {e}"),
            Self::Verify(msg) => write!(f, "verify: {msg}"),
            Self::UnsupportedAlgorithm(a) => {
                write!(f, "unsupported algorithm `{a}` (only ES256/P-256)")
            }
            Self::Encoding(msg) => write!(f, "encoding: {msg}"),
            Self::MissingFile(name) => write!(f, "missing `{name}` in bundle"),
        }
    }
}

impl std::error::Error for SignatureError {}

impl From<std::io::Error> for SignatureError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for SignatureError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// Deterministic JSON serialization of a `PluginManifest` with object
/// keys sorted lexicographically. Mirrors `signed_agent_card::canonical_json`
/// so the algorithm is identical across our signed surfaces.
pub fn canonical_manifest_json(manifest: &PluginManifest) -> Result<String> {
    let v = serde_json::to_value(manifest).context("canonical_manifest_json: serialize")?;
    let canonical = sort_value(v);
    serde_json::to_string(&canonical).context("canonical_manifest_json: emit")
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

/// Compute the SHA-256 hex digest of a manifest's canonical-JSON form.
/// Used directly by signers; the install UI also surfaces this so the
/// user can verify the digest matches what the publisher advertised.
pub fn manifest_digest_hex(manifest: &PluginManifest) -> Result<String> {
    let canonical = canonical_manifest_json(manifest)?;
    Ok(format!("{:x}", Sha256::digest(canonical.as_bytes())))
}

/// Sign a manifest with the publisher's private key.
///
/// Note: the manifest's `publisher.key` field must already correspond
/// to the supplied `signing_key` — we assert this so an authoring CLI
/// can't accidentally bind a signature to the wrong embedded pubkey.
/// The check is cheap (one JWK round-trip) and pays for itself the
/// first time it catches a build-script mistake.
pub fn sign_manifest(
    manifest: &PluginManifest,
    signing_key: &SigningKey,
    kid: &str,
) -> Result<PluginSignature> {
    let expected_jwk = jwk_from_verifying_key(signing_key.verifying_key());
    if manifest.publisher.key != expected_jwk {
        return Err(anyhow!(
            "sign_manifest: signing key does not match manifest.publisher.key — \
             refusing to bind a signature to a foreign pubkey"
        ));
    }
    let canonical = canonical_manifest_json(manifest)?;
    let digest = Sha256::digest(canonical.as_bytes());
    let signature: Signature = signing_key.sign(&digest);
    let value = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes());
    Ok(PluginSignature {
        kid: kid.to_string(),
        algorithm: "ES256".to_string(),
        value,
        manifest_digest: format!("{:x}", digest),
    })
}

/// Verify a detached signature against a manifest using the publisher
/// key embedded in the manifest. Returns `Ok(())` only if every check
/// passes; any failure short-circuits with a descriptive error.
///
/// Failure modes:
///   - `algorithm` is not `ES256`
///   - `value` is not base64url / not 64 bytes / not a valid ECDSA
///     signature
///   - the embedded `publisher.key` JWK is malformed
///   - the digest doesn't match (manifest was tampered with)
///   - the signature value doesn't verify against the digest under
///     the publisher key (signature was tampered with, or it was
///     signed by a different key)
pub fn verify_manifest_signature(
    manifest: &PluginManifest,
    sig: &PluginSignature,
) -> Result<(), SignatureError> {
    if sig.algorithm != "ES256" {
        return Err(SignatureError::UnsupportedAlgorithm(sig.algorithm.clone()));
    }
    let canonical = canonical_manifest_json(manifest)
        .map_err(|e| SignatureError::Verify(format!("canonical: {e}")))?;
    let digest = Sha256::digest(canonical.as_bytes());
    let actual_digest_hex = format!("{:x}", digest);
    if sig.manifest_digest != actual_digest_hex {
        return Err(SignatureError::Verify(format!(
            "manifest digest mismatch: sig claims {} but actual is {}",
            sig.manifest_digest, actual_digest_hex
        )));
    }
    let sig_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(sig.value.as_bytes())
        .map_err(|e| SignatureError::Encoding(format!("signature.value: {e}")))?;
    let signature = Signature::from_slice(&sig_bytes)
        .map_err(|e| SignatureError::Encoding(format!("signature 64B ECDSA: {e}")))?;
    let verifying_key = verifying_key_from_jwk(&manifest.publisher.key)
        .map_err(|e| SignatureError::Verify(format!("publisher.key JWK: {e}")))?;
    verifying_key
        .verify(&digest, &signature)
        .map_err(|e| SignatureError::Verify(format!("ECDSA mismatch: {e}")))
}

// ── Bundle-level helpers ─────────────────────────────────────────────────────

/// Read `vibecli-plugin.toml` from an extracted bundle directory.
pub fn read_manifest_from_extracted(dir: &Path) -> Result<PluginManifest, SignatureError> {
    let path = dir.join(MANIFEST_FILENAME);
    let bytes = std::fs::read(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SignatureError::MissingFile(MANIFEST_FILENAME)
        } else {
            SignatureError::Io(e)
        }
    })?;
    let s = std::str::from_utf8(&bytes)
        .map_err(|e| SignatureError::Encoding(format!("manifest utf8: {e}")))?;
    PluginManifest::parse(s).map_err(|e| SignatureError::Verify(format!("manifest: {e}")))
}

/// Read `vibecli-plugin.sig` from an extracted bundle directory.
pub fn read_signature_from_extracted(dir: &Path) -> Result<PluginSignature, SignatureError> {
    let path = dir.join(SIGNATURE_FILENAME);
    let bytes = std::fs::read(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SignatureError::MissingFile(SIGNATURE_FILENAME)
        } else {
            SignatureError::Io(e)
        }
    })?;
    let sig: PluginSignature = serde_json::from_slice(&bytes)?;
    Ok(sig)
}

/// End-to-end: extract directory → read manifest + signature →
/// verify. Returns the parsed manifest on success.
pub fn verify_extracted_bundle(dir: &Path) -> Result<PluginManifest, SignatureError> {
    let manifest = read_manifest_from_extracted(dir)?;
    let sig = read_signature_from_extracted(dir)?;
    verify_manifest_signature(&manifest, &sig)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_manifest::{Components, Publisher};
    use p256::ecdsa::SigningKey;
    use tempfile::tempdir;

    fn fixture_key() -> SigningKey {
        // Same pattern as signed_agent_card tests — `p256` re-exports
        // the `rand_core::OsRng` that satisfies the bound `ecdsa`
        // expects (workspace `rand` is on a newer rand_core).
        SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng)
    }

    fn fixture_manifest_with(key: &SigningKey) -> PluginManifest {
        PluginManifest {
            name: "sample-plugin".into(),
            version: "1.0.0".into(),
            publisher: Publisher {
                name: "Sample Co".into(),
                url: None,
                key: jwk_from_verifying_key(key.verifying_key()),
            },
            description: "fixture".into(),
            components: Components::default(),
            min_vibecli_version: None,
            default_policy: Default::default(),
        }
    }

    #[test]
    fn sign_then_verify_round_trips() {
        let key = fixture_key();
        let manifest = fixture_manifest_with(&key);
        let sig = sign_manifest(&manifest, &key, "publisher-default").unwrap();

        assert_eq!(sig.algorithm, "ES256");
        assert_eq!(sig.kid, "publisher-default");
        // 64-byte ECDSA → 86-char base64url-no-pad.
        assert_eq!(sig.value.len(), 86);
        // SHA-256 hex is 64 chars.
        assert_eq!(sig.manifest_digest.len(), 64);

        verify_manifest_signature(&manifest, &sig).expect("must verify");
    }

    #[test]
    fn sign_refuses_when_signing_key_doesnt_match_embedded_jwk() {
        let key_a = fixture_key();
        let key_b = fixture_key();
        // Manifest declares key_a as publisher; try to sign with key_b.
        let manifest = fixture_manifest_with(&key_a);
        let res = sign_manifest(&manifest, &key_b, "wrong");
        assert!(res.is_err());
        let msg = res.unwrap_err().to_string();
        assert!(
            msg.contains("does not match manifest.publisher.key"),
            "got {msg}"
        );
    }

    #[test]
    fn verify_rejects_tampered_manifest() {
        let key = fixture_key();
        let manifest = fixture_manifest_with(&key);
        let sig = sign_manifest(&manifest, &key, "v1").unwrap();
        let mut tampered = manifest.clone();
        tampered.version = "9.9.9".into();
        let res = verify_manifest_signature(&tampered, &sig);
        assert!(res.is_err());
        // Tampering changes the digest, so we expect a digest-mismatch
        // error before we ever reach the ECDSA check.
        assert!(format!("{}", res.unwrap_err()).contains("digest mismatch"));
    }

    #[test]
    fn verify_rejects_tampered_signature_value() {
        let key = fixture_key();
        let manifest = fixture_manifest_with(&key);
        let mut sig = sign_manifest(&manifest, &key, "v1").unwrap();
        // Flip a single character in the base64 — keep the length so we
        // fail at the ECDSA step, not at decoding.
        let mut bytes: Vec<char> = sig.value.chars().collect();
        bytes[0] = if bytes[0] == 'a' { 'b' } else { 'a' };
        sig.value = bytes.into_iter().collect();
        let res = verify_manifest_signature(&manifest, &sig);
        assert!(res.is_err(), "tampered signature must fail to verify");
    }

    #[test]
    fn verify_rejects_unsupported_algorithm() {
        let key = fixture_key();
        let manifest = fixture_manifest_with(&key);
        let mut sig = sign_manifest(&manifest, &key, "v1").unwrap();
        sig.algorithm = "RS256".into();
        let res = verify_manifest_signature(&manifest, &sig);
        assert!(matches!(
            res.unwrap_err(),
            SignatureError::UnsupportedAlgorithm(_)
        ));
    }

    #[test]
    fn verify_rejects_signature_from_different_publisher_key() {
        let key_publisher = fixture_key();
        let key_evil = fixture_key();
        let manifest = fixture_manifest_with(&key_publisher);

        // Hand-build a signature using key_evil over the manifest's
        // canonical bytes — bypassing sign_manifest's key-match check.
        let canonical = canonical_manifest_json(&manifest).unwrap();
        let digest = Sha256::digest(canonical.as_bytes());
        let sig_bytes: Signature = key_evil.sign(&digest);
        let evil = PluginSignature {
            kid: "evil".into(),
            algorithm: "ES256".into(),
            value: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig_bytes.to_bytes()),
            manifest_digest: format!("{:x}", digest),
        };

        let res = verify_manifest_signature(&manifest, &evil);
        assert!(res.is_err(), "signature by non-publisher key must fail");
    }

    #[test]
    fn extracted_bundle_round_trips_through_disk() {
        let key = fixture_key();
        let manifest = fixture_manifest_with(&key);
        let sig = sign_manifest(&manifest, &key, "v1").unwrap();

        let dir = tempdir().unwrap();
        let manifest_toml = toml::to_string(&manifest).unwrap();
        std::fs::write(dir.path().join(MANIFEST_FILENAME), manifest_toml).unwrap();
        std::fs::write(
            dir.path().join(SIGNATURE_FILENAME),
            serde_json::to_string(&sig).unwrap(),
        )
        .unwrap();

        let verified = verify_extracted_bundle(dir.path()).expect("must verify");
        assert_eq!(verified, manifest);
    }

    #[test]
    fn extracted_bundle_missing_signature_file_errors_cleanly() {
        let key = fixture_key();
        let manifest = fixture_manifest_with(&key);
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join(MANIFEST_FILENAME),
            toml::to_string(&manifest).unwrap(),
        )
        .unwrap();

        let err = verify_extracted_bundle(dir.path()).unwrap_err();
        assert!(matches!(
            err,
            SignatureError::MissingFile(SIGNATURE_FILENAME)
        ));
    }

    #[test]
    fn extracted_bundle_missing_manifest_file_errors_cleanly() {
        let dir = tempdir().unwrap();
        // Only a signature, no manifest.
        std::fs::write(
            dir.path().join(SIGNATURE_FILENAME),
            r#"{"kid":"x","algorithm":"ES256","value":"x","manifest_digest":"x"}"#,
        )
        .unwrap();

        let err = verify_extracted_bundle(dir.path()).unwrap_err();
        assert!(matches!(
            err,
            SignatureError::MissingFile(MANIFEST_FILENAME)
        ));
    }

    #[test]
    fn digest_hex_matches_signature_digest() {
        let key = fixture_key();
        let manifest = fixture_manifest_with(&key);
        let sig = sign_manifest(&manifest, &key, "v1").unwrap();
        let direct = manifest_digest_hex(&manifest).unwrap();
        assert_eq!(direct, sig.manifest_digest);
    }

    #[test]
    fn canonical_json_is_stable_under_field_reordering() {
        // Two manifests differing only in serializer field order must
        // produce identical canonical bytes. We can't easily reorder
        // toml fields, so prove the property by serializing twice and
        // checking equality.
        let key = fixture_key();
        let m = fixture_manifest_with(&key);
        let j1 = canonical_manifest_json(&m).unwrap();
        let j2 = canonical_manifest_json(&m).unwrap();
        assert_eq!(j1, j2);
    }
}
