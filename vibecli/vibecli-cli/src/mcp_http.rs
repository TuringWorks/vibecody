//! Real MCP Streamable HTTP + OAuth 2.1 with PKCE (US-004).
//!
//! [`mcp_streamable`] holds the in-memory data model (connection records,
//! token history, metrics, hand-rolled SHA-256). This module provides the
//! real HTTP side of the feature:
//!
//! - [`PkceChallengeV2`] — PKCE S256 challenges generated with `sha2` + CSPRNG
//! - [`build_auth_url`] — RFC-8252 authorization URL with PKCE + scopes
//! - [`McpOAuthClient::exchange_code`] — real POST to token endpoint
//! - [`McpOAuthClient::refresh_token`] — real POST with `grant_type=refresh_token`
//! - [`McpStreamClient::open_stream`] — opens a Bearer-authenticated SSE stream
//!
//! The existing `mcp_streamable` module is kept for backward compatibility with
//! the VibeUI panel and REPL command; real I/O goes through this module.
//!
//! ## Error handling
//!
//! - [`MpError::Http`] wraps reqwest errors
//! - [`MpError::Server`] captures non-2xx responses with status + body
//! - [`MpError::Parse`] covers JSON deserialization failures
//! - [`MpError::Unauthorized`] is distinct so callers can re-trigger the auth flow

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64_URL_NO_PAD;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ── Error type ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum McpError {
    Http(String),
    Server { status: u16, body: String },
    Parse(String),
    Unauthorized(String),
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(s) => write!(f, "http error: {s}"),
            Self::Server { status, body } => write!(f, "server {status}: {body}"),
            Self::Parse(s) => write!(f, "parse error: {s}"),
            Self::Unauthorized(s) => write!(f, "unauthorized: {s}"),
        }
    }
}

impl std::error::Error for McpError {}

// ── PKCE ────────────────────────────────────────────────────────────────────

/// A PKCE S256 challenge backed by `sha2::Sha256` and `rand::rngs::OsRng`.
///
/// Per RFC 7636 the verifier is 43–128 base64url characters. We generate
/// 32 random bytes → 43 base64url characters (the common case used by
/// clawcode, GitHub, Anthropic, and most OAuth 2.1 IdPs).
#[derive(Debug, Clone)]
pub struct PkceChallengeV2 {
    pub code_verifier: String,
    pub code_challenge: String,
}

impl PkceChallengeV2 {
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        let verifier = B64_URL_NO_PAD.encode(bytes);
        let challenge = Self::s256(&verifier);
        Self {
            code_verifier: verifier,
            code_challenge: challenge,
        }
    }

    /// `BASE64URL(SHA256(verifier))` — the S256 challenge per RFC 7636 §4.2.
    pub fn s256(verifier: &str) -> String {
        let hash = Sha256::digest(verifier.as_bytes());
        B64_URL_NO_PAD.encode(hash)
    }
}

// ── Authorization URL ───────────────────────────────────────────────────────

/// OAuth 2.1 authorization endpoint parameters.
#[derive(Debug, Clone)]
pub struct AuthUrlParams<'a> {
    pub auth_url: &'a str,
    pub client_id: &'a str,
    pub redirect_uri: &'a str,
    pub scopes: &'a [&'a str],
    pub state: &'a str,
    pub pkce: &'a PkceChallengeV2,
}

/// Build an authorization URL with PKCE S256 + scopes. Uses `urlencoding`
/// for all user-supplied values so scopes with `+` / redirect URIs with `:`
/// and `/` survive the trip.
pub fn build_auth_url(p: &AuthUrlParams<'_>) -> String {
    let mut url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&state={}&code_challenge={}&code_challenge_method=S256",
        p.auth_url,
        urlencoding::encode(p.client_id),
        urlencoding::encode(p.redirect_uri),
        urlencoding::encode(p.state),
        urlencoding::encode(&p.pkce.code_challenge),
    );
    if !p.scopes.is_empty() {
        let joined: Vec<String> = p
            .scopes
            .iter()
            .map(|s| urlencoding::encode(s).into_owned())
            .collect();
        // OAuth 2.1 spec: scopes are space-separated, but form encoding
        // turns spaces into '+'.
        url.push_str(&format!("&scope={}", joined.join("+")));
    }
    url
}

// ── Token payload ───────────────────────────────────────────────────────────

/// Standard OAuth 2.0 token response shape (RFC 6749 §5.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    #[serde(default)]
    pub expires_in: Option<i64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

// ── OAuth client ────────────────────────────────────────────────────────────

/// HTTP OAuth 2.1 client for token exchange + refresh.
pub struct McpOAuthClient {
    http: reqwest::Client,
}

impl McpOAuthClient {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    /// POST `grant_type=authorization_code` with code, verifier, redirect_uri,
    /// client_id to the token endpoint. Returns the parsed token response.
    pub async fn exchange_code(
        &self,
        token_url: &str,
        code: &str,
        code_verifier: &str,
        client_id: &str,
        redirect_uri: &str,
    ) -> Result<OAuthTokenResponse, McpError> {
        let form = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("code_verifier", code_verifier),
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
        ];
        let resp = self
            .http
            .post(token_url)
            .form(&form)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(McpError::Unauthorized(body));
        }
        if !status.is_success() {
            return Err(McpError::Server {
                status: status.as_u16(),
                body,
            });
        }
        serde_json::from_str(&body).map_err(|e| McpError::Parse(e.to_string()))
    }

    /// POST `grant_type=refresh_token` to obtain a new access token.
    pub async fn refresh_token(
        &self,
        token_url: &str,
        refresh_token: &str,
        client_id: &str,
    ) -> Result<OAuthTokenResponse, McpError> {
        let form = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ];
        let resp = self
            .http
            .post(token_url)
            .form(&form)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        if status == reqwest::StatusCode::UNAUTHORIZED {
            return Err(McpError::Unauthorized(body));
        }
        if !status.is_success() {
            return Err(McpError::Server {
                status: status.as_u16(),
                body,
            });
        }
        serde_json::from_str(&body).map_err(|e| McpError::Parse(e.to_string()))
    }
}

// ── MCP stream client (Bearer + SSE) ────────────────────────────────────────

/// A single SSE message parsed off the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseMessage {
    pub event: String,
    pub data: String,
}

/// Opens a Bearer-authenticated SSE stream against an MCP server and reads
/// up to `max_messages` SSE frames before closing.
pub struct McpStreamClient {
    http: reqwest::Client,
}

impl McpStreamClient {
    pub fn new(http: reqwest::Client) -> Self {
        Self { http }
    }

    pub async fn open_stream(
        &self,
        stream_url: &str,
        bearer: &str,
        max_messages: usize,
    ) -> Result<Vec<SseMessage>, McpError> {
        use futures::StreamExt;
        use std::time::Duration;

        let resp = self
            .http
            .get(stream_url)
            .bearer_auth(bearer)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED
            || status == reqwest::StatusCode::FORBIDDEN
        {
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::Unauthorized(body));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(McpError::Server {
                status: status.as_u16(),
                body,
            });
        }

        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut msgs: Vec<SseMessage> = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, stream.next()).await {
                Err(_) => break,
                Ok(None) => break,
                Ok(Some(Err(e))) => return Err(McpError::Http(e.to_string())),
                Ok(Some(Ok(chunk))) => {
                    buf.push_str(&String::from_utf8_lossy(&chunk));
                    while let Some(end) = buf.find("\n\n") {
                        let frame = buf[..end].to_string();
                        buf.drain(..end + 2);
                        let mut event = String::new();
                        let mut data = String::new();
                        for line in frame.lines() {
                            if let Some(rest) = line.strip_prefix("event:") {
                                event = rest.trim().to_string();
                            } else if let Some(rest) = line.strip_prefix("data:") {
                                if !data.is_empty() {
                                    data.push('\n');
                                }
                                data.push_str(rest.trim_start());
                            }
                        }
                        if !event.is_empty() || !data.is_empty() {
                            msgs.push(SseMessage { event, data });
                            if msgs.len() >= max_messages {
                                return Ok(msgs);
                            }
                        }
                    }
                }
            }
        }
        Ok(msgs)
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_generate_is_s256_of_verifier() {
        let p = PkceChallengeV2::generate();
        assert!(
            p.code_verifier.len() >= 43,
            "verifier len {}",
            p.code_verifier.len()
        );
        let recomputed = PkceChallengeV2::s256(&p.code_verifier);
        assert_eq!(p.code_challenge, recomputed);
        assert!(!p.code_challenge.contains('='));
        assert!(!p.code_challenge.contains('+'));
        assert!(!p.code_challenge.contains('/'));
    }

    #[test]
    fn auth_url_url_encodes_redirect_and_state() {
        let pkce = PkceChallengeV2 {
            code_verifier: "v".into(),
            code_challenge: "chal".into(),
        };
        let scopes = &["read", "write"];
        let url = build_auth_url(&AuthUrlParams {
            auth_url: "https://example.test/auth",
            client_id: "app-1",
            redirect_uri: "https://x/cb",
            scopes,
            state: "s-42",
            pkce: &pkce,
        });
        assert!(url.contains("client_id=app-1"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fx%2Fcb"));
        assert!(url.contains("state=s-42"));
        assert!(url.contains("code_challenge=chal"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("scope=read+write"));
    }

    #[test]
    fn pkce_two_generations_have_distinct_verifiers() {
        let a = PkceChallengeV2::generate();
        let b = PkceChallengeV2::generate();
        assert_ne!(a.code_verifier, b.code_verifier);
    }
}
