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
            let body = resp
                .text()
                .await
                .unwrap_or_else(|_| "<no body>".into());
            return Err(MintError::Upstream { status, body });
        }
        let parsed: AzureTokenResp = resp
            .json()
            .await
            .map_err(|e| MintError::Parse(e.to_string()))?;
        Ok(MintedToken::from_expires_in(parsed.access_token, parsed.expires_in))
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
        use base64::Engine as _;
        use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
        use rsa::pkcs8::DecodePrivateKey;
        use rsa::{RsaPrivateKey, pkcs1v15::SigningKey, signature::SignatureEncoding};
        use sha2::Sha256;

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

        let header_b64 = B64.encode(
            serde_json::to_vec(&header)
                .map_err(|e| MintError::Crypto(e.to_string()))?,
        );
        let claims_b64 = B64.encode(
            serde_json::to_vec(&claims)
                .map_err(|e| MintError::Crypto(e.to_string()))?,
        );
        let signing_input = format!("{header_b64}.{claims_b64}");

        let pk = RsaPrivateKey::from_pkcs8_pem(&self.private_key_pem)
            .map_err(|e| MintError::Crypto(format!("private key parse: {e}")))?;
        let signing_key = SigningKey::<Sha256>::new(pk);
        use rsa::signature::RandomizedSigner;
        let mut rng = rsa::rand_core::OsRng;
        let signature = signing_key.sign_with_rng(&mut rng, signing_input.as_bytes());
        let sig_b64 = B64.encode(signature.to_bytes());

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
        Ok(MintedToken::from_expires_in(parsed.access_token, parsed.expires_in))
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

#[cfg(test)]
mod tests {
    use super::*;

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
