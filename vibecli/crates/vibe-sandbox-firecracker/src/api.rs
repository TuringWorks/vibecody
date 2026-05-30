//! Firecracker REST API request shapes + JSON serialization.
//!
//! Slice F2.1 — the building blocks for `FirecrackerSandbox::spawn()`
//! (F2.2, Linux-only). Pure data + serde, no syscalls, no UDS I/O —
//! the actual HTTP-over-UDS client lives in `lifecycle.rs` (F2.2) and
//! consumes these structures.
//!
//! ## Why a separate slice
//!
//! Firecracker's REST API is documented at
//! <https://github.com/firecracker-microvm/firecracker/blob/main/src/api_server/swagger/firecracker.yaml>
//! but using `serde_json::Value::from` ad-hoc inside `spawn()` would
//! mean every typo or schema regression surfaces only at runtime —
//! on Linux, with KVM. By pinning the request shapes here, with
//! exhaustive serialization tests pinning the JSON layout, schema
//! regressions surface at `cargo test` on any platform.
//!
//! ## Coverage
//!
//! The minimum surface F2.2 needs:
//!
//! | Endpoint                 | Method  | Body              |
//! |--------------------------|---------|-------------------|
//! | `/boot-source`           | `PUT`   | [`BootSource`]    |
//! | `/machine-config`        | `PUT`   | [`MachineConfig`] |
//! | `/drives/{id}`           | `PUT`   | [`Drive`]         |
//! | `/vsock`                 | `PUT`   | [`Vsock`]         |
//! | `/actions`               | `PUT`   | [`Action`] (InstanceStart, SendCtrlAltDel) |
//! | `/snapshot/create`       | `PUT`   | (out of scope for F2.2; F5 pooling) |
//!
//! The full schema has 20+ endpoints; this slice ships the boot-path
//! ones. F5 (VM pooling) adds snapshot/restore.

use serde::{Deserialize, Serialize};

/// Boot source — kernel image + cmdline.
///
/// Firecracker boots a "linux-style" kernel directly (no bootloader).
/// `boot_args` is the kernel command-line; F2.2 will populate it with
/// the rootfs device + console settings derived from the
/// `MachineConfig` and `Drive` fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootSource {
    pub kernel_image_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initrd_path: Option<String>,
}

/// Machine config — CPU + memory.
///
/// Firecracker defaults: 1 vCPU, 128 MiB, SMT off. F2.2 will let
/// per-skill policy override these. Kept small here so the JSON
/// round-trip is precise.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MachineConfig {
    pub vcpu_count: u8,
    pub mem_size_mib: u32,
    pub smt: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_dirty_pages: Option<bool>,
}

impl Default for MachineConfig {
    fn default() -> Self {
        Self {
            vcpu_count: 1,
            mem_size_mib: 128,
            smt: false,
            track_dirty_pages: None,
        }
    }
}

/// Drive — a block device, typically the rootfs.
///
/// `drive_id` is the user-chosen name (e.g. `"rootfs"`); the API
/// path is `/drives/<drive_id>`. F2.2 attaches exactly one drive
/// (the rootfs); F3 (virtio-fs) adds the writable workspace
/// surface via a different mechanism (mount tag), not a drive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Drive {
    pub drive_id: String,
    pub path_on_host: String,
    pub is_root_device: bool,
    pub is_read_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_type: Option<DriveCacheType>,
}

/// Drive cache mode (firecracker enum).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum DriveCacheType {
    Unsafe,
    Writeback,
}

/// virtio-vsock device — the daemon-to-guest control plane.
///
/// `guest_cid` defaults to 3 (CID 2 is reserved for the host).
/// `uds_path` is the host-side Unix Domain Socket the daemon
/// connects to; firecracker creates `<uds_path>_<port>` for each
/// guest-initiated connection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vsock {
    pub guest_cid: u32,
    pub uds_path: String,
}

/// Action — lifecycle commands.
///
/// `InstanceStart` boots the VM after all config is in place.
/// `SendCtrlAltDel` triggers a graceful shutdown.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Action {
    pub action_type: ActionType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum ActionType {
    InstanceStart,
    SendCtrlAltDel,
    FlushMetrics,
}

/// One full boot sequence's worth of API state. F2.2 will walk this
/// in order, PUT-ing each section to its endpoint, then PUT-ing the
/// InstanceStart action.
///
/// Captured as a struct (rather than a series of `boot_source_request`
/// / `machine_config_request` free functions) so a single test fixture
/// can pin the full boot-sequence JSON.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VmConfig {
    pub boot_source: Option<BootSource>,
    pub machine_config: Option<MachineConfig>,
    pub drives: Vec<Drive>,
    pub vsock: Option<Vsock>,
}

impl VmConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_boot_source(mut self, b: BootSource) -> Self {
        self.boot_source = Some(b);
        self
    }
    pub fn with_machine_config(mut self, m: MachineConfig) -> Self {
        self.machine_config = Some(m);
        self
    }
    pub fn with_drive(mut self, d: Drive) -> Self {
        self.drives.push(d);
        self
    }
    pub fn with_vsock(mut self, v: Vsock) -> Self {
        self.vsock = Some(v);
        self
    }

    /// Walk the boot sequence in the order Firecracker requires:
    /// machine-config → boot-source → drives → vsock → InstanceStart.
    /// Returns a sequence of (path, json_body) pairs F2.2 will PUT.
    pub fn boot_sequence(&self) -> Vec<ApiRequest> {
        let mut out = Vec::new();
        if let Some(m) = &self.machine_config {
            out.push(ApiRequest {
                path: "/machine-config".to_string(),
                body: serde_json::to_value(m).expect("MachineConfig JSON"),
            });
        }
        if let Some(b) = &self.boot_source {
            out.push(ApiRequest {
                path: "/boot-source".to_string(),
                body: serde_json::to_value(b).expect("BootSource JSON"),
            });
        }
        for d in &self.drives {
            out.push(ApiRequest {
                path: format!("/drives/{}", d.drive_id),
                body: serde_json::to_value(d).expect("Drive JSON"),
            });
        }
        if let Some(v) = &self.vsock {
            out.push(ApiRequest {
                path: "/vsock".to_string(),
                body: serde_json::to_value(v).expect("Vsock JSON"),
            });
        }
        out.push(ApiRequest {
            path: "/actions".to_string(),
            body: serde_json::to_value(Action {
                action_type: ActionType::InstanceStart,
            })
            .expect("Action JSON"),
        });
        out
    }
}

/// A single PUT to the Firecracker API socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiRequest {
    pub path: String,
    pub body: serde_json::Value,
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── BootSource ───────────────────────────────────────────────────────

    #[test]
    fn boot_source_minimal_serializes_without_optionals() {
        let b = BootSource {
            kernel_image_path: "/boot/vmlinux.bin".into(),
            boot_args: None,
            initrd_path: None,
        };
        let v = serde_json::to_value(&b).unwrap();
        assert_eq!(v, json!({"kernel_image_path": "/boot/vmlinux.bin"}));
    }

    #[test]
    fn boot_source_with_boot_args_serializes() {
        let b = BootSource {
            kernel_image_path: "/boot/vmlinux.bin".into(),
            boot_args: Some("console=ttyS0 root=/dev/vda".into()),
            initrd_path: None,
        };
        let v = serde_json::to_value(&b).unwrap();
        assert_eq!(v["boot_args"], "console=ttyS0 root=/dev/vda");
    }

    #[test]
    fn boot_source_round_trips() {
        let b = BootSource {
            kernel_image_path: "/k".into(),
            boot_args: Some("init=/sbin/init".into()),
            initrd_path: Some("/initrd".into()),
        };
        let s = serde_json::to_string(&b).unwrap();
        let r: BootSource = serde_json::from_str(&s).unwrap();
        assert_eq!(b, r);
    }

    // ── MachineConfig ────────────────────────────────────────────────────

    #[test]
    fn machine_config_default_is_1vcpu_128mib_smt_off() {
        let m = MachineConfig::default();
        assert_eq!(m.vcpu_count, 1);
        assert_eq!(m.mem_size_mib, 128);
        assert!(!m.smt);
        assert!(m.track_dirty_pages.is_none());
    }

    #[test]
    fn machine_config_omits_optional_track_dirty_pages() {
        let m = MachineConfig::default();
        let v = serde_json::to_value(&m).unwrap();
        assert!(v.get("track_dirty_pages").is_none());
    }

    #[test]
    fn machine_config_with_dirty_pages_emits_field() {
        let m = MachineConfig {
            track_dirty_pages: Some(true),
            ..Default::default()
        };
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["track_dirty_pages"], true);
    }

    // ── Drive ────────────────────────────────────────────────────────────

    #[test]
    fn drive_rootfs_serializes() {
        let d = Drive {
            drive_id: "rootfs".into(),
            path_on_host: "/var/lib/vibe/rootfs.ext4".into(),
            is_root_device: true,
            is_read_only: true,
            cache_type: None,
        };
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(
            v,
            json!({
                "drive_id": "rootfs",
                "path_on_host": "/var/lib/vibe/rootfs.ext4",
                "is_root_device": true,
                "is_read_only": true,
            })
        );
    }

    #[test]
    fn drive_cache_type_pascal_case() {
        let d = Drive {
            drive_id: "scratch".into(),
            path_on_host: "/tmp/scratch".into(),
            is_root_device: false,
            is_read_only: false,
            cache_type: Some(DriveCacheType::Writeback),
        };
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["cache_type"], "Writeback");
    }

    // ── Vsock ────────────────────────────────────────────────────────────

    #[test]
    fn vsock_default_cid_3_serializes() {
        let v = Vsock {
            guest_cid: 3,
            uds_path: "/tmp/firecracker-vsock.sock".into(),
        };
        let j = serde_json::to_value(&v).unwrap();
        assert_eq!(j["guest_cid"], 3);
        assert_eq!(j["uds_path"], "/tmp/firecracker-vsock.sock");
    }

    // ── Action ───────────────────────────────────────────────────────────

    #[test]
    fn action_instance_start_pascal_case() {
        let a = Action {
            action_type: ActionType::InstanceStart,
        };
        let v = serde_json::to_value(&a).unwrap();
        assert_eq!(v, json!({"action_type": "InstanceStart"}));
    }

    #[test]
    fn action_send_ctrl_alt_del_serializes() {
        let a = Action {
            action_type: ActionType::SendCtrlAltDel,
        };
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            json!({"action_type": "SendCtrlAltDel"})
        );
    }

    // ── VmConfig boot_sequence ───────────────────────────────────────────

    #[test]
    fn boot_sequence_emits_endpoints_in_required_order() {
        // Firecracker requires machine-config + boot-source + drives
        // before InstanceStart; vsock is optional and goes before
        // start. Our boot_sequence() must reflect that ordering so
        // F2.2 can PUT in order.
        let cfg = VmConfig::new()
            .with_machine_config(MachineConfig::default())
            .with_boot_source(BootSource {
                kernel_image_path: "/boot/vmlinux.bin".into(),
                boot_args: Some("console=ttyS0".into()),
                initrd_path: None,
            })
            .with_drive(Drive {
                drive_id: "rootfs".into(),
                path_on_host: "/var/lib/vibe/rootfs.ext4".into(),
                is_root_device: true,
                is_read_only: true,
                cache_type: None,
            })
            .with_vsock(Vsock {
                guest_cid: 3,
                uds_path: "/run/vibe-vsock.sock".into(),
            });

        let seq = cfg.boot_sequence();
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
        // Final action must be InstanceStart.
        assert_eq!(seq.last().unwrap().body["action_type"], "InstanceStart");
    }

    #[test]
    fn boot_sequence_omits_vsock_when_not_configured() {
        let cfg = VmConfig::new()
            .with_machine_config(MachineConfig::default())
            .with_boot_source(BootSource {
                kernel_image_path: "/k".into(),
                boot_args: None,
                initrd_path: None,
            })
            .with_drive(Drive {
                drive_id: "rootfs".into(),
                path_on_host: "/r".into(),
                is_root_device: true,
                is_read_only: true,
                cache_type: None,
            });

        let seq = cfg.boot_sequence();
        let paths: Vec<&str> = seq.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "/machine-config",
                "/boot-source",
                "/drives/rootfs",
                "/actions"
            ]
        );
    }

    #[test]
    fn boot_sequence_handles_multiple_drives() {
        let cfg = VmConfig::new()
            .with_drive(Drive {
                drive_id: "rootfs".into(),
                path_on_host: "/r".into(),
                is_root_device: true,
                is_read_only: true,
                cache_type: None,
            })
            .with_drive(Drive {
                drive_id: "scratch".into(),
                path_on_host: "/s".into(),
                is_root_device: false,
                is_read_only: false,
                cache_type: Some(DriveCacheType::Unsafe),
            });

        let seq = cfg.boot_sequence();
        let paths: Vec<&str> = seq.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(paths, vec!["/drives/rootfs", "/drives/scratch", "/actions"]);
    }

    #[test]
    fn boot_sequence_always_ends_with_instance_start() {
        let cfg = VmConfig::new();
        let seq = cfg.boot_sequence();
        assert_eq!(seq.last().unwrap().path, "/actions");
        assert_eq!(seq.last().unwrap().body["action_type"], "InstanceStart");
    }

    // ── ApiRequest body shape ────────────────────────────────────────────

    #[test]
    fn api_request_body_is_object_not_string() {
        let cfg = VmConfig::new().with_machine_config(MachineConfig::default());
        let seq = cfg.boot_sequence();
        let mc = &seq[0];
        assert_eq!(mc.path, "/machine-config");
        // The body must be a JSON object (not a serialized-string blob),
        // because F2.2 will hand it to an HTTP client that handles the
        // Content-Type: application/json wrap itself.
        assert!(mc.body.is_object());
        assert!(mc.body["vcpu_count"].is_number());
    }

    // ── Schema invariants ────────────────────────────────────────────────

    #[test]
    fn drive_omits_optional_cache_type_by_default() {
        let d = Drive {
            drive_id: "rootfs".into(),
            path_on_host: "/r".into(),
            is_root_device: true,
            is_read_only: true,
            cache_type: None,
        };
        let v = serde_json::to_value(&d).unwrap();
        assert!(
            v.get("cache_type").is_none(),
            "default Drive must not emit cache_type — Firecracker treats absence as Unsafe"
        );
    }

    #[test]
    fn boot_source_strict_field_naming() {
        // Firecracker rejects camelCase; field names must be snake_case.
        let b = BootSource {
            kernel_image_path: "/k".into(),
            boot_args: Some("foo".into()),
            initrd_path: None,
        };
        let s = serde_json::to_string(&b).unwrap();
        assert!(s.contains("kernel_image_path"));
        assert!(!s.contains("kernelImagePath"));
        assert!(s.contains("boot_args"));
    }
}
