#![allow(dead_code)] // Staged wave6 / Phase 53 module — wired up in a later cycle
//! MCPB bundle format — local distribution package for MCP servers.
//!
//! Phase 53 P0 (A2 from v13 fitgap, MCP 2026 roadmap). MCPB is to MCP
//! servers what `.vsix` is to VS Code extensions: a single
//! self-contained file the user can install with one command. The on-
//! disk shape is a ZIP archive containing a `manifest.json` at the
//! root plus arbitrary additional files (binary, scripts, README,
//! etc.).
//!
//! Manifest shape (intentionally a strict subset of what hosts can
//! consume; hosts that need richer fields can extend without
//! breaking older bundles since unknown fields are preserved during
//! pack but ignored during extract):
//!
//! ```json
//! {
//!   "name":        "filesystem",
//!   "version":     "1.2.0",
//!   "command":     "node",
//!   "args":        ["server.js"],
//!   "env":         { "ALLOWED_PATHS": "/tmp" },
//!   "description": "filesystem MCP server"
//! }
//! ```
//!
//! `compute_manifest_digest` returns a SHA-256 hex digest of
//! `manifest.json` bytes — used by `vibecli mcp install <bundle>` to
//! pin the manifest the user reviewed against subsequent extracts.
//!
//! Implementation note: archive entries are stored without compression
//! (`CompressionMethod::Stored`) so the manifest digest stays stable
//! across zip implementations / versions. Bundles are typically tiny
//! (a few hundred KB) and compression here costs more in
//! reproducibility than it saves in size.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BundleManifest {
    pub name: String,
    pub version: String,
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Pack `src_dir` (which must contain a `manifest.json`) into an MCPB
/// archive at `dest_path`. Returns the parsed manifest for the
/// caller's convenience (so they don't immediately re-open the bundle).
pub fn pack_bundle(src_dir: &Path, dest_path: &Path) -> Result<BundleManifest> {
    let manifest_path = src_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(anyhow!(
            "pack_bundle: manifest.json missing from {}",
            src_dir.display()
        ));
    }
    let manifest_bytes = std::fs::read(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest: BundleManifest = serde_json::from_slice(&manifest_bytes)
        .with_context(|| format!("parse {}", manifest_path.display()))?;

    let f = std::fs::File::create(dest_path)
        .with_context(|| format!("create {}", dest_path.display()))?;
    let mut zw = zip::ZipWriter::new(f);
    let opts: zip::write::SimpleFileOptions =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Walk src_dir, adding every file. Sort entries by relative path
    // so the archive layout is deterministic.
    let entries = walk(src_dir)?;
    for rel in entries {
        let abs = src_dir.join(&rel);
        if !abs.is_file() {
            continue;
        }
        let body = std::fs::read(&abs).with_context(|| format!("read {}", abs.display()))?;
        let name = rel.to_string_lossy().replace('\\', "/");
        zw.start_file(name, opts).context("start_file")?;
        zw.write_all(&body).context("write_all")?;
    }
    zw.finish().context("zip finish")?;
    Ok(manifest)
}

/// Extract `bundle_path` into `dest_dir` and return the parsed manifest.
pub fn extract_bundle(bundle_path: &Path, dest_dir: &Path) -> Result<BundleManifest> {
    let f = std::fs::File::open(bundle_path)
        .with_context(|| format!("open {}", bundle_path.display()))?;
    let mut zr = zip::ZipArchive::new(f).context("ZipArchive::new")?;

    std::fs::create_dir_all(dest_dir)
        .with_context(|| format!("create_dir_all {}", dest_dir.display()))?;

    let mut manifest_bytes: Option<Vec<u8>> = None;
    for i in 0..zr.len() {
        let mut entry = zr.by_index(i).context("zip entry")?;
        let name = entry.name().to_string();
        let safe = sanitise_archive_path(&name)?;
        let out_path = dest_dir.join(&safe);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)
                .with_context(|| format!("create_dir_all {}", out_path.display()))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create_dir_all {}", parent.display()))?;
        }
        let mut body = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut body).context("read entry")?;
        if name == "manifest.json" {
            manifest_bytes = Some(body.clone());
        }
        std::fs::write(&out_path, &body)
            .with_context(|| format!("write {}", out_path.display()))?;
    }

    let bytes = manifest_bytes
        .ok_or_else(|| anyhow!("extract_bundle: manifest.json missing from archive"))?;
    serde_json::from_slice(&bytes).context("parse manifest.json")
}

/// SHA-256 hex digest of the bundle's `manifest.json` content.
pub fn compute_manifest_digest(bundle_path: &Path) -> Result<String> {
    let f = std::fs::File::open(bundle_path)
        .with_context(|| format!("open {}", bundle_path.display()))?;
    let mut zr = zip::ZipArchive::new(f).context("ZipArchive::new")?;
    let mut entry = zr
        .by_name("manifest.json")
        .context("manifest.json not in archive")?;
    let mut bytes = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut bytes).context("read manifest")?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("{:x}", digest))
}

/// List entries in a bundle without extracting.
pub fn list_bundle_entries(bundle_path: &Path) -> Result<Vec<PathBuf>> {
    let f = std::fs::File::open(bundle_path)
        .with_context(|| format!("open {}", bundle_path.display()))?;
    let mut zr = zip::ZipArchive::new(f).context("ZipArchive::new")?;
    let mut out: Vec<PathBuf> = Vec::with_capacity(zr.len());
    for i in 0..zr.len() {
        let entry = zr.by_index(i).context("zip entry")?;
        out.push(PathBuf::from(entry.name()));
    }
    out.sort();
    Ok(out)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn walk(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    walk_inner(dir, dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn walk_inner(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_inner(root, &path, out)?;
        } else {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}

/// Reject zip-slip style paths (`../`, absolute paths). Returns the
/// path unchanged if safe.
fn sanitise_archive_path(name: &str) -> Result<PathBuf> {
    let p = PathBuf::from(name);
    for comp in p.components() {
        match comp {
            std::path::Component::ParentDir => {
                return Err(anyhow!("archive entry contains '..': {}", name));
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err(anyhow!("archive entry is absolute: {}", name));
            }
            _ => {}
        }
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn fixture_manifest() -> BundleManifest {
        BundleManifest {
            name: "filesystem".into(),
            version: "1.2.0".into(),
            command: "node".into(),
            args: vec!["server.js".into()],
            env: [("ALLOWED_PATHS".to_string(), "/tmp".to_string())]
                .into_iter()
                .collect(),
            description: Some("filesystem MCP server".into()),
        }
    }

    fn make_src(dir: &Path) {
        let manifest = serde_json::to_string_pretty(&fixture_manifest()).unwrap();
        fs::write(dir.join("manifest.json"), manifest).unwrap();
        fs::write(dir.join("server.js"), "// noop").unwrap();
        fs::create_dir(dir.join("lib")).unwrap();
        fs::write(dir.join("lib/util.js"), "// util").unwrap();
    }

    #[test]
    fn pack_then_extract_round_trips_manifest_and_files() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();
        let dest_extract = tempdir().unwrap();
        make_src(src.path());

        let bundle = out.path().join("filesystem-1.2.0.mcpb");
        let packed = pack_bundle(src.path(), &bundle).unwrap();
        assert_eq!(packed, fixture_manifest());
        assert!(bundle.is_file());

        let extracted = extract_bundle(&bundle, dest_extract.path()).unwrap();
        assert_eq!(extracted, fixture_manifest());
        assert!(dest_extract.path().join("manifest.json").is_file());
        assert!(dest_extract.path().join("server.js").is_file());
        assert!(dest_extract.path().join("lib/util.js").is_file());
    }

    #[test]
    fn pack_fails_when_manifest_json_missing() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();
        fs::write(src.path().join("server.js"), "// noop").unwrap();
        let bundle = out.path().join("bad.mcpb");
        let res = pack_bundle(src.path(), &bundle);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("manifest.json"));
    }

    #[test]
    fn extract_fails_when_bundle_missing_manifest() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();
        let dest = tempdir().unwrap();
        fs::write(src.path().join("server.js"), "// noop").unwrap();
        // Build a bundle that intentionally lacks manifest.json by
        // calling the writer directly with raw zip — we test extract
        // robustness against a malformed bundle.
        let bundle = out.path().join("no-manifest.mcpb");
        let f = fs::File::create(&bundle).unwrap();
        let mut zw = zip::ZipWriter::new(f);
        let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        use std::io::Write;
        zw.start_file("server.js", opts).unwrap();
        zw.write_all(b"// noop").unwrap();
        zw.finish().unwrap();

        let res = extract_bundle(&bundle, dest.path());
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("manifest.json"));
    }

    #[test]
    fn compute_manifest_digest_is_stable_across_packs_of_same_input() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();
        make_src(src.path());

        let b1 = out.path().join("a.mcpb");
        let b2 = out.path().join("b.mcpb");
        pack_bundle(src.path(), &b1).unwrap();
        pack_bundle(src.path(), &b2).unwrap();

        let d1 = compute_manifest_digest(&b1).unwrap();
        let d2 = compute_manifest_digest(&b2).unwrap();
        assert_eq!(d1, d2, "same manifest input must produce same digest");
        assert_eq!(d1.len(), 64, "SHA-256 hex is 64 chars; got {}", d1);
    }

    #[test]
    fn compute_manifest_digest_changes_when_manifest_changes() {
        let src1 = tempdir().unwrap();
        let src2 = tempdir().unwrap();
        let out = tempdir().unwrap();
        make_src(src1.path());
        make_src(src2.path());

        // Mutate src2's manifest version.
        let mut m = fixture_manifest();
        m.version = "9.9.9".into();
        fs::write(
            src2.path().join("manifest.json"),
            serde_json::to_string_pretty(&m).unwrap(),
        )
        .unwrap();

        let b1 = out.path().join("v1.mcpb");
        let b2 = out.path().join("v2.mcpb");
        pack_bundle(src1.path(), &b1).unwrap();
        pack_bundle(src2.path(), &b2).unwrap();

        let d1 = compute_manifest_digest(&b1).unwrap();
        let d2 = compute_manifest_digest(&b2).unwrap();
        assert_ne!(d1, d2, "different manifest must produce different digest");
    }

    #[test]
    fn list_bundle_entries_returns_sorted_paths() {
        let src = tempdir().unwrap();
        let out = tempdir().unwrap();
        make_src(src.path());

        let bundle = out.path().join("fs.mcpb");
        pack_bundle(src.path(), &bundle).unwrap();

        let entries = list_bundle_entries(&bundle).unwrap();
        let names: Vec<String> = entries
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        // Sorted lexicographically; manifest.json first because of
        // the 'm' < 's' ordering, but more importantly the listing
        // is deterministic.
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "entries must come back sorted");
        assert!(names.iter().any(|n| n == "manifest.json"));
        assert!(names.iter().any(|n| n == "server.js"));
        assert!(names.iter().any(|n| n.contains("util.js")));
    }
}
