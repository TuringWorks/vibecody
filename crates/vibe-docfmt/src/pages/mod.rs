//! Apple Pages documents.
//!
//! A `.pages` file is a ZIP (or, when "save as package" is on, a directory)
//! whose `Index/*.iwa` archives hold the document as Snappy-compressed protobuf
//! with no published schema. What this module recovers is therefore **text**:
//! paragraphs, in document order, per text storage. Formatting, layout, tables,
//! shapes and images are not modelled — they are carried across a save
//! untouched because nothing outside the edited text field is ever rewritten.
//!
//! Writing back is real but bounded, and the bound is enforced rather than
//! documented-and-hoped: [`crate::write_text`] re-reads whatever it produced and
//! refuses to replace the original unless the text comes back exactly.

pub mod iwa;
pub mod protobuf;
pub mod snappy;
pub mod text;

use std::path::{Path, PathBuf};

use crate::error::DocError;
use crate::model::{Block, DocFormat, Document, Section, Span, Warning};
use crate::zipedit::{self, ZipEntry};

use iwa::Archive;
use text::Storage;

/// Preview images Pages embeds, best first.
const PREVIEW_NAMES: [&str; 3] = ["preview.jpg", "preview-web.jpg", "preview-micro.jpg"];

/// Where the `.iwa` archives live inside a container.
#[derive(Debug, Clone, PartialEq, Eq)]
enum IwaHome {
    /// `Index/*.iwa` directly in the container.
    Flat,
    /// Wrapped in a nested `Index.zip` (Pages 5.0-era documents).
    Nested(String),
}

/// One `.iwa`, decompressed and parsed.
struct IwaFile {
    name: String,
    archives: Vec<Archive>,
    /// Set once an edit has touched this file.
    dirty: bool,
}

/// The result of rewriting a `.pages` file.
#[derive(Debug)]
pub struct FileRewrite {
    pub bytes: Vec<u8>,
    pub effective: Document,
    pub warnings: Vec<Warning>,
}

/// The result of rewriting a `.pages` bundle: the files that changed.
#[derive(Debug)]
pub struct BundleRewrite {
    pub files: Vec<(PathBuf, Vec<u8>)>,
    pub effective: Document,
    pub warnings: Vec<Warning>,
}

// ── Container access ─────────────────────────────────────────────────

fn is_iwa(name: &str) -> bool {
    name.ends_with(".iwa")
}

fn load_from_zip(bytes: &[u8]) -> Result<(Vec<ZipEntry>, IwaHome, Vec<(String, Vec<u8>)>), DocError> {
    let entries = zipedit::read_entries(bytes)?;
    let flat: Vec<(String, Vec<u8>)> = entries
        .iter()
        .filter(|e| is_iwa(&e.name))
        .map(|e| (e.name.clone(), e.data.clone()))
        .collect();
    if !flat.is_empty() {
        return Ok((entries, IwaHome::Flat, sorted(flat)));
    }

    // Older packages keep the archives inside a nested Index.zip.
    let nested_name = entries
        .iter()
        .map(|e| e.name.clone())
        .find(|name| name == "Index.zip" || name.ends_with("/Index.zip"));
    if let Some(nested_name) = nested_name {
        let nested_entry = zipedit::find(&entries, &nested_name).ok_or_else(|| {
            DocError::Container(format!("{nested_name} disappeared while reading"))
        })?;
        let inner = zipedit::read_entries(&nested_entry.data)?;
        let iwas: Vec<(String, Vec<u8>)> = inner
            .iter()
            .filter(|e| is_iwa(&e.name))
            .map(|e| (e.name.clone(), e.data.clone()))
            .collect();
        if !iwas.is_empty() {
            return Ok((entries, IwaHome::Nested(nested_name), sorted(iwas)));
        }
    }

    Err(DocError::Parse(
        "this .pages file has no Index/*.iwa archives; \
         documents saved by Pages '09 and earlier are not supported"
            .into(),
    ))
}

fn sorted(mut iwas: Vec<(String, Vec<u8>)>) -> Vec<(String, Vec<u8>)> {
    // Deterministic order, so section ids and their markers do not move between
    // two reads of the same document.
    iwas.sort_by(|a, b| a.0.cmp(&b.0));
    iwas
}

fn load_from_bundle(dir: &Path) -> Result<Vec<(String, Vec<u8>)>, DocError> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), DocError> {
        let read = std::fs::read_dir(dir).map_err(|e| DocError::io(dir, e))?;
        for entry in read {
            let entry = entry.map_err(|e| DocError::io(dir, e))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            let relative = if prefix.is_empty() { name.clone() } else { format!("{prefix}/{name}") };
            let file_type = entry.file_type().map_err(|e| DocError::io(&path, e))?;
            if file_type.is_dir() {
                walk(&path, &relative, out)?;
            } else if is_iwa(&relative) {
                let data = std::fs::read(&path).map_err(|e| DocError::io(&path, e))?;
                out.push((relative, data));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(dir, "", &mut out)?;
    if out.is_empty() {
        return Err(DocError::Parse(format!(
            "{} has no *.iwa archives; it does not look like a Pages bundle",
            dir.display()
        )));
    }
    Ok(sorted(out))
}

fn parse_iwas(raw: &[(String, Vec<u8>)]) -> Result<Vec<IwaFile>, DocError> {
    raw.iter()
        .map(|(name, data)| {
            let plain = snappy::decompress(data)
                .map_err(|e| DocError::Parse(format!("{name}: {e}")))?;
            let archives = iwa::parse_stream(&plain)
                .map_err(|e| DocError::Parse(format!("{name}: {e}")))?;
            Ok(IwaFile { name: name.clone(), archives, dirty: false })
        })
        .collect()
}

fn storages_of(files: &[IwaFile]) -> Vec<(usize, Storage)> {
    files
        .iter()
        .enumerate()
        .flat_map(|(index, file)| {
            text::find_storages(&file.name, &file.archives)
                .into_iter()
                // A storage with no text has nothing to show and nothing to
                // edit; leaving it out keeps empty markers out of the buffer.
                .filter(|storage| !storage.text().trim().is_empty())
                .map(move |storage| (index, storage))
        })
        .collect()
}

fn document_from(storages: &[(usize, Storage)]) -> Document {
    let sections = storages
        .iter()
        .map(|(_, storage)| Section {
            id: storage.id(),
            title: None,
            blocks: paragraphs(&storage.text()),
        })
        .collect();

    let mut warnings = vec![Warning::new(
        "pages.text_only",
        "Pages documents are read as text: paragraphs are editable, but fonts, \
         layout, tables, shapes and images are not shown here. They are kept \
         unchanged when you save.",
    )];
    if storages.iter().any(|(_, s)| s.guessed) {
        warnings.push(Warning::new(
            "pages.text_field_guessed",
            "this document has no message of the expected text-storage type; \
             the text shown was identified by shape, so it may include strings \
             that are not body text",
        ));
    }
    Document { format: DocFormat::Pages, sections, warnings }
}

fn paragraphs(textual: &str) -> Vec<Block> {
    textual
        .split('\n')
        .map(|line| Block::Paragraph { spans: vec![Span::plain(line.to_string())] })
        .collect()
}

fn section_text(section: &Section) -> String {
    section.blocks.iter().map(Block::plain_text).collect::<Vec<_>>().join("\n")
}

// ── Read ─────────────────────────────────────────────────────────────

/// Read a `.pages` file.
pub fn read_file(bytes: &[u8]) -> Result<Document, DocError> {
    let (_, _, raw) = load_from_zip(bytes)?;
    let files = parse_iwas(&raw)?;
    Ok(document_from(&storages_of(&files)))
}

/// Read a `.pages` bundle directory.
pub fn read_bundle(dir: &Path) -> Result<Document, DocError> {
    let raw = load_from_bundle(dir)?;
    let files = parse_iwas(&raw)?;
    Ok(document_from(&storages_of(&files)))
}

/// The preview image Pages embeds, if there is one: `(mime type, bytes)`.
pub fn preview_file(bytes: &[u8]) -> Option<(String, Vec<u8>)> {
    let entries = zipedit::read_entries(bytes).ok()?;
    PREVIEW_NAMES.iter().find_map(|name| {
        zipedit::find(&entries, name).map(|e| ("image/jpeg".to_string(), e.data.clone()))
    })
}

/// The preview image inside a `.pages` bundle, if there is one.
pub fn preview_bundle(dir: &Path) -> Option<(String, Vec<u8>)> {
    PREVIEW_NAMES.iter().find_map(|name| {
        let path = dir.join(name);
        std::fs::read(&path).ok().map(|data| ("image/jpeg".to_string(), data))
    })
}

// ── Write ────────────────────────────────────────────────────────────

struct Applied {
    files: Vec<IwaFile>,
    effective: Document,
    warnings: Vec<Warning>,
}

fn apply(raw: &[(String, Vec<u8>)], target: &Document) -> Result<Applied, DocError> {
    let mut files = parse_iwas(raw)?;
    let storages = storages_of(&files);

    if target.sections.len() != storages.len() {
        return Err(DocError::Structure(format!(
            "this document has {} text storages but the edited text has {}; \
             keep every `<<< vibedoc:storage … >>>` marker in place",
            storages.len(),
            target.sections.len()
        )));
    }
    if let Some((expected, got)) = storages
        .iter()
        .zip(target.sections.iter())
        .find(|((_, storage), section)| !section.id.is_empty() && storage.id() != section.id)
        .map(|((_, storage), section)| (storage.id(), section.id.clone()))
    {
        return Err(DocError::Structure(format!(
            "a storage marker was changed: expected `{expected}`, found `{got}`"
        )));
    }

    let mut remapped = 0usize;
    let mut edited = 0usize;
    for ((file_index, storage), section) in storages.iter().zip(target.sections.iter()) {
        let new_text = section_text(section);
        if new_text == storage.text() {
            continue;
        }
        let file = files
            .get_mut(*file_index)
            .ok_or_else(|| DocError::Structure("archive file vanished mid-write".into()))?;
        let report = text::set_text(&mut file.archives, storage, &new_text)?;
        file.dirty = true;
        edited += 1;
        remapped += report.remapped_indices;
    }

    let mut warnings = Vec::new();
    if remapped > 0 {
        warnings.push(Warning::new(
            "pages.style_ranges_remapped",
            format!(
                "{remapped} style/attribute ranges were shifted to follow the new text. \
                 Pages' archive format is not published, so this is a best effort: \
                 check the document's formatting after opening it in Pages."
            ),
        ));
    }
    if edited > 0 {
        warnings.push(Warning::new(
            "pages.write_unverified_in_app",
            "text was written back into the Pages archives and read back correctly, \
             but this crate cannot check how Pages itself renders the result. \
             A backup of the original is written next to the document.",
        ));
    }

    let effective = Document {
        format: DocFormat::Pages,
        sections: storages
            .iter()
            .zip(target.sections.iter())
            .map(|((_, storage), section)| Section {
                id: storage.id(),
                title: None,
                blocks: paragraphs(&section_text(section)),
            })
            .collect(),
        warnings: Vec::new(),
    };
    Ok(Applied { files, effective, warnings })
}

fn recompress(files: &[IwaFile]) -> Result<Vec<(String, Vec<u8>)>, DocError> {
    files
        .iter()
        .filter(|file| file.dirty)
        .map(|file| {
            let plain = iwa::serialize_stream(&file.archives);
            let packed = snappy::compress(&plain)?;
            Ok((file.name.clone(), packed))
        })
        .collect()
}

/// Rewrite a `.pages` file with new text.
pub fn write_file(original: &[u8], target: &Document) -> Result<FileRewrite, DocError> {
    let (mut entries, home, raw) = load_from_zip(original)?;
    let applied = apply(&raw, target)?;
    let changed = recompress(&applied.files)?;

    match home {
        IwaHome::Flat => {
            for (name, data) in changed {
                if !zipedit::replace(&mut entries, &name, data) {
                    return Err(DocError::Structure(format!("{name} vanished mid-write")));
                }
            }
        }
        IwaHome::Nested(nested_name) => {
            let nested = zipedit::find(&entries, &nested_name).ok_or_else(|| {
                DocError::Container(format!("{nested_name} disappeared while writing"))
            })?;
            let mut inner = zipedit::read_entries(&nested.data)?;
            for (name, data) in changed {
                if !zipedit::replace(&mut inner, &name, data) {
                    return Err(DocError::Structure(format!("{name} vanished mid-write")));
                }
            }
            let repacked = zipedit::write_entries(&inner)?;
            zipedit::replace(&mut entries, &nested_name, repacked);
        }
    }

    Ok(FileRewrite {
        bytes: zipedit::write_entries(&entries)?,
        effective: applied.effective,
        warnings: applied.warnings,
    })
}

/// Rewrite a `.pages` bundle with new text, returning the files to write.
pub fn write_bundle(dir: &Path, target: &Document) -> Result<BundleRewrite, DocError> {
    let raw = load_from_bundle(dir)?;
    let applied = apply(&raw, target)?;
    let files = recompress(&applied.files)?
        .into_iter()
        .map(|(name, data)| (dir.join(name), data))
        .collect();
    Ok(BundleRewrite { files, effective: applied.effective, warnings: applied.warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use protobuf::Message;

    /// Build a `.pages`-shaped ZIP holding one text storage.
    fn pages_file(chunks: &[&str]) -> Vec<u8> {
        let payload = text::build_storage_payload(3, chunks);
        let mut info = Message::default();
        info.set_varint(1, 1001);
        let mut message_info = Message::default();
        message_info.set_varint(1, text::TEXT_STORAGE_TYPE);
        message_info.set_varint(3, payload.len() as u64);
        let stream = iwa::serialize_stream(&[Archive {
            identifier: 1001,
            info,
            messages: vec![iwa::ArchivedMessage {
                type_id: text::TEXT_STORAGE_TYPE,
                info: message_info,
                payload,
            }],
        }]);
        let packed = snappy::compress(&stream).expect("compress");

        zipedit::write_entries(&[
            ZipEntry {
                name: "Index/Document.iwa".to_string(),
                data: packed,
                compression: zip::CompressionMethod::Deflated,
                is_dir: false,
            },
            ZipEntry {
                name: "preview.jpg".to_string(),
                data: b"jpeg-bytes".to_vec(),
                compression: zip::CompressionMethod::Stored,
                is_dir: false,
            },
            ZipEntry {
                name: "Data/image-1.png".to_string(),
                data: b"png-bytes".to_vec(),
                compression: zip::CompressionMethod::Deflated,
                is_dir: false,
            },
        ])
        .expect("build fixture")
    }

    #[test]
    fn reads_paragraphs_from_a_text_storage() {
        let bytes = pages_file(&["First line.\nSecond line."]);
        let doc = read_file(&bytes).expect("read");
        assert_eq!(doc.sections.len(), 1);
        assert_eq!(doc.sections[0].blocks.len(), 2);
        assert_eq!(doc.sections[0].blocks[1].plain_text(), "Second line.");
        assert!(doc.warnings.iter().any(|w| w.code == "pages.text_only"));
    }

    #[test]
    fn writes_text_back_and_keeps_every_other_part() {
        let bytes = pages_file(&["Old text."]);
        let doc = read_file(&bytes).expect("read");
        let mut edited = doc.clone();
        edited.sections[0].blocks = paragraphs("New text, longer than before.");

        let rewritten = write_file(&bytes, &edited).expect("write");
        let reread = read_file(&rewritten.bytes).expect("re-read");
        assert_eq!(reread.sections[0].blocks[0].plain_text(), "New text, longer than before.");

        let entries = zipedit::read_entries(&rewritten.bytes).expect("entries");
        assert!(zipedit::find(&entries, "Data/image-1.png").is_some(), "assets preserved");
        assert!(zipedit::find(&entries, "preview.jpg").is_some(), "preview preserved");
        assert!(
            rewritten.warnings.iter().any(|w| w.code == "pages.write_unverified_in_app"),
            "the limit of the guarantee is stated: {:?}",
            rewritten.warnings
        );
    }

    #[test]
    fn a_changed_storage_marker_is_refused() {
        let bytes = pages_file(&["Text."]);
        let doc = read_file(&bytes).expect("read");
        let mut edited = doc.clone();
        edited.sections[0].id = "Index/Document.iwa:9999:0".to_string();
        let err = write_file(&bytes, &edited).expect_err("marker change refused");
        assert_eq!(err.kind(), "structure");
    }

    #[test]
    fn a_dropped_storage_is_refused() {
        let bytes = pages_file(&["Text."]);
        let mut edited = read_file(&bytes).expect("read");
        edited.sections.clear();
        let err = write_file(&bytes, &edited).expect_err("dropped storage refused");
        assert!(err.to_string().contains("1 text storages"), "{err}");
    }

    #[test]
    fn multi_chunk_storages_keep_their_chunk_count() {
        let bytes = pages_file(&["Chunk one. ", "Chunk two."]);
        let doc = read_file(&bytes).expect("read");
        assert_eq!(doc.sections[0].blocks[0].plain_text(), "Chunk one. Chunk two.");

        let mut edited = doc.clone();
        edited.sections[0].blocks = paragraphs("Chunk one. Chunk two, extended.");
        let rewritten = write_file(&bytes, &edited).expect("write");

        let entries = zipedit::read_entries(&rewritten.bytes).expect("entries");
        let iwa_bytes = &zipedit::find(&entries, "Index/Document.iwa").expect("iwa").data;
        let stream = snappy::decompress(iwa_bytes).expect("decompress");
        let archives = iwa::parse_stream(&stream).expect("parse");
        let parsed = Message::parse(&archives[0].messages[0].payload).expect("payload");
        assert_eq!(parsed.bytes_values(3).len(), 2, "still two chunks");
    }

    #[test]
    fn the_preview_image_is_available_for_the_viewer() {
        let bytes = pages_file(&["Text."]);
        let (mime, data) = preview_file(&bytes).expect("preview");
        assert_eq!(mime, "image/jpeg");
        assert_eq!(data, b"jpeg-bytes");
    }

    #[test]
    fn a_file_without_iwa_archives_is_refused_by_name() {
        let bytes = zipedit::write_entries(&[ZipEntry {
            name: "index.xml".to_string(),
            data: b"<document/>".to_vec(),
            compression: zip::CompressionMethod::Deflated,
            is_dir: false,
        }])
        .expect("build");
        let err = read_file(&bytes).expect_err("old format refused");
        assert!(err.to_string().contains("Pages '09"), "{err}");
    }
}
