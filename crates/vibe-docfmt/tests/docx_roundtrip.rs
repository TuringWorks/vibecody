//! DOCX read/write behaviour, built on a package assembled in-test so the
//! fixtures show exactly which XML each assertion depends on.

use vibe_docfmt::model::{Block, DocFormat, Span, SpanStyle};
use vibe_docfmt::zipedit::{self, ZipEntry};
use vibe_docfmt::{docx, markdown};

fn entry(name: &str, data: &str) -> ZipEntry {
    ZipEntry {
        name: name.to_string(),
        data: data.as_bytes().to_vec(),
        compression: zip::CompressionMethod::Deflated,
        is_dir: false,
    }
}

const W: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

fn package(body: &str) -> Vec<u8> {
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="{W}" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><w:body>{body}<w:sectPr><w:pgSz w:w="11906" w:h="16838"/></w:sectPr></w:body></w:document>"#
    );
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/></Relationships>"#;
    let numbering = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="{W}"><w:abstractNum w:abstractNumId="1"><w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/></w:lvl></w:abstractNum><w:abstractNum w:abstractNumId="2"><w:lvl w:ilvl="0"><w:numFmt w:val="decimal"/></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="1"/></w:num><w:num w:numId="2"><w:abstractNumId w:val="2"/></w:num></w:numbering>"#
    );
    let styles = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="{W}"><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/></w:style></w:styles>"#
    );
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#;

    zipedit::write_entries(&[
        entry("[Content_Types].xml", content_types),
        entry("word/document.xml", &document),
        entry("word/_rels/document.xml.rels", rels),
        entry("word/numbering.xml", &numbering),
        entry("word/styles.xml", &styles),
        entry("word/media/image1.png", "\u{0}PNG-bytes"),
    ])
    .expect("build fixture package")
}

fn para(text: &str) -> String {
    format!("<w:p><w:r><w:t>{text}</w:t></w:r></w:p>")
}

#[test]
fn reads_headings_emphasis_lists_and_tables() {
    let body = format!(
        "{}{}{}{}{}",
        r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>Title</w:t></w:r></w:p>"#,
        r#"<w:p><w:r><w:t>plain </w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>bold</w:t></w:r></w:p>"#,
        r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>bullet</w:t></w:r></w:p>"#,
        r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="2"/></w:numPr></w:pPr><w:r><w:t>numbered</w:t></w:r></w:p>"#,
        r#"<w:tbl><w:tr><w:tc><w:p><w:r><w:t>a</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>b</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"#,
    );
    let doc = docx::read(&package(&body)).expect("read");
    let blocks = &doc.sections[0].blocks;

    assert!(matches!(&blocks[0], Block::Heading { level: 1, spans } if spans[0].text == "Title"));
    match &blocks[1] {
        Block::Paragraph { spans } => {
            assert_eq!(spans[0].text, "plain ");
            assert!(!spans[0].style.bold);
            assert!(spans[1].style.bold, "second run is bold");
        }
        other => panic!("expected paragraph, got {other:?}"),
    }
    assert!(matches!(&blocks[2], Block::ListItem { ordered: false, .. }));
    assert!(matches!(&blocks[3], Block::ListItem { ordered: true, .. }));
    assert!(matches!(&blocks[4], Block::Table { rows } if rows[0].len() == 2));
}

#[test]
fn resolves_hyperlinks_through_the_rels_part() {
    let body = r#"<w:p><w:hyperlink r:id="rId1"><w:r><w:t>site</w:t></w:r></w:hyperlink></w:p>"#;
    let doc = docx::read(&package(body)).expect("read");
    match &doc.sections[0].blocks[0] {
        Block::Paragraph { spans } => {
            assert_eq!(spans[0].style.link.as_deref(), Some("https://example.com"))
        }
        other => panic!("expected paragraph, got {other:?}"),
    }
}

#[test]
fn empty_paragraphs_are_not_blocks_so_a_save_cannot_delete_them() {
    let body = format!("{}<w:p/>{}", para("one"), para("two"));
    let original = package(&body);
    let doc = docx::read(&original).expect("read");
    assert_eq!(doc.sections[0].blocks.len(), 2, "spacer paragraph is not a block");

    let rewritten = docx::write(&original, &doc).expect("write");
    let xml = document_xml(&rewritten.bytes);
    assert!(xml.contains("<w:p/>"), "spacer paragraph survived the save");
}

#[test]
fn editing_text_preserves_images_and_page_setup() {
    let body = format!(
        "{}{}",
        para("hello"),
        r#"<w:p><w:r><w:drawing><w:inline/></w:drawing></w:r></w:p>"#
    );
    let original = package(&body);
    let doc = docx::read(&original).expect("read");
    let edited = markdown::from_markdown(DocFormat::Docx, "goodbye\n");

    let rewritten = docx::write(&original, &edited).expect("write");
    let xml = document_xml(&rewritten.bytes);
    assert!(xml.contains("goodbye"), "new text written");
    assert!(!xml.contains("hello"), "old text replaced");
    assert!(xml.contains("<w:drawing>"), "image run preserved");
    assert!(xml.contains("<w:pgSz"), "page setup preserved");
    assert!(
        zipedit::find(&zipedit::read_entries(&rewritten.bytes).unwrap(), "word/media/image1.png")
            .is_some(),
        "media part preserved"
    );
    // The image paragraph carries no text, so it is not a block and is untouched.
    assert_eq!(doc.sections[0].blocks.len(), 1);
}

#[test]
fn round_trips_markdown_through_the_file() {
    let original = package(&para("start"));
    let source = "# Heading\n\nsome **bold** and *italic* and `code`\n\n- one\n- two\n\n1. first\n";
    let edited = markdown::from_markdown(DocFormat::Docx, source);
    let rewritten = docx::write(&original, &edited).expect("write");

    let reread = docx::read(&rewritten.bytes).expect("re-read");
    assert_eq!(
        markdown::to_markdown(&reread).trim(),
        markdown::to_markdown(&rewritten.effective).trim(),
        "what came back is what the writer said it stored"
    );
    assert_eq!(markdown::to_markdown(&reread).trim(), source.trim());
}

#[test]
fn a_new_hyperlink_gets_a_relationship() {
    let original = package(&para("start"));
    let edited = markdown::from_markdown(DocFormat::Docx, "see [docs](https://vibecody.dev)\n");
    let rewritten = docx::write(&original, &edited).expect("write");

    let entries = zipedit::read_entries(&rewritten.bytes).expect("entries");
    let rels = zipedit::text(zipedit::find(&entries, "word/_rels/document.xml.rels").unwrap())
        .expect("rels text");
    assert!(rels.contains("https://vibecody.dev"), "relationship added: {rels}");

    let reread = docx::read(&rewritten.bytes).expect("re-read");
    match &reread.sections[0].blocks[0] {
        Block::Paragraph { spans } => assert!(
            spans.iter().any(|s| s.style.link.as_deref() == Some("https://vibecody.dev")),
            "link reads back"
        ),
        other => panic!("expected paragraph, got {other:?}"),
    }
}

#[test]
fn code_blocks_are_reported_as_flattened_not_silently_changed() {
    let original = package(&para("start"));
    let edited = markdown::from_markdown(DocFormat::Docx, "```\nfn main() {}\n```\n");
    let rewritten = docx::write(&original, &edited).expect("write");

    assert!(
        rewritten.warnings.iter().any(|w| w.code == "docx.code_block_flattened"),
        "degradation is reported: {:?}",
        rewritten.warnings
    );
    let reread = docx::read(&rewritten.bytes).expect("re-read");
    assert_eq!(
        markdown::to_markdown(&reread).trim(),
        markdown::to_markdown(&rewritten.effective).trim(),
        "the file matches what the writer said it stored"
    );
}

#[test]
fn adding_a_table_row_is_refused_rather_than_guessed() {
    let body = r#"<w:tbl><w:tr><w:tc><w:p><w:r><w:t>a</w:t></w:r></w:p></w:tc></w:tr></w:tbl>"#;
    let original = package(body);
    let edited = markdown::from_markdown(DocFormat::Docx, "| a |\n| --- |\n| b |\n");
    let err = docx::write(&original, &edited).expect_err("row count change is refused");
    assert_eq!(err.kind(), "structure");
}

#[test]
fn a_bold_only_change_is_applied() {
    let original = package(&para("word"));
    let edited = markdown::from_markdown(DocFormat::Docx, "**word**\n");
    let rewritten = docx::write(&original, &edited).expect("write");
    let reread = docx::read(&rewritten.bytes).expect("re-read");
    match &reread.sections[0].blocks[0] {
        Block::Paragraph { spans } => assert_eq!(
            spans[0],
            Span { text: "word".into(), style: SpanStyle { bold: true, ..SpanStyle::plain() } }
        ),
        other => panic!("expected paragraph, got {other:?}"),
    }
}

fn document_xml(bytes: &[u8]) -> String {
    let entries = zipedit::read_entries(bytes).expect("entries");
    zipedit::text(zipedit::find(&entries, "word/document.xml").expect("document part"))
        .expect("utf-8")
}
