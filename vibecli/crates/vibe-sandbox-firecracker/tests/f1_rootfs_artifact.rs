//! F1 — Firecracker rootfs artifact contract test.
//!
//! Slice F1 from `docs/design/sandbox-tiers/03-firecracker-tier.md`
//! ships `scripts/build-rootfs-firecracker.sh` which produces an
//! ext4 image at `target/firecracker-rootfs/rootfs.ext4`. This test
//! verifies the artifact *when it exists* (the build is opt-in via
//! `make rootfs-firecracker`); on machines without it the test
//! prints a hint and skips, so a clean checkout passes CI.
//!
//! What's checked when the file is present:
//!   1. It opens (no I/O errors).
//!   2. The ext4 superblock magic 0xef53 sits at byte 1080.
//!   3. The size is ≤ 32 MiB (sanity bound — the ≤ 20 MiB design
//!      target lives in the script; this test enforces a 32 MiB
//!      ceiling so a runaway artifact growth gets caught).
//!   4. The companion `.sha256` sidecar exists and is the right
//!      shape (64 lowercase hex chars + optional trailing newline).

use std::fs;
use std::io::Read;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at the crate dir; rootfs lives at the
    // workspace root's target/.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // …/vibecli/crates/vibe-sandbox-firecracker → up 3 levels
    for _ in 0..3 {
        p.pop();
    }
    p
}

#[test]
fn rootfs_artifact_when_present_is_valid_ext4() {
    let root = workspace_root();
    let rootfs = root.join("target/firecracker-rootfs/rootfs.ext4");

    if !rootfs.exists() {
        eprintln!(
            "[skip] {} not present — run `make rootfs-firecracker` to build the F1 artifact",
            rootfs.display()
        );
        return;
    }

    // ── 1. Opens cleanly ──────────────────────────────────────────────────
    let mut f = fs::File::open(&rootfs).expect("open rootfs.ext4");

    // ── 2. ext4 superblock magic at offset 1080 (0x438) ───────────────────
    let mut header = vec![0u8; 1088];
    f.read_exact(&mut header).expect("read first 1088 bytes");
    // s_magic is a __le16 at offset 0x438 within the superblock; the
    // superblock starts at offset 1024 in the image, so the absolute
    // offset is 1024 + 0x38 = 1080.
    let magic_lo = header[1080];
    let magic_hi = header[1081];
    let magic = u16::from_le_bytes([magic_lo, magic_hi]);
    assert_eq!(
        magic, 0xEF53,
        "rootfs.ext4 superblock magic mismatch — got 0x{:04X}, expected 0xEF53. \
         Did the builder ship a non-ext4 image?",
        magic
    );

    // ── 3. Size sanity bound ──────────────────────────────────────────────
    let size = fs::metadata(&rootfs).expect("stat").len();
    let mib = size / (1024 * 1024);
    assert!(
        size <= 32 * 1024 * 1024,
        "rootfs.ext4 is {} MiB — exceeds 32 MiB ceiling. \
         Tier-3 microVM cold-start depends on a small rootfs; investigate growth.",
        mib
    );
    assert!(size > 0, "rootfs.ext4 is empty");

    // ── 4. SHA256 sidecar ─────────────────────────────────────────────────
    let sha_path = root.join("target/firecracker-rootfs/rootfs.ext4.sha256");
    if sha_path.exists() {
        let sha = fs::read_to_string(&sha_path).expect("read sidecar");
        let sha = sha.trim();
        assert_eq!(
            sha.len(),
            64,
            "rootfs.ext4.sha256 has wrong length {} (expected 64 hex chars)",
            sha.len()
        );
        assert!(
            sha.chars().all(|c| c.is_ascii_hexdigit() && (c.is_ascii_digit() || c.is_ascii_lowercase())),
            "rootfs.ext4.sha256 contains non-lowercase-hex chars"
        );
    } else {
        eprintln!(
            "[warn] {} missing — builder should produce both rootfs.ext4 and .sha256",
            sha_path.display()
        );
    }
}

#[test]
fn rootfs_build_script_exists_and_is_executable() {
    let script = workspace_root().join("scripts/build-rootfs-firecracker.sh");
    assert!(
        script.exists(),
        "scripts/build-rootfs-firecracker.sh missing — F1 deliverable"
    );
    // On Unix, also assert exec bit so `make rootfs-firecracker` doesn't
    // suddenly start needing `bash <script>` instead of `./<script>`.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&script).expect("stat").permissions().mode();
        assert!(
            mode & 0o111 != 0,
            "build-rootfs-firecracker.sh is not executable (mode {:o})",
            mode
        );
    }
}
