//! Slice B1.8 + B2.1: TLS termination + per-request inspection inside
//! the CONNECT tunnel.
//!
//! After `accept::dispatch_connect` writes `200 Connection Established`,
//! `run_mitm` is the next step:
//!   1. Mint (or fetch) a leaf cert for the requested SNI.
//!   2. Perform a rustls server handshake on the client-facing socket.
//!   3. Open TCP to the upstream `(host, port)` and TLS-handshake to it.
//!   4. Read each decrypted client request, run policy + SSRF + optional
//!      credential injection, then forward to upstream and stream the
//!      response back.
//!
//! v1 supports one request per CONNECT (read until `\r\n\r\n`, no body
//! re-emission, response streamed via `tokio::io::copy`). Keep-alive +
//! request bodies are a follow-up slice.

use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, RootCertStore, ServerConfig};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::policy::{Decision, Inject, Policy, Request as PolicyRequest};
use crate::secrets::SecretStore;
use crate::ssrf::{SsrfGuard, SsrfVerdict};
use crate::tls::{BrokerCa, TlsError};

#[derive(Debug, thiserror::Error)]
pub enum MitmError {
    #[error("tls config: {0}")]
    Config(String),
    #[error("tls handshake to client failed: {0}")]
    ServerHandshake(String),
    #[error("upstream connect failed: {0}")]
    UpstreamConnect(std::io::Error),
    #[error("tls handshake to upstream failed: {0}")]
    UpstreamHandshake(String),
    #[error("forwarding: {0}")]
    Forwarding(std::io::Error),
    #[error("tls module: {0}")]
    Tls(#[from] TlsError),
    #[error("malformed request from client")]
    MalformedRequest,
}

/// Bundle of the inspection-pipeline pieces the MITM loop needs.
pub struct InspectContext<'a> {
    pub policy: &'a Policy,
    pub ssrf: &'a SsrfGuard,
    pub secrets: &'a dyn SecretStore,
}

/// Run MITM after CONNECT has already received `200 Connection Established`.
///
/// `client` is the raw TCP stream the broker accepted. `host`/`port` are
/// the SNI + dest from the CONNECT line. `upstream_trust` is the rustls
/// RootCertStore used to verify the real upstream — defaults to
/// webpki-roots in production; tests inject self-signed roots.
#[allow(clippy::too_many_arguments)]
pub async fn run_mitm<S>(
    client: S,
    host: &str,
    port: u16,
    ca: &Arc<BrokerCa>,
    upstream_trust: Arc<RootCertStore>,
    timeout: Duration,
    inspect: InspectContext<'_>,
) -> Result<(), MitmError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // ---- 1. Server-side handshake (broker pretends to be `host`) ----
    let (chain, key) = ca.leaf_for_rustls(host)?;
    let server_cfg = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .map_err(|e| MitmError::Config(e.to_string()))?;
    let acceptor = TlsAcceptor::from(Arc::new(server_cfg));
    let mut client_tls = match tokio::time::timeout(timeout, acceptor.accept(client)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(MitmError::ServerHandshake(e.to_string())),
        Err(_) => return Err(MitmError::ServerHandshake("timeout".into())),
    };

    // ---- 2. Read one decrypted request from the client ------------
    let mut head_buf = vec![0u8; 16 * 1024];
    let n = read_until_headers(&mut client_tls, &mut head_buf).await
        .map_err(MitmError::Forwarding)?;
    let head = &head_buf[..n];
    let parsed = parse_head(head).ok_or(MitmError::MalformedRequest)?;

    // ---- 3. Policy + SSRF on the real (decrypted) URL --------------
    let absolute_url = format!(
        "https://{host}:{port}{path}",
        path = parsed.target_path()
    );
    if matches!(inspect.ssrf.check(&absolute_url), SsrfVerdict::Block) {
        let _ = write_simple_response(&mut client_tls, 451, "Unavailable", "ssrf_blocked").await;
        return Ok(());
    }
    let decision = inspect.policy.match_request(&PolicyRequest {
        method: &parsed.method,
        url: &absolute_url,
    });
    let inject = match decision {
        Decision::Deny => {
            let _ = write_simple_response(&mut client_tls, 451, "Unavailable", "policy_denied").await;
            return Ok(());
        }
        Decision::Allow { inject, .. } => inject.clone(),
    };

    // ---- 4. Open TCP + TLS-connect to the real upstream -----------
    let target = format!("{host}:{port}");
    let upstream_tcp = match tokio::time::timeout(timeout, TcpStream::connect(&target)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(MitmError::UpstreamConnect(e)),
        Err(_) => {
            return Err(MitmError::UpstreamConnect(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "upstream connect timeout",
            )))
        }
    };
    let client_cfg = ClientConfig::builder()
        .with_root_certificates(Arc::unwrap_or_clone(upstream_trust))
        .with_no_client_auth();
    let connector = TlsConnector::from(Arc::new(client_cfg));
    let server_name = ServerName::try_from(host.to_owned())
        .map_err(|e| MitmError::Config(format!("bad SNI {host}: {e}")))?;
    let mut upstream_tls = match tokio::time::timeout(
        timeout,
        connector.connect(server_name, upstream_tcp),
    )
    .await
    {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(MitmError::UpstreamHandshake(e.to_string())),
        Err(_) => return Err(MitmError::UpstreamHandshake("timeout".into())),
    };

    // ---- 5. Replay the request to upstream with optional injection -
    let req_bytes = build_outbound_request(&parsed, &inject, inspect.secrets, host, port);
    upstream_tls
        .write_all(&req_bytes)
        .await
        .map_err(MitmError::Forwarding)?;

    // ---- 6. Stream upstream response back to client ----------------
    let _ = tokio::io::copy(&mut upstream_tls, &mut client_tls).await;
    let _ = client_tls.shutdown().await;

    Ok(())
}

/// Default upstream-trust store backed by Mozilla's bundled root CAs via
/// the `webpki-roots` crate. Daemon callers will normally use this; tests
/// inject their own minimal store via `Broker::with_upstream_trust`.
pub fn default_upstream_roots() -> RootCertStore {
    let mut store = RootCertStore::empty();
    store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    store
}

// ---- Request parsing helpers ---------------------------------------

#[derive(Debug)]
struct ParsedReq {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
}

impl ParsedReq {
    fn target_path(&self) -> &str {
        &self.target
    }
}

fn parse_head(buf: &[u8]) -> Option<ParsedReq> {
    let text = std::str::from_utf8(buf).ok()?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();
    let version = parts.next()?;
    if !version.starts_with("HTTP/") {
        return None;
    }
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_owned(), v.trim().to_owned()));
        }
    }
    Some(ParsedReq {
        method,
        target,
        headers,
    })
}

async fn read_until_headers<R>(stream: &mut R, buf: &mut [u8]) -> std::io::Result<usize>
where
    R: AsyncRead + Unpin,
{
    let mut total = 0;
    loop {
        if total == buf.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "header section too large",
            ));
        }
        let n = stream.read(&mut buf[total..]).await?;
        if n == 0 {
            return Ok(total);
        }
        total += n;
        if buf[..total].windows(4).any(|w| w == b"\r\n\r\n") {
            return Ok(total);
        }
    }
}

fn build_outbound_request(
    parsed: &ParsedReq,
    inject: &Inject,
    secrets: &dyn SecretStore,
    host: &str,
    port: u16,
) -> Vec<u8> {
    let mut out = String::with_capacity(1024);
    out.push_str(&format!("{} {} HTTP/1.1\r\n", parsed.method, parsed.target));

    let injected_headers = resolve_injection(
        inject,
        secrets,
        &parsed.method,
        host,
        port,
        &parsed.target,
    );

    // Strip any header the broker is about to inject so the sandbox can't
    // smuggle a value in. Compare case-insensitively.
    let injected_lc: std::collections::HashSet<String> = injected_headers
        .iter()
        .map(|(k, _)| k.to_ascii_lowercase())
        .collect();

    let mut have_conn = false;
    for (k, v) in &parsed.headers {
        let lower = k.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "connection"
                | "proxy-connection"
                | "keep-alive"
                | "te"
                | "transfer-encoding"
                | "upgrade"
                | "trailer"
                | "proxy-authenticate"
                | "proxy-authorization"
        ) {
            continue;
        }
        if injected_lc.contains(&lower) {
            continue;
        }
        if lower == "connection" {
            have_conn = true;
        }
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    for (k, v) in &injected_headers {
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    if !have_conn {
        out.push_str("Connection: close\r\n");
    }
    out.push_str("\r\n");
    out.into_bytes()
}

/// Returns the headers the broker should add to the outbound request.
/// Bearer/Basic produce one Authorization header; SigV4 produces two or
/// three (Authorization + X-Amz-Date + optional X-Amz-Security-Token).
fn resolve_injection(
    inject: &Inject,
    secrets: &dyn SecretStore,
    method: &str,
    host: &str,
    port: u16,
    path_and_query: &str,
) -> Vec<(String, String)> {
    match inject {
        Inject::None => Vec::new(),
        Inject::Bearer { key } => match secrets.resolve(key) {
            Some(tok) => vec![("Authorization".into(), format!("Bearer {tok}"))],
            None => Vec::new(),
        },
        Inject::Basic { user, pass } => match (secrets.resolve(user), secrets.resolve(pass)) {
            (Some(u), Some(p)) => vec![(
                "Authorization".into(),
                format!("Basic {}", base64ish::encode_basic(&u, &p)),
            )],
            _ => Vec::new(),
        },
        Inject::AwsSigV4 { profile } => match secrets.resolve_aws(profile) {
            Some(creds) => sign_aws_v4(method, host, port, path_and_query, &creds)
                .unwrap_or_default(),
            None => Vec::new(),
        },
        Inject::GcpIam { .. } | Inject::AzureMsi { .. } | Inject::HeaderTemplate { .. } => {
            Vec::new()
        }
    }
}

/// AWS Signature V4 signer — pure inline implementation against
/// sha2 + hmac. No AWS SDK dependency. Returns the headers the broker
/// should add: Authorization, X-Amz-Date, optionally X-Amz-Security-Token,
/// and X-Amz-Content-Sha256 (always sent for clarity).
///
/// Spec: https://docs.aws.amazon.com/general/latest/gr/sigv4_signing.html
fn sign_aws_v4(
    method: &str,
    host: &str,
    port: u16,
    path_and_query: &str,
    creds: &crate::secrets::AwsCredentials,
) -> Option<Vec<(String, String)>> {
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    type HmacSha256 = Hmac<Sha256>;

    let now = chrono::Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();

    // Split path?query.
    let (path, query) = match path_and_query.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (path_and_query.to_string(), String::new()),
    };
    let canonical_uri = if path.is_empty() {
        "/".to_string()
    } else {
        path
    };
    let canonical_query = canonicalize_query(&query);

    // Body hash. v1 only signs empty bodies (B1.8 doesn't read request
    // bodies). aws_treats empty as the SHA256 of "".
    let payload_hash = sha256_hex(&[]);

    // Host header per SigV4: include port only if non-standard.
    let host_header_value = if port == 443 || port == 80 {
        host.to_string()
    } else {
        format!("{host}:{port}")
    };

    // Canonical headers — must be sorted by lowercased name. We sign
    // `host` and `x-amz-date` (and security-token if present), plus
    // `x-amz-content-sha256` for completeness.
    let mut signed: Vec<(&str, String)> = vec![
        ("host", host_header_value.clone()),
        ("x-amz-content-sha256", payload_hash.clone()),
        ("x-amz-date", amz_date.clone()),
    ];
    if let Some(token) = &creds.session_token {
        signed.push(("x-amz-security-token", token.clone()));
    }
    signed.sort_by(|a, b| a.0.cmp(b.0));

    let canonical_headers = signed
        .iter()
        .map(|(k, v)| format!("{k}:{}\n", v.trim()))
        .collect::<String>();
    let signed_headers = signed
        .iter()
        .map(|(k, _)| *k)
        .collect::<Vec<_>>()
        .join(";");

    let canonical_request = format!(
        "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );

    let credential_scope = format!(
        "{date_stamp}/{region}/{service}/aws4_request",
        region = creds.region,
        service = creds.service
    );
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        sha256_hex(canonical_request.as_bytes())
    );

    // Derive signing key.
    let k_date = hmac_sha256(
        format!("AWS4{}", creds.secret_access_key).as_bytes(),
        date_stamp.as_bytes(),
    );
    let k_region = hmac_sha256(&k_date, creds.region.as_bytes());
    let k_service = hmac_sha256(&k_region, creds.service.as_bytes());
    let k_signing = hmac_sha256(&k_service, b"aws4_request");

    let signature = {
        let mut mac = HmacSha256::new_from_slice(&k_signing).ok()?;
        mac.update(string_to_sign.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };

    let auth_value = format!(
        "AWS4-HMAC-SHA256 Credential={key}/{scope}, SignedHeaders={sh}, Signature={sig}",
        key = creds.access_key_id,
        scope = credential_scope,
        sh = signed_headers,
        sig = signature
    );

    let mut out = vec![
        ("X-Amz-Date".to_string(), amz_date),
        ("X-Amz-Content-Sha256".to_string(), payload_hash),
        ("Authorization".to_string(), auth_value),
    ];
    if let Some(token) = &creds.session_token {
        out.push(("X-Amz-Security-Token".to_string(), token.clone()));
    }
    Some(out)
}

fn sha256_hex(input: &[u8]) -> String {
    use sha2::{Digest as _, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input);
    hex::encode(hasher.finalize())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key).expect("hmac key length valid");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// Canonicalize the query string per SigV4: sort by name, percent-encode
/// each name and value with the unreserved-character set, join with `&`.
/// v1 implementation handles the common no-query and simple `k=v&k2=v2`
/// cases; complex cases (empty values, repeated keys) get correct ordering
/// but conservative encoding.
fn canonicalize_query(query: &str) -> String {
    if query.is_empty() {
        return String::new();
    }
    let mut pairs: Vec<(String, String)> = query
        .split('&')
        .map(|p| match p.split_once('=') {
            Some((k, v)) => (
                aws_uri_encode(k, false),
                aws_uri_encode(v, false),
            ),
            None => (aws_uri_encode(p, false), String::new()),
        })
        .collect();
    pairs.sort();
    pairs
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// AWS URI encoding: encode everything except `A-Z`, `a-z`, `0-9`, `-`,
/// `_`, `.`, `~`. When `path=true`, also leave `/` unencoded.
fn aws_uri_encode(input: &str, path: bool) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for &b in input.as_bytes() {
        let unreserved = matches!(
            b,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~'
        );
        if unreserved || (path && b == b'/') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

mod base64ish {
    /// Tiny Base64 encoder — avoids pulling the `base64` crate just for
    /// the Authorization: Basic header.
    pub fn encode_basic(user: &str, pass: &str) -> String {
        let combined = format!("{user}:{pass}");
        encode(combined.as_bytes())
    }

    pub fn encode(input: &[u8]) -> String {
        const CHARSET: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
        for chunk in input.chunks(3) {
            let b0 = chunk[0];
            let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
            let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
            let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
            out.push(CHARSET[((n >> 18) & 63) as usize] as char);
            out.push(CHARSET[((n >> 12) & 63) as usize] as char);
            out.push(if chunk.len() > 1 {
                CHARSET[((n >> 6) & 63) as usize] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                CHARSET[(n & 63) as usize] as char
            } else {
                '='
            });
        }
        out
    }
}

async fn write_simple_response<S>(
    stream: &mut S,
    code: u16,
    phrase: &str,
    reason: &str,
) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    let body = format!("Denied: {reason}\n");
    let resp = format!(
        "HTTP/1.1 {code} {phrase}\r\n\
         X-Vibe-Broker-Reason: {reason}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_upstream_roots_is_nonempty() {
        let store = default_upstream_roots();
        assert!(store.len() > 0);
    }

    #[test]
    fn parse_head_basic() {
        let raw = b"GET /v1/messages HTTP/1.1\r\nHost: api.openai.com\r\nAuthorization: existing\r\n\r\n";
        let p = parse_head(raw).unwrap();
        assert_eq!(p.method, "GET");
        assert_eq!(p.target, "/v1/messages");
        assert_eq!(p.headers.len(), 2);
    }

    #[test]
    fn parse_head_rejects_garbage() {
        assert!(parse_head(b"NOTHTTP\r\n\r\n").is_none());
    }

    #[test]
    fn build_outbound_strips_hop_by_hop() {
        let parsed = ParsedReq {
            method: "GET".into(),
            target: "/".into(),
            headers: vec![
                ("Connection".into(), "keep-alive".into()),
                ("Host".into(), "api.example.com".into()),
                ("Transfer-Encoding".into(), "chunked".into()),
            ],
        };
        let store = crate::secrets::EmptySecretStore;
        let req = build_outbound_request(&parsed, &Inject::None, &store, "api.example.com", 443);
        let s = String::from_utf8(req).unwrap();
        assert!(!s.contains("keep-alive"));
        assert!(!s.contains("Transfer-Encoding"));
        assert!(s.contains("Host: api.example.com"));
        assert!(s.contains("Connection: close"));
    }

    #[test]
    fn build_outbound_injects_bearer_and_drops_existing_auth() {
        let parsed = ParsedReq {
            method: "POST".into(),
            target: "/v1/messages".into(),
            headers: vec![
                ("Host".into(), "api.openai.com".into()),
                ("Authorization".into(), "Bearer sandbox-fake".into()),
            ],
        };
        let store = crate::secrets::InMemorySecretStore::new();
        store.set("@profile.openai_key", "sk-real-token");
        let inject = Inject::Bearer {
            key: crate::policy::SecretRef("@profile.openai_key".into()),
        };
        let req = build_outbound_request(&parsed, &inject, &store, "api.openai.com", 443);
        let s = String::from_utf8(req).unwrap();
        assert!(s.contains("Authorization: Bearer sk-real-token"));
        assert!(!s.contains("sandbox-fake"));
    }

    #[test]
    fn sigv4_injection_adds_authorization_and_x_amz_date() {
        let parsed = ParsedReq {
            method: "GET".into(),
            target: "/".into(),
            headers: vec![("Host".into(), "127.0.0.1:443".into())],
        };
        let store = crate::secrets::InMemorySecretStore::new();
        store.set_aws(
            "@workspace.aws_default",
            crate::secrets::AwsCredentials {
                access_key_id: "AKIAIOSFODNN7EXAMPLE".into(),
                secret_access_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
                session_token: None,
                region: "us-east-1".into(),
                service: "s3".into(),
            },
        );
        let inject = Inject::AwsSigV4 {
            profile: crate::policy::SecretRef("@workspace.aws_default".into()),
        };
        let req = build_outbound_request(&parsed, &inject, &store, "127.0.0.1", 443);
        let s = String::from_utf8(req).unwrap();
        assert!(
            s.contains("Authorization: AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/"),
            "missing SigV4 Authorization: {s}"
        );
        assert!(s.to_ascii_lowercase().contains("x-amz-date:"));
        assert!(s.contains("SignedHeaders="));
        assert!(s.contains("Signature="));
    }

    #[test]
    fn base64_encode_known_vector() {
        assert_eq!(base64ish::encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64ish::encode(b"f"), "Zg==");
        assert_eq!(base64ish::encode(b"fo"), "Zm8=");
    }
}
