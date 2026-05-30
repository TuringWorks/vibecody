//! Per-skill sandbox policy — composes F2.1 + F3.1 + F4.1 into one
//! coherent shape that skill manifests (or daemon config) reference.
//!
//! Slice F8 — replaces ad-hoc "spawn a microVM with these settings"
//! call sites with a single typed policy that:
//!
//! 1. **Validates** at config-load time, not at boot time (so a typo
//!    in a skill manifest fails the load, not the first sandbox).
//! 2. **Composes** the F-series structural types: `MachineConfig` (F2.1)
//!    + `VirtioFsShare`[] (F3.1) + optional `BridgeConfig` (F4.1).
//! 3. **Resolves** to a Firecracker `VmConfig` once the rootfs path
//!    + kernel path are bound (`into_vm_config`).
//!
//! Wire format: serde-tagged JSON. Skill manifests today are markdown
//! with YAML frontmatter; the YAML key for this policy is `sandbox`,
//! and the loader converts that subtree into JSON before
//! deserializing into [`SkillSandboxPolicy`]. Same struct, two
//! source formats, one validator.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::api::{BootSource, Drive, MachineConfig, VmConfig};
use crate::bridge::BridgeConfig;
use crate::virtiofs::{VirtioFsError, VirtioFsShare};

/// Per-skill sandbox policy.
///
/// All fields are optional with sensible defaults; the minimal valid
/// policy is the empty object `{}`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SkillSandboxPolicy {
    /// CPU + memory override. None → MachineConfig::default()
    /// (1 vCPU, 128 MiB, SMT off).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<MachineConfig>,

    /// virtio-fs shares (workspace + system binds). Validated at
    /// load time against the credential-dir deny-list (`VirtioFsShare::new`).
    #[serde(default)]
    pub shares: Vec<VirtioFsShare>,

    /// Optional vsock broker bridge. None → no network at all from
    /// inside the microVM (NetPolicy::None from the host's POV).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridge: Option<BridgeConfig>,

    /// Extra env vars to inject into the guest (in addition to the
    /// bridge's HTTP_PROXY etc.). Stored in BTreeMap for stable
    /// audit-log ordering.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra_env: BTreeMap<String, String>,

    /// Extra kernel command-line args appended after the boot
    /// args derived from F2.1's `BootSource`. Skill authors can use
    /// this to enable specific kernel features (e.g. `apparmor=0`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_boot_args: Option<String>,

    /// Hard ceiling on how long the microVM may run before the
    /// daemon kills it. `None` = no daemon-side timeout (per-call
    /// time limits still apply at the agent loop level).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_clock_timeout_ms: Option<u32>,
}

/// Errors from policy validation / composition.
#[derive(Debug, Error)]
pub enum SkillPolicyError {
    #[error("share validation failed: {0}")]
    Share(#[from] VirtioFsError),

    #[error("invalid machine config: {field} = {value}: {reason}")]
    InvalidMachine {
        field: &'static str,
        value: String,
        reason: &'static str,
    },

    #[error("invalid env var name (not POSIX): {0}")]
    InvalidEnvName(String),

    #[error("wall_clock_timeout_ms must be > 0 when set (got 0)")]
    ZeroTimeout,

    #[error("kernel cmdline exceeds Firecracker's 4096-char limit: got {0} chars")]
    KernelCmdlineTooLong(usize),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

impl SkillSandboxPolicy {
    /// Parse policy from a JSON string (also the on-disk format
    /// after YAML→JSON conversion).
    pub fn from_json(s: &str) -> Result<Self, SkillPolicyError> {
        let p: Self = serde_json::from_str(s)?;
        p.validate()?;
        Ok(p)
    }

    /// Validate all fields. Called automatically by `from_json`;
    /// callers that build the struct in code should call this
    /// explicitly before consuming.
    pub fn validate(&self) -> Result<(), SkillPolicyError> {
        if let Some(m) = &self.machine {
            validate_machine(m)?;
        }
        // Shares are already validated at VirtioFsShare::new time —
        // but a hand-deserialized share skipped that constructor.
        // Re-validate the host path + tag.
        for s in &self.shares {
            // Calling new() with the share's fields re-runs the deny
            // list. We don't keep the result; only need the side
            // effect of the error if any.
            let _ = VirtioFsShare::new(
                s.host_path.clone(),
                s.mount_tag.clone(),
                s.socket_path.clone(),
                s.read_only,
            )?;
        }
        for k in self.extra_env.keys() {
            validate_env_name(k)?;
        }
        if let Some(0) = self.wall_clock_timeout_ms {
            return Err(SkillPolicyError::ZeroTimeout);
        }
        Ok(())
    }

    /// Compose this policy + a rootfs path + a kernel path into a
    /// fully-resolved Firecracker [`VmConfig`] (F2.1).
    ///
    /// The composition is:
    ///   • machine_config = self.machine OR MachineConfig::default()
    ///   • boot_source = {kernel_path, boot_args = "console=ttyS0 root=/dev/vda" + extra_boot_args}
    ///   • drives = [rootfs from rootfs_path, ro]
    ///   • vsock = self.bridge.as_firecracker_vsock() if set
    pub fn into_vm_config(
        self,
        rootfs_path: impl Into<String>,
        kernel_path: impl Into<String>,
    ) -> Result<VmConfig, SkillPolicyError> {
        self.validate()?;

        let mut boot_args = String::from("console=ttyS0 root=/dev/vda");
        if let Some(extra) = &self.extra_boot_args {
            boot_args.push(' ');
            boot_args.push_str(extra);
        }
        if boot_args.len() > 4096 {
            return Err(SkillPolicyError::KernelCmdlineTooLong(boot_args.len()));
        }

        let machine = self.machine.unwrap_or_default();
        let mut cfg = VmConfig::new()
            .with_machine_config(machine)
            .with_boot_source(BootSource {
                kernel_image_path: kernel_path.into(),
                boot_args: Some(boot_args),
                initrd_path: None,
            })
            .with_drive(Drive {
                drive_id: "rootfs".into(),
                path_on_host: rootfs_path.into(),
                is_root_device: true,
                is_read_only: true,
                cache_type: None,
            });

        if let Some(b) = &self.bridge {
            cfg = cfg.with_vsock(b.as_firecracker_vsock());
        }

        Ok(cfg)
    }
}

fn validate_machine(m: &MachineConfig) -> Result<(), SkillPolicyError> {
    if m.vcpu_count == 0 {
        return Err(SkillPolicyError::InvalidMachine {
            field: "vcpu_count",
            value: "0".into(),
            reason: "Firecracker requires at least 1 vCPU",
        });
    }
    if m.vcpu_count > 32 {
        return Err(SkillPolicyError::InvalidMachine {
            field: "vcpu_count",
            value: m.vcpu_count.to_string(),
            reason:
                "vcpu_count > 32 is rejected (sanity bound; raise the cap if a workload needs it)",
        });
    }
    if m.mem_size_mib < 32 {
        return Err(SkillPolicyError::InvalidMachine {
            field: "mem_size_mib",
            value: m.mem_size_mib.to_string(),
            reason: "< 32 MiB starves the BusyBox+bash rootfs init",
        });
    }
    if m.mem_size_mib > 32 * 1024 {
        return Err(SkillPolicyError::InvalidMachine {
            field: "mem_size_mib",
            value: m.mem_size_mib.to_string(),
            reason: "> 32 GiB is almost certainly a typo (raise the cap if a workload needs it)",
        });
    }
    Ok(())
}

fn validate_env_name(k: &str) -> Result<(), SkillPolicyError> {
    if k.is_empty() {
        return Err(SkillPolicyError::InvalidEnvName(k.to_string()));
    }
    let first = k.chars().next().unwrap();
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(SkillPolicyError::InvalidEnvName(k.to_string()));
    }
    let ok = k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !ok {
        return Err(SkillPolicyError::InvalidEnvName(k.to_string()));
    }
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── Default + minimal ────────────────────────────────────────────────

    #[test]
    fn default_policy_is_valid() {
        let p = SkillSandboxPolicy::default();
        assert!(p.validate().is_ok());
        assert!(p.machine.is_none());
        assert!(p.shares.is_empty());
        assert!(p.bridge.is_none());
    }

    #[test]
    fn empty_json_object_parses_to_default() {
        let p = SkillSandboxPolicy::from_json("{}").unwrap();
        assert_eq!(p, SkillSandboxPolicy::default());
    }

    // ── Unknown fields rejected (deny_unknown_fields) ────────────────────

    #[test]
    fn unknown_field_rejected() {
        let r = SkillSandboxPolicy::from_json(r#"{"unknown_field": 1}"#);
        assert!(r.is_err(), "deny_unknown_fields must reject typos");
    }

    // ── Machine validation ───────────────────────────────────────────────

    #[test]
    fn zero_vcpu_rejected() {
        let p = SkillSandboxPolicy {
            machine: Some(MachineConfig {
                vcpu_count: 0,
                mem_size_mib: 128,
                smt: false,
                track_dirty_pages: None,
            }),
            ..Default::default()
        };
        match p.validate().unwrap_err() {
            SkillPolicyError::InvalidMachine { field, .. } => {
                assert_eq!(field, "vcpu_count");
            }
            other => panic!("expected InvalidMachine, got {:?}", other),
        }
    }

    #[test]
    fn excessive_vcpu_rejected() {
        let p = SkillSandboxPolicy {
            machine: Some(MachineConfig {
                vcpu_count: 33,
                mem_size_mib: 128,
                smt: false,
                track_dirty_pages: None,
            }),
            ..Default::default()
        };
        assert!(matches!(
            p.validate(),
            Err(SkillPolicyError::InvalidMachine { .. })
        ));
    }

    #[test]
    fn tiny_memory_rejected() {
        let p = SkillSandboxPolicy {
            machine: Some(MachineConfig {
                vcpu_count: 1,
                mem_size_mib: 16, // too small
                smt: false,
                track_dirty_pages: None,
            }),
            ..Default::default()
        };
        match p.validate().unwrap_err() {
            SkillPolicyError::InvalidMachine { field, .. } => {
                assert_eq!(field, "mem_size_mib");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn excessive_memory_rejected() {
        let p = SkillSandboxPolicy {
            machine: Some(MachineConfig {
                vcpu_count: 1,
                mem_size_mib: 64 * 1024, // 64 GiB — too big
                smt: false,
                track_dirty_pages: None,
            }),
            ..Default::default()
        };
        assert!(matches!(
            p.validate(),
            Err(SkillPolicyError::InvalidMachine { .. })
        ));
    }

    // ── Share validation re-runs the deny-list ───────────────────────────

    #[test]
    fn share_with_aws_creds_rejected() {
        // Deserialize a hand-built JSON that bypasses VirtioFsShare::new.
        // Validate must catch it via re-validation.
        let s = json!({
            "shares": [{
                "host_path": "/home/u/.aws/credentials",
                "mount_tag": "leak",
                "socket_path": "/run/s.sock",
                "read_only": true,
                "xattr_passthrough": false,
                "cache_mode": "auto",
            }]
        });
        let p: SkillSandboxPolicy = serde_json::from_value(s).unwrap();
        let err = p.validate().unwrap_err();
        assert!(matches!(err, SkillPolicyError::Share(_)));
    }

    #[test]
    fn share_with_neutral_path_accepted() {
        let s = json!({
            "shares": [{
                "host_path": "/var/work",
                "mount_tag": "workspace",
                "socket_path": "/run/s.sock",
                "read_only": false,
                "xattr_passthrough": false,
                "cache_mode": "auto",
            }]
        });
        let p: SkillSandboxPolicy = serde_json::from_value(s).unwrap();
        assert!(p.validate().is_ok());
    }

    // ── Env var name validation ──────────────────────────────────────────

    #[test]
    fn env_name_with_dash_rejected() {
        let mut p = SkillSandboxPolicy::default();
        p.extra_env.insert("MY-VAR".into(), "x".into());
        assert!(matches!(
            p.validate(),
            Err(SkillPolicyError::InvalidEnvName(_))
        ));
    }

    #[test]
    fn env_name_starting_with_digit_rejected() {
        let mut p = SkillSandboxPolicy::default();
        p.extra_env.insert("1FOO".into(), "x".into());
        assert!(matches!(
            p.validate(),
            Err(SkillPolicyError::InvalidEnvName(_))
        ));
    }

    #[test]
    fn env_name_with_underscore_accepted() {
        let mut p = SkillSandboxPolicy::default();
        p.extra_env.insert("_FOO_BAR".into(), "x".into());
        assert!(p.validate().is_ok());
    }

    // ── wall_clock_timeout_ms ────────────────────────────────────────────

    #[test]
    fn zero_timeout_rejected() {
        let p = SkillSandboxPolicy {
            wall_clock_timeout_ms: Some(0),
            ..Default::default()
        };
        assert!(matches!(p.validate(), Err(SkillPolicyError::ZeroTimeout)));
    }

    #[test]
    fn positive_timeout_accepted() {
        let p = SkillSandboxPolicy {
            wall_clock_timeout_ms: Some(60_000),
            ..Default::default()
        };
        assert!(p.validate().is_ok());
    }

    // ── into_vm_config composition ───────────────────────────────────────

    #[test]
    fn into_vm_config_uses_defaults_when_machine_unset() {
        let p = SkillSandboxPolicy::default();
        let cfg = p
            .into_vm_config("/var/lib/vibe/rootfs.ext4", "/boot/vmlinux.bin")
            .unwrap();
        let m = cfg.machine_config.unwrap();
        assert_eq!(m.vcpu_count, 1);
        assert_eq!(m.mem_size_mib, 128);
    }

    #[test]
    fn into_vm_config_boot_args_contain_console_and_root() {
        let p = SkillSandboxPolicy::default();
        let cfg = p.into_vm_config("/r.ext4", "/k").unwrap();
        let b = cfg.boot_source.unwrap();
        assert!(b.boot_args.as_ref().unwrap().contains("console=ttyS0"));
        assert!(b.boot_args.as_ref().unwrap().contains("root=/dev/vda"));
    }

    #[test]
    fn into_vm_config_appends_extra_boot_args() {
        let p = SkillSandboxPolicy {
            extra_boot_args: Some("apparmor=0".into()),
            ..Default::default()
        };
        let cfg = p.into_vm_config("/r.ext4", "/k").unwrap();
        assert!(cfg
            .boot_source
            .unwrap()
            .boot_args
            .unwrap()
            .contains("apparmor=0"));
    }

    #[test]
    fn into_vm_config_drive_is_readonly_root() {
        let p = SkillSandboxPolicy::default();
        let cfg = p.into_vm_config("/r.ext4", "/k").unwrap();
        assert_eq!(cfg.drives.len(), 1);
        let d = &cfg.drives[0];
        assert_eq!(d.drive_id, "rootfs");
        assert!(d.is_root_device);
        assert!(d.is_read_only);
        assert_eq!(d.path_on_host, "/r.ext4");
    }

    #[test]
    fn into_vm_config_adds_vsock_when_bridge_set() {
        let p = SkillSandboxPolicy {
            bridge: Some(BridgeConfig {
                sandbox_id: "sb1".into(),
                policy_id: "default".into(),
                guest_cid: 3,
                host_uds_path: "/run/v.sock".into(),
                guest_proxy_url: "http://169.254.0.2:8888".into(),
                no_proxy: vec![],
            }),
            ..Default::default()
        };
        let cfg = p.into_vm_config("/r.ext4", "/k").unwrap();
        let v = cfg.vsock.unwrap();
        assert_eq!(v.guest_cid, 3);
        assert_eq!(v.uds_path, "/run/v.sock");
    }

    #[test]
    fn into_vm_config_rejects_overlong_cmdline() {
        let p = SkillSandboxPolicy {
            extra_boot_args: Some("a".repeat(5000)),
            ..Default::default()
        };
        assert!(matches!(
            p.into_vm_config("/r.ext4", "/k"),
            Err(SkillPolicyError::KernelCmdlineTooLong(_))
        ));
    }

    // ── boot_sequence integration ────────────────────────────────────────

    #[test]
    fn into_vm_config_boot_sequence_emits_expected_endpoints() {
        let p = SkillSandboxPolicy {
            bridge: Some(BridgeConfig {
                sandbox_id: "sb1".into(),
                policy_id: "default".into(),
                guest_cid: 3,
                host_uds_path: "/run/v.sock".into(),
                guest_proxy_url: "http://169.254.0.2:8888".into(),
                no_proxy: vec![],
            }),
            ..Default::default()
        };
        let seq = p.into_vm_config("/r.ext4", "/k").unwrap().boot_sequence();
        let paths: Vec<&str> = seq.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "/machine-config",
                "/boot-source",
                "/drives/rootfs",
                "/vsock",
                "/actions",
            ]
        );
    }

    // ── serde round-trip ─────────────────────────────────────────────────

    #[test]
    fn serde_round_trip_preserves_all_fields() {
        let mut env = BTreeMap::new();
        env.insert("FOO".into(), "bar".into());
        let p = SkillSandboxPolicy {
            machine: Some(MachineConfig::default()),
            shares: vec![
                VirtioFsShare::new("/var/work", "workspace", "/run/s.sock", false).unwrap(),
            ],
            bridge: None,
            extra_env: env,
            extra_boot_args: Some("quiet".into()),
            wall_clock_timeout_ms: Some(30_000),
        };
        let s = serde_json::to_string(&p).unwrap();
        let back = SkillSandboxPolicy::from_json(&s).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn omits_optional_fields_when_default() {
        let p = SkillSandboxPolicy::default();
        let v = serde_json::to_value(&p).unwrap();
        // Optional fields must not serialize when None/empty.
        assert!(v.get("machine").is_none());
        assert!(v.get("bridge").is_none());
        assert!(v.get("extra_env").is_none());
        assert!(v.get("extra_boot_args").is_none());
        assert!(v.get("wall_clock_timeout_ms").is_none());
        // `shares` is a Vec (no skip_serializing_if), so it emits [].
        assert_eq!(v["shares"], json!([]));
    }
}
