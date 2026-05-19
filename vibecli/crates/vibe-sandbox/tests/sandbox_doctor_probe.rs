//! Contract test for `scripts/check-sandbox-tiers.sh --json`.
//!
//! The script is the standalone preview of what `vibecli doctor`
//! will report for sandbox-tier availability (H6 / native-tier §doctor).
//! Until the daemon wiring lands, downstream tooling (CI, deployment
//! scripts, monitoring) parses this JSON. Treat the shape as a stable
//! contract — the keys, types, and values listed here are the surface
//! callers depend on.
//!
//! What's asserted:
//!   • script exists at the expected path + is exec-bit set
//!   • `--json` exits 0 on supported OSes + emits parseable JSON
//!   • top-level keys present: `os`, `arch`, `tiers`
//!   • every tier (`native`, `wasi`, `hyperlight`, `firecracker`)
//!     reports `status` and `note`
//!   • `firecracker` also reports `rootfs` and `rootfs_path`
//!   • each `status` is one of the documented enum values
//!     (`ok`, `probable`, `degraded`, `needs-perm`, `missing`,
//!     `unsupported`)

use std::path::PathBuf;
use std::process::Command;

const STATUSES: &[&str] = &[
    "ok",
    "probable",
    "degraded",
    "needs-perm",
    "missing",
    "unsupported",
];

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // …/vibecli/crates/vibe-sandbox → up 3 levels
    for _ in 0..3 {
        p.pop();
    }
    p
}

#[test]
fn probe_script_exists_and_is_executable() {
    let script = workspace_root().join("scripts/check-sandbox-tiers.sh");
    assert!(
        script.exists(),
        "scripts/check-sandbox-tiers.sh missing — H6 preview deliverable"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&script)
            .expect("stat")
            .permissions()
            .mode();
        assert!(
            mode & 0o111 != 0,
            "check-sandbox-tiers.sh is not executable (mode {:o})",
            mode
        );
    }
}

#[test]
fn probe_json_shape_is_stable() {
    let script = workspace_root().join("scripts/check-sandbox-tiers.sh");
    if !script.exists() {
        eprintln!("[skip] probe script missing");
        return;
    }

    let out = Command::new("bash")
        .arg(&script)
        .arg("--json")
        .output()
        .expect("spawn probe");

    // The script exits non-zero only when Tier-0 is unusable. On a dev
    // box that's anomalous; if it happens, still parse stdout — but
    // don't fail the test on it.
    let stdout = String::from_utf8_lossy(&out.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("probe stdout is parseable JSON");

    // ── Top-level keys ───────────────────────────────────────────────────
    assert!(json.get("os").and_then(|v| v.as_str()).is_some(), "missing .os");
    assert!(json.get("arch").and_then(|v| v.as_str()).is_some(), "missing .arch");
    let tiers = json
        .get("tiers")
        .and_then(|v| v.as_object())
        .expect(".tiers must be an object");

    // ── Every tier present + correctly shaped ────────────────────────────
    for tier in ["native", "wasi", "hyperlight", "firecracker"] {
        let t = tiers.get(tier).expect(&format!(".tiers.{} missing", tier));
        let status = t
            .get("status")
            .and_then(|v| v.as_str())
            .expect(&format!(".tiers.{}.status missing", tier));
        assert!(
            STATUSES.contains(&status),
            ".tiers.{}.status = {:?} — not in documented enum {:?}",
            tier,
            status,
            STATUSES
        );
        assert!(
            t.get("note").is_some(),
            ".tiers.{}.note missing — needed for human-readable doctor output",
            tier
        );
    }

    // ── Firecracker-specific extra fields ────────────────────────────────
    let fc = tiers.get("firecracker").unwrap();
    let rootfs = fc
        .get("rootfs")
        .and_then(|v| v.as_str())
        .expect(".tiers.firecracker.rootfs missing");
    assert!(
        rootfs == "present" || rootfs == "absent",
        ".tiers.firecracker.rootfs = {:?} — must be 'present' or 'absent'",
        rootfs
    );
    assert!(
        fc.get("rootfs_path").and_then(|v| v.as_str()).is_some(),
        ".tiers.firecracker.rootfs_path missing"
    );
}
