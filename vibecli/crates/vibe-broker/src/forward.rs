//! Upstream forwarder (slice B1.5).
//!
//! When `Broker::handle` decides Allow, it currently returns a stub 200.
//! This module is the dispatcher that actually opens a TCP connection to
//! the resolved upstream, replays the request, and streams the response
//! body back. v1 is plain HTTP only — TLS interception (with rcgen-minted
//! per-broker root CA) is slice B1.4-tls.

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Debug)]
pub struct ForwardRequest<'a> {
    pub method: &'a str,
    pub url: &'a url::Url,
    pub headers: &'a [(String, String)],
    pub body: &'a [u8],
}

#[derive(Debug)]
pub struct ForwardResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum ForwardError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("scheme {0} is not supported by the v1 forwarder")]
    UnsupportedScheme(String),
    #[error("missing host in url")]
    MissingHost,
    #[error("upstream returned malformed response")]
    BadResponse,
    #[error("upstream timeout after {0:?}")]
    Timeout(Duration),
}

pub async fn forward_plain_http(
    req: ForwardRequest<'_>,
    timeout: Duration,
) -> Result<ForwardResponse, ForwardError> {
    if req.url.scheme() != "http" {
        return Err(ForwardError::UnsupportedScheme(req.url.scheme().to_owned()));
    }
    let host = req.url.host_str().ok_or(ForwardError::MissingHost)?;
    let port = req.url.port().unwrap_or(80);
    let target = format!("{host}:{port}");

    let connect = tokio::time::timeout(timeout, TcpStream::connect(&target));
    let mut stream = match connect.await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return Err(ForwardError::Io(e)),
        Err(_) => return Err(ForwardError::Timeout(timeout)),
    };

    let path_and_query = format!(
        "{}{}",
        req.url.path(),
        req.url
            .query()
            .map(|q| format!("?{q}"))
            .unwrap_or_default()
    );

    let mut head = format!("{} {} HTTP/1.1\r\n", req.method, path_and_query);
    let mut have_host = false;
    let mut have_cl = false;
    let mut have_conn = false;
    for (k, v) in req.headers {
        let k_lower = k.to_ascii_lowercase();
        if k_lower == "host" {
            have_host = true;
        }
        if k_lower == "content-length" {
            have_cl = true;
        }
        if k_lower == "connection" {
            have_conn = true;
        }
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    if !have_host {
        head.push_str(&format!("Host: {host}\r\n"));
    }
    if !have_cl && !req.body.is_empty() {
        head.push_str(&format!("Content-Length: {}\r\n", req.body.len()));
    }
    if !have_conn {
        head.push_str("Connection: close\r\n");
    }
    head.push_str("\r\n");

    stream.write_all(head.as_bytes()).await?;
    if !req.body.is_empty() {
        stream.write_all(req.body).await?;
    }

    let read_fut = async {
        let mut buf = Vec::with_capacity(4096);
        stream.read_to_end(&mut buf).await?;
        std::io::Result::Ok(buf)
    };
    let raw = match tokio::time::timeout(timeout, read_fut).await {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => return Err(ForwardError::Io(e)),
        Err(_) => return Err(ForwardError::Timeout(timeout)),
    };

    parse_response(&raw)
}

fn parse_response(buf: &[u8]) -> Result<ForwardResponse, ForwardError> {
    let split_pos = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or(ForwardError::BadResponse)?;
    let head_text = std::str::from_utf8(&buf[..split_pos]).map_err(|_| ForwardError::BadResponse)?;
    let body = buf[split_pos + 4..].to_vec();

    let mut lines = head_text.split("\r\n");
    let status_line = lines.next().ok_or(ForwardError::BadResponse)?;
    let mut parts = status_line.splitn(3, ' ');
    let _ = parts.next();
    let status: u16 = parts
        .next()
        .ok_or(ForwardError::BadResponse)?
        .parse()
        .map_err(|_| ForwardError::BadResponse)?;
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_owned(), v.trim().to_owned()));
        }
    }
    Ok(ForwardResponse {
        status,
        headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response_extracts_status_and_body() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nhello";
        let r = parse_response(raw).unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(r.headers.len(), 1);
        assert_eq!(r.body, b"hello");
    }

    #[test]
    fn parse_response_rejects_truncated_head() {
        let raw = b"HTTP/1.1 200 OK\r\nMissingHeaderTerm";
        assert!(parse_response(raw).is_err());
    }

    #[test]
    fn parse_response_rejects_bad_status_line() {
        let raw = b"NOTHTTP\r\n\r\n";
        assert!(parse_response(raw).is_err());
    }
}
