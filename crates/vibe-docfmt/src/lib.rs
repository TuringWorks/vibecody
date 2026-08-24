//! Read and write rich document formats as text.
//!
//! Three formats — DOCX, EPUB and Apple Pages — become an editable text buffer:
//! Markdown where the format has structure to carry it, plain text for Pages,
//! whose archives yield paragraphs and nothing more. Saving edits the *original*
//! container, so everything the text model does not describe (images, styles,
//! page setup, metadata) is carried across untouched.
//!
//! Nothing is written until it has been checked. [`write_text`] rewrites into
//! memory, re-reads the result, compares it against the text the caller asked
//! for, and only then replaces the file. A mismatch returns
//! [`DocError::Verification`] with the original still in place — the alternative,
//! reporting a save that silently mangled a document, is the failure mode this
//! crate exists to prevent.

pub mod docx;
pub mod epub;
pub mod error;
pub mod markdown;
pub mod model;
pub mod pages;
pub mod surgical;
pub mod xmltree;
pub mod zipedit;

use std::path::{Path, PathBuf};

pub use error::DocError;
pub use model::{Block, DocFormat, Document, Section, Span, SpanStyle, Syntax, Warning};

/// A document as an editable text buffer.
#[derive(Debug, Clone)]
pub struct DocumentText {
    pub format: DocFormat,
    pub syntax: Syntax,
    /// The buffer itself.
    pub text: String,
    /// How many sections (DOCX: 1, EPUB: chapters, Pages: text storages) the
    /// buffer holds. Section markers must survive an edit, so the UI shows this.
    pub sections: usize,
    /// Everything the reader could not represent faithfully.
    pub warnings: Vec<Warning>,
    /// Whether [`write_text`] can write this format back.
    pub writable: bool,
}

/// What a completed write did.
#[derive(Debug, Clone)]
pub struct WriteReport {
    pub format: DocFormat,
    pub bytes_written: u64,
    /// Set when the writer copied the original aside first.
    pub backup: Option<PathBuf>,
    pub warnings: Vec<Warning>,
    /// Always true on success: a write that could not be verified is an error,
    /// never a report.
    pub verified: bool,
}

/// Identify a document by path.
///
/// Pages documents can be a file or a bundle directory; both answer
/// [`DocFormat::Pages`].
pub fn detect_format(path: &Path) -> Option<DocFormat> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "docx" => Some(DocFormat::Docx),
        "epub" => Some(DocFormat::Epub),
        "pages" => Some(DocFormat::Pages),
        _ => None,
    }
}

/// Whether this crate handles the path's format.
pub fn is_document_path(path: &Path) -> bool {
    detect_format(path).is_some()
}

/// Read a document into its editable buffer.
pub fn read_text(path: &Path) -> Result<DocumentText, DocError> {
    let format = detect_format(path)
        .ok_or_else(|| DocError::Unsupported(path.display().to_string()))?;
    let document = read_document(path, format)?;
    let text = render(&document);
    Ok(DocumentText {
        format,
        syntax: format.syntax(),
        text,
        sections: document.sections.len(),
        warnings: document.warnings,
        writable: true,
    })
}

/// Read a document into the block model.
pub fn read_document(path: &Path, format: DocFormat) -> Result<Document, DocError> {
    match format {
        DocFormat::Docx => docx::read(&read_bytes(path)?),
        DocFormat::Epub => epub::read(&read_bytes(path)?),
        DocFormat::Pages => {
            if path.is_dir() {
                pages::read_bundle(path)
            } else {
                pages::read_file(&read_bytes(path)?)
            }
        }
    }
}

/// The preview image a format embeds, if any: `(mime type, bytes)`.
///
/// Only Pages has one — it is what makes a document whose layout this crate
/// cannot render still show the reader what the page looks like.
pub fn read_preview(path: &Path) -> Option<(String, Vec<u8>)> {
    match detect_format(path)? {
        DocFormat::Pages => {
            if path.is_dir() {
                pages::preview_bundle(path)
            } else {
                pages::preview_file(&std::fs::read(path).ok()?)
            }
        }
        _ => None,
    }
}

/// Render a document into its editable buffer.
pub fn render(document: &Document) -> String {
    match document.format.syntax() {
        Syntax::Markdown => markdown::to_markdown(document),
        Syntax::PlainText => markdown::to_plain_text(document),
    }
}

/// Parse an edited buffer back into the block model.
pub fn parse_text(format: DocFormat, text: &str) -> Document {
    match format.syntax() {
        Syntax::Markdown => markdown::from_markdown(format, text),
        Syntax::PlainText => markdown::from_plain_text(text),
    }
}

/// Write an edited buffer back to the document.
///
/// The original is replaced only after the rewritten document has been re-read
/// and found to carry exactly the text that was asked for.
pub fn write_text(path: &Path, text: &str) -> Result<WriteReport, DocError> {
    let format = detect_format(path)
        .ok_or_else(|| DocError::Unsupported(path.display().to_string()))?;
    let target = parse_text(format, text);

    match format {
        DocFormat::Docx => {
            let original = read_bytes(path)?;
            let rewrite = docx::write(&original, &target)?;
            verify(&rewrite.bytes, format, &rewrite.effective)?;
            let bytes_written = rewrite.bytes.len() as u64;
            replace_file(path, &rewrite.bytes)?;
            Ok(WriteReport {
                format,
                bytes_written,
                backup: None,
                warnings: rewrite.warnings,
                verified: true,
            })
        }
        DocFormat::Epub => {
            let original = read_bytes(path)?;
            let rewrite = epub::write(&original, &target)?;
            verify(&rewrite.bytes, format, &rewrite.effective)?;
            let bytes_written = rewrite.bytes.len() as u64;
            replace_file(path, &rewrite.bytes)?;
            Ok(WriteReport {
                format,
                bytes_written,
                backup: None,
                warnings: rewrite.warnings,
                verified: true,
            })
        }
        // Pages is the one format whose container is reverse-engineered, so it
        // is the one that keeps a copy of the original next to the document.
        DocFormat::Pages if path.is_dir() => write_pages_bundle(path, &target),
        DocFormat::Pages => {
            let original = read_bytes(path)?;
            let rewrite = pages::write_file(&original, &target)?;
            verify(&rewrite.bytes, format, &rewrite.effective)?;
            let backup = backup_file(path)?;
            let bytes_written = rewrite.bytes.len() as u64;
            replace_file(path, &rewrite.bytes)?;
            Ok(WriteReport {
                format,
                bytes_written,
                backup: Some(backup),
                warnings: rewrite.warnings,
                verified: true,
            })
        }
    }
}

fn write_pages_bundle(dir: &Path, target: &Document) -> Result<WriteReport, DocError> {
    let rewrite = pages::write_bundle(dir, target)?;

    // Verify against a copy: the bundle on disk must not change until the copy
    // has been read back and found correct.
    let staging = tempdir_beside(dir)?;
    copy_dir(dir, &staging)?;
    for (path, data) in &rewrite.files {
        let relative = path.strip_prefix(dir).map_err(|_| {
            DocError::Structure(format!("{} is not inside the bundle", path.display()))
        })?;
        write_file_all(&staging.join(relative), data)?;
    }
    let reread = pages::read_bundle(&staging).and_then(|doc| {
        compare(&doc, &rewrite.effective)?;
        Ok(doc)
    });
    let outcome = reread.map(|_| ());
    let cleanup = std::fs::remove_dir_all(&staging);
    outcome?;
    cleanup.map_err(|e| DocError::io(&staging, e))?;

    // The backup goes *beside* the bundle, not inside it: a stray `.bak` among
    // a package's own files is something Pages would have to make sense of.
    let backup = backup_file(dir)?;
    let mut bytes_written = 0u64;
    for (path, data) in &rewrite.files {
        write_file_all(path, data)?;
        bytes_written += data.len() as u64;
    }

    Ok(WriteReport {
        format: DocFormat::Pages,
        bytes_written,
        backup: Some(backup),
        warnings: rewrite.warnings,
        verified: true,
    })
}

/// Re-read rewritten bytes and confirm they carry the intended text.
fn verify(bytes: &[u8], format: DocFormat, effective: &Document) -> Result<(), DocError> {
    let reread = match format {
        DocFormat::Docx => docx::read(bytes),
        DocFormat::Epub => epub::read(bytes),
        DocFormat::Pages => pages::read_file(bytes),
    }
    .map_err(|e| {
        DocError::Verification(format!("the rewritten document could not be read back: {e}"))
    })?;
    compare(&reread, effective)
}

fn compare(reread: &Document, effective: &Document) -> Result<(), DocError> {
    let got = render(reread);
    let want = render(effective);
    if got.trim_end() == want.trim_end() {
        return Ok(());
    }
    Err(DocError::Verification(format!(
        "the rewritten document does not read back as the text you saved{}. \
         Your file has not been changed.",
        first_difference(&want, &got)
            .map(|d| format!(" (first difference at line {}: expected {:?}, found {:?})", d.0, d.1, d.2))
            .unwrap_or_default()
    )))
}

fn first_difference(want: &str, got: &str) -> Option<(usize, String, String)> {
    want.lines()
        .zip(got.lines())
        .enumerate()
        .find(|(_, (a, b))| a != b)
        .map(|(i, (a, b))| (i + 1, truncate(a), truncate(b)))
        .or_else(|| {
            let (want_len, got_len) = (want.lines().count(), got.lines().count());
            (want_len != got_len).then(|| {
                (
                    want_len.min(got_len) + 1,
                    format!("{want_len} lines"),
                    format!("{got_len} lines"),
                )
            })
        })
}

fn truncate(line: &str) -> String {
    let limit = 60;
    if line.chars().count() <= limit {
        return line.to_string();
    }
    format!("{}…", line.chars().take(limit).collect::<String>())
}

// ── Filesystem helpers ───────────────────────────────────────────────

fn read_bytes(path: &Path) -> Result<Vec<u8>, DocError> {
    std::fs::read(path).map_err(|e| DocError::io(path, e))
}

/// Write via a sibling temp file and rename, so an interrupted write cannot
/// leave a half-written document behind.
fn replace_file(path: &Path, data: &[u8]) -> Result<(), DocError> {
    let temp = temp_path(path, "tmp");
    std::fs::write(&temp, data).map_err(|e| DocError::io(&temp, e))?;
    std::fs::rename(&temp, path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        DocError::io(path, e)
    })
}

fn write_file_all(path: &Path, data: &[u8]) -> Result<(), DocError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| DocError::io(parent, e))?;
    }
    std::fs::write(path, data).map_err(|e| DocError::io(path, e))
}

/// Copy a file (or directory) aside as `<name>.bak`, replacing a previous one.
fn backup_file(path: &Path) -> Result<PathBuf, DocError> {
    let backup = temp_path(path, "bak");
    if backup.exists() {
        if backup.is_dir() {
            std::fs::remove_dir_all(&backup).map_err(|e| DocError::io(&backup, e))?;
        } else {
            std::fs::remove_file(&backup).map_err(|e| DocError::io(&backup, e))?;
        }
    }
    if path.is_dir() {
        copy_dir(path, &backup)?;
    } else {
        std::fs::copy(path, &backup).map_err(|e| DocError::io(&backup, e))?;
    }
    Ok(backup)
}

fn temp_path(path: &Path, suffix: &str) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    path.with_file_name(format!("{name}.{suffix}"))
}

fn tempdir_beside(dir: &Path) -> Result<PathBuf, DocError> {
    let staging = temp_path(dir, "vibedoc-staging");
    if staging.exists() {
        std::fs::remove_dir_all(&staging).map_err(|e| DocError::io(&staging, e))?;
    }
    std::fs::create_dir_all(&staging).map_err(|e| DocError::io(&staging, e))?;
    Ok(staging)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), DocError> {
    std::fs::create_dir_all(to).map_err(|e| DocError::io(to, e))?;
    for entry in std::fs::read_dir(from).map_err(|e| DocError::io(from, e))? {
        let entry = entry.map_err(|e| DocError::io(from, e))?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        let file_type = entry.file_type().map_err(|e| DocError::io(&source, e))?;
        if file_type.is_dir() {
            copy_dir(&source, &target)?;
        } else {
            std::fs::copy(&source, &target).map_err(|e| DocError::io(&source, e))?;
        }
    }
    Ok(())
}
