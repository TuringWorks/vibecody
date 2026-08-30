//! Read-only browsing of archive files, plus extraction when the user wants to
//! edit something inside one.
//!
//! The explorer addresses a file inside an archive with a *virtual path*:
//! the archive's real path, the separator `!/`, then the member's path inside
//! the archive (always `/`-separated, never leading-slashed):
//!
//! ```text
//! /home/me/proj/dist.zip!/dist/index.js
//! ```
//!
//! The separator is the one Java has used for `jar:` URLs since 1997, which is
//! why it is spelled that way here: `!` cannot appear in a Windows path at all,
//! and a POSIX path containing `!/` is rare enough that treating it as a member
//! reference is the better trade. `split_virtual` resolves the ambiguity by
//! checking the *left* side against the extension table — `a!/b.zip` is a plain
//! path, `a.zip!/b` is a member.
//!
//! Everything here is read-only except [`extract_to`], which is the single
//! escape hatch: writing back into an archive in place would have to re-encode
//! containers we do not fully understand (see `vibe_docfmt::zipedit` for what
//! that costs even when we do), so an edit becomes an extraction instead.
//!
//! ## Bounds
//!
//! Archive headers are attacker-controlled input in the same sense a
//! `Content-Length` is: a member can *declare* 8 GiB and a 40 KiB file can
//! expand to a terabyte. So no allocation here is ever sized from a declared
//! size — every read goes through [`read_bounded`], which caps by reading, and
//! extraction stops on entry count and cumulative output bytes as well.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

/// Separates the archive path from the member path inside it.
pub const SEPARATOR: &str = "!/";

/// Largest single member we will decode into memory for the editor (64 MiB).
/// Well past any source file, well short of anything that hurts a desktop app.
pub const MAX_MEMBER_BYTES: u64 = 64 * 1024 * 1024;

/// Largest total output an extraction may produce (2 GiB).
pub const MAX_EXTRACT_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Largest number of members we will index or extract.
pub const MAX_ENTRIES: usize = 200_000;

/// Largest compressed archive we will stream-index in full. Only applies to
/// the tar family — a zip's central directory is read without touching the
/// member bodies, so a 40 GiB zip still lists instantly.
pub const MAX_TAR_SCAN_BYTES: u64 = 8 * 1024 * 1024 * 1024;

// ── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("not a recognized archive: {0}")]
    NotAnArchive(String),
    #[error("no such entry in archive: {0}")]
    NoSuchEntry(String),
    #[error("{what} is {actual} bytes, over the {limit} byte limit")]
    TooLarge {
        what: String,
        actual: u64,
        limit: u64,
    },
    #[error("archive has more than {0} entries")]
    TooManyEntries(usize),
    #[error("unsafe entry path in archive: {0}")]
    UnsafePath(String),
    #[error("destination already exists: {0}")]
    DestinationExists(PathBuf),
    #[error("{0}")]
    Corrupt(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

type Result<T> = std::result::Result<T, ArchiveError>;

impl From<zip::result::ZipError> for ArchiveError {
    fn from(e: zip::result::ZipError) -> Self {
        ArchiveError::Corrupt(e.to_string())
    }
}

// ── Kinds ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Compression {
    None,
    Gzip,
    Bzip2,
    Zstd,
    Xz,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArchiveKind {
    /// PKZIP container — `.zip` and the dozen formats that are a zip wearing a
    /// different extension (`.jar`, `.whl`, `.vsix`, …).
    Zip,
    /// A tar, optionally compressed as a whole.
    Tar(Compression),
    /// One compressed file, no container: `server.log.gz`. Browsing it shows a
    /// single member named after the archive with the compression suffix cut.
    Single(Compression),
}

/// Extensions that are a zip container. DOCX / XLSX / PPTX / ODT / EPUB are
/// deliberately absent: they are zips, but the editor already opens them as
/// documents, and having a `.docx` expand into a tree of XML parts instead of
/// rendering would be a regression, not a feature.
const ZIP_EXTENSIONS: &[&str] = &[
    "zip", "jar", "war", "ear", "apk", "aab", "ipa", "whl", "egg", "vsix", "xpi", "nupkg", "zipx",
    "maff", "sketch",
];

/// `(suffix, kind)`, longest suffix first — `.tar.gz` has to be tested before
/// `.gz` or every tarball would browse as a single member holding a tar.
const SUFFIX_TABLE: &[(&str, ArchiveKind)] = &[
    (".tar.gz", ArchiveKind::Tar(Compression::Gzip)),
    (".tar.bz2", ArchiveKind::Tar(Compression::Bzip2)),
    (".tar.zst", ArchiveKind::Tar(Compression::Zstd)),
    (".tar.zstd", ArchiveKind::Tar(Compression::Zstd)),
    (".tar.xz", ArchiveKind::Tar(Compression::Xz)),
    (".tgz", ArchiveKind::Tar(Compression::Gzip)),
    (".tbz", ArchiveKind::Tar(Compression::Bzip2)),
    (".tbz2", ArchiveKind::Tar(Compression::Bzip2)),
    (".tzst", ArchiveKind::Tar(Compression::Zstd)),
    (".txz", ArchiveKind::Tar(Compression::Xz)),
    (".tar", ArchiveKind::Tar(Compression::None)),
    (".gz", ArchiveKind::Single(Compression::Gzip)),
    (".bz2", ArchiveKind::Single(Compression::Bzip2)),
    (".zst", ArchiveKind::Single(Compression::Zstd)),
    (".zstd", ArchiveKind::Single(Compression::Zstd)),
    (".xz", ArchiveKind::Single(Compression::Xz)),
];

/// Classify a file *by name*. Nothing is opened, so this is safe to call for
/// every row of a directory listing.
pub fn archive_kind(name: &str) -> Option<ArchiveKind> {
    let lower = name.rsplit(['/', '\\']).next().unwrap_or(name).to_lowercase();
    // A dotfile with no other extension (`.gz`) is a name, not a suffix.
    let stem_len = lower.len();
    for (suffix, kind) in SUFFIX_TABLE {
        if lower.ends_with(suffix) && stem_len > suffix.len() {
            return Some(*kind);
        }
    }
    let ext = lower.rsplit_once('.').map(|(_, e)| e)?;
    ZIP_EXTENSIONS
        .contains(&ext)
        .then_some(ArchiveKind::Zip)
        .filter(|_| stem_len > ext.len() + 1)
}

/// Whether the explorer should offer this file as an expandable node.
pub fn is_archive_file(name: &str) -> bool {
    archive_kind(name).is_some()
}

/// The archive's name with its archive extension removed — the folder an
/// extraction creates. `dist.tar.gz` → `dist`, `plugin.vsix` → `plugin`.
pub fn strip_archive_extension(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let lower = base.to_lowercase();
    for (suffix, _) in SUFFIX_TABLE {
        if lower.ends_with(suffix) && lower.len() > suffix.len() {
            return base[..base.len() - suffix.len()].to_string();
        }
    }
    match base.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() && ZIP_EXTENSIONS.contains(&ext.to_lowercase().as_str()) => {
            stem.to_string()
        }
        _ => base.to_string(),
    }
}

// ── Virtual paths ────────────────────────────────────────────────────────────

/// Split `archive!/inner` into its two halves, or `None` if `path` is an
/// ordinary path. The left half must itself name an archive, so a real
/// directory that happens to contain `!` does not become a member reference.
///
/// Nested archives resolve to the *outermost* archive we can actually open, so
/// `a.zip!/b.jar!/C.class` splits at the last separator whose left side is a
/// real file on disk; when nothing on disk matches, the first one wins.
pub fn split_virtual(path: &str) -> Option<(&str, &str)> {
    let mut found: Option<(&str, &str)> = None;
    let mut offset = 0usize;
    while let Some(at) = path[offset..].find(SEPARATOR) {
        let cut = offset + at;
        let (left, right) = (&path[..cut], &path[cut + SEPARATOR.len()..]);
        if is_archive_file(left) {
            let hit = (left, right);
            // The first archive-looking prefix that exists on disk is the real
            // container; anything past it is a member path of that container.
            if Path::new(left).is_file() {
                return Some(hit);
            }
            found.get_or_insert(hit);
        }
        offset = cut + SEPARATOR.len();
    }
    found
}

/// Build the virtual path for `inner` inside `archive`.
pub fn join_virtual(archive: &str, inner: &str) -> String {
    format!("{archive}{SEPARATOR}{}", inner.trim_start_matches('/'))
}

/// The on-disk file backing a path that may be virtual.
pub fn container_of(path: &str) -> &str {
    split_virtual(path).map(|(a, _)| a).unwrap_or(path)
}

// ── Index ────────────────────────────────────────────────────────────────────

/// One member of an archive, as the index sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveMember {
    /// `/`-separated path inside the archive, no leading slash, no trailing
    /// slash even for directories.
    pub path: String,
    pub is_dir: bool,
    /// Uncompressed size as *declared* by the archive. Advisory only — nothing
    /// in this module allocates from it.
    pub size: Option<u64>,
}

/// A listing row for one level of the tree, in the shape the explorer wants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub name: String,
    /// Virtual path — `archive!/inner`.
    pub path: String,
    pub is_directory: bool,
    pub size: Option<u64>,
}

/// Normalize a member path as stored in the archive: `/`-separated, no `./`,
/// no leading or trailing slash. Returns `None` when the path escapes the
/// archive root (`..`, absolute, or a Windows drive prefix) — a zip-slip
/// attempt, and the same check that guards extraction.
fn safe_member_path(raw: &str) -> Option<String> {
    let unified = raw.replace('\\', "/");
    let mut parts: Vec<&str> = Vec::new();
    for segment in unified.split('/') {
        match segment {
            "" | "." => continue,
            ".." => return None,
            s if s.contains(':') && parts.is_empty() && s.len() >= 2 => return None,
            s => parts.push(s),
        }
    }
    if unified.starts_with('/') || parts.is_empty() {
        return None;
    }
    Some(parts.join("/"))
}

/// Every member of an archive, cached per (path, mtime, size).
pub fn read_index(archive: &Path) -> Result<Arc<Vec<ArchiveMember>>> {
    let stamp = FileStamp::of(archive)?;
    if let Some(hit) = INDEX_CACHE.get(archive, &stamp) {
        return Ok(hit);
    }
    let name = archive
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let kind = archive_kind(&name).ok_or_else(|| ArchiveError::NotAnArchive(name.clone()))?;
    let members = Arc::new(match kind {
        ArchiveKind::Zip => zip_index(archive)?,
        ArchiveKind::Tar(comp) => tar_index(archive, comp)?,
        ArchiveKind::Single(_) => vec![ArchiveMember {
            path: strip_archive_extension(&name),
            is_dir: false,
            size: None,
        }],
    });
    INDEX_CACHE.put(archive, stamp, Arc::clone(&members));
    Ok(members)
}

fn zip_index(archive: &Path) -> Result<Vec<ArchiveMember>> {
    let mut zip = zip::ZipArchive::new(BufReader::new(File::open(archive)?))?;
    if zip.len() > MAX_ENTRIES {
        return Err(ArchiveError::TooManyEntries(MAX_ENTRIES));
    }
    // A zip's directory bit is `name ends with /`, and plenty of writers emit
    // no directory entries at all — `synthesize_dirs` fills those in, so both
    // shapes browse the same.
    let members = (0..zip.len())
        .filter_map(|i| {
            let file = zip.by_index_raw(i).ok()?;
            let is_dir = file.is_dir();
            let path = safe_member_path(file.name())?;
            Some(ArchiveMember {
                path,
                is_dir,
                size: (!is_dir).then(|| file.size()),
            })
        })
        .collect();
    Ok(synthesize_dirs(members))
}

fn tar_index(archive: &Path, comp: Compression) -> Result<Vec<ArchiveMember>> {
    let compressed = archive.metadata()?.len();
    if compressed > MAX_TAR_SCAN_BYTES {
        return Err(ArchiveError::TooLarge {
            what: archive.display().to_string(),
            actual: compressed,
            limit: MAX_TAR_SCAN_BYTES,
        });
    }
    let mut tar = tar::Archive::new(decompressing_reader(archive, comp)?);
    let mut members = Vec::new();
    for entry in tar.entries()? {
        let entry = entry.map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
        if members.len() >= MAX_ENTRIES {
            return Err(ArchiveError::TooManyEntries(MAX_ENTRIES));
        }
        let header = entry.header();
        let is_dir = header.entry_type().is_dir();
        // Symlinks, hard links, devices and the GNU/PAX metadata pseudo-entries
        // are not files anyone can open in an editor, and following one would
        // be the tar equivalent of zip-slip. They are simply not listed.
        if !is_dir && !header.entry_type().is_file() {
            continue;
        }
        let raw = entry
            .path()
            .map_err(|e| ArchiveError::Corrupt(e.to_string()))?
            .to_string_lossy()
            .to_string();
        let Some(path) = safe_member_path(&raw) else {
            continue;
        };
        members.push(ArchiveMember {
            path,
            is_dir,
            size: (!is_dir).then(|| header.size().unwrap_or(0)),
        });
    }
    Ok(synthesize_dirs(members))
}

/// Add the ancestor directories that the archive never named, and drop
/// duplicates. Zip and tar both allow a member at `a/b/c.txt` with no entry for
/// `a` or `a/b`; without this the tree would show nothing at the root.
fn synthesize_dirs(members: Vec<ArchiveMember>) -> Vec<ArchiveMember> {
    let mut by_path: HashMap<String, ArchiveMember> = HashMap::with_capacity(members.len() * 2);
    for member in members {
        let mut ancestor = member.path.as_str();
        while let Some((parent, _)) = ancestor.rsplit_once('/') {
            if by_path.contains_key(parent) {
                break;
            }
            by_path.insert(
                parent.to_string(),
                ArchiveMember {
                    path: parent.to_string(),
                    is_dir: true,
                    size: None,
                },
            );
            ancestor = parent;
        }
        // A real entry always beats a synthesized one.
        by_path.insert(member.path.clone(), member);
    }
    let mut out: Vec<ArchiveMember> = by_path.into_values().collect();
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// The children of `inner` (the empty string = the archive's root), sorted
/// directories-first then by name, matching the on-disk explorer.
pub fn list_dir(archive: &Path, inner: &str) -> Result<Vec<ArchiveEntry>> {
    let members = read_index(archive)?;
    let prefix = inner.trim_matches('/');
    let container = archive.to_string_lossy().to_string();
    let mut rows: Vec<ArchiveEntry> = members
        .iter()
        .filter_map(|member| {
            let rest = if prefix.is_empty() {
                member.path.as_str()
            } else {
                member
                    .path
                    .strip_prefix(prefix)
                    .and_then(|r| r.strip_prefix('/'))?
            };
            // Direct children only — anything with a slash left belongs to a
            // deeper level and will be listed when that level is expanded.
            (!rest.is_empty() && !rest.contains('/')).then(|| ArchiveEntry {
                name: rest.to_string(),
                path: join_virtual(&container, &member.path),
                is_directory: member.is_dir,
                size: member.size,
            })
        })
        .collect();
    if rows.is_empty() && !prefix.is_empty() && !members.iter().any(|m| m.path == prefix) {
        return Err(ArchiveError::NoSuchEntry(prefix.to_string()));
    }
    rows.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(rows)
}

// ── Reading a member ─────────────────────────────────────────────────────────

/// Read at most `limit` bytes, and fail rather than truncate past it.
///
/// This is the whole defense against a declared size that lies: the cap is
/// enforced *by reading*, so a member claiming 8 GiB costs 64 MiB and an error,
/// not an 8 GiB allocation. `take(limit + 1)` is what makes "exactly at the
/// limit" distinguishable from "over it".
fn read_bounded(reader: &mut impl Read, limit: u64, what: &str) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let read = reader.take(limit + 1).read_to_end(&mut buf)? as u64;
    if read > limit {
        return Err(ArchiveError::TooLarge {
            what: what.to_string(),
            actual: read,
            limit,
        });
    }
    Ok(buf)
}

/// The bytes of one member. Capped at [`MAX_MEMBER_BYTES`].
pub fn read_member(archive: &Path, inner: &str) -> Result<Vec<u8>> {
    let name = archive
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let kind = archive_kind(&name).ok_or_else(|| ArchiveError::NotAnArchive(name.clone()))?;
    let wanted =
        safe_member_path(inner).ok_or_else(|| ArchiveError::UnsafePath(inner.to_string()))?;
    match kind {
        ArchiveKind::Zip => {
            let mut zip = zip::ZipArchive::new(BufReader::new(File::open(archive)?))?;
            let index = (0..zip.len())
                .find(|&i| {
                    zip.by_index_raw(i)
                        .ok()
                        .and_then(|f| safe_member_path(f.name()))
                        .as_deref()
                        == Some(wanted.as_str())
                })
                .ok_or_else(|| ArchiveError::NoSuchEntry(wanted.clone()))?;
            let mut file = zip.by_index(index)?;
            read_bounded(&mut file, MAX_MEMBER_BYTES, &wanted)
        }
        ArchiveKind::Tar(comp) => {
            let mut tar = tar::Archive::new(decompressing_reader(archive, comp)?);
            for entry in tar.entries()? {
                let mut entry = entry.map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
                let raw = entry
                    .path()
                    .map_err(|e| ArchiveError::Corrupt(e.to_string()))?
                    .to_string_lossy()
                    .to_string();
                if safe_member_path(&raw).as_deref() == Some(wanted.as_str()) {
                    return read_bounded(&mut entry, MAX_MEMBER_BYTES, &wanted);
                }
            }
            Err(ArchiveError::NoSuchEntry(wanted))
        }
        ArchiveKind::Single(comp) => {
            if wanted != strip_archive_extension(&name) {
                return Err(ArchiveError::NoSuchEntry(wanted));
            }
            let mut reader = decompressing_reader(archive, comp)?;
            read_bounded(&mut reader, MAX_MEMBER_BYTES, &wanted)
        }
    }
}

/// A `Read` over the archive's decompressed bytes.
///
/// Every codec here streams except xz: `lzma-rs` decodes to a `Write` in one
/// shot, so that branch buffers — bounded by [`MAX_EXTRACT_BYTES`] through
/// [`BoundedWriter`], which is why an xz bomb fails with an error instead of
/// taking the process with it.
fn decompressing_reader(archive: &Path, comp: Compression) -> Result<Box<dyn Read>> {
    let file = BufReader::new(File::open(archive)?);
    Ok(match comp {
        Compression::None => Box::new(file),
        Compression::Gzip => Box::new(flate2::read::MultiGzDecoder::new(file)),
        Compression::Bzip2 => Box::new(bzip2::read::MultiBzDecoder::new(file)),
        Compression::Zstd => Box::new(zstd::stream::read::Decoder::new(file)?),
        Compression::Xz => {
            let mut input = file;
            let mut out = BoundedWriter::new(MAX_EXTRACT_BYTES, archive.display().to_string());
            lzma_rs::xz_decompress(&mut input, &mut out)
                .map_err(|e| ArchiveError::Corrupt(format!("xz: {e}")))?;
            Box::new(Cursor::new(out.into_inner()))
        }
    })
}

/// A `Write` that refuses to grow past a byte budget.
struct BoundedWriter {
    buf: Vec<u8>,
    limit: u64,
    what: String,
}

impl BoundedWriter {
    fn new(limit: u64, what: String) -> Self {
        Self {
            buf: Vec::new(),
            limit,
            what,
        }
    }
    fn into_inner(self) -> Vec<u8> {
        self.buf
    }
}

impl Write for BoundedWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.buf.len() as u64 + data.len() as u64 > self.limit {
            return Err(io::Error::other(format!(
                "{} expands past the {} byte limit",
                self.what, self.limit
            )));
        }
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// ── Extraction ───────────────────────────────────────────────────────────────

/// What an extraction produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractOutcome {
    /// The folder that now holds the archive's contents.
    pub dest: PathBuf,
    pub files: usize,
    pub directories: usize,
    /// Entries left behind: symlinks, devices, and anything whose path tried to
    /// escape the destination.
    pub skipped: usize,
    pub bytes: u64,
}

/// The folder an extraction of `archive` would create: a sibling named after
/// the archive with its extension removed, suffixed `-1`, `-2`, … if taken.
pub fn extraction_dir(archive: &Path) -> PathBuf {
    let parent = archive.parent().unwrap_or(Path::new("."));
    let stem = strip_archive_extension(&archive.file_name().unwrap_or_default().to_string_lossy());
    let base = parent.join(&stem);
    if !base.exists() {
        return base;
    }
    (1u32..10_000)
        .map(|n| parent.join(format!("{stem}-{n}")))
        .find(|candidate| !candidate.exists())
        .unwrap_or(base)
}

/// Extract `archive` into `dest` (default: [`extraction_dir`]).
///
/// The destination must not already exist — this never merges into a populated
/// folder, because "extract to edit" silently overwriting a previous extraction
/// is how someone loses the edits they made last time.
pub fn extract_to(archive: &Path, dest: Option<&Path>) -> Result<ExtractOutcome> {
    let dest = dest.map(Path::to_path_buf).unwrap_or_else(|| extraction_dir(archive));
    if dest.exists() {
        return Err(ArchiveError::DestinationExists(dest));
    }
    let name = archive
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let kind = archive_kind(&name).ok_or_else(|| ArchiveError::NotAnArchive(name.clone()))?;
    std::fs::create_dir_all(&dest)?;

    let mut outcome = ExtractOutcome {
        dest: dest.clone(),
        files: 0,
        directories: 0,
        skipped: 0,
        bytes: 0,
    };
    let result = match kind {
        ArchiveKind::Zip => extract_zip(archive, &dest, &mut outcome),
        ArchiveKind::Tar(comp) => extract_tar(archive, comp, &dest, &mut outcome),
        ArchiveKind::Single(comp) => {
            let mut reader = decompressing_reader(archive, comp)?;
            let target = dest.join(strip_archive_extension(&name));
            write_member(&mut reader, &target, &mut outcome)
        }
    };
    match result {
        Ok(()) => Ok(outcome),
        Err(e) => {
            // A half-extracted folder is worse than none: the user would open
            // it, find their file missing, and have no way to tell a truncated
            // extraction from an archive that never held the file.
            let _ = std::fs::remove_dir_all(&dest);
            Err(e)
        }
    }
}

fn extract_zip(archive: &Path, dest: &Path, outcome: &mut ExtractOutcome) -> Result<()> {
    let mut zip = zip::ZipArchive::new(BufReader::new(File::open(archive)?))?;
    if zip.len() > MAX_ENTRIES {
        return Err(ArchiveError::TooManyEntries(MAX_ENTRIES));
    }
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        // `enclosed_name` is the zip crate's own zip-slip guard; the local
        // check runs too, because it is the one that also covers tar.
        let Some(relative) = file.enclosed_name().and_then(|p| {
            safe_member_path(&p.to_string_lossy()).map(PathBuf::from)
        }) else {
            outcome.skipped += 1;
            continue;
        };
        let target = dest.join(&relative);
        if !target.starts_with(dest) {
            outcome.skipped += 1;
            continue;
        }
        if file.is_dir() {
            std::fs::create_dir_all(&target)?;
            outcome.directories += 1;
            continue;
        }
        write_member(&mut file, &target, outcome)?;
    }
    Ok(())
}

fn extract_tar(
    archive: &Path,
    comp: Compression,
    dest: &Path,
    outcome: &mut ExtractOutcome,
) -> Result<()> {
    let mut tar = tar::Archive::new(decompressing_reader(archive, comp)?);
    for entry in tar.entries()? {
        let mut entry = entry.map_err(|e| ArchiveError::Corrupt(e.to_string()))?;
        let entry_type = entry.header().entry_type();
        let raw = entry
            .path()
            .map_err(|e| ArchiveError::Corrupt(e.to_string()))?
            .to_string_lossy()
            .to_string();
        let Some(relative) = safe_member_path(&raw) else {
            outcome.skipped += 1;
            continue;
        };
        let target = dest.join(&relative);
        if !target.starts_with(dest) {
            outcome.skipped += 1;
            continue;
        }
        if entry_type.is_dir() {
            std::fs::create_dir_all(&target)?;
            outcome.directories += 1;
            continue;
        }
        // Links and devices are not recreated — see `tar_index`.
        if !entry_type.is_file() {
            outcome.skipped += 1;
            continue;
        }
        write_member(&mut entry, &target, outcome)?;
    }
    Ok(())
}

/// Stream one member to disk, charging its bytes against the extraction budget.
fn write_member(
    reader: &mut impl Read,
    target: &Path,
    outcome: &mut ExtractOutcome,
) -> Result<()> {
    if outcome.files >= MAX_ENTRIES {
        return Err(ArchiveError::TooManyEntries(MAX_ENTRIES));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let budget = MAX_EXTRACT_BYTES - outcome.bytes;
    let mut file = File::create(target)?;
    // `io::copy` on a `take` is the streaming form of `read_bounded`: bounded
    // by how much is read, never by a size the archive claims.
    let written = io::copy(&mut reader.take(budget + 1), &mut file)?;
    if written > budget {
        return Err(ArchiveError::TooLarge {
            what: target.display().to_string(),
            actual: MAX_EXTRACT_BYTES + 1,
            limit: MAX_EXTRACT_BYTES,
        });
    }
    outcome.files += 1;
    outcome.bytes += written;
    Ok(())
}

// ── Index cache ──────────────────────────────────────────────────────────────

/// Identity of a file as far as the cache is concerned. mtime *and* length,
/// because a same-length rewrite within one mtime tick is exactly the shape a
/// rebuild produces.
#[derive(Debug, Clone, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified: Option<SystemTime>,
}

impl FileStamp {
    fn of(path: &Path) -> Result<Self> {
        let meta = path.metadata()?;
        Ok(Self {
            len: meta.len(),
            modified: meta.modified().ok(),
        })
    }
}

/// A tar index costs a full decompression pass, and the explorer asks for one
/// listing per expanded folder — without this, opening five folders in a
/// tarball decompressed it five times. Bounded to a handful of archives; this
/// is a browsing aid, not a store.
const INDEX_CACHE_CAPACITY: usize = 8;

struct IndexCache {
    entries: Mutex<Vec<(PathBuf, FileStamp, Arc<Vec<ArchiveMember>>)>>,
}

impl IndexCache {
    fn get(&self, path: &Path, stamp: &FileStamp) -> Option<Arc<Vec<ArchiveMember>>> {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let at = entries
            .iter()
            .position(|(p, s, _)| p == path && s == stamp)?;
        // Most-recently-used to the front, so the eviction below drops the
        // archive nobody has looked at.
        let hit = entries.remove(at);
        let members = Arc::clone(&hit.2);
        entries.insert(0, hit);
        Some(members)
    }

    fn put(&self, path: &Path, stamp: FileStamp, members: Arc<Vec<ArchiveMember>>) {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.retain(|(p, _, _)| p != path);
        entries.insert(0, (path.to_path_buf(), stamp, members));
        entries.truncate(INDEX_CACHE_CAPACITY);
    }
}

static INDEX_CACHE: LazyLock<IndexCache> = LazyLock::new(|| IndexCache {
    entries: Mutex::new(Vec::new()),
});

/// Drop cached indexes. Called after an extraction so a subsequent listing of
/// a rewritten archive cannot be answered from a stale index.
pub fn clear_index_cache() {
    INDEX_CACHE
        .entries
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

/// Path components that are plain names, for callers that want to validate a
/// destination before handing it to [`extract_to`].
pub fn is_plain_relative(path: &Path) -> bool {
    path.components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zip_fixture(dir: &Path) -> PathBuf {
        let path = dir.join("bundle.zip");
        let file = File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("README.md", opts).unwrap();
        zip.write_all(b"# hello\n").unwrap();
        zip.start_file("src/main.rs", opts).unwrap();
        zip.write_all(b"fn main() {}\n").unwrap();
        zip.start_file("src/util/mod.rs", opts).unwrap();
        zip.write_all(b"pub fn ok() {}\n").unwrap();
        zip.finish().unwrap();
        path
    }

    fn targz_fixture(dir: &Path) -> PathBuf {
        let path = dir.join("bundle.tar.gz");
        let file = File::create(&path).unwrap();
        let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(encoder);
        for (name, body) in [
            ("README.md", &b"# hello\n"[..]),
            ("src/main.rs", &b"fn main() {}\n"[..]),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, name, body).unwrap();
        }
        tar.into_inner().unwrap().finish().unwrap();
        path
    }

    #[test]
    fn recognizes_the_archive_families() {
        assert_eq!(archive_kind("a.zip"), Some(ArchiveKind::Zip));
        assert_eq!(archive_kind("plugin.VSIX"), Some(ArchiveKind::Zip));
        assert_eq!(
            archive_kind("dist.tar.gz"),
            Some(ArchiveKind::Tar(Compression::Gzip))
        );
        assert_eq!(
            archive_kind("dist.tgz"),
            Some(ArchiveKind::Tar(Compression::Gzip))
        );
        assert_eq!(
            archive_kind("server.log.gz"),
            Some(ArchiveKind::Single(Compression::Gzip))
        );
        assert_eq!(archive_kind("main.rs"), None);
        // A document is a zip, but it already has a viewer — see ZIP_EXTENSIONS.
        assert_eq!(archive_kind("report.docx"), None);
        // A bare suffix is a filename, not an archive.
        assert_eq!(archive_kind(".gz"), None);
    }

    #[test]
    fn strips_the_whole_archive_suffix() {
        assert_eq!(strip_archive_extension("dist.tar.gz"), "dist");
        assert_eq!(strip_archive_extension("plugin.vsix"), "plugin");
        assert_eq!(strip_archive_extension("server.log.gz"), "server.log");
        assert_eq!(strip_archive_extension("/tmp/a/b.zip"), "b");
    }

    #[test]
    fn splits_virtual_paths_only_at_a_real_archive() {
        assert_eq!(
            split_virtual("/tmp/a.zip!/src/main.rs"),
            Some(("/tmp/a.zip", "src/main.rs"))
        );
        // `!/` in an ordinary path: the left side is not an archive, so this is
        // a plain path and must not be mistaken for a member reference.
        assert_eq!(split_virtual("/tmp/we!/there/main.rs"), None);
        assert_eq!(split_virtual("/tmp/main.rs"), None);
        assert_eq!(container_of("/tmp/a.zip!/x"), "/tmp/a.zip");
        assert_eq!(container_of("/tmp/x.rs"), "/tmp/x.rs");
    }

    #[test]
    fn rejects_escaping_member_paths() {
        assert_eq!(safe_member_path("a/b.txt").as_deref(), Some("a/b.txt"));
        assert_eq!(safe_member_path("./a/./b.txt").as_deref(), Some("a/b.txt"));
        assert_eq!(safe_member_path("a\\b.txt").as_deref(), Some("a/b.txt"));
        assert_eq!(safe_member_path("../etc/passwd"), None);
        assert_eq!(safe_member_path("a/../../etc/passwd"), None);
        assert_eq!(safe_member_path("/etc/passwd"), None);
        assert_eq!(safe_member_path("C:/Windows/system32"), None);
        assert_eq!(safe_member_path(""), None);
    }

    #[test]
    fn lists_a_zip_level_at_a_time() {
        let dir = tempfile::tempdir().unwrap();
        let zip = zip_fixture(dir.path());

        let root = list_dir(&zip, "").unwrap();
        let names: Vec<&str> = root.iter().map(|e| e.name.as_str()).collect();
        // Directories first, then files — the on-disk explorer's order.
        assert_eq!(names, vec!["src", "README.md"]);
        assert!(root[0].is_directory);
        assert_eq!(root[1].path, format!("{}!/README.md", zip.display()));

        let src = list_dir(&zip, "src").unwrap();
        let names: Vec<&str> = src.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["util", "main.rs"]);

        // `src/util` was never an entry in the zip — it exists only because
        // `src/util/mod.rs` does.
        let util = list_dir(&zip, "src/util").unwrap();
        assert_eq!(util.len(), 1);
        assert_eq!(util[0].name, "mod.rs");
    }

    #[test]
    fn reads_a_member_of_each_family() {
        let dir = tempfile::tempdir().unwrap();
        let zip = zip_fixture(dir.path());
        assert_eq!(
            String::from_utf8(read_member(&zip, "src/main.rs").unwrap()).unwrap(),
            "fn main() {}\n"
        );
        let targz = targz_fixture(dir.path());
        assert_eq!(
            String::from_utf8(read_member(&targz, "README.md").unwrap()).unwrap(),
            "# hello\n"
        );
        assert!(matches!(
            read_member(&zip, "nope.txt"),
            Err(ArchiveError::NoSuchEntry(_))
        ));
    }

    /// A tar holding one file, compressed with `comp`, written to `path`.
    fn tar_with(path: &Path, comp: Compression) {
        let mut plain = Vec::new();
        {
            let mut tar = tar::Builder::new(&mut plain);
            let body = b"compressed\n";
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, "note.txt", &body[..]).unwrap();
            tar.finish().unwrap();
        }
        let out = File::create(path).unwrap();
        match comp {
            Compression::None => {
                let mut out = out;
                out.write_all(&plain).unwrap();
            }
            Compression::Gzip => {
                let mut e = flate2::write::GzEncoder::new(out, flate2::Compression::default());
                e.write_all(&plain).unwrap();
                e.finish().unwrap();
            }
            Compression::Bzip2 => {
                let mut e = bzip2::write::BzEncoder::new(out, bzip2::Compression::default());
                e.write_all(&plain).unwrap();
                e.finish().unwrap();
            }
            Compression::Zstd => {
                let mut e = zstd::stream::write::Encoder::new(out, 3).unwrap();
                e.write_all(&plain).unwrap();
                e.finish().unwrap();
            }
            Compression::Xz => {
                let mut compressed = Vec::new();
                lzma_rs::xz_compress(&mut Cursor::new(&plain), &mut compressed).unwrap();
                let mut out = out;
                out.write_all(&compressed).unwrap();
            }
        }
    }

    /// Every codec is wired to the extension that names it. A decoder attached
    /// to the wrong suffix fails only when someone opens that kind of file, so
    /// each one is exercised here rather than trusted.
    #[test]
    fn every_compressor_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        for (name, comp) in [
            ("a.tar", Compression::None),
            ("b.tar.gz", Compression::Gzip),
            ("c.tar.bz2", Compression::Bzip2),
            ("d.tar.zst", Compression::Zstd),
            ("e.tar.xz", Compression::Xz),
        ] {
            let path = dir.path().join(name);
            tar_with(&path, comp);
            assert_eq!(archive_kind(name), Some(ArchiveKind::Tar(comp)), "{name}");
            let root = list_dir(&path, "").unwrap();
            assert_eq!(root.len(), 1, "{name}");
            assert_eq!(root[0].name, "note.txt", "{name}");
            assert_eq!(
                String::from_utf8(read_member(&path, "note.txt").unwrap()).unwrap(),
                "compressed\n",
                "{name}",
            );
            let outcome = extract_to(&path, None).unwrap();
            assert_eq!(
                std::fs::read_to_string(outcome.dest.join("note.txt")).unwrap(),
                "compressed\n",
                "{name}",
            );
        }
    }

    #[test]
    fn lists_a_compressed_tarball() {
        let dir = tempfile::tempdir().unwrap();
        let targz = targz_fixture(dir.path());
        let root = list_dir(&targz, "").unwrap();
        let names: Vec<&str> = root.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["src", "README.md"]);
    }

    #[test]
    fn single_file_compression_browses_as_one_member() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("server.log.gz");
        let mut encoder = flate2::write::GzEncoder::new(
            File::create(&path).unwrap(),
            flate2::Compression::default(),
        );
        encoder.write_all(b"line one\n").unwrap();
        encoder.finish().unwrap();

        let root = list_dir(&path, "").unwrap();
        assert_eq!(root.len(), 1);
        assert_eq!(root[0].name, "server.log");
        assert_eq!(
            String::from_utf8(read_member(&path, "server.log").unwrap()).unwrap(),
            "line one\n"
        );
    }

    #[test]
    fn extracts_into_a_folder_named_after_the_archive() {
        let dir = tempfile::tempdir().unwrap();
        let zip = zip_fixture(dir.path());

        let outcome = extract_to(&zip, None).unwrap();
        assert_eq!(outcome.dest, dir.path().join("bundle"));
        assert_eq!(outcome.files, 3);
        assert_eq!(
            std::fs::read_to_string(outcome.dest.join("src/main.rs")).unwrap(),
            "fn main() {}\n"
        );

        // A second extraction never merges into the first.
        let again = extract_to(&zip, None).unwrap();
        assert_eq!(again.dest, dir.path().join("bundle-1"));
        assert!(matches!(
            extract_to(&zip, Some(&dir.path().join("bundle"))),
            Err(ArchiveError::DestinationExists(_))
        ));
    }

    #[test]
    fn extraction_refuses_to_escape_the_destination() {
        let dir = tempfile::tempdir().unwrap();
        let zip_path = dir.path().join("evil.zip");
        {
            let mut zip = zip::ZipWriter::new(File::create(&zip_path).unwrap());
            let opts = zip::write::SimpleFileOptions::default();
            // The classic zip-slip payload.
            zip.start_file("../../escaped.txt", opts).unwrap();
            zip.write_all(b"pwned").unwrap();
            zip.start_file("safe.txt", opts).unwrap();
            zip.write_all(b"fine").unwrap();
            zip.finish().unwrap();
        }
        let outcome = extract_to(&zip_path, None).unwrap();
        assert_eq!(outcome.files, 1);
        assert_eq!(outcome.skipped, 1);
        assert!(outcome.dest.join("safe.txt").exists());
        assert!(!dir.path().join("escaped.txt").exists());
        assert!(!dir.path().parent().unwrap().join("escaped.txt").exists());
    }

    #[test]
    fn a_member_over_the_limit_is_an_error_not_an_allocation() {
        let mut source = Cursor::new(vec![7u8; 4096]);
        let err = read_bounded(&mut source, 1024, "big.bin").unwrap_err();
        assert!(matches!(err, ArchiveError::TooLarge { .. }), "{err:?}");
        // Exactly at the limit still succeeds.
        let mut exact = Cursor::new(vec![7u8; 1024]);
        assert_eq!(read_bounded(&mut exact, 1024, "ok.bin").unwrap().len(), 1024);
    }

    #[test]
    fn the_index_cache_notices_a_rewritten_archive() {
        let dir = tempfile::tempdir().unwrap();
        let zip = zip_fixture(dir.path());
        assert_eq!(list_dir(&zip, "").unwrap().len(), 2);

        // Rewrite with different contents; the stamp changes, so the cached
        // index must not answer.
        std::fs::remove_file(&zip).unwrap();
        {
            let mut w = zip::ZipWriter::new(File::create(&zip).unwrap());
            w.start_file("only.txt", zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(b"x").unwrap();
            w.finish().unwrap();
        }
        let root = list_dir(&zip, "").unwrap();
        assert_eq!(root.len(), 1);
        assert_eq!(root[0].name, "only.txt");
    }
}
