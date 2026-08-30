//! The path-level API: read a document to a buffer, save the buffer back, and
//! never replace a file with something that does not read back correctly.

use std::path::Path;

use vibe_docfmt::model::DocFormat;
use vibe_docfmt::pages::{iwa, protobuf::Message, snappy, text};
use vibe_docfmt::zipedit::{self, ZipEntry};
use vibe_docfmt::{detect_format, read_preview, read_text, write_text, Syntax};

fn entry(name: &str, data: Vec<u8>) -> ZipEntry {
    ZipEntry {
        name: name.to_string(),
        data,
        compression: zip::CompressionMethod::Deflated,
        is_dir: false,
    }
}

fn docx_bytes(body: &str) -> Vec<u8> {
    let w = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="{w}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>{body}</w:body></w:document>"#
    );
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#;
    zipedit::write_entries(&[
        entry("word/document.xml", document.into_bytes()),
        entry("word/_rels/document.xml.rels", rels.as_bytes().to_vec()),
    ])
    .expect("docx fixture")
}

fn epub_bytes(paragraph: &str) -> Vec<u8> {
    let chapter = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Ch</title></head><body><p>{paragraph}</p></body></html>"#
    );
    let opf = r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata/><manifest><item id="c0" href="ch0.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="c0"/></spine></package>"#;
    let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/book.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;
    zipedit::write_entries(&[
        entry("mimetype", b"application/epub+zip".to_vec()),
        entry("META-INF/container.xml", container.as_bytes().to_vec()),
        entry("OEBPS/book.opf", opf.as_bytes().to_vec()),
        entry("OEBPS/ch0.xhtml", chapter.into_bytes()),
    ])
    .expect("epub fixture")
}

fn pages_iwa(body: &str) -> Vec<u8> {
    let payload = text::build_storage_payload(3, &[body]);
    let mut info = Message::default();
    info.set_varint(1, 1001);
    let mut message_info = Message::default();
    message_info.set_varint(1, text::TEXT_STORAGE_TYPE);
    message_info.set_varint(3, payload.len() as u64);
    let stream = iwa::serialize_stream(&[iwa::Archive {
        identifier: 1001,
        info,
        messages: vec![iwa::ArchivedMessage {
            type_id: text::TEXT_STORAGE_TYPE,
            info: message_info,
            payload,
        }],
    }]);
    snappy::compress(&stream).expect("compress")
}

fn pages_bytes(body: &str) -> Vec<u8> {
    zipedit::write_entries(&[
        entry("Index/Document.iwa", pages_iwa(body)),
        entry("preview.jpg", b"jpeg".to_vec()),
    ])
    .expect("pages fixture")
}

fn write_fixture(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("write fixture");
    path
}

#[test]
fn detects_the_four_formats_and_nothing_else() {
    assert_eq!(detect_format(Path::new("/a/b.docx")), Some(DocFormat::Docx));
    assert_eq!(detect_format(Path::new("/a/b.EPUB")), Some(DocFormat::Epub));
    assert_eq!(
        detect_format(Path::new("/a/b.pages")),
        Some(DocFormat::Pages)
    );
    assert_eq!(detect_format(Path::new("/a/b.pdf")), Some(DocFormat::Pdf));
    assert_eq!(detect_format(Path::new("/a/b.md")), None);
}

#[test]
fn docx_reads_as_markdown_and_saves_back() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_fixture(
        dir.path(),
        "note.docx",
        &docx_bytes(r#"<w:p><w:r><w:t>before</w:t></w:r></w:p>"#),
    );

    let buffer = read_text(&path).expect("read");
    assert_eq!(buffer.syntax, Syntax::Markdown);
    assert_eq!(buffer.text.trim(), "before");
    assert!(buffer.writable);

    let report = write_text(&path, "# after\n\nsecond paragraph\n").expect("write");
    assert!(report.verified);
    assert!(report.backup.is_none(), "DOCX does not need a backup copy");

    let reread = read_text(&path).expect("re-read");
    assert_eq!(reread.text.trim(), "# after\n\nsecond paragraph");
}

#[test]
fn epub_reads_as_markdown_and_saves_back() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_fixture(dir.path(), "book.epub", &epub_bytes("before"));

    let buffer = read_text(&path).expect("read");
    assert_eq!(buffer.sections, 1);
    assert_eq!(buffer.text.trim(), "before");

    write_text(&path, "after **bold**\n").expect("write");
    let reread = read_text(&path).expect("re-read");
    assert_eq!(reread.text.trim(), "after **bold**");
}

#[test]
fn pages_reads_as_plain_text_saves_back_and_keeps_a_backup() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_fixture(dir.path(), "memo.pages", &pages_bytes("first\nsecond"));

    let buffer = read_text(&path).expect("read");
    assert_eq!(
        buffer.syntax,
        Syntax::PlainText,
        "Pages has no emphasis to edit"
    );
    assert_eq!(buffer.text, "first\nsecond\n");
    assert!(
        buffer.warnings.iter().any(|w| w.code == "pages.text_only"),
        "the reader states what it does not model: {:?}",
        buffer.warnings
    );

    let report = write_text(&path, "first\nsecond, edited\nthird\n").expect("write");
    assert!(report.verified);
    let backup = report.backup.expect("Pages keeps a backup");
    assert!(backup.exists(), "backup written next to the document");

    let reread = read_text(&path).expect("re-read");
    assert_eq!(reread.text, "first\nsecond, edited\nthird\n");

    // The backup is still the document as it was before the edit.
    let saved = vibe_docfmt::pages::read_file(&std::fs::read(&backup).expect("backup bytes"))
        .expect("backup is a readable Pages document");
    assert_eq!(
        saved.sections[0].blocks[1].plain_text(),
        "second",
        "backup predates the edit"
    );
}

#[test]
fn pages_bundles_are_read_and_written_in_place() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bundle = dir.path().join("bundle.pages");
    std::fs::create_dir_all(bundle.join("Index")).expect("bundle dirs");
    std::fs::write(bundle.join("Index/Document.iwa"), pages_iwa("bundle text")).expect("iwa");
    std::fs::write(bundle.join("preview.jpg"), b"jpeg").expect("preview");

    let buffer = read_text(&bundle).expect("read bundle");
    assert_eq!(buffer.text, "bundle text\n");

    let report = write_text(&bundle, "bundle text, revised\n").expect("write bundle");
    assert!(report.verified);
    let backup = report.backup.expect("bundle backup");
    assert!(
        backup.exists(),
        "the bundle was copied before being written"
    );
    assert_eq!(
        backup.parent(),
        bundle.parent(),
        "the copy sits beside the package, not inside it"
    );
    let stray: Vec<String> = std::fs::read_dir(bundle.join("Index"))
        .expect("list Index")
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .filter(|name| name.ends_with(".bak"))
        .collect();
    assert!(
        stray.is_empty(),
        "no backup files left inside the package: {stray:?}"
    );
    assert_eq!(
        read_text(&bundle).expect("re-read").text,
        "bundle text, revised\n"
    );

    // The staging copy used for verification is cleaned up.
    let leftovers: Vec<String> = std::fs::read_dir(dir.path())
        .expect("list")
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned()))
        .filter(|name| name.contains("staging"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "staging directory removed: {leftovers:?}"
    );
}

#[test]
fn a_refused_write_leaves_the_document_untouched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_fixture(dir.path(), "memo.pages", &pages_bytes("only storage"));
    let before = std::fs::read(&path).expect("before");

    // A storage marker that names a storage the document does not have.
    let err = write_text(&path, "<<< vibedoc:storage nope >>>\nnew text\n").expect_err("refused");
    assert_eq!(err.kind(), "structure");
    assert_eq!(
        std::fs::read(&path).expect("after"),
        before,
        "file is byte-identical"
    );
}

#[test]
fn the_pages_preview_image_is_exposed_for_the_viewer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_fixture(dir.path(), "memo.pages", &pages_bytes("text"));
    let (mime, data) = read_preview(&path).expect("preview");
    assert_eq!(mime, "image/jpeg");
    assert_eq!(data, b"jpeg");
    assert!(
        read_preview(Path::new("/a/b.docx")).is_none(),
        "only Pages embeds one"
    );
}
