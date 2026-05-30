//! Daemon-side rootfs verification + content-addressed cache.
//!
//! Slice F1 daemon-side — completes the F1 deliverable started by
//! `scripts/build-rootfs-firecracker.sh` + the CI publish job.
//!
//! ## Why daemon-side verification matters
//!
//! The microVM (slice F2) is only as trustworthy as the rootfs it
//! boots. If the daemon downloads a release artifact from a GitHub
//! release URL, three things can go wrong between "release published"
//! and "VM boots":
//!
//! 1. **Bit-flips / partial downloads** — the SHA256 sidecar (also
//!    published by the F1 CI job) catches this.
//! 2. **Tampered mirror / MITM** — same sidecar, since cosign signs
//!    the artifact bytes from the GHA OIDC token; a mirror that
//!    swaps bytes can't forge the Rekor-logged signature.
//! 3. **Wrong version pinned** — content-addressed cache keyed on
//!    SHA256 means asking for SHA `abc…` always gets bytes whose
//!    SHA256 is `abc…` or fails closed.
//!
//! ## Why this module is in `vibe-sandbox-firecracker` and not the daemon
//!
//! The daemon does the *download* (it already has reqwest +
//! workspace auth tokens). The verification + caching contract is
//! tier-specific (Tier-3-only) so it lives next to the tier's
//! lifecycle code. This crate stays HTTP-client-free.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;

/// Errors surfaced by the rootfs manager.
#[derive(Debug, Error)]
pub enum RootfsError {
    #[error("image not found: {0}")]
    NotFound(PathBuf),

    #[error("sidecar .sha256 not found next to {0}")]
    SidecarMissing(PathBuf),

    #[error("sha256 mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("image is empty: {0}")]
    Empty(PathBuf),

    #[error("image exceeds size ceiling {limit_bytes}: {path} is {actual_bytes} bytes")]
    TooLarge {
        path: PathBuf,
        actual_bytes: u64,
        limit_bytes: u64,
    },

    #[error("ext4 superblock magic mismatch — not a valid ext4 image: {0}")]
    NotExt4(PathBuf),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("cosign verification failed: {0}")]
    CosignFailed(String),

    #[error("cache directory not initialized: {0}")]
    CacheInit(String),
}

/// Sanity ceiling on a rootfs image — keeps a hostile artifact from
/// filling the cache disk. Matches the artifact-contract test bound.
pub const MAX_ROOTFS_BYTES: u64 = 64 * 1024 * 1024;

/// ext4 superblock magic in little-endian (0xEF53 at offset 1080).
const EXT4_MAGIC: [u8; 2] = [0x53, 0xEF];

/// Daemon-side rootfs manager.
///
/// Owns a cache directory (typically `~/.vibecli/firecracker/rootfs/`)
/// where verified images live under content-addressed names like
/// `rootfs-<sha256>.ext4`. Callers that already have an image on disk
/// (built locally by `make rootfs-firecracker`, or downloaded from a
/// release) hand the path to `install`; subsequent boots resolve via
/// `cached(sha)`.
#[derive(Debug, Clone)]
pub struct RootfsManager {
    cache_dir: PathBuf,
}

impl RootfsManager {
    /// Create a manager rooted at `cache_dir`. The directory is created
    /// on first use; this constructor does no I/O.
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
        }
    }

    /// Cache directory path (for diagnostics / `vibecli doctor`).
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Compute the SHA256 of `path`'s bytes. Streams in 64 KB blocks
    /// so a 20 MiB image doesn't allocate 20 MiB.
    pub fn compute_sha256(&self, path: &Path) -> Result<String, RootfsError> {
        let mut f = fs::File::open(path).map_err(|_| RootfsError::NotFound(path.to_path_buf()))?;
        let mut hasher = Sha256Stream::new();
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(hasher.hex())
    }

    /// Read the `<path>.sha256` sidecar; whitespace-trimmed lower-hex.
    pub fn read_sidecar(&self, path: &Path) -> Result<String, RootfsError> {
        let mut sidecar = path.as_os_str().to_owned();
        sidecar.push(".sha256");
        let sidecar = PathBuf::from(sidecar);
        if !sidecar.exists() {
            return Err(RootfsError::SidecarMissing(path.to_path_buf()));
        }
        let raw = fs::read_to_string(&sidecar)?;
        // sha256sum format is "<hex>  <name>" — keep only the first field.
        let hex = raw.split_whitespace().next().unwrap_or("").to_lowercase();
        Ok(hex)
    }

    /// Verify an image: ext4 magic + non-empty + ≤ ceiling + matches
    /// the expected SHA256 (either explicit or from sidecar).
    pub fn verify(
        &self,
        path: &Path,
        expected_sha: Option<&str>,
    ) -> Result<RootfsVerification, RootfsError> {
        // 1. Exists + size sanity
        let meta = fs::metadata(path).map_err(|_| RootfsError::NotFound(path.to_path_buf()))?;
        let size = meta.len();
        if size == 0 {
            return Err(RootfsError::Empty(path.to_path_buf()));
        }
        if size > MAX_ROOTFS_BYTES {
            return Err(RootfsError::TooLarge {
                path: path.to_path_buf(),
                actual_bytes: size,
                limit_bytes: MAX_ROOTFS_BYTES,
            });
        }

        // 2. ext4 superblock magic at offset 1080.
        let mut f = fs::File::open(path)?;
        let mut header = [0u8; 1088];
        f.read_exact(&mut header)
            .map_err(|_| RootfsError::NotExt4(path.to_path_buf()))?;
        if header[1080..1082] != EXT4_MAGIC {
            return Err(RootfsError::NotExt4(path.to_path_buf()));
        }

        // 3. SHA256 — fall through to sidecar if no explicit expected.
        let expected: String = match expected_sha {
            Some(s) => s.trim().to_lowercase(),
            None => self.read_sidecar(path)?,
        };
        let actual = self.compute_sha256(path)?;
        if expected != actual {
            return Err(RootfsError::HashMismatch { expected, actual });
        }

        Ok(RootfsVerification {
            sha256: actual,
            size_bytes: size,
        })
    }

    /// Install a verified image into the cache under a
    /// content-addressed name. Idempotent: re-installing the same
    /// SHA is a no-op.
    ///
    /// Returns the path inside the cache.
    pub fn install(
        &self,
        source: &Path,
        expected_sha: Option<&str>,
    ) -> Result<PathBuf, RootfsError> {
        let v = self.verify(source, expected_sha)?;
        fs::create_dir_all(&self.cache_dir).map_err(|e| RootfsError::CacheInit(e.to_string()))?;
        let dst = self.cache_path(&v.sha256);
        if dst.exists() {
            // Already cached — sanity-check the bytes still match.
            let cached_sha = self.compute_sha256(&dst)?;
            if cached_sha == v.sha256 {
                return Ok(dst);
            }
            // Stale / corrupted cache entry — overwrite atomically.
            fs::remove_file(&dst).ok();
        }
        // Atomic install: copy to .tmp then rename.
        let tmp = dst.with_extension("tmp");
        let mut r = fs::File::open(source)?;
        let mut w = fs::File::create(&tmp)?;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = r.read(&mut buf)?;
            if n == 0 {
                break;
            }
            w.write_all(&buf[..n])?;
        }
        w.sync_all()?;
        drop(w);
        fs::rename(&tmp, &dst)?;
        Ok(dst)
    }

    /// Resolve a previously-installed image by its SHA256. Returns
    /// `Some(path)` if cached, `None` if not.
    pub fn cached(&self, sha: &str) -> Option<PathBuf> {
        let p = self.cache_path(&sha.to_lowercase());
        if p.exists() {
            Some(p)
        } else {
            None
        }
    }

    /// Best-effort cosign keyless verification via subprocess.
    ///
    /// Looks for the `cosign` binary on PATH. If absent, returns
    /// `Ok(false)` and the caller can decide how strict to be (CI
    /// presence of cosign is a separate concern from runtime
    /// requirement — many enterprise deployments will skip it).
    ///
    /// Returns `Ok(true)` only when cosign was found AND verification
    /// succeeded; `Ok(false)` when cosign is absent; `Err` when cosign
    /// ran but refused the bundle.
    pub fn verify_cosign(&self, image: &Path, bundle: &Path) -> Result<bool, RootfsError> {
        let cosign = match which("cosign") {
            Some(p) => p,
            None => return Ok(false),
        };
        if !bundle.exists() {
            return Err(RootfsError::CosignFailed(format!(
                "bundle file not found: {}",
                bundle.display()
            )));
        }
        let out = std::process::Command::new(&cosign)
            .arg("verify-blob")
            .arg("--bundle")
            .arg(bundle)
            .arg(image)
            .output()
            .map_err(|e| RootfsError::CosignFailed(format!("spawn cosign: {}", e)))?;
        if out.status.success() {
            Ok(true)
        } else {
            Err(RootfsError::CosignFailed(
                String::from_utf8_lossy(&out.stderr).into_owned(),
            ))
        }
    }

    fn cache_path(&self, sha: &str) -> PathBuf {
        self.cache_dir.join(format!("rootfs-{}.ext4", sha))
    }
}

/// Result of a successful `verify` call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootfsVerification {
    pub sha256: String,
    pub size_bytes: u64,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Find an executable on `PATH`. Returns the resolved path.
fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let cand = dir.join(name);
        if cand.is_file() {
            return Some(cand);
        }
        #[cfg(target_os = "windows")]
        {
            let exe = dir.join(format!("{}.exe", name));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

/// Minimal in-tree SHA256 streaming hasher. Avoids a `sha2` workspace
/// dep on the leaf sandbox crate — `vibecli-cli` already has sha2 for
/// SigV4 signing, but this crate stays lean. The implementation is the
/// straight FIPS 180-4 spec; tested against published vectors below.
struct Sha256Stream {
    h: [u32; 8],
    buf: [u8; 64],
    buf_len: usize,
    len_bits: u64,
}

impl Sha256Stream {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    fn new() -> Self {
        Self {
            h: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buf: [0; 64],
            buf_len: 0,
            len_bits: 0,
        }
    }

    fn update(&mut self, mut data: &[u8]) {
        self.len_bits = self.len_bits.wrapping_add((data.len() as u64) * 8);
        if self.buf_len > 0 {
            let take = (64 - self.buf_len).min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == 64 {
                Self::compress(&mut self.h, &self.buf);
                self.buf_len = 0;
            }
        }
        while data.len() >= 64 {
            let mut blk = [0u8; 64];
            blk.copy_from_slice(&data[..64]);
            Self::compress(&mut self.h, &blk);
            data = &data[64..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.buf_len = data.len();
        }
    }

    fn hex(mut self) -> String {
        // Padding: 0x80 then zeros to 56 mod 64, then 8-byte big-endian length.
        let bits = self.len_bits;
        self.buf[self.buf_len] = 0x80;
        self.buf_len += 1;
        if self.buf_len > 56 {
            for b in &mut self.buf[self.buf_len..64] {
                *b = 0;
            }
            Self::compress(&mut self.h, &self.buf);
            self.buf_len = 0;
        }
        for b in &mut self.buf[self.buf_len..56] {
            *b = 0;
        }
        self.buf[56..64].copy_from_slice(&bits.to_be_bytes());
        Self::compress(&mut self.h, &self.buf);
        let mut hex = String::with_capacity(64);
        for word in &self.h {
            hex.push_str(&format!("{:08x}", word));
        }
        hex
    }

    fn compress(h: &mut [u32; 8], block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(Self::K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let mj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(mj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmpdir(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("vibe_rootfs_test_{}_{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn write_ext4_stub(path: &Path, size_bytes: usize) {
        let mut buf = vec![0u8; size_bytes.max(1088)];
        // ext4 magic at offset 1080.
        buf[1080] = 0x53;
        buf[1081] = 0xEF;
        // Add some non-zero content so the image isn't trivially compressible.
        for (i, b) in buf.iter_mut().enumerate().take(2048).skip(1088) {
            *b = (i % 256) as u8;
        }
        fs::File::create(path).unwrap().write_all(&buf).unwrap();
    }

    // ── Sha256 — published NIST vectors ──────────────────────────────────

    #[test]
    fn sha256_empty_string() {
        let s = Sha256Stream::new().hex();
        assert_eq!(
            s,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_abc() {
        // NIST FIPS 180-2 Appendix B.1.
        let mut h = Sha256Stream::new();
        h.update(b"abc");
        assert_eq!(
            h.hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_long_string() {
        // 1,000,000 'a's — classic FIPS test vector.
        let mut h = Sha256Stream::new();
        let chunk = vec![b'a'; 10_000];
        for _ in 0..100 {
            h.update(&chunk);
        }
        assert_eq!(
            h.hex(),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    #[test]
    fn sha256_streaming_matches_one_shot() {
        let data: Vec<u8> = (0..200_000u32).map(|i| (i % 256) as u8).collect();
        let mut a = Sha256Stream::new();
        a.update(&data);
        let one_shot = a.hex();
        let mut b = Sha256Stream::new();
        for chunk in data.chunks(123) {
            b.update(chunk);
        }
        assert_eq!(one_shot, b.hex());
    }

    // ── compute_sha256 on a real file ────────────────────────────────────

    #[test]
    fn compute_sha256_of_known_bytes() {
        let dir = tmpdir("compute_sha");
        let p = dir.join("a.bin");
        fs::write(&p, b"abc").unwrap();
        let m = RootfsManager::new(dir.clone());
        let sha = m.compute_sha256(&p).unwrap();
        assert_eq!(
            sha,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn compute_sha256_missing_file_errors() {
        let dir = tmpdir("compute_missing");
        let m = RootfsManager::new(dir.clone());
        let err = m
            .compute_sha256(&dir.join("nope.bin"))
            .expect_err("missing file");
        assert!(matches!(err, RootfsError::NotFound(_)));
    }

    // ── verify ───────────────────────────────────────────────────────────

    #[test]
    fn verify_rejects_empty_image() {
        let dir = tmpdir("verify_empty");
        let p = dir.join("empty.ext4");
        fs::File::create(&p).unwrap();
        let m = RootfsManager::new(dir);
        let err = m
            .verify(&p, Some("00".repeat(32).as_str()))
            .expect_err("empty");
        assert!(matches!(err, RootfsError::Empty(_)));
    }

    #[test]
    fn verify_rejects_oversized_image() {
        let dir = tmpdir("verify_big");
        let p = dir.join("big.ext4");
        // Use a sparse file via set_len so we don't actually write 64+ MiB.
        let f = fs::File::create(&p).unwrap();
        f.set_len(MAX_ROOTFS_BYTES + 1).unwrap();
        let m = RootfsManager::new(dir);
        let err = m
            .verify(&p, Some("00".repeat(32).as_str()))
            .expect_err("too big");
        assert!(matches!(err, RootfsError::TooLarge { .. }));
    }

    #[test]
    fn verify_rejects_non_ext4_image() {
        let dir = tmpdir("verify_notext4");
        let p = dir.join("garbage.ext4");
        // Big enough to not trip the empty check, but no ext4 magic.
        fs::write(&p, vec![0u8; 4096]).unwrap();
        let m = RootfsManager::new(dir);
        let err = m
            .verify(&p, Some("00".repeat(32).as_str()))
            .expect_err("not ext4");
        assert!(matches!(err, RootfsError::NotExt4(_)));
    }

    #[test]
    fn verify_rejects_sha_mismatch() {
        let dir = tmpdir("verify_mismatch");
        let p = dir.join("ok.ext4");
        write_ext4_stub(&p, 8 * 1024);
        let m = RootfsManager::new(dir);
        let err = m
            .verify(&p, Some(&"11".repeat(32)))
            .expect_err("hash mismatch");
        assert!(matches!(err, RootfsError::HashMismatch { .. }));
    }

    #[test]
    fn verify_accepts_correct_sha() {
        let dir = tmpdir("verify_ok");
        let p = dir.join("ok.ext4");
        write_ext4_stub(&p, 8 * 1024);
        let m = RootfsManager::new(dir.clone());
        let sha = m.compute_sha256(&p).unwrap();
        let v = m.verify(&p, Some(&sha)).unwrap();
        assert_eq!(v.sha256, sha);
        assert!(v.size_bytes >= 8 * 1024);
    }

    #[test]
    fn verify_uses_sidecar_when_no_explicit_sha() {
        let dir = tmpdir("verify_sidecar");
        let p = dir.join("rootfs.ext4");
        write_ext4_stub(&p, 8 * 1024);
        let m = RootfsManager::new(dir.clone());
        let sha = m.compute_sha256(&p).unwrap();
        let sidecar = dir.join("rootfs.ext4.sha256");
        // Use sha256sum-style format so we exercise the trim/split logic.
        fs::write(&sidecar, format!("{}  rootfs.ext4\n", sha)).unwrap();
        let v = m.verify(&p, None).unwrap();
        assert_eq!(v.sha256, sha);
    }

    #[test]
    fn verify_missing_sidecar_errors_when_no_explicit() {
        let dir = tmpdir("verify_no_sidecar");
        let p = dir.join("rootfs.ext4");
        write_ext4_stub(&p, 8 * 1024);
        let m = RootfsManager::new(dir);
        let err = m.verify(&p, None).expect_err("no sidecar");
        assert!(matches!(err, RootfsError::SidecarMissing(_)));
    }

    // ── install + cached ─────────────────────────────────────────────────

    #[test]
    fn install_writes_content_addressed_copy() {
        let dir = tmpdir("install");
        let src = dir.join("src.ext4");
        write_ext4_stub(&src, 8 * 1024);
        let m = RootfsManager::new(dir.join("cache"));
        let sha = m.compute_sha256(&src).unwrap();

        let dst = m.install(&src, Some(&sha)).unwrap();
        assert!(dst.exists());
        assert!(
            dst.file_name().unwrap().to_string_lossy().contains(&sha),
            "cache filename should embed sha: {:?}",
            dst.file_name()
        );
        assert_eq!(m.compute_sha256(&dst).unwrap(), sha);
    }

    #[test]
    fn install_is_idempotent() {
        let dir = tmpdir("install_idempotent");
        let src = dir.join("src.ext4");
        write_ext4_stub(&src, 8 * 1024);
        let m = RootfsManager::new(dir.join("cache"));
        let sha = m.compute_sha256(&src).unwrap();

        let dst1 = m.install(&src, Some(&sha)).unwrap();
        let mtime1 = fs::metadata(&dst1).unwrap().modified().unwrap();
        // Sleep enough for a file-system mtime to tick on platforms with
        // 1-second resolution; we want to detect re-copy, not flake.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let dst2 = m.install(&src, Some(&sha)).unwrap();
        let mtime2 = fs::metadata(&dst2).unwrap().modified().unwrap();
        assert_eq!(dst1, dst2);
        assert_eq!(mtime1, mtime2, "second install should not rewrite the file");
    }

    #[test]
    fn cached_returns_some_after_install_none_otherwise() {
        let dir = tmpdir("cached");
        let src = dir.join("src.ext4");
        write_ext4_stub(&src, 8 * 1024);
        let m = RootfsManager::new(dir.join("cache"));
        let sha = m.compute_sha256(&src).unwrap();

        assert!(m.cached(&sha).is_none());
        m.install(&src, Some(&sha)).unwrap();
        assert!(m.cached(&sha).is_some());
        // Case-insensitive resolution.
        assert!(m.cached(&sha.to_uppercase()).is_some());
        // Bogus SHA returns None.
        assert!(m.cached(&"00".repeat(32)).is_none());
    }

    #[test]
    fn install_refuses_corrupt_source() {
        let dir = tmpdir("install_corrupt");
        let src = dir.join("src.ext4");
        write_ext4_stub(&src, 8 * 1024);
        let m = RootfsManager::new(dir.join("cache"));
        // Wrong sha → verify fails → install fails.
        let err = m
            .install(&src, Some(&"22".repeat(32)))
            .expect_err("corrupt");
        assert!(matches!(err, RootfsError::HashMismatch { .. }));
    }

    // ── verify_cosign ────────────────────────────────────────────────────

    #[test]
    fn verify_cosign_returns_false_when_binary_missing() {
        // Test environment scrubs PATH to nothing → cosign not findable.
        let dir = tmpdir("cosign_absent");
        let p = dir.join("img.ext4");
        write_ext4_stub(&p, 8 * 1024);
        let bundle = dir.join("img.ext4.cosign.bundle");
        fs::write(&bundle, "").unwrap();
        let m = RootfsManager::new(dir);

        // Save + restore PATH.
        let saved = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", "");
        }
        let result = m.verify_cosign(&p, &bundle);
        unsafe {
            if let Some(p) = saved {
                std::env::set_var("PATH", p);
            } else {
                std::env::remove_var("PATH");
            }
        }
        assert!(matches!(result, Ok(false)));
    }

    #[test]
    fn verify_cosign_errors_when_bundle_missing_even_with_path() {
        // We can only run this branch if cosign is actually on PATH
        // (test environment dependent). Skip otherwise.
        if which("cosign").is_none() {
            eprintln!("[skip] cosign not on PATH — skipping cosign branch test");
            return;
        }
        let dir = tmpdir("cosign_no_bundle");
        let p = dir.join("img.ext4");
        write_ext4_stub(&p, 8 * 1024);
        let bundle = dir.join("img.ext4.cosign.bundle"); // does NOT exist
        let m = RootfsManager::new(dir);
        let err = m.verify_cosign(&p, &bundle).expect_err("no bundle");
        assert!(matches!(err, RootfsError::CosignFailed(_)));
    }
}
