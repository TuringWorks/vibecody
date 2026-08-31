//! EPUB read/write behaviour against a container assembled in-test.

use vibe_docfmt::model::{Block, DocFormat};
use vibe_docfmt::zipedit::{self, ZipEntry};
use vibe_docfmt::{epub, markdown};

fn entry(name: &str, data: &str) -> ZipEntry {
    ZipEntry {
        name: name.to_string(),
        data: data.as_bytes().to_vec(),
        compression: zip::CompressionMethod::Deflated,
        is_dir: false,
    }
}

fn chapter(title: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<html xmlns="http://www.w3.org/1999/xhtml"><head><title>{title}</title><link rel="stylesheet" href="style.css"/></head><body>{body}</body></html>"#
    )
}

fn book(chapters: &[(&str, &str)]) -> Vec<u8> {
    let manifest: String = chapters
        .iter()
        .enumerate()
        .map(|(i, _)| {
            format!(r#"<item id="c{i}" href="ch{i}.xhtml" media-type="application/xhtml+xml"/>"#)
        })
        .collect();
    let spine: String = chapters
        .iter()
        .enumerate()
        .map(|(i, _)| format!(r#"<itemref idref="c{i}"/>"#))
        .collect();
    let opf = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata><dc:title xmlns:dc="http://purl.org/dc/elements/1.1/">Test</dc:title></metadata><manifest>{manifest}<item id="css" href="style.css" media-type="text/css"/></manifest><spine>{spine}</spine></package>"#
    );
    let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/book.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;

    let mut entries = vec![
        entry("mimetype", "application/epub+zip"),
        entry("META-INF/container.xml", container),
        entry("OEBPS/book.opf", &opf),
        entry("OEBPS/style.css", "p { margin: 0 }"),
    ];
    for (i, (title, body)) in chapters.iter().enumerate() {
        entries.push(entry(&format!("OEBPS/ch{i}.xhtml"), &chapter(title, body)));
    }
    // `mimetype` must be first and stored; the fixture keeps that shape so the
    // writer's preservation of entry order is actually exercised.
    entries[0].compression = zip::CompressionMethod::Stored;
    zipedit::write_entries(&entries).expect("build fixture book")
}

#[test]
fn reads_chapters_headings_lists_and_emphasis() {
    let bytes = book(&[(
        "One",
        "<h1>Chapter One</h1><p>Hello <strong>world</strong> and <em>more</em>.</p><ul><li>a</li><li>b</li></ul><ol><li>first</li></ol><pre>code()</pre><hr/>",
    )]);
    let doc = epub::read(&bytes).expect("read");
    assert_eq!(doc.sections.len(), 1);
    assert_eq!(doc.sections[0].title.as_deref(), Some("One"));

    let blocks = &doc.sections[0].blocks;
    assert!(matches!(&blocks[0], Block::Heading { level: 1, .. }));
    assert!(matches!(&blocks[1], Block::Paragraph { spans } if spans[1].style.bold));
    assert!(matches!(
        &blocks[2],
        Block::ListItem {
            ordered: false,
            level: 0,
            ..
        }
    ));
    assert!(matches!(&blocks[4], Block::ListItem { ordered: true, .. }));
    assert!(matches!(&blocks[5], Block::Code { text } if text == "code()"));
    assert!(matches!(&blocks[6], Block::Rule));
}

#[test]
fn xhtml_indentation_does_not_leak_into_the_text() {
    let bytes = book(&[("One", "<p>\n      spread over\n      lines\n    </p>")]);
    let doc = epub::read(&bytes).expect("read");
    match &doc.sections[0].blocks[0] {
        Block::Paragraph { spans } => assert_eq!(spans[0].text, "spread over lines"),
        other => panic!("expected paragraph, got {other:?}"),
    }
}

#[test]
fn multi_chapter_buffers_carry_section_markers() {
    let bytes = book(&[("One", "<p>first</p>"), ("Two", "<p>second</p>")]);
    let doc = epub::read(&bytes).expect("read");
    let text = markdown::to_markdown(&doc);
    assert!(
        text.contains(r#"<!-- vibedoc:section id="OEBPS/ch0.xhtml" title="One" -->"#),
        "{text}"
    );

    let parsed = markdown::from_markdown(DocFormat::Epub, &text);
    assert_eq!(parsed.sections.len(), 2);
    assert_eq!(parsed.sections[1].id, "OEBPS/ch1.xhtml");
}

#[test]
fn editing_a_chapter_preserves_the_rest_of_the_container() {
    let bytes = book(&[("One", "<p>old text</p><p><img src=\"a.png\"/></p>")]);
    let doc = epub::read(&bytes).expect("read");
    let mut edited = doc.clone();
    edited.sections[0].blocks = markdown::from_markdown(DocFormat::Epub, "new text\n").sections[0]
        .blocks
        .clone();

    let rewritten = epub::write(&bytes, &edited).expect("write");
    let entries = zipedit::read_entries(&rewritten.bytes).expect("entries");
    assert_eq!(entries[0].name, "mimetype", "mimetype stays first");
    assert_eq!(
        entries[0].compression,
        zip::CompressionMethod::Stored,
        "and stays stored"
    );
    assert!(
        zipedit::find(&entries, "OEBPS/style.css").is_some(),
        "stylesheet preserved"
    );

    let xhtml = zipedit::text(zipedit::find(&entries, "OEBPS/ch0.xhtml").unwrap()).unwrap();
    assert!(xhtml.contains("new text"));
    assert!(!xhtml.contains("old text"));
    assert!(
        xhtml.contains("<img src=\"a.png\"/>"),
        "image kept: {xhtml}"
    );
    assert!(xhtml.contains("style.css"), "chapter head preserved");
}

#[test]
fn round_trips_markdown_through_the_book() {
    let bytes = book(&[("One", "<p>start</p>"), ("Two", "<p>second</p>")]);
    let doc = epub::read(&bytes).expect("read");
    let source = markdown::to_markdown(&doc).replace("start", "**changed**");
    let edited = markdown::from_markdown(DocFormat::Epub, &source);

    let rewritten = epub::write(&bytes, &edited).expect("write");
    let reread = epub::read(&rewritten.bytes).expect("re-read");
    assert_eq!(
        markdown::to_markdown(&reread).trim(),
        markdown::to_markdown(&rewritten.effective).trim(),
    );
    assert!(markdown::to_markdown(&reread).contains("**changed**"));
}

#[test]
fn adding_a_list_item_lands_inside_the_list() {
    let bytes = book(&[("One", "<ul><li>a</li></ul>")]);
    let doc = epub::read(&bytes).expect("read");
    let mut edited = doc.clone();
    edited.sections[0].blocks = markdown::from_markdown(DocFormat::Epub, "- a\n- b\n").sections[0]
        .blocks
        .clone();

    let rewritten = epub::write(&bytes, &edited).expect("write");
    let entries = zipedit::read_entries(&rewritten.bytes).expect("entries");
    let xhtml = zipedit::text(zipedit::find(&entries, "OEBPS/ch0.xhtml").unwrap()).unwrap();
    assert!(xhtml.contains("<ul><li>a</li><li>b</li></ul>"), "{xhtml}");
}

#[test]
fn dropping_a_chapter_marker_is_refused() {
    let bytes = book(&[("One", "<p>first</p>"), ("Two", "<p>second</p>")]);
    let edited = markdown::from_markdown(DocFormat::Epub, "just one section\n");
    let err = epub::write(&bytes, &edited).expect_err("chapter count change is refused");
    assert_eq!(err.kind(), "structure");
    assert!(err.to_string().contains("2 chapters"), "{err}");
}

/// Every case below came from a real book that opened in the editor and then
/// refused to save, because the text the reader produced was text XHTML cannot
/// store.
mod whitespace {
    use super::*;

    #[test]
    fn a_space_run_that_straddles_two_elements_is_collapsed_once() {
        // `<b>MEAP Edition </b> <i> Manning</i>` collapses each text node on its
        // own, so the buffer showed three spaces — text that, written back,
        // comes out as one.
        let book = book(&[(
            "One",
            "<p><b>MEAP Edition </b> <i> Manning Early Access</i></p>",
        )]);
        let document = epub::read(&book).expect("read");
        // One space, kept where it was drawn — at the end of the bold run.
        assert_eq!(
            markdown::to_markdown(&document).trim(),
            "**MEAP Edition **_Manning Early Access_"
        );

        // And it survives a save, which three spaces could not.
        let rewrite = epub::write(&book, &document).expect("write");
        let reread = epub::read(&rewrite.bytes).expect("re-read");
        assert_eq!(
            markdown::to_markdown(&reread),
            markdown::to_markdown(&document)
        );
    }

    #[test]
    fn an_empty_trailing_element_does_not_leave_a_space_behind() {
        let book = book(&[("One", "<h1>21st Century C <i> </i></h1>")]);
        let document = epub::read(&book).expect("read");
        assert_eq!(markdown::to_markdown(&document).trim(), "# 21st Century C");
    }

    #[test]
    fn typed_whitespace_is_dropped_and_reported_rather_than_claimed() {
        let book = book(&[("One", "<p>a line</p>")]);
        let edited = markdown::from_markdown(DocFormat::Epub, "a  longer   line \n");
        let rewrite = epub::write(&book, &edited).expect("write");

        assert!(
            rewrite
                .warnings
                .iter()
                .any(|w| w.code == "epub.whitespace_collapsed"),
            "{:?}",
            rewrite.warnings
        );
        let reread = epub::read(&rewrite.bytes).expect("re-read");
        assert_eq!(markdown::to_markdown(&reread).trim(), "a longer line");
        assert_eq!(
            markdown::to_markdown(&reread).trim(),
            markdown::to_markdown(&rewrite.effective).trim(),
            "what came back is what the writer said it stored"
        );
    }
}
