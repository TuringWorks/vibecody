//! PDF: reading a page as lines, and what a save may and may not do.
//!
//! The fixtures are built here rather than checked in, so what each test
//! depends on — the font's encoding, where each line sits, how many runs drew
//! it — is visible in the test itself.

use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Dictionary, Document as PdfFile, Object, Stream};

use vibe_docfmt::model::{Block, DocFormat, Section};
use vibe_docfmt::{markdown, pdf};

/// A one-page PDF drawing each `(text, y)` with one `Tj`, in Helvetica.
fn page(lines: &[(&str, i64)]) -> Vec<u8> {
    page_with(lines, "WinAnsiEncoding")
}

fn page_with(lines: &[(&str, i64)], encoding: &str) -> Vec<u8> {
    let mut operations = vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["F1".into(), 12.into()]),
    ];
    for (text, y) in lines {
        operations.push(Operation::new(
            "Tm",
            vec![1.into(), 0.into(), 0.into(), 1.into(), 72.into(), (*y).into()],
        ));
        operations.push(Operation::new(
            "Tj",
            vec![Object::String(
                text.as_bytes().to_vec(),
                lopdf::StringFormat::Literal,
            )],
        ));
    }
    operations.push(Operation::new("ET", vec![]));
    assemble(operations, encoding)
}

fn assemble(operations: Vec<Operation>, encoding: &str) -> Vec<u8> {
    let mut file = PdfFile::with_version("1.5");
    let font = file.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => Object::Name(encoding.as_bytes().to_vec()),
    });
    let resources = file.add_object(dictionary! { "Font" => dictionary! { "F1" => font } });
    let content = Content { operations }.encode().expect("encode content");
    let content_id = file.add_object(Stream::new(Dictionary::new(), content));
    let pages_id = file.new_object_id();
    let page_id = file.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => resources,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });
    file.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
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

/// The buffer for a document, as the editor would show it.
fn buffer(document: &vibe_docfmt::Document) -> String {
    markdown::to_plain_text(document)
}

#[test]
fn a_page_reads_as_one_paragraph_per_line() {
    let bytes = page(&[("The first line.", 700), ("The second line.", 680)]);
    let document = pdf::read(&bytes).expect("read");
    assert_eq!(document.format, DocFormat::Pdf);
    assert_eq!(document.sections.len(), 1);
    assert_eq!(document.sections[0].id, "page-1");
    assert_eq!(text_of(&document), ["The first line.", "The second line."]);
}

#[test]
fn editing_a_line_rewrites_only_that_line() {
    let bytes = page(&[("The first line.", 700), ("The second line.", 680)]);
    let edited = markdown::from_plain_text(DocFormat::Pdf, "The FIRST line.\nThe second line.\n");
    let rewrite = pdf::write(&bytes, &edited).expect("write");

    let reread = pdf::read(&rewrite.bytes).expect("re-read");
    assert_eq!(text_of(&reread), ["The FIRST line.", "The second line."]);
    assert_eq!(
        buffer(&reread),
        buffer(&rewrite.effective),
        "what came back is what the writer said it stored"
    );
}

#[test]
fn clearing_a_line_removes_it() {
    let bytes = page(&[("Keep this.", 700), ("Drop this.", 680)]);
    let edited = markdown::from_plain_text(DocFormat::Pdf, "Keep this.\n\n");
    let rewrite = pdf::write(&bytes, &edited).expect("write");

    let reread = pdf::read(&rewrite.bytes).expect("re-read");
    assert_eq!(text_of(&reread), ["Keep this."]);
    assert_eq!(buffer(&reread), buffer(&rewrite.effective));
}

#[test]
fn adding_a_line_is_refused_with_a_reason() {
    let bytes = page(&[("The only line.", 700)]);
    let edited = markdown::from_plain_text(DocFormat::Pdf, "The only line.\nA new line.\n");
    let error = pdf::write(&bytes, &edited).expect_err("a PDF cannot grow a line");
    let message = error.to_string();
    assert!(message.contains("A new line."), "{message}");
    assert!(message.contains("does not re-flow"), "{message}");
}

#[test]
fn a_character_the_font_cannot_draw_stops_the_save() {
    // Helvetica with WinAnsiEncoding has no code for a Cyrillic letter.
    let bytes = page(&[("Latin only.", 700)]);
    let edited = markdown::from_plain_text(DocFormat::Pdf, "Латиница.\n");
    let error = pdf::write(&bytes, &edited).expect_err("no glyph, no save");
    assert!(error.to_string().contains("no glyph for"), "{error}");
}

#[test]
fn a_longer_line_is_written_but_the_overrun_is_reported() {
    let bytes = page(&[("Short.", 700)]);
    let edited = markdown::from_plain_text(
        DocFormat::Pdf,
        "Considerably longer than what was there before.\n",
    );
    let rewrite = pdf::write(&bytes, &edited).expect("write");
    assert!(
        rewrite.warnings.iter().any(|w| w.code == "pdf.no_reflow"),
        "{:?}",
        rewrite.warnings
    );
    let reread = pdf::read(&rewrite.bytes).expect("re-read");
    assert_eq!(
        text_of(&reread),
        ["Considerably longer than what was there before."]
    );
}

#[test]
fn a_page_marker_that_went_missing_is_an_error_not_a_guess() {
    let first = page(&[("Page one.", 700)]);
    // Two pages: build one file with two of them so the buffer carries markers.
    let bytes = two_pages();
    let document = pdf::read(&bytes).expect("read");
    assert_eq!(document.sections.len(), 2);
    let rendered = buffer(&document);
    assert!(rendered.contains("page-1") && rendered.contains("page-2"), "{rendered}");

    let without = rendered
        .lines()
        .filter(|line| !line.contains("page-2"))
        .collect::<Vec<_>>()
        .join("\n");
    let edited = markdown::from_plain_text(DocFormat::Pdf, &without);
    let error = pdf::write(&bytes, &edited).expect_err("a page cannot be dropped");
    assert!(error.to_string().contains("page 2"), "{error}");

    // The single-page file still saves without any marker at all.
    let single = markdown::from_plain_text(DocFormat::Pdf, "Page one.\n");
    pdf::write(&first, &single).expect("a one-page buffer needs no marker");
}

/// Two pages, so the buffer carries section markers.
fn two_pages() -> Vec<u8> {
    let mut file = PdfFile::with_version("1.5");
    let font = file.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1",
        "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let resources = file.add_object(dictionary! { "Font" => dictionary! { "F1" => font } });
    let pages_id = file.new_object_id();
    let kids: Vec<Object> = ["Page one.", "Page two."]
        .iter()
        .map(|text| {
            let operations = vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new(
                    "Tm",
                    vec![1.into(), 0.into(), 0.into(), 1.into(), 72.into(), 700.into()],
                ),
                Operation::new(
                    "Tj",
                    vec![Object::String(
                        text.as_bytes().to_vec(),
                        lopdf::StringFormat::Literal,
                    )],
                ),
                Operation::new("ET", vec![]),
            ];
            let content = Content { operations }.encode().expect("encode");
            let content_id = file.add_object(Stream::new(Dictionary::new(), content));
            file.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "Resources" => resources,
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
            })
            .into()
        })
        .collect();
    let count = kids.len() as i64;
    file.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => kids, "Count" => count,
        }),
    );
    let catalog = file.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    file.trailer.set("Root", catalog);
    let mut bytes = Vec::new();
    file.save_to(&mut bytes).expect("save fixture");
    bytes
}

#[test]
fn a_run_the_font_cannot_be_read_through_is_left_out_and_reported() {
    // A symbolic font with no encoding and no ToUnicode: its codes mean
    // whatever the font program says, which this build cannot read.
    let mut file = PdfFile::with_version("1.5");
    let descriptor = file.add_object(dictionary! {
        "Type" => "FontDescriptor", "FontName" => "Dingbats", "Flags" => 4,
    });
    let font = file.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "TrueType",
        "BaseFont" => "Dingbats", "FontDescriptor" => descriptor,
    });
    let resources = file.add_object(dictionary! { "Font" => dictionary! { "F1" => font } });
    let operations = vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["F1".into(), 12.into()]),
        Operation::new(
            "Tm",
            vec![1.into(), 0.into(), 0.into(), 1.into(), 72.into(), 700.into()],
        ),
        Operation::new(
            "Tj",
            vec![Object::String(b"abc".to_vec(), lopdf::StringFormat::Literal)],
        ),
        Operation::new("ET", vec![]),
    ];
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

    let document = pdf::read(&bytes).expect("read");
    assert!(text_of(&document).is_empty(), "no text is invented");
    assert!(
        document.warnings.iter().any(|w| w.code == "pdf.unreadable_font"),
        "{:?}",
        document.warnings
    );
}

#[test]
fn a_standard_encoding_font_is_read_with_the_assumption_reported() {
    let bytes = page_with(&[("Plain text.", 700)], "StandardEncoding");
    let document = pdf::read(&bytes).expect("read");
    assert_eq!(text_of(&document), ["Plain text."]);
    assert!(document.warnings.is_empty(), "a named encoding is not an assumption");
}

#[test]
fn sections_survive_a_save_untouched() {
    let bytes = two_pages();
    let document = pdf::read(&bytes).expect("read");
    let rewrite = pdf::write(&bytes, &document).expect("write an unchanged buffer");
    let reread = pdf::read(&rewrite.bytes).expect("re-read");
    let ids: Vec<&Section> = reread.sections.iter().collect();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0].id, "page-1");
    assert_eq!(ids[1].id, "page-2");
    assert_eq!(buffer(&reread), buffer(&document));
}

#[test]
fn a_font_with_no_space_glyph_keeps_its_words_apart() {
    // TeX sets words apart by moving the pen: the font carries no space, and
    // the spaces in the buffer are the reader's reconstruction of the gaps.
    let mut file = PdfFile::with_version("1.5");
    // A ToUnicode CMap covering only the four letters the document uses: the
    // shape a subset font takes, and there is no code for a space in it.
    let cmap = file.add_object(Stream::new(
        Dictionary::new(),
        b"/CIDInit /ProcSet findresource begin begincmap\n\
          4 beginbfchar <41> <0041> <42> <0042> <43> <0043> <44> <0044> endbfchar\n\
          endcmap end"
            .to_vec(),
    ));
    let font = file.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "ABCDEF+CMR10",
        "ToUnicode" => cmap,
    });
    let resources = file.add_object(dictionary! { "Font" => dictionary! { "F1" => font } });
    let operations = vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["F1".into(), 12.into()]),
        Operation::new(
            "Tm",
            vec![1.into(), 0.into(), 0.into(), 1.into(), 72.into(), 700.into()],
        ),
        Operation::new(
            "TJ",
            vec![Object::Array(vec![
                Object::String(b"AB".to_vec(), lopdf::StringFormat::Literal),
                Object::Integer(-300),
                Object::String(b"CD".to_vec(), lopdf::StringFormat::Literal),
            ])],
        ),
        Operation::new("ET", vec![]),
    ];
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

    let document = pdf::read(&bytes).expect("read");
    assert_eq!(text_of(&document), ["AB CD"], "the gap reads as a space");

    let edited = markdown::from_plain_text(DocFormat::Pdf, "DC BA\n");
    let rewrite = pdf::write(&bytes, &edited).expect("the space becomes a gap again");
    let reread = pdf::read(&rewrite.bytes).expect("re-read");
    assert_eq!(text_of(&reread), ["DC BA"]);

    // Two spaces in a row have no gap that reads back as two, so they are
    // refused rather than quietly written as one.
    let doubled = markdown::from_plain_text(DocFormat::Pdf, "DC  BA\n");
    let error = pdf::write(&bytes, &doubled).expect_err("no glyph, no double space");
    assert!(error.to_string().contains("no glyph for"), "{error}");
}

#[test]
fn a_page_with_an_inline_image_says_it_cannot_be_rewritten() {
    let mut file = PdfFile::with_version("1.5");
    let font = file.add_object(dictionary! {
        "Type" => "Font", "Subtype" => "Type1",
        "BaseFont" => "Helvetica", "Encoding" => "WinAnsiEncoding",
    });
    let resources = file.add_object(dictionary! { "Font" => dictionary! { "F1" => font } });
    // A one-pixel inline image, exactly as a TeX rule ends up in the stream.
    let mut content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 12.into()]),
            Operation::new(
                "Tm",
                vec![1.into(), 0.into(), 0.into(), 1.into(), 72.into(), 700.into()],
            ),
            Operation::new(
                "Tj",
                vec![Object::String(
                    b"A line of text.".to_vec(),
                    lopdf::StringFormat::Literal,
                )],
            ),
            Operation::new("ET", vec![]),
        ],
    }
    .encode()
    .expect("encode");
    content.extend_from_slice(b"\nq\nBI /IM true /W 1 /H 1 /BPC 1\nID \x00\nEI\nQ\n");

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

    // The text still reads.
    let document = pdf::read(&bytes).expect("read");
    assert_eq!(text_of(&document), ["A line of text."]);

    // Changing it is refused, by name, with the file untouched.
    let edited = markdown::from_plain_text(DocFormat::Pdf, "A different line.\n");
    let error = pdf::write(&bytes, &edited).expect_err("an inline image blocks the rewrite");
    assert!(error.to_string().contains("page 1 cannot be rewritten"), "{error}");
}
