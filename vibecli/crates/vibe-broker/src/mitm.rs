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
    let req_bytes = build_outbound_request(&parsed, &inject, inspect.secrets);
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
) -> Vec<u8> {
    let mut out = String::with_capacity(1024);
    out.push_str(&format!("{} {} HTTP/1.1\r\n", parsed.method, parsed.target));

    let injected_auth = resolve_injection(inject, secrets);

    let mut have_conn = false;
    for (k, v) in &parsed.headers {
        let lower = k.to_ascii_lowercase();
        // Drop hop-by-hop and any pre-existing Authorization the sandbox
        // tried to set — credentials only come from the broker.
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
        if injected_auth.is_some() && lower == "authorization" {
            continue;
        }
        if lower == "connection" {
            have_conn = true;
        }
        out.push_str(&format!("{k}: {v}\r\n"));
    }
    if let Some(auth) = injected_auth {
        out.push_str(&format!("Authorization: {auth}\r\n"));
    }
    if !have_conn {
        out.push_str("Connection: close\r\n");
    }
    out.push_str("\r\n");
    out.into_bytes()
}

fn resolve_injection(inject: &Inject, secrets: &dyn SecretStore) -> Option<String> {
    match inject {
        Inject::None => None,
        Inject::Bearer { key } => {
            let tok = secrets.resolve(key)?;
            Some(format!("Bearer {tok}"))
        }
        Inject::Basic { user, pass } => {
            let u = secrets.resolve(user)?;
            let p = secrets.resolve(pass)?;
            Some(format!("Basic {}", base64ish::encode_basic(&u, &p)))
        }
        // SigV4, GCP IAM, Azure MSI, header templates land in B2.2+
        Inject::AwsSigV4 { .. }
        | Inject::GcpIam { .. }
        | Inject::AzureMsi { .. }
        | Inject::HeaderTemplate { .. } => None,
    }
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
        let req = build_outbound_request(&parsed, &Inject::None, &store);
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
        let req = build_outbound_request(&parsed, &inject, &store);
        let s = String::from_utf8(req).unwrap();
        assert!(s.contains("Authorization: Bearer sk-real-token"));
        assert!(!s.contains("sandbox-fake"));
    }

    #[test]
    fn base64_encode_known_vector() {
        assert_eq!(base64ish::encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64ish::encode(b"f"), "Zg==");
        assert_eq!(base64ish::encode(b"fo"), "Zm8=");
    }
}
