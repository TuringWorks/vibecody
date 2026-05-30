//! A2A v1.2 signed agent cards — P-256 ECDSA over the canonical JSON of
//! an [`AgentCard`].
//!
//! The Linux Foundation's Agentic AI Foundation A2A spec (v1.2, Q1 2026)
//! adds cryptographic signatures to agent cards so consuming agents can
//! verify the issuing domain before submitting tasks. This module is the
//! signing / verifying side; the route handler (`/.well-known/agent.json`)
//! lives alongside the existing A2A surface in [`crate::a2a_http`].
//!
//! Key choice: P-256 (`secp256r1`) ECDSA. This is the same curve used by
//! `watch_auth.rs` for device pairing (Apple Secure Enclave constraint —
//! see CLAUDE.md cross-cutting invariants). One curve across the daemon
//! keeps key management coherent.
//!
//! Wire shape — pragmatic JWS-detached pattern with the public key
//! inlined as a JWK so downstream verifiers don't need a separate JWKS
//! fetch:
//!
//! ```json
//! {
//!   "card":       { ...AgentCard... },
//!   "signatures": [{
//!     "kid":       "vibecli-default",
//!     "algorithm": "ES256",
//!     "publicKey": { "kty": "EC", "crv": "P-256", "x": "...", "y": "..." },
//!     "value":     "<base64url(ECDSA(SHA-256(canonical_json_of_card)))>"
//!   }]
//! }
//! ```
//!
//! Canonicalisation: keys are JSON-stringified in sorted order via
//! [`canonical_json`] so the signature is deterministic across
//! serializer revisions and platforms.

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use p256::ecdsa::{
    signature::{Signer, Verifier},
    Signature, SigningKey, VerifyingKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::a2a_protocol::AgentCard;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentCardSignature {
    pub kid: String,
    pub algorithm: String,
    #[serde(rename = "publicKey")]
    pub public_key: PublicKeyJwk,
    /// base64url(ECDSA(SHA-256(canonical_json(card))))
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PublicKeyJwk {
    pub kty: String,
    pub crv: String,
    pub x: String,
    pub y: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedAgentCard {
    pub card: AgentCard,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signatures: Vec<AgentCardSignature>,
}

/// Sign an `AgentCard` with the daemon's identity key.
///
/// Returns a `SignedAgentCard` with a single signature entry whose `kid`
/// is the supplied identifier (e.g. `vibecli-default`). Multiple
/// signatures over the same card are supported by calling this function
/// repeatedly with different keys and merging the result.
pub fn sign_agent_card(
    card: AgentCard,
    signing_key: &SigningKey,
    kid: &str,
) -> Result<SignedAgentCard> {
    let canonical = canonical_json(&card)?;
    let digest = Sha256::digest(canonical.as_bytes());
    let signature: Signature = signing_key.sign(&digest);
    let value = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes());
    let public_key = jwk_from_verifying_key(signing_key.verifying_key());
    Ok(SignedAgentCard {
        card,
        signatures: vec![AgentCardSignature {
            kid: kid.to_string(),
            algorithm: "ES256".to_string(),
            public_key,
            value,
        }],
    })
}

/// Verify every signature on a `SignedAgentCard` against its embedded
/// JWK. Returns `Ok(())` only if all signatures verify; the first
/// failure short-circuits with a descriptive error.
pub fn verify_signed_agent_card(signed: &SignedAgentCard) -> Result<()> {
    if signed.signatures.is_empty() {
        return Err(anyhow!("verify: no signatures present"));
    }
    let canonical = canonical_json(&signed.card)?;
    let digest = Sha256::digest(canonical.as_bytes());
    for sig in &signed.signatures {
        if sig.algorithm != "ES256" {
            return Err(anyhow!(
                "verify: unsupported algorithm '{}' (only ES256/P-256 is implemented)",
                sig.algorithm
            ));
        }
        let verifying_key = verifying_key_from_jwk(&sig.public_key)
            .with_context(|| format!("verify: bad JWK for kid '{}'", sig.kid))?;
        let sig_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(sig.value.as_bytes())
            .with_context(|| format!("verify: signature for kid '{}' is not base64url", sig.kid))?;
        let signature = Signature::from_slice(&sig_bytes).with_context(|| {
            format!(
                "verify: signature for kid '{}' is not 64-byte ECDSA",
                sig.kid
            )
        })?;
        verifying_key
            .verify(&digest, &signature)
            .map_err(|e| anyhow!("verify: signature mismatch for kid '{}': {e}", sig.kid))?;
    }
    Ok(())
}

/// Deterministic JSON serialization of an [`AgentCard`] with object keys
/// sorted lexicographically. Required for signature determinism.
pub fn canonical_json(card: &AgentCard) -> Result<String> {
    let v = serde_json::to_value(card).context("canonical_json: serialize AgentCard")?;
    let canonical = sort_value(v);
    serde_json::to_string(&canonical).context("canonical_json: emit sorted JSON")
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

/// Encode a P-256 verifying key as a JWK per RFC 7517 + RFC 7518.
pub fn jwk_from_verifying_key(vk: &VerifyingKey) -> PublicKeyJwk {
    let point = vk.to_encoded_point(false);
    let x = point.x().expect("P-256 point has x coordinate");
    let y = point.y().expect("P-256 point has y coordinate");
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    PublicKeyJwk {
        kty: "EC".to_string(),
        crv: "P-256".to_string(),
        x: engine.encode(x),
        y: engine.encode(y),
    }
}

/// Decode a JWK back into a P-256 verifying key.
pub fn verifying_key_from_jwk(jwk: &PublicKeyJwk) -> Result<VerifyingKey> {
    if jwk.kty != "EC" {
        return Err(anyhow!("jwk: kty must be 'EC', got '{}'", jwk.kty));
    }
    if jwk.crv != "P-256" {
        return Err(anyhow!("jwk: crv must be 'P-256', got '{}'", jwk.crv));
    }
    let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let x = engine
        .decode(jwk.x.as_bytes())
        .context("jwk: x is not base64url")?;
    let y = engine
        .decode(jwk.y.as_bytes())
        .context("jwk: y is not base64url")?;
    if x.len() != 32 || y.len() != 32 {
        return Err(anyhow!(
            "jwk: P-256 coordinates must be 32 bytes (got x={}, y={})",
            x.len(),
            y.len()
        ));
    }
    let mut sec1 = Vec::with_capacity(65);
    sec1.push(0x04);
    sec1.extend_from_slice(&x);
    sec1.extend_from_slice(&y);
    VerifyingKey::from_sec1_bytes(&sec1).context("jwk: not a valid P-256 point")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::a2a_protocol::AgentCard;
    use p256::ecdsa::SigningKey;

    fn fixture_card() -> AgentCard {
        AgentCard::new(
            "vibecli",
            "VibeCody coding agent",
            "https://localhost:7878/a2a",
            "0.5.7",
        )
    }

    fn fixture_key() -> SigningKey {
        // Match watch_auth.rs — `p256` re-exports the `rand_core` whose
        // `OsRng` actually implements the `CryptoRngCore` bound `ecdsa`
        // expects. Workspace `rand` is on a newer `rand_core` and
        // doesn't satisfy that bound directly.
        SigningKey::random(&mut p256::elliptic_curve::rand_core::OsRng)
    }

    #[test]
    fn sign_agent_card_emits_es256_signature_with_inline_jwk() {
        let key = fixture_key();
        let signed = sign_agent_card(fixture_card(), &key, "vibecli-default").unwrap();

        assert_eq!(signed.signatures.len(), 1);
        let sig = &signed.signatures[0];
        assert_eq!(sig.kid, "vibecli-default");
        assert_eq!(sig.algorithm, "ES256");
        assert_eq!(sig.public_key.kty, "EC");
        assert_eq!(sig.public_key.crv, "P-256");
        assert!(!sig.value.is_empty());
        // ECDSA P-256 signature is 64 bytes → base64url-no-pad = 86 chars.
        assert_eq!(
            sig.value.len(),
            86,
            "expected 86-char base64url for 64B ECDSA, got {}",
            sig.value.len()
        );
    }

    #[test]
    fn verify_accepts_freshly_signed_card() {
        let key = fixture_key();
        let signed = sign_agent_card(fixture_card(), &key, "vibecli-default").unwrap();
        verify_signed_agent_card(&signed).unwrap();
    }

    #[test]
    fn verify_rejects_tampered_card() {
        let key = fixture_key();
        let mut signed = sign_agent_card(fixture_card(), &key, "vibecli-default").unwrap();
        signed.card.version = "9.9.9".to_string();
        let res = verify_signed_agent_card(&signed);
        assert!(res.is_err(), "verification must fail on tampered card");
        assert!(res.unwrap_err().to_string().contains("signature mismatch"));
    }

    #[test]
    fn verify_rejects_unsupported_algorithm() {
        let key = fixture_key();
        let mut signed = sign_agent_card(fixture_card(), &key, "vibecli-default").unwrap();
        signed.signatures[0].algorithm = "RS256".to_string();
        let res = verify_signed_agent_card(&signed);
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("unsupported algorithm"));
    }

    #[test]
    fn canonical_json_sorts_object_keys() {
        let card = fixture_card();
        let json_str = canonical_json(&card).unwrap();
        let first_key = json_str.splitn(3, '"').nth(1).expect("first JSON key");
        // AgentCard fields include name, description, url, ...; the
        // alphabetically smallest is "authentication".
        assert_eq!(
            first_key, "authentication",
            "canonical_json must sort keys; got first={first_key}\nfull={json_str}"
        );
    }

    #[test]
    fn jwk_round_trips_through_verifying_key() {
        let key = fixture_key();
        let vk = key.verifying_key();
        let jwk = jwk_from_verifying_key(vk);
        let recovered = verifying_key_from_jwk(&jwk).unwrap();

        let original = vk.to_encoded_point(false);
        let recovered_pt = recovered.to_encoded_point(false);
        assert_eq!(original.as_bytes(), recovered_pt.as_bytes());
    }

    // ── Scenario 7: end-to-end route hit returns a verifiable signed card ──

    #[tokio::test]
    async fn well_known_agent_json_route_returns_verifiable_signed_card() {
        use axum::{
            body::to_bytes, body::Body, extract::State, http::Request, routing::get, Json, Router,
        };
        use std::sync::Arc;
        use tower::ServiceExt;

        // Daemon-side state — one signing key + a fixture card. Wrapping
        // SigningKey in Arc lets the handler share it across requests.
        #[derive(Clone)]
        struct AppState {
            card: AgentCard,
            signing_key: Arc<SigningKey>,
        }

        async fn handler(State(s): State<AppState>) -> Json<SignedAgentCard> {
            let signed = sign_agent_card(s.card.clone(), &s.signing_key, "vibecli-default")
                .expect("sign should succeed");
            Json(signed)
        }

        let key = fixture_key();
        let state = AppState {
            card: fixture_card(),
            signing_key: Arc::new(key),
        };
        let app = Router::new()
            .route("/.well-known/agent.json", get(handler))
            .with_state(state);

        let req = Request::builder()
            .method("GET")
            .uri("/.well-known/agent.json")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);

        let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
        let parsed: SignedAgentCard = serde_json::from_slice(&bytes).unwrap();

        // Verifier on the consuming side recomputes canonical_json,
        // SHA-256, and ECDSA-verifies against the inlined JWK.
        verify_signed_agent_card(&parsed).expect("signature must verify");
        assert_eq!(parsed.card.name, "vibecli");
        assert_eq!(parsed.signatures.len(), 1);
        assert_eq!(parsed.signatures[0].kid, "vibecli-default");
    }
}
