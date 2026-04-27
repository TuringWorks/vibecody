//! Broker accept loop.
//!
//! Tokio-driven listener that parses each incoming HTTP/1.1 request,
//! routes through `SsrfGuard` then `Policy`, and either denies (451),
//! forwards upstream (slice B1.5), or returns a stub 200.
//!
//! Slice B1.4: TCP listener. Slice B1.6: Unix-domain-socket listener
//! (used to bind the broker into Tier-0 sandboxes on Linux/macOS).
//! Both transports share the same handler — only the acceptor differs.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::forward::{ForwardError, ForwardRequest, forward_plain_http};
use crate::policy::{Decision, Policy, Request as PolicyRequest};
use crate::ssrf::{SsrfGuard, SsrfVerdict};

/// Address the broker is bound to. Returned by `Broker::start_*`.
#[derive(Debug, Clone)]
pub enum BoundAddr {
    Tcp(std::net::SocketAddr),
    Unix(PathBuf),
}

/// Owns the listening socket so it's torn down on drop. For UDS, also
/// removes the path on drop so a subsequent `start_uds` on the same path
/// works without manual cleanup.
pub struct BrokerHandle {
    pub addr: BoundAddr,
    pub join: JoinHandle<()>,
    _cleanup: Option<UdsCleanup>,
}

impl BrokerHandle {
    /// Abort the accept loop. Does not wait for in-flight connections.
    pub fn abort(&self) {
        self.join.abort();
    }
}

struct UdsCleanup {
    path: PathBuf,
}

impl Drop for UdsCleanup {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Compose the broker's request-handling pipeline. One per running broker.
#[derive(Clone)]
pub struct Broker {
    pub policy: Arc<Policy>,
    pub ssrf: Arc<SsrfGuard>,
    /// When true, allowed requests are forwarded to the real upstream. When
    /// false (default), allowed requests get a stub 200 — useful for tests
    /// that don't want to exercise the network forwarder.
    pub forward_upstream: bool,
    /// Per-request timeout when forwarding upstream.
    pub upstream_timeout: Duration,
}

impl Broker {
    pub fn new(policy: Policy, ssrf: SsrfGuard) -> Self {
        Broker {
            policy: Arc::new(policy),
            ssrf: Arc::new(ssrf),
            forward_upstream: false,
            upstream_timeout: Duration::from_secs(15),
        }
    }

    pub fn with_upstream(mut self) -> Self {
        self.forward_upstream = true;
        self
    }

    pub fn with_upstream_timeout(mut self, t: Duration) -> Self {
        self.upstream_timeout = t;
        self
    }

    /// Start the broker on a TCP listener. Useful for tests, for vsock
    /// bridging in Tier-3 (Firecracker), and as a fallback on hosts that
    /// can't expose a UDS into the sandbox.
    pub async fn start_tcp(self, addr: &str) -> std::io::Result<BrokerHandle> {
        let listener = TcpListener::bind(addr).await?;
        let bound = listener.local_addr()?;
        let join = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _peer)) => {
                        let broker = self.clone();
                        tokio::spawn(async move {
                            if let Err(e) = broker.handle(stream).await {
                                tracing::debug!("broker connection error: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        tracing::warn!("broker accept failed: {e}");
                        break;
                    }
                }
            }
        });
        Ok(BrokerHandle {
            addr: BoundAddr::Tcp(bound),
            join,
            _cleanup: None,
        })
    }

    /// Start the broker on a Unix domain socket. The path is the canonical
    /// transport for bind-mounting the broker into a Tier-0 sandbox on
    /// Linux/macOS. The socket file is removed when the returned handle
    /// is dropped.
    #[cfg(unix)]
    pub async fn start_uds(self, path: &Path) -> std::io::Result<BrokerHandle> {
        // If a stale socket from a previous run is sitting at this path
        // and no one is listening, remove it. We refuse to clobber a path
        // that isn't a socket (avoids deleting unrelated files).
        if let Ok(meta) = std::fs::metadata(path) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::FileTypeExt;
                if !meta.file_type().is_socket() {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::AlreadyExists,
                        format!("path is not a socket: {}", path.display()),
                    ));
                }
            }
            let _ = std::fs::remove_file(path);
        }

        let listener = tokio::net::UnixListener::bind(path)?;
        let owned_path = path.to_path_buf();
        let join = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _peer)) => {
                        let broker = self.clone();
                        tokio::spawn(async move {
                            if let Err(e) = broker.handle(stream).await {
                                tracing::debug!("broker connection error: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        tracing::warn!("broker accept failed: {e}");
                        break;
                    }
                }
            }
        });
        Ok(BrokerHandle {
            addr: BoundAddr::Unix(owned_path.clone()),
            join,
            _cleanup: Some(UdsCleanup { path: owned_path }),
        })
    }

    async fn handle<S>(&self, mut stream: S) -> std::io::Result<()>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        let mut buf = vec![0u8; 8 * 1024];
        let n = read_until_headers(&mut stream, &mut buf).await?;
        let raw = &buf[..n];

        let parsed = match parse_request_head(raw) {
            Ok(p) => p,
            Err(_) => {
                stream
                    .write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .await?;
                return Ok(());
            }
        };

        // Build the URL the policy/SSRF want to see. Forward-proxy clients
        // send absolute URIs (`GET http://host/path HTTP/1.1`); plain clients
        // send relative + Host: header.
        let url = absolute_url(&parsed);

        let ssrf = self.ssrf.check(&url);
        if matches!(ssrf, SsrfVerdict::Block) {
            return write_denied(&mut stream, "ssrf_blocked").await;
        }

        let decision = self.policy.match_request(&PolicyRequest {
            method: &parsed.method,
            url: &url,
        });
        match decision {
            Decision::Deny => write_denied(&mut stream, "policy_denied").await,
            Decision::Allow { .. } if self.forward_upstream => {
                self.do_forward(&mut stream, &parsed, &url).await
            }
            Decision::Allow { .. } => write_allow_stub(&mut stream).await,
        }
    }

    async fn do_forward<S>(
        &self,
        stream: &mut S,
        parsed: &ParsedHead,
        url_str: &str,
    ) -> std::io::Result<()>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send,
    {
        let parsed_url = match url::Url::parse(url_str) {
            Ok(u) => u,
            Err(_) => return write_denied(stream, "policy_denied").await,
        };
        // Filter out hop-by-hop headers before replaying.
        let headers: Vec<(String, String)> = parsed
            .headers
            .iter()
            .filter(|(k, _)| !is_hop_by_hop(k))
            .cloned()
            .collect();
        let req = ForwardRequest {
            method: &parsed.method,
            url: &parsed_url,
            headers: &headers,
            body: &[],
        };
        let result = forward_plain_http(req, self.upstream_timeout).await;
        match result {
            Ok(resp) => write_forwarded(stream, resp).await,
            Err(ForwardError::Timeout(_)) => write_upstream_error(stream, 504, "upstream_timeout").await,
            Err(ForwardError::UnsupportedScheme(_)) => {
                write_denied(stream, "policy_denied").await
            }
            Err(_) => write_upstream_error(stream, 502, "upstream_error").await,
        }
    }
}

fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "proxy-connection"
            | "keep-alive"
            | "te"
            | "transfer-encoding"
            | "upgrade"
            | "trailer"
            | "proxy-authenticate"
            | "proxy-authorization"
    )
}

#[derive(Debug)]
struct ParsedHead {
    method: String,
    target: String,
    host: Option<String>,
    scheme_hint: String,
    headers: Vec<(String, String)>,
}

fn parse_request_head(buf: &[u8]) -> Result<ParsedHead, &'static str> {
    let text = std::str::from_utf8(buf).map_err(|_| "non-utf8")?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next().ok_or("missing request line")?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().ok_or("no method")?.to_string();
    let target = parts.next().ok_or("no target")?.to_string();
    let version = parts.next().ok_or("no version")?;
    if !version.starts_with("HTTP/") {
        return Err("bad version");
    }
    let mut host = None;
    let mut scheme_hint = String::from("http");
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            let n = name.trim();
            let n_lower = n.to_ascii_lowercase();
            let v = value.trim();
            if n_lower == "host" {
                host = Some(v.to_owned());
            } else if n_lower == "x-forwarded-proto" {
                scheme_hint = v.to_owned();
            }
            headers.push((n.to_owned(), v.to_owned()));
        }
    }
    Ok(ParsedHead {
        method,
        target,
        host,
        scheme_hint,
        headers,
    })
}

fn absolute_url(p: &ParsedHead) -> String {
    if p.target.starts_with("http://") || p.target.starts_with("https://") {
        return p.target.clone();
    }
    let host = p.host.clone().unwrap_or_default();
    format!("{}://{}{}", p.scheme_hint, host, p.target)
}

async fn write_denied<S>(stream: &mut S, reason: &str) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    let body = format!("Denied: {reason}\n");
    let resp = format!(
        "HTTP/1.1 451 Unavailable For Legal Reasons\r\n\
         X-Vibe-Broker-Reason: {reason}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n{body}",
        body.len()
    );
    stream.write_all(resp.as_bytes()).await
}

async fn write_allow_stub<S>(stream: &mut S) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    let body = b"vibe-broker stub: enable Broker::with_upstream() for live forwarding\n";
    let resp = format!(
        "HTTP/1.1 200 OK\r\n\
         X-Vibe-Broker-Stub: true\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        body.len()
    );
    stream.write_all(resp.as_bytes()).await?;
    stream.write_all(body).await
}

async fn write_forwarded<S>(
    stream: &mut S,
    resp: crate::forward::ForwardResponse,
) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    let mut head = format!("HTTP/1.1 {} OK\r\n", resp.status);
    head.push_str("X-Vibe-Broker-Forwarded: true\r\n");
    let mut have_cl = false;
    for (k, v) in &resp.headers {
        let lower = k.to_ascii_lowercase();
        if lower == "transfer-encoding"
            || lower == "connection"
            || lower == "keep-alive"
            || lower == "trailer"
            || lower == "te"
            || lower == "upgrade"
        {
            continue;
        }
        if lower == "content-length" {
            have_cl = true;
        }
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    if !have_cl {
        head.push_str(&format!("Content-Length: {}\r\n", resp.body.len()));
    }
    head.push_str("Connection: close\r\n\r\n");
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(&resp.body).await
}

async fn write_upstream_error<S>(
    stream: &mut S,
    code: u16,
    reason: &str,
) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    let phrase = match code {
        502 => "Bad Gateway",
        504 => "Gateway Timeout",
        _ => "Upstream Error",
    };
    let body = format!("Upstream: {reason}\n");
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

async fn read_until_headers<S>(stream: &mut S, buf: &mut [u8]) -> std::io::Result<usize>
where
    S: AsyncRead + Unpin,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_get() {
        let raw = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let p = parse_request_head(raw).unwrap();
        assert_eq!(p.method, "GET");
        assert_eq!(p.target, "/");
        assert_eq!(p.host.as_deref(), Some("example.com"));
    }

    #[test]
    fn absolute_url_uses_host_header() {
        let p = ParsedHead {
            method: "GET".into(),
            target: "/path".into(),
            host: Some("api.example.com".into()),
            scheme_hint: "http".into(),
            headers: vec![],
        };
        assert_eq!(absolute_url(&p), "http://api.example.com/path");
    }

    #[test]
    fn absolute_url_passes_through_absolute_target() {
        let p = ParsedHead {
            method: "GET".into(),
            target: "https://api.example.com/path".into(),
            host: None,
            scheme_hint: "http".into(),
            headers: vec![],
        };
        assert_eq!(absolute_url(&p), "https://api.example.com/path");
    }

    #[test]
    fn hop_by_hop_classification() {
        assert!(is_hop_by_hop("Connection"));
        assert!(is_hop_by_hop("transfer-encoding"));
        assert!(!is_hop_by_hop("authorization"));
    }

    #[test]
    fn parse_rejects_garbage_request_line() {
        let raw = b"NOTHTTP\r\n\r\n";
        assert!(parse_request_head(raw).is_err());
    }
}
