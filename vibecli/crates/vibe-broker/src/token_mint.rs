//! OAuth2 token-mint flows for cloud providers.
//!
//! - GCP: build a JWT claim, sign with the service-account RSA key,
//!   exchange at `https://oauth2.googleapis.com/token` for an access
//!   token. Used when the operator configures a service-account JSON
//!   key file rather than pre-minting tokens externally.
//! - Azure: client_credentials grant — POST `client_id` + `client_secret`
//!   to `https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token`
//!   with scope, get back an access token.
//!
//! Both flows return a `MintedToken` that carries the bearer string and
//! the absolute expiration time. `CachedMinter` wraps any minter with a
//! refresh-aware cache so the broker doesn't mint per-request.

use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct MintedToken {
    pub access_token: String,
    pub expires_at: SystemTime,
}

impl MintedToken {
    pub fn from_expires_in(token: impl Into<String>, expires_in: u64) -> Self {
        let now = SystemTime::now();
        MintedToken {
            access_token: token.into(),
            expires_at: now + Duration::from_secs(expires_in),
        }
    }

    pub fn seconds_remaining(&self) -> u64 {
        self.expires_at
            .duration_since(SystemTime::now())
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MintError {
    #[error("http: {0}")]
    Http(String),
    #[error("response parse: {0}")]
    Parse(String),
    #[error("crypto: {0}")]
    Crypto(String),
    #[error("config: {0}")]
    Config(String),
    #[error("upstream returned status {status}: {body}")]
    Upstream { status: u16, body: String },
}

#[async_trait]
pub trait TokenMinter: Send + Sync {
    async fn mint(&self) -> Result<MintedToken, MintError>;
}

// ---- Azure client_credentials minter ----------------------------------

#[derive(Debug, Clone)]
pub struct AzureClientCredentialsMinter {
    pub endpoint: String,
    pub tenant: String,
    pub client_id: String,
    pub client_secret: String,
    pub scope: String,
}

impl AzureClientCredentialsMinter {
    pub fn new(
        tenant: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        scope: impl Into<String>,
    ) -> Self {
        AzureClientCredentialsMinter {
            endpoint: "https://login.microsoftonline.com".into(),
            tenant: tenant.into(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            scope: scope.into(),
        }
    }

    /// Override the endpoint (used by tests pointing at a stub server).
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }
}

#[derive(Deserialize)]
struct AzureTokenResp {
    access_token: String,
    expires_in: u64,
}

#[async_trait]
impl TokenMinter for AzureClientCredentialsMinter {
    async fn mint(&self) -> Result<MintedToken, MintError> {
        let url = format!(
            "{}/{}/oauth2/v2.0/token",
            self.endpoint.trim_end_matches('/'),
            self.tenant
        );
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("scope", self.scope.as_str()),
        ];
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| MintError::Http(e.to_string()))?;
        let resp = client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| MintError::Http(e.to_string()))?;
        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_else(|_| "<no body>".into());
            return Err(MintError::Upstream { status, body });
        }
        let parsed: AzureTokenResp = resp
            .json()
            .await
            .map_err(|e| MintError::Parse(e.to_string()))?;
        Ok(MintedToken::from_expires_in(
            parsed.access_token,
            parsed.expires_in,
        ))
    }
}

// ---- GCP service-account minter ---------------------------------------

#[derive(Debug, Clone)]
pub struct GcpServiceAccountMinter {
    pub endpoint: String,
    pub client_email: String,
    pub private_key_pem: String,
    pub scope: String,
    pub audience: String,
}

impl GcpServiceAccountMinter {
    pub fn new(
        client_email: impl Into<String>,
        private_key_pem: impl Into<String>,
        scope: impl Into<String>,
    ) -> Self {
        GcpServiceAccountMinter {
            endpoint: "https://oauth2.googleapis.com".into(),
            client_email: client_email.into(),
            private_key_pem: private_key_pem.into(),
            scope: scope.into(),
            audience: "https://oauth2.googleapis.com/token".into(),
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = endpoint.into();
        self
    }

    fn build_signed_jwt(&self) -> Result<String, MintError> {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
        use base64::Engine as _;
        use ring::rand::SystemRandom;
        use ring::signature::RsaKeyPair;

        #[derive(Serialize)]
        struct Header<'a> {
            alg: &'a str,
            typ: &'a str,
        }
        #[derive(Serialize)]
        struct Claims<'a> {
            iss: &'a str,
            scope: &'a str,
            aud: &'a str,
            iat: u64,
            exp: u64,
        }

        let header = Header {
            alg: "RS256",
            typ: "JWT",
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| MintError::Crypto(e.to_string()))?
            .as_secs();
        let claims = Claims {
            iss: &self.client_email,
            scope: &self.scope,
            aud: &self.audience,
            iat: now,
            exp: now + 3600,
        };

        let header_b64 =
            B64.encode(serde_json::to_vec(&header).map_err(|e| MintError::Crypto(e.to_string()))?);
        let claims_b64 =
            B64.encode(serde_json::to_vec(&claims).map_err(|e| MintError::Crypto(e.to_string()))?);
        let signing_input = format!("{header_b64}.{claims_b64}");

        // Signed with `ring`, not the `rsa` crate: RUSTSEC-2023-0071 (Marvin)
        // is a timing side-channel in `rsa`'s private-key path, and it has no
        // patched release — the advisory is open with `patched: []`. Exploiting
        // it needs a timing oracle over many operations, which a local
        // once-an-hour JWT signature does not hand out, but "hard to reach" is
        // a weaker property than "not present". `ring` blinds the operation and
        // was already compiled in via rustls and rcgen, so this removes the
        // advisory without adding a dependency.
        let der = pkcs8_pem_to_der(&self.private_key_pem)?;
        let key_pair = RsaKeyPair::from_pkcs8(&der)
            .map_err(|e| MintError::Crypto(format!("private key parse: {e}")))?;

        let mut signature = vec![0u8; key_pair.public().modulus_len()];
        key_pair
            .sign(
                &ring::signature::RSA_PKCS1_SHA256,
                &SystemRandom::new(),
                signing_input.as_bytes(),
                &mut signature,
            )
            .map_err(|_| MintError::Crypto("RS256 signing failed".into()))?;
        let sig_b64 = B64.encode(&signature);

        Ok(format!("{signing_input}.{sig_b64}"))
    }
}

#[derive(Deserialize)]
struct GcpTokenResp {
    access_token: String,
    expires_in: u64,
}

#[async_trait]
impl TokenMinter for GcpServiceAccountMinter {
    async fn mint(&self) -> Result<MintedToken, MintError> {
        let jwt = self.build_signed_jwt()?;
        let url = format!("{}/token", self.endpoint.trim_end_matches('/'));
        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", jwt.as_str()),
        ];
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| MintError::Http(e.to_string()))?;
        let resp = client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| MintError::Http(e.to_string()))?;
        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_else(|_| "<no body>".into());
            return Err(MintError::Upstream { status, body });
        }
        let parsed: GcpTokenResp = resp
            .json()
            .await
            .map_err(|e| MintError::Parse(e.to_string()))?;
        Ok(MintedToken::from_expires_in(
            parsed.access_token,
            parsed.expires_in,
        ))
    }
}

// ---- Refresh-aware cache wrapper --------------------------------------

#[derive(Debug, Default)]
struct CacheState {
    token: Option<MintedToken>,
    inner_calls: u64,
}

pub struct CachedMinter<M: TokenMinter> {
    inner: M,
    refresh_buffer: Duration,
    state: Mutex<CacheState>,
}

impl<M: TokenMinter> CachedMinter<M> {
    pub fn new(inner: M, refresh_buffer: Duration) -> Self {
        CachedMinter {
            inner,
            refresh_buffer,
            state: Mutex::new(CacheState::default()),
        }
    }

    /// Number of times the underlying minter was actually called. Used in
    /// tests to assert caching behaviour.
    pub fn underlying_call_count(&self) -> u64 {
        self.state.lock().unwrap().inner_calls
    }

    fn cached_if_fresh(&self) -> Option<MintedToken> {
        let s = self.state.lock().unwrap();
        match &s.token {
            Some(t) => {
                let remaining = t
                    .expires_at
                    .duration_since(SystemTime::now())
                    .unwrap_or(Duration::ZERO);
                if remaining > self.refresh_buffer {
                    Some(t.clone())
                } else {
                    None
                }
            }
            None => None,
        }
    }
}

#[async_trait]
impl<M: TokenMinter> TokenMinter for CachedMinter<M> {
    async fn mint(&self) -> Result<MintedToken, MintError> {
        if let Some(t) = self.cached_if_fresh() {
            return Ok(t);
        }
        let fresh = self.inner.mint().await?;
        let mut s = self.state.lock().unwrap();
        s.inner_calls += 1;
        s.token = Some(fresh.clone());
        Ok(fresh)
    }
}

/// Decode a PKCS#8 **PEM** private key into the DER bytes crypto backends want.
///
/// GCP service-account JSON carries `private_key` as PEM; `ring` takes DER.
/// Hand-rolled rather than pulling a PEM crate: this is base64 between two
/// fixed markers, and the alternative (`rustls-pemfile`) is itself flagged
/// unmaintained by RUSTSEC-2025-0134.
fn pkcs8_pem_to_der(pem: &str) -> Result<Vec<u8>, MintError> {
    use base64::engine::general_purpose::STANDARD as B64_STD;
    use base64::Engine as _;

    const BEGIN: &str = "-----BEGIN PRIVATE KEY-----";
    const END: &str = "-----END PRIVATE KEY-----";

    let start = pem
        .find(BEGIN)
        .ok_or_else(|| MintError::Crypto("private key is not PKCS#8 PEM (no BEGIN marker)".into()))?
        + BEGIN.len();
    let end = pem[start..]
        .find(END)
        .ok_or_else(|| MintError::Crypto("private key is not PKCS#8 PEM (no END marker)".into()))?
        + start;

    // Service-account JSON stores the key with literal "\n" escapes decoded to
    // real newlines; either way the body is base64 split across lines.
    let body: String = pem[start..end].chars().filter(|c| !c.is_whitespace()).collect();
    B64_STD
        .decode(body.as_bytes())
        .map_err(|e| MintError::Crypto(format!("private key base64: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── RS256 JWT signing ──────────────────────────────────────────────────
    //
    // `build_signed_jwt` had no coverage at all, which is why these exist:
    // they pin the wire format and the signature's validity so the crypto
    // backend underneath can be swapped without changing what GCP receives.
    // Verification goes through `ring` against the key's own public half, so
    // the assertion is "a real RS256 verifier accepts this", not "the code
    // did what it did last time".

    /// 2048-bit RSA key generated solely for these tests. Not a credential —
    /// it signs nothing outside this file. gitleaks:allow
    const TEST_KEY_PEM: &str = include_str!("../tests/fixtures/gcp_sa_test_key.pem");

    fn test_minter() -> GcpServiceAccountMinter {
        GcpServiceAccountMinter::new(
            "svc@project.iam.gserviceaccount.com",
            TEST_KEY_PEM,
            "https://www.googleapis.com/auth/cloud-platform",
        )
    }

    fn b64url(part: &str) -> Vec<u8> {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
        use base64::Engine as _;
        B64.decode(part).expect("JWT part is base64url")
    }

    #[test]
    fn signed_jwt_has_three_base64url_parts_and_an_rs256_header() {
        let jwt = test_minter().build_signed_jwt().expect("sign");
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT is header.claims.signature");

        let header: serde_json::Value =
            serde_json::from_slice(&b64url(parts[0])).expect("header is JSON");
        assert_eq!(header["alg"], "RS256");
        assert_eq!(header["typ"], "JWT");
        // Base64url, not standard base64: '+' and '/' would be rejected by GCP.
        assert!(!jwt.contains('+') && !jwt.contains('/') && !jwt.contains('='));
    }

    #[test]
    fn signed_jwt_claims_carry_the_service_account_and_a_one_hour_window() {
        let jwt = test_minter().build_signed_jwt().expect("sign");
        let parts: Vec<&str> = jwt.split('.').collect();
        let claims: serde_json::Value =
            serde_json::from_slice(&b64url(parts[1])).expect("claims are JSON");

        assert_eq!(claims["iss"], "svc@project.iam.gserviceaccount.com");
        assert_eq!(claims["aud"], "https://oauth2.googleapis.com/token");
        assert_eq!(claims["scope"], "https://www.googleapis.com/auth/cloud-platform");
        let iat = claims["iat"].as_u64().expect("iat");
        let exp = claims["exp"].as_u64().expect("exp");
        assert_eq!(exp - iat, 3600, "GCP rejects assertions older than an hour");
    }

    #[test]
    fn signed_jwt_signature_verifies_against_the_key() {
        // The whole point of the token: if this fails, GCP returns
        // invalid_grant and every cloud credential injection stops working.
        let jwt = test_minter().build_signed_jwt().expect("sign");
        let (signing_input, sig_b64) = jwt.rsplit_once('.').expect("signature is last");
        let sig = b64url(sig_b64);

        use ring::signature::KeyPair as _;
        let der = pkcs8_pem_to_der(TEST_KEY_PEM).expect("fixture is PKCS#8 PEM");
        let key_pair = ring::signature::RsaKeyPair::from_pkcs8(&der).expect("fixture parses");
        let public = ring::signature::UnparsedPublicKey::new(
            &ring::signature::RSA_PKCS1_2048_8192_SHA256,
            key_pair.public_key().as_ref(),
        );
        public
            .verify(signing_input.as_bytes(), &sig)
            .expect("RS256 signature must verify");
    }

    #[test]
    fn signed_jwt_signature_does_not_cover_a_tampered_payload() {
        // Guards against a signature computed over the wrong bytes — which
        // would still "verify" in a test that signed and checked the same
        // mistake.
        let jwt = test_minter().build_signed_jwt().expect("sign");
        let (signing_input, sig_b64) = jwt.rsplit_once('.').expect("signature is last");
        let sig = b64url(sig_b64);

        use ring::signature::KeyPair as _;
        let der = pkcs8_pem_to_der(TEST_KEY_PEM).unwrap();
        let key_pair = ring::signature::RsaKeyPair::from_pkcs8(&der).unwrap();
        let public = ring::signature::UnparsedPublicKey::new(
            &ring::signature::RSA_PKCS1_2048_8192_SHA256,
            key_pair.public_key().as_ref(),
        );
        let tampered = format!("{signing_input}x");
        assert!(public.verify(tampered.as_bytes(), &sig).is_err());
    }

    #[test]
    fn a_non_pkcs8_private_key_is_rejected_with_a_readable_error() {
        let minter = GcpServiceAccountMinter::new("svc@x.iam", "not a key at all", "scope");
        let err = minter.build_signed_jwt().expect_err("must not sign");
        assert!(
            format!("{err}").contains("PKCS#8") || format!("{err}").contains("private key"),
            "error should name the problem, got: {err}"
        );
    }

    #[test]
    fn minted_token_seconds_remaining_is_finite() {
        let t = MintedToken::from_expires_in("x", 3600);
        let r = t.seconds_remaining();
        assert!(r > 3500 && r <= 3600);
    }

    #[test]
    fn minted_token_zero_after_expiry() {
        let t = MintedToken {
            access_token: "x".into(),
            expires_at: SystemTime::now() - Duration::from_secs(1),
        };
        assert_eq!(t.seconds_remaining(), 0);
    }
}
