//! HTTP-over-UDS client for Firecracker's API socket.
//!
//! Slice F2.2-A — the wire client F2.2-B's microVM lifecycle uses to
//! talk to Firecracker. Firecracker speaks HTTP/1.1 over a Unix-domain
//! socket (`--api-sock /path/to/firecracker.sock`); this client is
//! the smallest surface that does: connect → write request → read
//! response → close.
//!
//! ## Why pure std (no hyper / reqwest)
//!
//! 1. **No new deps on a leaf crate.** vibe-sandbox-firecracker is a
//!    leaf in the dep graph; adding hyper pulls h2, tower, tokio,
//!    rustls — overkill for ~5 PUTs to a Unix socket.
//! 2. **Sync client suffices.** F2.2-B's microVM boot is a serial
//!    sequence (~5 PUTs over ~10 ms total). Async buys nothing.
//! 3. **Testable without spinning Firecracker.** We can stand up a
//!    stub `UnixListener` in a background thread that reads + replies,
//!    pinning the wire format on any platform.
//!
//! ## Scope
//!
//! Only what F2.2-B needs:
//!
//! * `PUT /endpoint` with JSON body
//! * Status code + optional body in response
//! * Connect with deadline
//! * No keep-alive (each request opens a fresh connection — fine for
//!   the boot sequence)
//! * No streaming, no chunked encoding (Firecracker responses are
//!   always Content-Length-bounded)
//!
//! ## Cross-platform note
//!
//! UDS is Unix-only at the std API level. Windows has UDS support
//! since 1803, but `std::os::unix::net::UnixStream` isn't available
//! on Windows targets. The whole module is `#[cfg(unix)]`-gated; on
//! Windows the consumer (F2.2-B) is already gated to Linux only
//! because Firecracker is Linux-only.

#![cfg(unix)]

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::{Duration, Instant};

use thiserror::Error;

/// Errors from the API client.
#[derive(Debug, Error)]
pub enum ApiClientError {
    #[error("connect failed: {0}")]
    Connect(std::io::Error),

    #[error("write failed: {0}")]
    Write(std::io::Error),

    #[error("read failed: {0}")]
    Read(std::io::Error),

    #[error("malformed response: {0}")]
    BadResponse(String),

    #[error("firecracker rejected request: status={status}, body={body:?}")]
    HttpError { status: u16, body: Option<String> },

    #[error("timed out waiting for UDS at {path}")]
    SocketWaitTimeout { path: String },
}

/// One HTTP response from the Firecracker API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiResponse {
    pub status: u16,
    pub body: Option<String>,
}

impl ApiResponse {
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// PUT a JSON body to the given API path on Firecracker's UDS.
///
/// Firecracker uses 204 No Content as the success status for most
/// boot-path endpoints; the boot-source / machine-config / drives /
/// vsock / actions endpoints all return 204 on success and 400 with
/// a `{"fault_message": "..."}` body on failure.
pub fn put_json(
    socket_path: &Path,
    api_path: &str,
    body: &serde_json::Value,
    timeout: Duration,
) -> Result<ApiResponse, ApiClientError> {
    let mut stream = connect_with_timeout(socket_path, timeout)?;

    let body_bytes = serde_json::to_vec(body).map_err(|e| {
        ApiClientError::BadResponse(format!("encode body: {}", e))
    })?;

    let request = format!(
        "PUT {} HTTP/1.1\r\n\
         Host: localhost\r\n\
         Accept: application/json\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        api_path,
        body_bytes.len()
    );

    stream
        .write_all(request.as_bytes())
        .map_err(ApiClientError::Write)?;
    stream
        .write_all(&body_bytes)
        .map_err(ApiClientError::Write)?;
    stream.flush().map_err(ApiClientError::Write)?;

    let resp = read_response(&mut stream)?;
    if !resp.is_success() {
        return Err(ApiClientError::HttpError {
            status: resp.status,
            body: resp.body,
        });
    }
    Ok(resp)
}

/// GET the given path. Used by health probes (e.g. `GET /` returns
/// instance metadata).
pub fn get(
    socket_path: &Path,
    api_path: &str,
    timeout: Duration,
) -> Result<ApiResponse, ApiClientError> {
    let mut stream = connect_with_timeout(socket_path, timeout)?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: localhost\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        api_path
    );
    stream
        .write_all(request.as_bytes())
        .map_err(ApiClientError::Write)?;
    stream.flush().map_err(ApiClientError::Write)?;
    read_response(&mut stream)
}

/// Wait up to `timeout` for the API socket to appear + accept
/// connections. Firecracker creates the socket asynchronously after
/// `--api-sock` is passed; F2.2-B needs to wait for it before
/// issuing PUTs.
pub fn wait_for_socket(socket_path: &Path, timeout: Duration) -> Result<(), ApiClientError> {
    let deadline = Instant::now() + timeout;
    loop {
        if socket_path.exists() {
            match UnixStream::connect(socket_path) {
                Ok(_) => return Ok(()),
                Err(_) => { /* still booting */ }
            }
        }
        if Instant::now() >= deadline {
            return Err(ApiClientError::SocketWaitTimeout {
                path: socket_path.display().to_string(),
            });
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

// ── Internals ────────────────────────────────────────────────────────────────

fn connect_with_timeout(path: &Path, timeout: Duration) -> Result<UnixStream, ApiClientError> {
    let stream = UnixStream::connect(path).map_err(ApiClientError::Connect)?;
    // Read + write timeouts so a stuck firecracker doesn't hang the
    // boot sequence forever. The connect itself isn't timeout-able
    // via std (UDS rarely blocks on connect anyway); we apply the
    // budget to the round-trip.
    stream
        .set_read_timeout(Some(timeout))
        .map_err(ApiClientError::Connect)?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(ApiClientError::Connect)?;
    Ok(stream)
}

fn read_response(stream: &mut UnixStream) -> Result<ApiResponse, ApiClientError> {
    let mut buf = Vec::new();
    // Read the full response. Firecracker uses Connection: close so
    // the stream ends at body end; for HTTP/1.1 keep-alive we'd need
    // a smarter parser, but the boot sequence never reuses.
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) => return Err(ApiClientError::Read(e)),
        }
    }

    parse_http_response(&buf)
}

/// Strict HTTP/1.1 response parser scoped to what Firecracker emits.
/// Recognized shape:
///
///   HTTP/1.1 <status> <reason>\r\n
///   Header: Value\r\n
///   ...
///   \r\n
///   <body>
fn parse_http_response(buf: &[u8]) -> Result<ApiResponse, ApiClientError> {
    // Find header/body separator.
    let sep = find_subsequence(buf, b"\r\n\r\n")
        .ok_or_else(|| ApiClientError::BadResponse("no header/body separator".into()))?;
    let headers = &buf[..sep];
    let body = &buf[sep + 4..];

    // First line: status.
    let first_line_end = find_subsequence(headers, b"\r\n").unwrap_or(headers.len());
    let first_line = std::str::from_utf8(&headers[..first_line_end])
        .map_err(|_| ApiClientError::BadResponse("non-UTF8 status line".into()))?;
    // Expected: "HTTP/1.1 204 No Content"
    let mut parts = first_line.splitn(3, ' ');
    let _proto = parts
        .next()
        .ok_or_else(|| ApiClientError::BadResponse("missing protocol".into()))?;
    let status_str = parts
        .next()
        .ok_or_else(|| ApiClientError::BadResponse("missing status code".into()))?;
    let status: u16 = status_str
        .parse()
        .map_err(|_| ApiClientError::BadResponse(format!("bad status code: {}", status_str)))?;

    // Body — only meaningful if non-empty.
    let body_str = if body.is_empty() {
        None
    } else {
        Some(
            std::str::from_utf8(body)
                .map_err(|_| ApiClientError::BadResponse("non-UTF8 body".into()))?
                .to_string(),
        )
    };
    Ok(ApiResponse {
        status,
        body: body_str,
    })
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::sync::mpsc;
    use std::thread;

    /// Spawn a one-shot UDS server that accepts one connection, captures
    /// the bytes the client sends, and writes a canned response.
    /// Returns (socket_path, request_receiver, server_thread).
    fn spawn_one_shot(
        canned_response: &'static str,
    ) -> (
        std::path::PathBuf,
        mpsc::Receiver<Vec<u8>>,
        thread::JoinHandle<()>,
    ) {
        let dir = std::env::temp_dir().join(format!(
            "vibe_api_client_test_{}_{}",
            std::process::id(),
            rand_suffix()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let sock = dir.join("api.sock");
        let listener = UnixListener::bind(&sock).expect("bind");
        let (tx, rx) = mpsc::channel();

        let h = thread::spawn(move || {
            if let Ok((mut s, _)) = listener.accept() {
                let mut buf = Vec::new();
                let mut chunk = [0u8; 4096];
                // Read until the client closes — but bounded so a
                // buggy client doesn't hang us.
                s.set_read_timeout(Some(Duration::from_secs(2))).ok();
                loop {
                    match s.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => {
                            buf.extend_from_slice(&chunk[..n]);
                            // Once we have the header end + content-length
                            // bytes of body, write the response and break.
                            // For tests we just bail after the first chunk
                            // if it contains \r\n\r\n.
                            if find_subsequence(&buf, b"\r\n\r\n").is_some() {
                                // Read the body if any (Content-Length-bounded).
                                let cl = parse_content_length(&buf).unwrap_or(0);
                                let body_start =
                                    find_subsequence(&buf, b"\r\n\r\n").unwrap() + 4;
                                let body_have = buf.len() - body_start;
                                if body_have < cl {
                                    // Read remainder.
                                    let need = cl - body_have;
                                    let mut tmp = vec![0u8; need];
                                    let n = s.read(&mut tmp).unwrap_or(0);
                                    buf.extend_from_slice(&tmp[..n]);
                                }
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                tx.send(buf).ok();
                let _ = s.write_all(canned_response.as_bytes());
                let _ = s.flush();
            }
        });

        (sock, rx, h)
    }

    fn parse_content_length(buf: &[u8]) -> Option<usize> {
        let s = std::str::from_utf8(buf).ok()?;
        for line in s.split("\r\n") {
            let lower = line.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("content-length:") {
                return rest.trim().parse().ok();
            }
        }
        None
    }

    fn rand_suffix() -> u64 {
        // Monotonically increasing per-process counter — guarantees
        // uniqueness across parallel tests in the same `cargo test`
        // process. (The earlier Instant-based helper was buggy:
        // `Instant::now().elapsed()` is always ~0, so all parallel
        // tests collided on the same socket path.)
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    // ── parse_http_response — unit-level ─────────────────────────────────

    #[test]
    fn parses_204_no_body() {
        let raw = b"HTTP/1.1 204 No Content\r\nServer: Firecracker\r\n\r\n";
        let r = parse_http_response(raw).unwrap();
        assert_eq!(r.status, 204);
        assert!(r.body.is_none());
        assert!(r.is_success());
    }

    #[test]
    fn parses_200_with_body() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"id\":\"i-42\"}";
        let r = parse_http_response(raw).unwrap();
        assert_eq!(r.status, 200);
        assert_eq!(r.body.as_deref(), Some("{\"id\":\"i-42\"}"));
        assert!(r.is_success());
    }

    #[test]
    fn parses_400_with_fault_message() {
        let raw = b"HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\n\r\n{\"fault_message\":\"missing kernel\"}";
        let r = parse_http_response(raw).unwrap();
        assert_eq!(r.status, 400);
        assert!(r.body.as_deref().unwrap().contains("missing kernel"));
        assert!(!r.is_success());
    }

    #[test]
    fn rejects_response_with_no_separator() {
        let raw = b"HTTP/1.1 200 OK\r\nNo separator here";
        let err = parse_http_response(raw).unwrap_err();
        assert!(matches!(err, ApiClientError::BadResponse(_)));
    }

    #[test]
    fn rejects_response_with_bad_status() {
        let raw = b"HTTP/1.1 NaN Whatever\r\n\r\n";
        let err = parse_http_response(raw).unwrap_err();
        assert!(matches!(err, ApiClientError::BadResponse(_)));
    }

    // ── End-to-end: PUT JSON over real UDS ───────────────────────────────

    #[test]
    fn put_json_sends_correct_request() {
        let (sock, rx, h) = spawn_one_shot("HTTP/1.1 204 No Content\r\n\r\n");
        let body = serde_json::json!({"vcpu_count": 1, "mem_size_mib": 128, "smt": false});
        let resp = put_json(&sock, "/machine-config", &body, Duration::from_secs(2)).unwrap();

        assert_eq!(resp.status, 204);
        assert!(resp.body.is_none());

        h.join().unwrap();
        let captured = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let s = String::from_utf8(captured).unwrap();
        // Request-line + headers.
        assert!(s.starts_with("PUT /machine-config HTTP/1.1\r\n"));
        assert!(s.contains("Host: localhost\r\n"));
        assert!(s.contains("Content-Type: application/json\r\n"));
        assert!(s.contains("Connection: close\r\n"));
        // Body separated by blank line, parses back to the original JSON.
        let body_start = s.find("\r\n\r\n").unwrap() + 4;
        let body_str = &s[body_start..];
        let parsed: serde_json::Value = serde_json::from_str(body_str).unwrap();
        assert_eq!(parsed["vcpu_count"], 1);
        assert_eq!(parsed["mem_size_mib"], 128);
        assert_eq!(parsed["smt"], false);

        std::fs::remove_file(sock).ok();
    }

    #[test]
    fn put_json_returns_http_error_on_400() {
        let (sock, _rx, h) = spawn_one_shot(
            "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\n\r\n{\"fault_message\":\"bad\"}",
        );
        let err = put_json(
            &sock,
            "/machine-config",
            &serde_json::json!({}),
            Duration::from_secs(2),
        )
        .unwrap_err();
        match err {
            ApiClientError::HttpError { status, body } => {
                assert_eq!(status, 400);
                assert!(body.unwrap().contains("bad"));
            }
            other => panic!("expected HttpError, got {:?}", other),
        }
        h.join().unwrap();
        std::fs::remove_file(sock).ok();
    }

    #[test]
    fn put_json_content_length_matches_body() {
        let (sock, rx, h) = spawn_one_shot("HTTP/1.1 204 No Content\r\n\r\n");
        let body = serde_json::json!({"path": "/boot/vmlinux.bin", "boot_args": "console=ttyS0"});
        put_json(&sock, "/boot-source", &body, Duration::from_secs(2)).unwrap();
        h.join().unwrap();
        let captured = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let s = String::from_utf8(captured).unwrap();

        let body_start = s.find("\r\n\r\n").unwrap() + 4;
        let body_bytes = &s.as_bytes()[body_start..];

        // Extract content-length header.
        let cl_line = s
            .lines()
            .find(|l| l.to_ascii_lowercase().starts_with("content-length:"))
            .unwrap();
        let cl: usize = cl_line
            .split(':')
            .nth(1)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        assert_eq!(
            cl, body_bytes.len(),
            "content-length must match the JSON body byte length exactly"
        );
        std::fs::remove_file(sock).ok();
    }

    // ── wait_for_socket ──────────────────────────────────────────────────

    #[test]
    fn wait_for_socket_returns_quickly_when_already_bound() {
        let (sock, _rx, h) = spawn_one_shot("HTTP/1.1 204 No Content\r\n\r\n");
        let start = Instant::now();
        wait_for_socket(&sock, Duration::from_secs(1)).unwrap();
        assert!(start.elapsed() < Duration::from_millis(500));
        // Drain the listener so the thread exits cleanly.
        let _ = put_json(
            &sock,
            "/foo",
            &serde_json::json!({}),
            Duration::from_secs(2),
        );
        h.join().unwrap();
        std::fs::remove_file(sock).ok();
    }

    #[test]
    fn wait_for_socket_times_out_when_missing() {
        let missing = std::env::temp_dir().join(format!(
            "vibe_api_client_missing_{}",
            rand_suffix()
        ));
        let err = wait_for_socket(&missing, Duration::from_millis(150)).unwrap_err();
        assert!(matches!(err, ApiClientError::SocketWaitTimeout { .. }));
    }

    // ── ApiResponse.is_success ───────────────────────────────────────────

    #[test]
    fn is_success_only_for_2xx() {
        assert!(ApiResponse {
            status: 200,
            body: None
        }
        .is_success());
        assert!(ApiResponse {
            status: 204,
            body: None
        }
        .is_success());
        assert!(ApiResponse {
            status: 299,
            body: None
        }
        .is_success());
        assert!(!ApiResponse {
            status: 199,
            body: None
        }
        .is_success());
        assert!(!ApiResponse {
            status: 300,
            body: None
        }
        .is_success());
        assert!(!ApiResponse {
            status: 400,
            body: None
        }
        .is_success());
        assert!(!ApiResponse {
            status: 500,
            body: None
        }
        .is_success());
    }
}
