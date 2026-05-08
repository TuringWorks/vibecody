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
//! Red commit: types + signatures + 6 BDD scenarios. Impl bodies
//! `todo!()` so tests panic at runtime — TDD red. Green commit fills in
//! the bodies.

use anyhow::Result;
use p256::ecdsa::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

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

pub fn sign_agent_card(
    _card: AgentCard,
    _signing_key: &SigningKey,
    _kid: &str,
) -> Result<SignedAgentCard> {
    todo!("B6: sign canonical_json with P-256 ECDSA, embed JWK and base64url signature");
}

pub fn verify_signed_agent_card(_signed: &SignedAgentCard) -> Result<()> {
    todo!("B6: recompute SHA-256 of canonical_json, ECDSA-verify each signature against its embedded JWK");
}

pub fn canonical_json(_card: &AgentCard) -> Result<String> {
    todo!("B6: deterministic JSON with object keys sorted lexicographically");
}

pub fn jwk_from_verifying_key(_vk: &VerifyingKey) -> PublicKeyJwk {
    todo!("B6: SEC1 uncompressed point → base64url(x), base64url(y), kty=EC, crv=P-256");
}

pub fn verifying_key_from_jwk(_jwk: &PublicKeyJwk) -> Result<VerifyingKey> {
    todo!("B6: assemble SEC1 uncompressed point from x/y, decode base64url, return VerifyingKey");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::a2a_protocol::AgentCard;
    use p256::ecdsa::SigningKey;
    use p256::elliptic_curve::sec1::ToEncodedPoint;
    use rand::rngs::OsRng;

    fn fixture_card() -> AgentCard {
        AgentCard::new(
            "vibecli",
            "VibeCody coding agent",
            "https://localhost:7878/a2a",
            "0.5.7",
        )
    }

    fn fixture_key() -> SigningKey {
        SigningKey::random(&mut OsRng)
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
        assert_eq!(sig.value.len(), 86, "expected 86-char base64url for 64B ECDSA, got {}", sig.value.len());
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
        assert!(res.unwrap_err().to_string().contains("unsupported algorithm"));
    }

    #[test]
    fn canonical_json_sorts_object_keys() {
        let card = fixture_card();
        let json_str = canonical_json(&card).unwrap();
        let first_key = json_str
            .splitn(3, '"')
            .nth(1)
            .expect("first JSON key");
        // AgentCard fields include name, description, url, ...; the
        // alphabetically smallest is "authentication".
        assert_eq!(first_key, "authentication", "canonical_json must sort keys; got first={first_key}\nfull={json_str}");
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
}
