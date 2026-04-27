//! IMDSv2 faker (slice B3).
//!
//! AWS SDKs walk a credential chain that ends at the EC2 instance
//! metadata service at `169.254.169.254`. When the sandbox runs in a
//! brokered environment with no env-var creds and no `~/.aws/credentials`,
//! the SDK times out on IMDS and surfaces "could not load credentials".
//! This faker fills that hole: the broker stands up a tiny IMDSv2 server
//! that answers the SDK with role-shaped credentials derived from the
//! same `SecretStore` the broker uses for SigV4 injection.
//!
//! v1 implements IMDSv2 (the modern path AWS SDKs default to). Operators
//! alias 169.254.169.254 to the chosen loopback in production; tests
//! bind to `127.0.0.1:0` and drive the dance directly.
//!
//! Endpoints supported:
//!   PUT  /latest/api/token                                                      → 200 token
//!   GET  /latest/meta-data/iam/security-credentials/                            → 200 role name
//!   GET  /latest/meta-data/iam/security-credentials/<role>                       → 200 creds JSON

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

use crate::audit::{AuditSink, EgressOutcome, NullAuditSink, baseline_egress_request};
use crate::policy::SecretRef;
use crate::secrets::SecretStore;

/// IMDSv2 server config. The role name surfaces in the role-list endpoint
/// and is what AWS SDKs use to build the cred-fetch URL. The `secret_ref`
/// is what the server resolves against the SecretStore on each cred-fetch
/// call so token rotation in the daemon is picked up immediately.
#[derive(Clone)]
pub struct ImdsServer {
    pub role_name: String,
    pub secret_ref: SecretRef,
    pub secrets: Arc<dyn SecretStore>,
    /// Synthetic IMDSv2 token we hand out on PUT /latest/api/token.
    /// Requests without a matching `X-aws-ec2-metadata-token` header get
    /// 401 (matches IMDSv2 enforcement).
    pub session_token: String,
    /// Sink for structured audit events. Defaults to NullAuditSink.
    pub audit: Arc<dyn AuditSink>,
}

pub struct ImdsHandle {
    pub addr: std::net::SocketAddr,
    pub join: JoinHandle<()>,
}

impl ImdsHandle {
    pub fn abort(&self) {
        self.join.abort();
    }
}

impl ImdsServer {
    pub fn new(
        role_name: impl Into<String>,
        secret_ref: SecretRef,
        secrets: Arc<dyn SecretStore>,
    ) -> Self {
        ImdsServer {
            role_name: role_name.into(),
            secret_ref,
            secrets,
            session_token: random_token(),
            audit: Arc::new(NullAuditSink),
        }
    }

    pub fn with_audit_sink(mut self, sink: Arc<dyn AuditSink>) -> Self {
        self.audit = sink;
        self
    }

    /// Bind on `addr` and start serving. Tests pass `127.0.0.1:0`; ops
    /// pass `169.254.169.254:80` (after creating the loopback alias).
    pub async fn start(self, addr: &str) -> std::io::Result<ImdsHandle> {
        let listener = TcpListener::bind(addr).await?;
        let bound = listener.local_addr()?;
        let join = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let s = self.clone();
                        tokio::spawn(async move {
                            if let Err(e) = s.handle(stream).await {
                                tracing::debug!("imds connection error: {e}");
                            }
                        });
                    }
                    Err(e) => {
                        tracing::warn!("imds accept failed: {e}");
                        break;
                    }
                }
            }
        });
        Ok(ImdsHandle { addr: bound, join })
    }

    async fn handle(&self, mut stream: TcpStream) -> std::io::Result<()> {
        let mut buf = vec![0u8; 8 * 1024];
        let n = read_until_headers(&mut stream, &mut buf).await?;
        let raw = &buf[..n];

        let parsed = match parse_request(raw) {
            Some(p) => p,
            None => {
                let mut event =
                    baseline_egress_request("imds", "broker:imds", "?", "169.254.169.254", "?");
                event.outcome = EgressOutcome::UpstreamError;
                event.status = Some(400);
                self.audit.record(event);
                return write_response(&mut stream, 400, "Bad Request", "text/plain", b"")
                    .await;
            }
        };

        let mut event = baseline_egress_request(
            "imds",
            "broker:imds",
            &parsed.method,
            "169.254.169.254",
            &parsed.path,
        );
        event.inject = "Imds".into();

        // PUT /latest/api/token: hand out the synthetic token.
        if parsed.method.eq_ignore_ascii_case("PUT")
            && parsed.path == "/latest/api/token"
        {
            event.outcome = EgressOutcome::Ok;
            event.status = Some(200);
            self.audit.record(event);
            return write_response(
                &mut stream,
                200,
                "OK",
                "text/plain",
                self.session_token.as_bytes(),
            )
            .await;
        }

        // Everything else requires a matching session token.
        let presented_token = parsed
            .headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("X-aws-ec2-metadata-token"))
            .map(|(_, v)| v.as_str());
        if presented_token != Some(self.session_token.as_str()) {
            event.outcome = EgressOutcome::PolicyDenied;
            event.status = Some(401);
            self.audit.record(event);
            return write_response(&mut stream, 401, "Unauthorized", "text/plain", b"")
                .await;
        }

        if parsed.method.eq_ignore_ascii_case("GET") {
            if parsed.path == "/latest/meta-data/iam/security-credentials/"
                || parsed.path == "/latest/meta-data/iam/security-credentials"
            {
                event.outcome = EgressOutcome::Ok;
                event.status = Some(200);
                self.audit.record(event);
                return write_response(
                    &mut stream,
                    200,
                    "OK",
                    "text/plain",
                    self.role_name.as_bytes(),
                )
                .await;
            }
            let prefix = "/latest/meta-data/iam/security-credentials/";
            if let Some(role) = parsed.path.strip_prefix(prefix) {
                if role == self.role_name {
                    event.outcome = EgressOutcome::Ok;
                    event.status = Some(200);
                    self.audit.record(event);
                    return self.serve_creds(&mut stream).await;
                }
                event.outcome = EgressOutcome::UpstreamError;
                event.status = Some(404);
                self.audit.record(event);
                return write_response(&mut stream, 404, "Not Found", "text/plain", b"")
                    .await;
            }
        }

        event.outcome = EgressOutcome::UpstreamError;
        event.status = Some(404);
        self.audit.record(event);
        write_response(&mut stream, 404, "Not Found", "text/plain", b"").await
    }

    async fn serve_creds(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        let creds = match self.secrets.resolve_aws(&self.secret_ref) {
            Some(c) => c,
            None => {
                return write_response(
                    &mut *stream,
                    500,
                    "Internal Server Error",
                    "text/plain",
                    b"creds_unresolved",
                )
                .await;
            }
        };

        // 15-minute window — the daemon refreshes upstream creds before
        // expiration; SDKs cache up to this point.
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
        let exp = now + Duration::from_secs(15 * 60);
        let last_updated = format_iso8601(now);
        let expiration = format_iso8601(exp);

        let mut json = String::new();
        json.push('{');
        push_json_pair(&mut json, "Code", "Success", false);
        push_json_pair(&mut json, "LastUpdated", &last_updated, true);
        push_json_pair(&mut json, "Type", "AWS-HMAC", true);
        push_json_pair(&mut json, "AccessKeyId", &creds.access_key_id, true);
        push_json_pair(&mut json, "SecretAccessKey", &creds.secret_access_key, true);
        match &creds.session_token {
            Some(t) => push_json_pair(&mut json, "Token", t, true),
            None => push_json_pair(&mut json, "Token", "", true),
        }
        push_json_pair(&mut json, "Expiration", &expiration, true);
        json.push('}');

        write_response(&mut *stream, 200, "OK", "application/json", json.as_bytes()).await
    }
}

fn push_json_pair(out: &mut String, key: &str, value: &str, leading_comma: bool) {
    if leading_comma {
        out.push(',');
    }
    out.push('"');
    out.push_str(key);
    out.push_str("\":\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn format_iso8601(d: Duration) -> String {
    // Build YYYY-MM-DDTHH:MM:SSZ from a Unix duration without pulling
    // chrono. Uses a small civil-from-days conversion (Howard Hinnant).
    let secs = d.as_secs() as i64;
    let days = secs.div_euclid(86_400);
    let secs_of_day = secs.rem_euclid(86_400) as u32;
    let hh = secs_of_day / 3600;
    let mm = (secs_of_day / 60) % 60;
    let ss = secs_of_day % 60;
    let (y, mo, day) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{day:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    // Howard Hinnant's "days_from_civil" inverse.
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[derive(Debug)]
struct Parsed {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
}

fn parse_request(buf: &[u8]) -> Option<Parsed> {
    let text = std::str::from_utf8(buf).ok()?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
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
    Some(Parsed { method, path, headers })
}

async fn read_until_headers(stream: &mut TcpStream, buf: &mut [u8]) -> std::io::Result<usize> {
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

async fn write_response(
    stream: &mut TcpStream,
    code: u16,
    phrase: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {code} {phrase}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await
}

fn random_token() -> String {
    // We don't need cryptographic randomness here — the IMDS token is
    // proof-of-control, not a secret. A hash of process id + a timestamp
    // gives us enough entropy to discriminate retried clients.
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(std::process::id().to_le_bytes());
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    hasher.update(ns.to_le_bytes());
    hex::encode(hasher.finalize())[..56].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple() {
        let raw = b"GET /latest/meta-data/iam/security-credentials/ HTTP/1.1\r\nHost: 169.254.169.254\r\n\r\n";
        let p = parse_request(raw).unwrap();
        assert_eq!(p.method, "GET");
        assert_eq!(p.path, "/latest/meta-data/iam/security-credentials/");
    }

    #[test]
    fn iso8601_known_vector() {
        // Unix epoch = 1970-01-01T00:00:00Z.
        assert_eq!(format_iso8601(Duration::from_secs(0)), "1970-01-01T00:00:00Z");
        // 86_400 seconds = exactly 1 day.
        assert_eq!(format_iso8601(Duration::from_secs(86_400)), "1970-01-02T00:00:00Z");
        // 2000-03-01T00:00:00Z — exercises the leap-year branch around
        // the Y2K boundary in civil_from_days.
        assert_eq!(
            format_iso8601(Duration::from_secs(951_868_800)),
            "2000-03-01T00:00:00Z"
        );
    }

    #[test]
    fn json_pair_escapes_quote() {
        let mut s = String::new();
        push_json_pair(&mut s, "k", "a\"b", false);
        assert_eq!(s, r#""k":"a\"b""#);
    }

    #[test]
    fn random_token_has_expected_length() {
        let t = random_token();
        assert_eq!(t.len(), 56);
    }
}
