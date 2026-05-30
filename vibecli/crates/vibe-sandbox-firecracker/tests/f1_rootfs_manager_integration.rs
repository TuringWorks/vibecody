//! Integration test: end-to-end `RootfsManager` against the real F1 artifact.
//!
//! Skips cleanly when `target/firecracker-rootfs/rootfs.ext4` is absent
//! (developer hasn't run `make rootfs-firecracker` yet). When the artifact
//! is present, this test pins the daemon-side contract:
//!
//!   1. Manager computes the same SHA256 the builder wrote to the
//!      `.sha256` sidecar — proves the streaming hasher matches
//!      `sha256sum` / `shasum -a 256`.
//!   2. `verify(path, None)` reads the sidecar and accepts.
//!   3. `install` copies to a content-addressed cache path.
//!   4. `cached(sha)` resolves the same path after install.
//!   5. Re-installing the same source is idempotent.

use std::path::PathBuf;

use vibe_sandbox_firecracker::rootfs::RootfsManager;

fn artifact() -> Option<PathBuf> {
    // CARGO_MANIFEST_DIR = …/vibecli/crates/vibe-sandbox-firecracker
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..3 {
        p.pop();
    }
    let img = p.join("target/firecracker-rootfs/rootfs.ext4");
    if img.exists() {
        Some(img)
    } else {
        None
    }
}

#[test]
fn manager_round_trip_against_real_artifact() {
    let Some(img) = artifact() else {
        eprintln!("[skip] target/firecracker-rootfs/rootfs.ext4 not built — run `make rootfs-firecracker`");
        return;
    };

    let cache =
        std::env::temp_dir().join(format!("vibe_rootfs_integration_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);

    let mgr = RootfsManager::new(&cache);

    // 1. SHA256 round-trip — manager value matches the sidecar.
    let sha_from_mgr = mgr.compute_sha256(&img).expect("compute_sha256");
    let sidecar = mgr.read_sidecar(&img).expect("read_sidecar");
    assert_eq!(
        sha_from_mgr, sidecar,
        "manager sha disagrees with builder sidecar — streaming hasher broken?"
    );

    // 2. verify(path, None) accepts.
    let v = mgr.verify(&img, None).expect("verify");
    assert_eq!(v.sha256, sha_from_mgr);
    assert!(v.size_bytes > 0);
    assert!(v.size_bytes <= 32 * 1024 * 1024);

    // 3. install + 4. cached round-trip.
    assert!(mgr.cached(&sha_from_mgr).is_none(), "fresh cache");
    let cached = mgr.install(&img, Some(&sha_from_mgr)).expect("install");
    assert!(cached.exists());
    let resolved = mgr.cached(&sha_from_mgr).expect("cached after install");
    assert_eq!(cached, resolved);

    // 5. Re-install is idempotent.
    let mtime_before = std::fs::metadata(&cached).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let cached2 = mgr.install(&img, Some(&sha_from_mgr)).expect("re-install");
    let mtime_after = std::fs::metadata(&cached2).unwrap().modified().unwrap();
    assert_eq!(cached, cached2);
    assert_eq!(
        mtime_before, mtime_after,
        "re-install should not rewrite the file"
    );

    let _ = std::fs::remove_dir_all(&cache);
}
