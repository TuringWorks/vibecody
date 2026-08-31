//! A line set in more than one font is the ordinary case — a heading with one
//! bold word, a sentence with a monospaced identifier — and each font in a
//! modern PDF is *subset*: it carries only the glyphs its own words used. There
//! is therefore no single font on such a line that can draw the whole of it, and
//! writing the line back through the first one fails on the first character the
//! others contributed.
//!
//! These pin the behaviour that makes those lines editable: every character goes
//! back to the run that drew it.

use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Dictionary, Document as PdfFile, Object, Stream};

use vibe_docfmt::model::{Block, DocFormat};
use vibe_docfmt::{markdown, pdf};

/// A font that can draw exactly `glyphs`, and nothing else.
fn subset(file: &mut PdfFile, name: &str, glyphs: &str) -> Object {
    let entries: String = glyphs
        .chars()
        .map(|c| format!("<{:02x}> <{:04x}>", c as u32, c as u32))
        .collect::<Vec<_>>()
        .join(" ");
    let cmap = file.add_object(Stream::new(
        Dictionary::new(),
        format!(
            "/CIDInit /ProcSet findresource begin begincmap\n\
             {} beginbfchar {entries} endbfchar\nendcmap end",
            glyphs.chars().count()
        )
        .into_bytes(),
    ));
    file.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => Object::Name(format!("ABCDEF+{name}").into_bytes()),
        "ToUnicode" => cmap,
    })
    .into()
}

/// One page, one line, drawn as `(font resource, text)` runs side by side.
fn page(runs: &[(&str, &str)], fonts: &[(&str, &str)]) -> Vec<u8> {
    let mut file = PdfFile::with_version("1.5");
    let mut resources = Dictionary::new();
    for (name, glyphs) in fonts {
        resources.set(*name, subset(&mut file, name, glyphs));
    }
    let resources = file.add_object(dictionary! { "Font" => resources });

    let mut operations = vec![
        Operation::new("BT", vec![]),
        Operation::new(
            "Tm",
            vec![
                1.into(),
                0.into(),
                0.into(),
                1.into(),
                72.into(),
                700.into(),
            ],
        ),
    ];
    for (font, text) in runs {
        operations.push(Operation::new("Tf", vec![(*font).into(), 12.into()]));
        operations.push(Operation::new(
            "Tj",
            vec![Object::String(
                text.bytes().collect(),
                lopdf::StringFormat::Literal,
            )],
        ));
    }
    operations.push(Operation::new("ET", vec![]));

    let content = Content { operations }.encode().expect("encode");
    let content_id = file.add_object(Stream::new(Dictionary::new(), content));
    let pages_id = file.new_object_id();
    let page_id = file.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages_id, "Contents" => content_id,
        "Resources" => resources,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });
    file.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1,
        }),
    );
    let catalog = file.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    file.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    file.save_to(&mut bytes).expect("save fixture");
    bytes
}

fn text_of(document: &vibe_docfmt::Document) -> Vec<String> {
    document
        .sections
        .iter()
        .flat_map(|s| s.blocks.iter().map(Block::plain_text))
        .collect()
}

const ROMAN: &str = "Read the manual first ";
const BOLD: &str = "carefully";

#[test]
fn a_line_in_two_fonts_reads_as_one_line() {
    let bytes = page(
        &[("F1", "Read the "), ("F2", "carefully"), ("F1", " manual")],
        &[("F1", ROMAN), ("F2", BOLD)],
    );
    let document = pdf::read(&bytes).expect("read");
    assert_eq!(text_of(&document), ["Read the carefully manual"]);
}

#[test]
fn an_edit_goes_back_to_the_run_that_drew_it() {
    // "carefully" → "careful": the change is entirely inside the second font's
    // run, and the first font has no `y` to fall back on.
    let bytes = page(
        &[("F1", "Read the "), ("F2", "carefully"), ("F1", " manual")],
        &[("F1", ROMAN), ("F2", BOLD)],
    );
    let edited = markdown::from_plain_text(DocFormat::Pdf, "Read the careful manual\n");
    let rewrite = pdf::write(&bytes, &edited).expect("write");

    let reread = pdf::read(&rewrite.bytes).expect("re-read");
    assert_eq!(text_of(&reread), ["Read the careful manual"]);
    assert!(
        !rewrite.warnings.iter().any(|w| w.code == "pdf.line_joined"),
        "the runs were kept apart: {:?}",
        rewrite.warnings
    );
}

#[test]
fn a_change_in_the_first_font_leaves_the_second_alone() {
    let bytes = page(
        &[("F1", "Read the "), ("F2", "carefully"), ("F1", " manual")],
        &[("F1", ROMAN), ("F2", BOLD)],
    );
    let edited = markdown::from_plain_text(DocFormat::Pdf, "Read a carefully manual\n");
    let rewrite = pdf::write(&bytes, &edited).expect("write");
    let reread = pdf::read(&rewrite.bytes).expect("re-read");
    assert_eq!(text_of(&reread), ["Read a carefully manual"]);
}

#[test]
fn a_character_no_font_on_the_line_has_is_still_refused_by_name() {
    let bytes = page(
        &[("F1", "Read the "), ("F2", "carefully"), ("F1", " manual")],
        &[("F1", ROMAN), ("F2", BOLD)],
    );
    let edited = markdown::from_plain_text(DocFormat::Pdf, "Read the carefully manual!\n");
    let error = pdf::write(&bytes, &edited).expect_err("no font here draws '!'");
    assert!(error.to_string().contains("no glyph for '!'"), "{error}");
}
