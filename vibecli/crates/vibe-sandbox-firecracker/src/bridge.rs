//! vsock ↔ broker bridge configuration.
//!
//! Slice F4.1 — the structural piece of F4 (egress over virtio-vsock
//! to the daemon-side `vibe-broker`). The runtime piece (the
//! vsock-listener thread on the broker side + the in-guest proxy
//! shim) is F4.2, Linux-only.
//!
//! ## How the bridge works
//!
//! 1. **Daemon** boots a microVM and gives Firecracker a `/vsock`
//!    config pointing at a per-sandbox UDS path on the host (e.g.
//!    `/run/vibe/<sandbox-id>/vsock.sock`).
//! 2. **Broker** binds a listener on that UDS (the broker's existing
//!    UDS-accept code path already handles this — see
//!    [`vibe-broker::accept::Broker::start_uds`]).
//! 3. **In-guest init** reads the kernel cmdline + the env passed
//!    via vsock CID-0 handshake, then exports
//!    `HTTP_PROXY=http://<some-host>:<port>` so any `curl`, `npm`,
//!    `pip`, etc. inside the VM routes through the broker — which
//!    sees plaintext (after MITM), applies the per-sandbox policy,
//!    and emits an audit-log entry per request.
//! 4. **Audit** entries are tagged with the sandbox id from the
//!    handshake, so the recap pipeline can correlate broker logs
//!    with the agent's `Task` that spawned the VM.
//!
//! ## What this module ships
//!
//! Pure data — no syscalls, no I/O. Three types:
//!
//! * [`BridgeConfig`] — what the daemon constructs and hands to both
//!   the Firecracker config layer (for `/vsock`) AND the broker (for
//!   the listener registration). One struct keeps both sides in sync.
//! * [`PolicyHandshake`] — the JSON the daemon POSTs to the broker's
//!   control channel: "set up a bridge for sandbox X with policy Y."
//!   Broker replies with a [`BridgeAttachResponse`].
//! * Helpers for [`kernel_env_vars`] — `HTTP_PROXY` / `HTTPS_PROXY`
//!   / `NO_PROXY` env shape the in-guest init script consumes.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::api::Vsock;

/// One bridge between a sandbox's virtio-vsock device and the
/// broker's UDS listener.
///
/// Held by the daemon for the lifetime of the sandbox; copied into
/// Firecracker's `/vsock` config (via [`as_firecracker_vsock`]) and
/// into the broker's bridge registration ([`as_policy_handshake`]).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BridgeConfig {
    /// Opaque sandbox identifier — used by the broker to tag audit
    /// log entries and by the recap pipeline to correlate.
    pub sandbox_id: String,
    /// Policy identifier (a key into the broker's loaded policy
    /// catalog — e.g. `"default"`, `"npm-only"`, etc.).
    pub policy_id: String,
    /// vsock CID inside the guest. Defaults to 3 (CID 2 is reserved
    /// for the host).
    pub guest_cid: u32,
    /// UDS path on the host where the broker listens. Firecracker's
    /// `/vsock` config points at this path.
    pub host_uds_path: String,
    /// HTTP proxy URL the in-guest agent should see (e.g.
    /// `http://169.254.0.2:8888`). The local kernel routes this
    /// endpoint through vsock to the broker's listener.
    pub guest_proxy_url: String,
    /// Hosts the guest should *not* proxy — typically localhost +
    /// the metadata-IP for the in-VM IMDS faker. Stored as a
    /// comma-joined list so it maps directly to `NO_PROXY`.
    #[serde(default)]
    pub no_proxy: Vec<String>,
}

impl BridgeConfig {
    /// Convert to the Firecracker `/vsock` API struct (F2.1).
    pub fn as_firecracker_vsock(&self) -> Vsock {
        Vsock {
            guest_cid: self.guest_cid,
            uds_path: self.host_uds_path.clone(),
        }
    }

    /// Convert to the policy-handshake the daemon sends to the broker.
    pub fn as_policy_handshake(&self) -> PolicyHandshake {
        PolicyHandshake {
            sandbox_id: self.sandbox_id.clone(),
            policy_id: self.policy_id.clone(),
            uds_path: self.host_uds_path.clone(),
        }
    }

    /// Env vars the in-guest init script injects so HTTP clients
    /// auto-route via the broker. Returns a sorted map so the
    /// in-guest /proc/cmdline scrape (or `/etc/profile.d` write)
    /// produces a stable ordering — easier to diff in audit logs.
    pub fn kernel_env_vars(&self) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("HTTP_PROXY".into(), self.guest_proxy_url.clone());
        m.insert("HTTPS_PROXY".into(), self.guest_proxy_url.clone());
        // Some clients only look at the lowercase form (urllib / curl
        // in some build modes); inject both.
        m.insert("http_proxy".into(), self.guest_proxy_url.clone());
        m.insert("https_proxy".into(), self.guest_proxy_url.clone());
        if !self.no_proxy.is_empty() {
            let joined = self.no_proxy.join(",");
            m.insert("NO_PROXY".into(), joined.clone());
            m.insert("no_proxy".into(), joined);
        }
        m
    }

    /// kernel-cmdline fragment for the in-guest init to pick up.
    /// Same key=value pattern as virtiofs.
    pub fn kernel_cmdline_fragment(&self) -> String {
        format!(
            "vibe.bridge={}:{}:{}",
            self.sandbox_id, self.policy_id, self.guest_proxy_url
        )
    }
}

/// Daemon → broker control message: "attach a bridge for sandbox X
/// to policy Y on UDS Z."
///
/// The broker replies with [`BridgeAttachResponse`] once the listener
/// is bound. F4.2's broker-side handler consumes this; the wire
/// format is JSON over the broker's existing daemon control channel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyHandshake {
    pub sandbox_id: String,
    pub policy_id: String,
    pub uds_path: String,
}

/// Broker → daemon response after a [`PolicyHandshake`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BridgeAttachResponse {
    /// Listener bound, ready for the microVM to boot.
    Ready {
        /// Echoed for correlation.
        sandbox_id: String,
        /// The same UDS path the daemon requested; included so the
        /// daemon can sanity-check.
        uds_path: String,
    },
    /// Broker refused — policy not loaded, UDS not creatable, etc.
    Refused { sandbox_id: String, reason: String },
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cfg() -> BridgeConfig {
        BridgeConfig {
            sandbox_id: "agent-42-task-7".into(),
            policy_id: "default".into(),
            guest_cid: 3,
            host_uds_path: "/run/vibe/agent-42-task-7/vsock.sock".into(),
            guest_proxy_url: "http://169.254.0.2:8888".into(),
            no_proxy: vec!["localhost".into(), "169.254.0.1".into()],
        }
    }

    // ── BridgeConfig → Vsock round-trip ──────────────────────────────────

    #[test]
    fn as_firecracker_vsock_preserves_cid_and_path() {
        let c = cfg();
        let v = c.as_firecracker_vsock();
        assert_eq!(v.guest_cid, 3);
        assert_eq!(v.uds_path, "/run/vibe/agent-42-task-7/vsock.sock");
    }

    #[test]
    fn as_policy_handshake_includes_sandbox_policy_uds() {
        let c = cfg();
        let h = c.as_policy_handshake();
        assert_eq!(h.sandbox_id, "agent-42-task-7");
        assert_eq!(h.policy_id, "default");
        assert_eq!(h.uds_path, "/run/vibe/agent-42-task-7/vsock.sock");
    }

    // ── kernel_env_vars ──────────────────────────────────────────────────

    #[test]
    fn kernel_env_vars_emits_upper_and_lowercase_proxy() {
        let c = cfg();
        let e = c.kernel_env_vars();
        assert_eq!(e["HTTP_PROXY"], "http://169.254.0.2:8888");
        assert_eq!(e["HTTPS_PROXY"], "http://169.254.0.2:8888");
        assert_eq!(e["http_proxy"], "http://169.254.0.2:8888");
        assert_eq!(e["https_proxy"], "http://169.254.0.2:8888");
    }

    #[test]
    fn kernel_env_vars_no_proxy_joins_comma() {
        let c = cfg();
        let e = c.kernel_env_vars();
        assert_eq!(e["NO_PROXY"], "localhost,169.254.0.1");
        assert_eq!(e["no_proxy"], "localhost,169.254.0.1");
    }

    #[test]
    fn kernel_env_vars_omits_no_proxy_when_empty() {
        let mut c = cfg();
        c.no_proxy.clear();
        let e = c.kernel_env_vars();
        assert!(!e.contains_key("NO_PROXY"));
        assert!(!e.contains_key("no_proxy"));
        // HTTP_PROXY etc. still present.
        assert!(e.contains_key("HTTP_PROXY"));
    }

    #[test]
    fn kernel_env_vars_btreemap_iter_is_sorted() {
        let c = cfg();
        let e = c.kernel_env_vars();
        let keys: Vec<&String> = e.keys().collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted, "BTreeMap iteration must yield sorted keys");
    }

    // ── kernel_cmdline_fragment ──────────────────────────────────────────

    #[test]
    fn cmdline_fragment_has_structured_token() {
        let c = cfg();
        assert_eq!(
            c.kernel_cmdline_fragment(),
            "vibe.bridge=agent-42-task-7:default:http://169.254.0.2:8888"
        );
    }

    // ── serde round-trip ─────────────────────────────────────────────────

    #[test]
    fn bridge_config_round_trip() {
        let c = cfg();
        let s = serde_json::to_string(&c).unwrap();
        let back: BridgeConfig = serde_json::from_str(&s).unwrap();
        assert_eq!(c, back);
    }

    #[test]
    fn policy_handshake_json_shape_is_stable() {
        let h = PolicyHandshake {
            sandbox_id: "sb1".into(),
            policy_id: "default".into(),
            uds_path: "/run/vibe/sb1.sock".into(),
        };
        let v = serde_json::to_value(&h).unwrap();
        assert_eq!(
            v,
            json!({
                "sandbox_id": "sb1",
                "policy_id": "default",
                "uds_path": "/run/vibe/sb1.sock",
            })
        );
    }

    // ── BridgeAttachResponse — tagged enum ───────────────────────────────

    #[test]
    fn attach_response_ready_serializes() {
        let r = BridgeAttachResponse::Ready {
            sandbox_id: "sb1".into(),
            uds_path: "/run/vibe/sb1.sock".into(),
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["status"], "ready");
        assert_eq!(v["sandbox_id"], "sb1");
        assert_eq!(v["uds_path"], "/run/vibe/sb1.sock");
    }

    #[test]
    fn attach_response_refused_serializes_with_reason() {
        let r = BridgeAttachResponse::Refused {
            sandbox_id: "sb1".into(),
            reason: "policy 'default' not loaded".into(),
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["status"], "refused");
        assert_eq!(v["reason"], "policy 'default' not loaded");
    }

    #[test]
    fn attach_response_round_trip_through_serde() {
        let r = BridgeAttachResponse::Ready {
            sandbox_id: "x".into(),
            uds_path: "/y".into(),
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: BridgeAttachResponse = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }

    // ── Default no_proxy field is empty when missing in JSON ─────────────

    #[test]
    fn bridge_config_no_proxy_defaults_to_empty() {
        let s = r#"{
            "sandbox_id": "sb1",
            "policy_id": "default",
            "guest_cid": 3,
            "host_uds_path": "/run/vibe/sb1.sock",
            "guest_proxy_url": "http://169.254.0.2:8888"
        }"#;
        let c: BridgeConfig = serde_json::from_str(s).unwrap();
        assert!(c.no_proxy.is_empty());
    }
}
