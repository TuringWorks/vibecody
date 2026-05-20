//! Host function ABI for the Hyperlight WASM guest.
//!
//! Slices H2 + H3 — defines the wire shape between the in-VM WASI
//! extension and the daemon-side host functions it calls into:
//!
//! * **H2: FS host functions** — `open`, `read`, `write`, `close`,
//!   `readdir`, `stat` bound to the extension's declared `bind_rw` /
//!   `bind_ro` paths. The Hyperlight partition has no FS of its own;
//!   every path operation crosses into the host via these stubs.
//!
//! * **H3: broker host function** — `vibe_egress_request` shaped after
//!   HTTP CONNECT + a small JSON envelope so guests can issue
//!   `https://example.com/v1/foo` requests that route through the
//!   daemon's broker with the per-sandbox policy applied.
//!
//! ## Why this is a separate slice
//!
//! The Hyperlight runtime binding (H1) is gated on the upstream
//! Wasmtime-on-Hyperlight release pipeline + a Linux+KVM CI runner.
//! That's months out. But the *ABI* — what shapes do guests send,
//! what shapes do hosts return — can be locked down today, with
//! cross-platform unit tests. Once H1 lands, the binding is a thin
//! wrapper around these structs.
//!
//! ## Wire format
//!
//! serde-derived JSON. The host registers each function with a
//! string name (`fs.open`, `fs.read`, …, `egress.request`) and the
//! guest passes a JSON-serialized request struct; the host returns
//! a JSON-serialized response. Each `*Response` is a tagged enum so
//! a successful read and a `Permission denied` errno share the same
//! call surface.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── H2: FS host function ABI ────────────────────────────────────────────────

/// FS host function request — sent by the guest to the host.
///
/// File descriptors (`fd`) are guest-side u32 handles minted by the
/// host on `Open` success. They are *not* host kernel fds; the host
/// maintains a per-extension fd table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum FsRequest {
    Open {
        path: PathBuf,
        #[serde(default)]
        flags: FsOpenFlags,
    },
    Read {
        fd: u32,
        len: u32,
    },
    Write {
        fd: u32,
        #[serde(with = "base64_bytes")]
        data: Vec<u8>,
    },
    Close {
        fd: u32,
    },
    ReadDir {
        path: PathBuf,
    },
    Stat {
        path: PathBuf,
    },
}

/// Per-request flags for `open`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsOpenFlags {
    /// Open in read mode. Default true.
    #[serde(default = "default_true")]
    pub read: bool,
    /// Open in write mode. Default false.
    #[serde(default)]
    pub write: bool,
    /// Create the file if missing. Default false. Requires write.
    #[serde(default)]
    pub create: bool,
    /// Truncate on open. Default false. Requires write.
    #[serde(default)]
    pub truncate: bool,
}

fn default_true() -> bool {
    true
}

impl Default for FsOpenFlags {
    /// Read-only — matches the serde `#[default = "default_true"]`
    /// on the `read` field so a hand-built `FsOpenFlags::default()`
    /// and a JSON `{}` deserialize to the same value.
    fn default() -> Self {
        Self {
            read: true,
            write: false,
            create: false,
            truncate: false,
        }
    }
}

impl FsOpenFlags {
    /// Reject combinations that would silently misbehave (e.g.
    /// `truncate` without `write`). Called by the host before
    /// translating into a real `OpenOptions`.
    pub fn validate(&self) -> Result<(), HostAbiError> {
        if (self.create || self.truncate) && !self.write {
            return Err(HostAbiError::InvalidFlags(
                "create/truncate require write=true",
            ));
        }
        if !self.read && !self.write {
            return Err(HostAbiError::InvalidFlags(
                "open must request read or write (or both)",
            ));
        }
        Ok(())
    }
}

/// FS host function response — returned by the host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum FsResponse {
    /// `open` succeeded — caller holds the minted `fd`.
    Opened { fd: u32 },
    /// `read` returned `data`. Empty `data` = EOF.
    Read {
        #[serde(with = "base64_bytes")]
        data: Vec<u8>,
    },
    /// `write` accepted `bytes_written` of the input.
    Wrote { bytes_written: u32 },
    /// `close`, `readdir`-no-data, void success.
    Ok,
    /// `readdir` entries.
    DirEntries { entries: Vec<DirEntry> },
    /// `stat` metadata.
    StatResult { stat: FileStat },
    /// Any failure — errno style.
    Err { errno: i32, message: String },
}

/// One entry in a `readdir` response. Mirrors POSIX `struct dirent`
/// but kept minimal (no inode, no offset).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub kind: FileKind,
}

/// File metadata. Just what an in-VM agent typically needs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileStat {
    pub size: u64,
    pub kind: FileKind,
    pub readonly: bool,
    /// Modification time as a Unix epoch (seconds). The microVM
    /// clock may be skewed; consumers should treat this as
    /// approximate.
    pub mtime_unix_secs: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    File,
    Dir,
    Symlink,
    Other,
}

// ── H3: Egress host function ABI ────────────────────────────────────────────

/// Egress request — the guest calls into the host's broker bridge.
///
/// Bodies use base64 because JSON can't hold raw bytes (encoding /
/// decoding cost is negligible relative to the actual network
/// round-trip).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EgressRequest {
    /// HTTP method (`GET`, `POST`, `PUT`, `DELETE`, …).
    pub method: String,
    /// Full URL including scheme. The broker applies the per-sandbox
    /// policy to host + path before forwarding.
    pub url: String,
    /// Header list. Ordering preserved.
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    /// Request body. Empty = no body.
    #[serde(default, with = "base64_bytes")]
    pub body: Vec<u8>,
    /// Hard ceiling on how long this single request may take, in
    /// milliseconds. Caps a misbehaving target host from wedging
    /// the extension call.
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u32,
}

fn default_timeout_ms() -> u32 {
    30_000
}

impl EgressRequest {
    /// Reject obvious malformed shapes before serialization to the
    /// broker. The broker has its own deny-list (per-policy host
    /// match, scheme allow-list, etc.) — this just catches the
    /// crashy stuff.
    pub fn validate(&self) -> Result<(), HostAbiError> {
        if self.method.is_empty() {
            return Err(HostAbiError::InvalidEgress("method is empty"));
        }
        if !self
            .method
            .chars()
            .all(|c| c.is_ascii_uppercase() || c == '-')
        {
            // RFC 9110: methods are case-sensitive tokens; reject
            // anything that isn't the canonical UPPERCASE form so
            // policy matchers don't have to lowercase-normalize.
            return Err(HostAbiError::InvalidEgress(
                "method must be uppercase ASCII",
            ));
        }
        if self.url.is_empty() {
            return Err(HostAbiError::InvalidEgress("url is empty"));
        }
        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err(HostAbiError::InvalidEgress(
                "url scheme must be http or https",
            ));
        }
        if self.timeout_ms == 0 {
            return Err(HostAbiError::InvalidEgress(
                "timeout_ms must be > 0",
            ));
        }
        if self.timeout_ms > 5 * 60 * 1000 {
            return Err(HostAbiError::InvalidEgress(
                "timeout_ms must be ≤ 5 minutes",
            ));
        }
        Ok(())
    }
}

/// Egress response — what the broker returns to the guest after
/// applying policy + forwarding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum EgressResponse {
    Ok {
        status: u16,
        #[serde(default)]
        headers: Vec<(String, String)>,
        #[serde(with = "base64_bytes")]
        body: Vec<u8>,
    },
    /// Broker denied the request before sending — policy mismatch,
    /// SSRF guard, etc. The guest can't retry; this is final.
    Denied { reason: String },
    /// Network-level failure (DNS, TCP, TLS). Distinguished from
    /// `Denied` because it's potentially retryable.
    NetworkError { reason: String },
}

// ── Host function names ──────────────────────────────────────────────────────

/// Function names the host registers + the guest imports. Single
/// source of truth so a typo here breaks the test, not a runtime
/// `function not found` two slices later.
pub mod fn_names {
    pub const FS_OPEN: &str = "fs.open";
    pub const FS_READ: &str = "fs.read";
    pub const FS_WRITE: &str = "fs.write";
    pub const FS_CLOSE: &str = "fs.close";
    pub const FS_READDIR: &str = "fs.readdir";
    pub const FS_STAT: &str = "fs.stat";
    pub const EGRESS_REQUEST: &str = "egress.request";

    pub const ALL: &[&str] = &[
        FS_OPEN,
        FS_READ,
        FS_WRITE,
        FS_CLOSE,
        FS_READDIR,
        FS_STAT,
        EGRESS_REQUEST,
    ];
}

// ── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HostAbiError {
    #[error("invalid open flags: {0}")]
    InvalidFlags(&'static str),

    #[error("invalid egress request: {0}")]
    InvalidEgress(&'static str),
}

// ── base64 helper module for serde ──────────────────────────────────────────

mod base64_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        b64_encode(bytes).serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(d)?;
        b64_decode(&s).map_err(serde::de::Error::custom)
    }

    // Tiny in-tree RFC 4648 base64 encoder/decoder — same rationale
    // as the in-tree SHA-256 in `rootfs.rs`: avoids pulling a crate
    // for a few hundred lines of operations on a leaf crate.
    const ALPHA: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    fn b64_encode(input: &[u8]) -> String {
        let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
        let mut chunks = input.chunks_exact(3);
        for c in chunks.by_ref() {
            let b = ((c[0] as u32) << 16) | ((c[1] as u32) << 8) | (c[2] as u32);
            out.push(ALPHA[((b >> 18) & 0x3F) as usize] as char);
            out.push(ALPHA[((b >> 12) & 0x3F) as usize] as char);
            out.push(ALPHA[((b >> 6) & 0x3F) as usize] as char);
            out.push(ALPHA[(b & 0x3F) as usize] as char);
        }
        let rem = chunks.remainder();
        match rem.len() {
            0 => {}
            1 => {
                let b = (rem[0] as u32) << 16;
                out.push(ALPHA[((b >> 18) & 0x3F) as usize] as char);
                out.push(ALPHA[((b >> 12) & 0x3F) as usize] as char);
                out.push('=');
                out.push('=');
            }
            2 => {
                let b = ((rem[0] as u32) << 16) | ((rem[1] as u32) << 8);
                out.push(ALPHA[((b >> 18) & 0x3F) as usize] as char);
                out.push(ALPHA[((b >> 12) & 0x3F) as usize] as char);
                out.push(ALPHA[((b >> 6) & 0x3F) as usize] as char);
                out.push('=');
            }
            _ => unreachable!(),
        }
        out
    }

    fn b64_decode(input: &str) -> Result<Vec<u8>, String> {
        let bytes = input.as_bytes();
        if bytes.is_empty() {
            return Ok(Vec::new());
        }
        if bytes.len() % 4 != 0 {
            return Err(format!("base64 length not multiple of 4: {}", bytes.len()));
        }
        let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
        let mut buf = [0u8; 4];
        for chunk in bytes.chunks_exact(4) {
            let mut pad = 0;
            for (i, &c) in chunk.iter().enumerate() {
                buf[i] = match c {
                    b'A'..=b'Z' => c - b'A',
                    b'a'..=b'z' => c - b'a' + 26,
                    b'0'..=b'9' => c - b'0' + 52,
                    b'+' => 62,
                    b'/' => 63,
                    b'=' => {
                        pad += 1;
                        0
                    }
                    _ => return Err(format!("invalid base64 character: {:?}", c as char)),
                };
            }
            let v =
                ((buf[0] as u32) << 18) | ((buf[1] as u32) << 12) | ((buf[2] as u32) << 6) | (buf[3] as u32);
            out.push(((v >> 16) & 0xFF) as u8);
            if pad < 2 {
                out.push(((v >> 8) & 0xFF) as u8);
            }
            if pad < 1 {
                out.push((v & 0xFF) as u8);
            }
        }
        Ok(out)
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── FsOpenFlags ──────────────────────────────────────────────────────

    #[test]
    fn open_flags_default_is_read_only() {
        let f = FsOpenFlags::default();
        assert!(f.read);
        assert!(!f.write);
        assert!(!f.create);
        assert!(!f.truncate);
        assert!(f.validate().is_ok());
    }

    #[test]
    fn open_flags_truncate_without_write_rejected() {
        let f = FsOpenFlags {
            read: true,
            write: false,
            create: false,
            truncate: true,
        };
        assert!(matches!(f.validate(), Err(HostAbiError::InvalidFlags(_))));
    }

    #[test]
    fn open_flags_neither_read_nor_write_rejected() {
        let f = FsOpenFlags {
            read: false,
            write: false,
            create: false,
            truncate: false,
        };
        assert!(matches!(f.validate(), Err(HostAbiError::InvalidFlags(_))));
    }

    #[test]
    fn open_flags_rw_create_truncate_accepted() {
        let f = FsOpenFlags {
            read: true,
            write: true,
            create: true,
            truncate: true,
        };
        assert!(f.validate().is_ok());
    }

    // ── FsRequest tagged-enum encoding ───────────────────────────────────

    #[test]
    fn fs_request_open_tags_op_field() {
        let r = FsRequest::Open {
            path: PathBuf::from("/work/foo.txt"),
            flags: FsOpenFlags::default(),
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["op"], "open");
        assert_eq!(v["path"], "/work/foo.txt");
    }

    #[test]
    fn fs_request_read_serializes_fd_and_len() {
        let r = FsRequest::Read { fd: 7, len: 4096 };
        assert_eq!(
            serde_json::to_value(&r).unwrap(),
            json!({"op": "read", "fd": 7, "len": 4096})
        );
    }

    #[test]
    fn fs_request_write_base64_encodes_body() {
        let r = FsRequest::Write {
            fd: 7,
            data: b"hello".to_vec(),
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["op"], "write");
        assert_eq!(v["fd"], 7);
        // "hello" → base64 "aGVsbG8="
        assert_eq!(v["data"], "aGVsbG8=");
    }

    #[test]
    fn fs_request_round_trip() {
        let r = FsRequest::Write {
            fd: 42,
            data: vec![0, 1, 2, 3, 0xFF, 0xFE],
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: FsRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn fs_request_readdir_serializes_path() {
        let r = FsRequest::ReadDir {
            path: PathBuf::from("/work"),
        };
        let v = serde_json::to_value(&r).unwrap();
        // serde rename_all = "snake_case" → ReadDir becomes read_dir.
        // The fn name constant `FS_READDIR` is "fs.readdir" (no
        // underscore, terser); they're independent — the op tag is
        // for the enum-discriminator in the JSON, the fn name is
        // for the Hyperlight function-registry lookup. Keep both
        // pinned so a future renamer touches both sites consciously.
        assert_eq!(v["op"], "read_dir");
        assert_eq!(v["path"], "/work");
    }

    // ── FsResponse tagged-enum encoding ──────────────────────────────────

    #[test]
    fn fs_response_opened_tags_result() {
        let r = FsResponse::Opened { fd: 3 };
        assert_eq!(
            serde_json::to_value(&r).unwrap(),
            json!({"result": "opened", "fd": 3})
        );
    }

    #[test]
    fn fs_response_err_includes_errno_and_message() {
        let r = FsResponse::Err {
            errno: 13,
            message: "Permission denied".into(),
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["result"], "err");
        assert_eq!(v["errno"], 13);
        assert_eq!(v["message"], "Permission denied");
    }

    #[test]
    fn fs_response_dir_entries_serializes() {
        let r = FsResponse::DirEntries {
            entries: vec![
                DirEntry {
                    name: "a.txt".into(),
                    kind: FileKind::File,
                },
                DirEntry {
                    name: "sub".into(),
                    kind: FileKind::Dir,
                },
            ],
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["result"], "dir_entries");
        assert_eq!(v["entries"][0]["name"], "a.txt");
        assert_eq!(v["entries"][0]["kind"], "file");
        assert_eq!(v["entries"][1]["kind"], "dir");
    }

    #[test]
    fn fs_response_stat_round_trip() {
        let r = FsResponse::StatResult {
            stat: FileStat {
                size: 1024,
                kind: FileKind::File,
                readonly: false,
                mtime_unix_secs: 1_716_400_000,
            },
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: FsResponse = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }

    // ── EgressRequest validation ─────────────────────────────────────────

    fn ok_egress() -> EgressRequest {
        EgressRequest {
            method: "GET".into(),
            url: "https://api.example.com/v1/things".into(),
            headers: vec![("Accept".into(), "application/json".into())],
            body: vec![],
            timeout_ms: 5_000,
        }
    }

    #[test]
    fn egress_request_accepts_get_https() {
        assert!(ok_egress().validate().is_ok());
    }

    #[test]
    fn egress_request_rejects_empty_method() {
        let mut r = ok_egress();
        r.method = "".into();
        assert!(matches!(r.validate(), Err(HostAbiError::InvalidEgress(_))));
    }

    #[test]
    fn egress_request_rejects_lowercase_method() {
        let mut r = ok_egress();
        r.method = "get".into();
        assert!(matches!(r.validate(), Err(HostAbiError::InvalidEgress(_))));
    }

    #[test]
    fn egress_request_rejects_non_http_scheme() {
        let mut r = ok_egress();
        r.url = "ftp://files.example.com/".into();
        assert!(matches!(r.validate(), Err(HostAbiError::InvalidEgress(_))));
    }

    #[test]
    fn egress_request_rejects_zero_timeout() {
        let mut r = ok_egress();
        r.timeout_ms = 0;
        assert!(matches!(r.validate(), Err(HostAbiError::InvalidEgress(_))));
    }

    #[test]
    fn egress_request_rejects_timeout_over_5_minutes() {
        let mut r = ok_egress();
        r.timeout_ms = 5 * 60 * 1000 + 1;
        assert!(matches!(r.validate(), Err(HostAbiError::InvalidEgress(_))));
    }

    #[test]
    fn egress_request_default_timeout_is_30s() {
        let r: EgressRequest = serde_json::from_value(json!({
            "method": "GET",
            "url": "https://x.example.com/",
        }))
        .unwrap();
        assert_eq!(r.timeout_ms, 30_000);
        assert!(r.body.is_empty());
        assert!(r.headers.is_empty());
    }

    #[test]
    fn egress_request_body_base64_encoded() {
        let r = EgressRequest {
            method: "POST".into(),
            url: "https://api.example.com/upload".into(),
            headers: vec![],
            body: b"raw\x00bytes".to_vec(),
            timeout_ms: 1000,
        };
        let s = serde_json::to_string(&r).unwrap();
        // "raw\x00bytes" → base64 "cmF3AGJ5dGVz"
        assert!(s.contains("cmF3AGJ5dGVz"));
    }

    // ── EgressResponse tagged-enum encoding ──────────────────────────────

    #[test]
    fn egress_response_ok_round_trip() {
        let r = EgressResponse::Ok {
            status: 200,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: b"{\"x\":1}".to_vec(),
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: EgressResponse = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn egress_response_denied_tag() {
        let r = EgressResponse::Denied {
            reason: "host not in allow list".into(),
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["result"], "denied");
        assert_eq!(v["reason"], "host not in allow list");
    }

    #[test]
    fn egress_response_network_error_distinguishes_from_denied() {
        let r = EgressResponse::NetworkError {
            reason: "dns lookup failed".into(),
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["result"], "network_error");
    }

    // ── fn_names ─────────────────────────────────────────────────────────

    #[test]
    fn fn_names_all_unique() {
        let mut sorted = fn_names::ALL.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), fn_names::ALL.len());
    }

    #[test]
    fn fn_names_match_constants() {
        assert!(fn_names::ALL.contains(&fn_names::FS_OPEN));
        assert!(fn_names::ALL.contains(&fn_names::FS_READ));
        assert!(fn_names::ALL.contains(&fn_names::EGRESS_REQUEST));
        // No fn name has spaces or path separators.
        for n in fn_names::ALL {
            assert!(!n.contains(' '));
            assert!(!n.contains('/'));
            assert!(!n.is_empty());
        }
    }

    // ── base64 internal ─────────────────────────────────────────────────

    #[test]
    fn base64_known_vectors() {
        // RFC 4648 §10.
        let cases: &[(&[u8], &str)] = &[
            (b"", ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg=="),
            (b"fooba", "Zm9vYmE="),
            (b"foobar", "Zm9vYmFy"),
        ];
        for (input, expected) in cases {
            let r = FsRequest::Write {
                fd: 0,
                data: input.to_vec(),
            };
            let v = serde_json::to_value(&r).unwrap();
            assert_eq!(v["data"], *expected, "encode {:?} → {}", input, expected);

            // Round-trip.
            let s = serde_json::to_string(&r).unwrap();
            let back: FsRequest = serde_json::from_str(&s).unwrap();
            assert_eq!(r, back);
        }
    }

    #[test]
    fn base64_rejects_invalid_chars() {
        let s = r#"{"op": "write", "fd": 0, "data": "@@@@"}"#;
        let r: Result<FsRequest, _> = serde_json::from_str(s);
        assert!(r.is_err());
    }
}
