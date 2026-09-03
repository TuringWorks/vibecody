//! Portable Document Format (`.pdf`).
//!
//! A PDF has no paragraphs. It has glyphs at coordinates, so the text you read
//! on a page is something a reader *reconstructs* — which is why this module
//! opens a PDF as **plain text, one line per line of the page**, rather than
//! Markdown. There is no emphasis to recover and no structure to trust, and
//! offering a Markdown buffer would invite edits the format cannot store.
//!
//! What a save does, and does not do:
//!
//! * **Rewrites the words on a line.** Each character goes back to the run
//!   that drew the one it replaced, and is encoded through *that* run's font —
//!   which is what lets a line set in more than one font be edited at all,
//!   since each font in a modern PDF carries only the glyphs its own words
//!   used. Where the change cannot be dealt back that way, the line goes into
//!   its first run and the rest are emptied.
//! * **Deletes a line** when its text is cleared.
//! * **Refuses to add one.** A PDF does not reflow: a new line has no position,
//!   no font and no place in the page's content stream, so asking for one is an
//!   error rather than a guess.
//! * **Never re-flows.** Glyph positions are absolute. A longer line is not
//!   re-wrapped and may overrun the margin; the writer says so in a warning
//!   rather than letting the result be a surprise.
//! * **Refuses text the font cannot draw.** A subset font carries only the
//!   glyphs the document already used. A character outside that set would be
//!   written as a code the reader renders as something else, so the save stops
//!   and names the character.

mod font;

use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document as PdfFile, IncrementalDocument, Object, ObjectId, Stream};

use crate::error::DocError;
use crate::markdown;
use crate::model::{Block, DocFormat, Document, Section, Span, Warning};
use font::Encoder;

use similar::{Algorithm, DiffOp, TextDiff};

/// The result of rewriting a document.
#[derive(Debug)]
pub struct Rewrite {
    pub bytes: Vec<u8>,
    /// What the writer actually stored, for verification to compare against.
    pub effective: Document,
    pub warnings: Vec<Warning>,
    /// Whether `bytes` have already been read back and found to carry
    /// `effective`.
    ///
    /// Parsing a PDF is by far the most expensive thing here — a 1 373-page
    /// document measured 174 s in the parser and 6 s in everything this module
    /// does — so the caller is told when the check has already happened rather
    /// than paying for it twice.
    pub verified: bool,
}

/// A page's object id, as `lopdf` addresses it.
type PageId = (u32, u16);

/// The fonts of a page, by the resource name the content stream calls them.
type PageEncoders = BTreeMap<Vec<u8>, Rc<Encoder>>;

/// Encoders already built, by the object each font lives in.
///
/// A book's body font is on every one of its pages, and building an encoder
/// means parsing a `/ToUnicode` CMap and expanding its ranges. Doing that once
/// per page cost a 48 MB document twelve minutes and 900 MB before this
/// existed; done once per font it is not measurable.
type FontCache = HashMap<ObjectId, Rc<Encoder>>;

/// A `page-N` section id. Pages are numbered as the PDF numbers them.
fn page_section_id(page: u32) -> String {
    format!("page-{page}")
}

// ── Read ─────────────────────────────────────────────────────────────

/// Parse a `.pdf` into the document model: one section per page, one paragraph
/// per line of text.
pub fn read(bytes: &[u8]) -> Result<Document, DocError> {
    let file = load(bytes)?;
    let mut warnings = Vec::new();
    let mut cache = FontCache::new();
    let sections = file
        .get_pages()
        .into_iter()
        .map(|(number, id)| {
            let lines = read_page(&file, id, &mut cache, &mut warnings)?;
            Ok(Section {
                id: page_section_id(number),
                title: None,
                blocks: lines
                    .into_iter()
                    .map(|line| Block::Paragraph {
                        spans: vec![Span::plain(line.text)],
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, DocError>>()?;

    Ok(Document {
        format: DocFormat::Pdf,
        sections,
        warnings,
    })
}

fn load(bytes: &[u8]) -> Result<PdfFile, DocError> {
    let file = PdfFile::load_mem(bytes).map_err(|e| DocError::Parse(e.to_string()))?;
    if file.is_encrypted() {
        return Err(DocError::Parse(
            "this PDF is encrypted; open it in a reader that has the password and save an \
             unprotected copy before editing it here"
                .into(),
        ));
    }
    Ok(file)
}

/// One reconstructed line of a page, and the show operators that drew it.
struct Line {
    text: String,
    runs: Vec<RunRef>,
    /// Which run drew each character of `text`. `None` marks a space the reader
    /// put there because the page moved the pen — it belongs to no run, and
    /// putting it back means leaving the movement alone.
    owners: Vec<Option<usize>>,
    /// Per run: whether reading the line back inserts a space before it.
    gap_before: Vec<bool>,
}

impl Line {
    fn open() -> Line {
        Line {
            text: String::new(),
            runs: Vec::new(),
            owners: Vec::new(),
            gap_before: Vec::new(),
        }
    }
}

/// Where a run of text lives in a page's content stream.
#[derive(Clone)]
struct RunRef {
    /// Index into the page's operation list.
    operation: usize,
    /// Index into a `TJ` array, or `None` for the single string of `Tj`/`'`/`"`.
    element: Option<usize>,
    /// The font resource name in force when the run was drawn.
    font: Vec<u8>,
}

fn read_page(
    file: &PdfFile,
    page: PageId,
    cache: &mut FontCache,
    warnings: &mut Vec<Warning>,
) -> Result<Vec<Line>, DocError> {
    let Ok(content) = file.get_page_content(page) else {
        // A page with no content stream draws nothing. That is not an error.
        return Ok(Vec::new());
    };
    let operations = Content::decode(&content)
        .map_err(|e| DocError::Parse(format!("page content stream: {e}")))?
        .operations;
    let encoders = page_encoders(file, page, cache, warnings);
    Ok(scan(&operations, &encoders))
}

/// Build one encoder per font resource on the page, reporting what it had to
/// assume or could not read.
fn page_encoders(
    file: &PdfFile,
    page: PageId,
    cache: &mut FontCache,
    warnings: &mut Vec<Warning>,
) -> PageEncoders {
    page_fonts(file, page)
        .into_iter()
        .map(|(name, id, dict)| {
            if let Some(cached) = id.and_then(|id| cache.get(&id)) {
                return (name, Rc::clone(cached));
            }
            let report = Encoder::for_font(dict, file);
            let label = font_label(dict, &name);
            if report.assumed_base_encoding {
                push_once(
                    warnings,
                    Warning::new(
                        "pdf.assumed_encoding",
                        format!(
                            "font {label} names no encoding and carries no ToUnicode map; \
                             its codes are read as StandardEncoding, which may be wrong for \
                             quotes and accented letters"
                        ),
                    ),
                );
            }
            if !report.unknown_glyph_names.is_empty() {
                push_once(
                    warnings,
                    Warning::new(
                        "pdf.unknown_glyphs",
                        format!(
                            "font {label} names glyphs this build does not know ({}); \
                             text drawn with them is left out of the buffer rather than guessed",
                            report.unknown_glyph_names.join(", ")
                        ),
                    ),
                );
            }
            if !report.encoder.is_writable() {
                push_once(
                    warnings,
                    Warning::new(
                        "pdf.unreadable_font",
                        format!(
                            "font {label} maps its codes through a scheme this build cannot \
                             read; text drawn with it is not shown and lines using it cannot \
                             be edited"
                        ),
                    ),
                );
            }
            let encoder = Rc::new(report.encoder);
            if let Some(id) = id {
                cache.insert(id, Rc::clone(&encoder));
            }
            (name, encoder)
        })
        .collect()
}

/// A page's font resources: the name the content stream uses, the object the
/// font lives in when it has one, and the font dictionary itself.
///
/// `lopdf` has `get_page_fonts`, but it hands back only the dictionaries — and
/// without the object id there is nothing to cache an encoder against.
fn page_fonts(file: &PdfFile, page: PageId) -> Vec<(Vec<u8>, Option<ObjectId>, &Dictionary)> {
    let Ok((own, inherited)) = file.get_page_resources(page) else {
        return Vec::new();
    };
    let resources = own.into_iter().chain(
        inherited
            .into_iter()
            .filter_map(|id| file.get_dictionary(id).ok()),
    );

    let mut fonts: Vec<(Vec<u8>, Option<ObjectId>, &Dictionary)> = Vec::new();
    for resource in resources {
        let table = match resource.get(b"Font") {
            Ok(Object::Reference(id)) => file.get_dictionary(*id).ok(),
            Ok(Object::Dictionary(dict)) => Some(dict),
            _ => None,
        };
        let Some(table) = table else { continue };
        for (name, value) in table.iter() {
            // Resources closer to the page win over inherited ones.
            if fonts.iter().any(|(seen, _, _)| seen == name) {
                continue;
            }
            match value {
                Object::Reference(id) => {
                    if let Ok(dict) = file.get_dictionary(*id) {
                        fonts.push((name.clone(), Some(*id), dict));
                    }
                }
                Object::Dictionary(dict) => fonts.push((name.clone(), None, dict)),
                _ => {}
            }
        }
    }
    fonts
}

fn font_label(dict: &Dictionary, resource: &[u8]) -> String {
    dict.get(b"BaseFont")
        .and_then(Object::as_name)
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .unwrap_or_else(|_| format!("/{}", String::from_utf8_lossy(resource)))
}

fn push_once(warnings: &mut Vec<Warning>, warning: Warning) {
    if !warnings.iter().any(|w| w.code == warning.code) {
        warnings.push(warning);
    }
}

// ── Walking a content stream ─────────────────────────────────────────

/// A 2×3 affine matrix, in PDF's row-vector order: `[a b c d e f]`.
type Matrix = [f64; 6];

const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn multiply(m: Matrix, n: Matrix) -> Matrix {
    [
        m[0] * n[0] + m[1] * n[2],
        m[0] * n[1] + m[1] * n[3],
        m[2] * n[0] + m[3] * n[2],
        m[2] * n[1] + m[3] * n[3],
        m[4] * n[0] + m[5] * n[2] + n[4],
        m[4] * n[1] + m[5] * n[3] + n[5],
    ]
}

fn translation(tx: f64, ty: f64) -> Matrix {
    [1.0, 0.0, 0.0, 1.0, tx, ty]
}

/// Where the text origin currently sits on the page.
fn device_origin(text: Matrix, ctm: Matrix) -> (f64, f64) {
    let m = multiply(text, ctm);
    (m[4], m[5])
}

fn number(operand: Option<&Object>) -> f64 {
    match operand {
        Some(Object::Integer(n)) => *n as f64,
        Some(Object::Real(n)) => *n as f64,
        _ => 0.0,
    }
}

/// A `TJ` adjustment at least this large (in thousandths of an em) is a gap
/// between words rather than kerning. The value is the one PDF text extractors
/// have converged on; anything smaller is letter-fitting.
const WORD_GAP: f64 = 180.0;

/// Two text origins whose page `y` differs by less than this are on one line.
const SAME_LINE: f64 = 0.5;

/// Reconstruct the page's lines from its operations.
fn scan(operations: &[Operation], encoders: &PageEncoders) -> Vec<Line> {
    let mut lines: Vec<Line> = Vec::new();
    let mut open: Option<(Line, f64)> = None;
    let mut ctm = IDENTITY;
    let mut stack: Vec<Matrix> = Vec::new();
    let mut text = IDENTITY;
    let mut line_matrix = IDENTITY;
    let mut leading = 0.0f64;
    let mut font: Vec<u8> = Vec::new();
    let mut gap = false;

    let show = |bytes: &[u8],
                reference: RunRef,
                open: &mut Option<(Line, f64)>,
                lines: &mut Vec<Line>,
                text: Matrix,
                ctm: Matrix,
                gap: &mut bool| {
        let decoded = encoders
            .get(&reference.font)
            .and_then(|encoder| encoder.decode(bytes));
        let (_, y) = device_origin(text, ctm);

        let starts_new_line = match open {
            Some((_, line_y)) => (y - *line_y).abs() > SAME_LINE,
            None => true,
        };
        if starts_new_line {
            if let Some((finished, _)) = open.take() {
                if !finished.text.is_empty() {
                    lines.push(finished);
                }
            }
            *open = Some((Line::open(), y));
            *gap = false;
        }
        let Some((line, _)) = open.as_mut() else {
            return;
        };
        line.runs.push(reference);
        line.gap_before.push(false);

        // A run whose codes have no characters contributes nothing: the reader
        // saw glyphs this build cannot name, and inventing them is worse than
        // leaving the gap visible.
        let Some(decoded) = decoded.filter(|d| !d.is_empty()) else {
            return;
        };
        if *gap && !line.text.is_empty() && !line.text.ends_with(' ') && !decoded.starts_with(' ') {
            line.text.push(' ');
            line.owners.push(None);
            if let Some(slot) = line.gap_before.last_mut() {
                *slot = true;
            }
        }
        let run = line.runs.len() - 1;
        line.text.push_str(&decoded);
        line.owners.extend(decoded.chars().map(|_| Some(run)));
        *gap = false;
    };

    for (index, operation) in operations.iter().enumerate() {
        let ops = &operation.operands;
        match operation.operator.as_str() {
            "q" => stack.push(ctm),
            "Q" => ctm = stack.pop().unwrap_or(IDENTITY),
            "cm" => {
                let m = [
                    number(ops.first()),
                    number(ops.get(1)),
                    number(ops.get(2)),
                    number(ops.get(3)),
                    number(ops.get(4)),
                    number(ops.get(5)),
                ];
                ctm = multiply(m, ctm);
            }
            "BT" => {
                text = IDENTITY;
                line_matrix = IDENTITY;
            }
            "Tf" => {
                font = match ops.first() {
                    Some(Object::Name(name)) => name.clone(),
                    _ => Vec::new(),
                }
            }
            "TL" => leading = number(ops.first()),
            "Td" => {
                line_matrix = multiply(
                    translation(number(ops.first()), number(ops.get(1))),
                    line_matrix,
                );
                text = line_matrix;
                gap = true;
            }
            "TD" => {
                leading = -number(ops.get(1));
                line_matrix = multiply(
                    translation(number(ops.first()), number(ops.get(1))),
                    line_matrix,
                );
                text = line_matrix;
                gap = true;
            }
            "Tm" => {
                line_matrix = [
                    number(ops.first()),
                    number(ops.get(1)),
                    number(ops.get(2)),
                    number(ops.get(3)),
                    number(ops.get(4)),
                    number(ops.get(5)),
                ];
                text = line_matrix;
                gap = true;
            }
            "T*" => {
                line_matrix = multiply(translation(0.0, -leading), line_matrix);
                text = line_matrix;
                gap = true;
            }
            "Tj" => {
                if let Some(Object::String(bytes, _)) = ops.first() {
                    let reference = RunRef {
                        operation: index,
                        element: None,
                        font: font.clone(),
                    };
                    show(bytes, reference, &mut open, &mut lines, text, ctm, &mut gap);
                }
            }
            "'" | "\"" => {
                line_matrix = multiply(translation(0.0, -leading), line_matrix);
                text = line_matrix;
                gap = true;
                if let Some(Object::String(bytes, _)) = ops.last() {
                    let reference = RunRef {
                        operation: index,
                        element: None,
                        font: font.clone(),
                    };
                    show(bytes, reference, &mut open, &mut lines, text, ctm, &mut gap);
                }
            }
            "TJ" => {
                let Some(Object::Array(items)) = ops.first() else {
                    continue;
                };
                for (element, item) in items.iter().enumerate() {
                    match item {
                        Object::String(bytes, _) => {
                            let reference = RunRef {
                                operation: index,
                                element: Some(element),
                                font: font.clone(),
                            };
                            show(bytes, reference, &mut open, &mut lines, text, ctm, &mut gap);
                        }
                        Object::Integer(_) | Object::Real(_) if -number(Some(item)) >= WORD_GAP => {
                            gap = true;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    if let Some((finished, _)) = open {
        if !finished.text.is_empty() {
            lines.push(finished);
        }
    }
    lines
}

// ── Write ────────────────────────────────────────────────────────────

/// Rewrite a `.pdf`, replacing the text of the lines that changed.
pub fn write(original: &[u8], target: &Document) -> Result<Rewrite, DocError> {
    let file = load(original)?;
    let pages = file.get_pages();
    let wanted = expected_sections(&pages, target)?;

    let mut warnings = Vec::new();
    let mut sections = Vec::new();
    let mut changes: Vec<(PageId, Vec<u8>)> = Vec::new();
    let mut cache = FontCache::new();

    for (number, id) in &pages {
        let blocks = wanted.get(number).cloned().unwrap_or_default();
        let (stored, content) =
            rewrite_page(&file, *id, *number, &blocks, &mut cache, &mut warnings)?;
        if let Some(content) = content {
            changes.push((*id, content));
        }
        sections.push(Section {
            id: page_section_id(*number),
            title: None,
            blocks: stored,
        });
    }

    let effective = Document {
        format: DocFormat::Pdf,
        sections,
        warnings: Vec::new(),
    };

    // Nothing changed: hand back exactly the bytes that came in. Saving a
    // buffer nobody edited must not rewrite the file.
    if changes.is_empty() {
        return Ok(Rewrite {
            bytes: original.to_vec(),
            effective,
            warnings,
            // The bytes are the ones that were read to build `effective`.
            verified: true,
        });
    }

    // An incremental update leaves every original byte where it is and appends
    // only the pages that changed, which keeps a packed document packed —
    // rebuilding one costs up to 70% more bytes. It is only used when the
    // result reads back as what was asked for; otherwise the file is rebuilt,
    // which always does.
    let (bytes, verified) = match append_update(original, file, &changes) {
        Ok(appended) if reads_back(&appended, &effective) => (appended, true),
        _ => (rebuild(original, &changes)?, false),
    };

    Ok(Rewrite {
        bytes,
        effective,
        warnings,
        verified,
    })
}

/// Whether a written file carries exactly the text of `effective`.
fn reads_back(bytes: &[u8], effective: &Document) -> bool {
    read(bytes)
        .map(|reread| render(&reread) == render(effective))
        .unwrap_or(false)
}

/// Append the changed pages as an incremental update to the original bytes.
fn append_update(
    original: &[u8],
    file: PdfFile,
    changes: &[(PageId, Vec<u8>)],
) -> Result<Vec<u8>, DocError> {
    let mut update = IncrementalDocument::create_from(original.to_vec(), file);
    for (page, content) in changes {
        let stream = update.new_document.add_object(content_stream(content));
        update
            .opt_clone_object_to_new_document(*page)
            .map_err(|e| DocError::Container(e.to_string()))?;
        update
            .new_document
            .get_dictionary_mut(*page)
            .map_err(|e| DocError::Structure(e.to_string()))?
            .set("Contents", Object::Reference(stream));
    }
    let mut bytes = Vec::new();
    update
        .save_to(&mut bytes)
        .map_err(|e| DocError::Container(e.to_string()))?;
    Ok(bytes)
}

/// Write the whole file out again with the changed pages in place.
fn rebuild(original: &[u8], changes: &[(PageId, Vec<u8>)]) -> Result<Vec<u8>, DocError> {
    let mut file = load(original)?;
    for (page, content) in changes {
        let stream = file.add_object(content_stream(content));
        file.get_dictionary_mut(*page)
            .map_err(|e| DocError::Structure(e.to_string()))?
            .set("Contents", Object::Reference(stream));
    }
    let mut bytes = Vec::new();
    file.save_to(&mut bytes)
        .map_err(|e| DocError::Container(e.to_string()))?;
    Ok(bytes)
}

/// A page's content as one stream.
///
/// A page whose content arrived as several streams is stored as one: the parts
/// are only meaningful concatenated, and splitting the rewritten operators back
/// across the original boundaries could cut a `BT … ET` block in half.
fn content_stream(content: &[u8]) -> Stream {
    let mut stream = Stream::new(Dictionary::new(), content.to_vec());
    // Compression is a courtesy, not a requirement: an uncompressed stream is
    // still a valid one, so a failure here must not fail the save.
    let _ = stream.compress();
    stream
}

/// Match the buffer's sections to the file's pages.
///
/// A page marker that was deleted, renamed or duplicated would silently move
/// one page's text onto another, so the mismatch is an error naming what went
/// missing rather than a best guess.
fn expected_sections(
    pages: &BTreeMap<u32, PageId>,
    target: &Document,
) -> Result<BTreeMap<u32, Vec<Block>>, DocError> {
    // A single-page document carries no markers, so its one unnamed section is
    // that page.
    if pages.len() == 1 && target.sections.len() == 1 && target.sections[0].id.is_empty() {
        let only = *pages.keys().next().unwrap_or(&1);
        return Ok(BTreeMap::from([(only, target.sections[0].blocks.clone())]));
    }

    let mut wanted = BTreeMap::new();
    for section in &target.sections {
        let number = pages
            .keys()
            .find(|page| page_section_id(**page) == section.id)
            .ok_or_else(|| {
                DocError::Structure(format!(
                    "the buffer has a section {:?} that is not a page of this PDF; \
                     page markers must be left as they are",
                    section.id
                ))
            })?;
        if wanted.insert(*number, section.blocks.clone()).is_some() {
            return Err(DocError::Structure(format!(
                "page {number} appears twice in the buffer"
            )));
        }
    }
    for page in pages.keys() {
        if !wanted.contains_key(page) {
            return Err(DocError::Structure(format!(
                "the marker for page {page} is missing from the buffer; \
                 a page cannot be removed from a PDF here"
            )));
        }
    }
    Ok(wanted)
}

/// Apply one page's blocks to its content stream.
///
/// Returns what was stored, and the page's new content when anything changed.
fn rewrite_page(
    file: &PdfFile,
    page: PageId,
    number: u32,
    blocks: &[Block],
    cache: &mut FontCache,
    warnings: &mut Vec<Warning>,
) -> Result<(Vec<Block>, Option<Vec<u8>>), DocError> {
    let Ok(content) = file.get_page_content(page) else {
        return Ok((Vec::new(), None));
    };
    let mut operations = Content::decode(&content)
        .map_err(|e| DocError::Parse(format!("page content stream: {e}")))?
        .operations;
    // Font warnings were already reported by the read that filled the buffer.
    let encoders = page_encoders(file, page, cache, &mut Vec::new());
    let lines = scan(&operations, &encoders);

    // A blank line in the buffer is not a line of the page: the reader never
    // produces one, so it is neither an insert nor a change. The test is
    // emptiness and not blankness, because `scan` keeps a line of spaces — the
    // two must agree or an untouched buffer would look like a deletion.
    let target: Vec<String> = blocks
        .iter()
        .map(Block::plain_text)
        .filter(|text| !text.is_empty())
        .collect();

    let edits = align(&lines, &target)?;
    // Pixels written straight into the content stream between `BI` and `EI` do
    // not survive being taken apart and put back together, and what follows one
    // is lost with it. The page is left alone rather than half-rewritten.
    if !edits.is_empty() && operations.iter().any(|op| op.operator == "BI") {
        return Err(inline_image(number));
    }
    for (index, text) in &edits {
        set_line_text(&mut operations, &lines[*index], text, &encoders, warnings)?;
    }

    let changed = match edits.is_empty() {
        true => None,
        false => {
            let encoded = Content { operations }
                .encode()
                .map_err(|e| DocError::Container(e.to_string()))?;
            confirm_page(&encoded, &encoders, &target, number)?;
            Some(encoded)
        }
    };

    let stored = target
        .into_iter()
        .map(|text| Block::Paragraph {
            spans: vec![Span::plain(text)],
        })
        .collect();
    Ok((stored, changed))
}

/// The page holds an inline image, so its content stream cannot be rebuilt.
fn inline_image(number: u32) -> DocError {
    DocError::Structure(format!(
        "page {number} cannot be rewritten: it draws an inline image, and a content \
         stream holding one does not survive being rebuilt. Its text is readable \
         here, but saving a change to it would not give back the page you see."
    ))
}

/// Read the rewritten content stream back and confirm it draws the intended
/// lines, before it is put anywhere near the file.
///
/// The inline-image check above covers the one construct known to break; this
/// is the backstop for whatever else a content stream can hold. Rather than let
/// the whole-file check report a mangled page with no explanation, the page says
/// here that it cannot be rewritten, and the file is never touched.
fn confirm_page(
    encoded: &[u8],
    encoders: &PageEncoders,
    target: &[String],
    number: u32,
) -> Result<(), DocError> {
    let refused = |reason: &str| {
        Err(DocError::Structure(format!(
            "page {number} cannot be rewritten: {reason}. Its text is readable here, \
             but saving a change to it would not give back the page you see."
        )))
    };
    let Ok(reread) = Content::decode(encoded) else {
        return refused("the rewritten content stream does not parse");
    };
    let lines = scan(&reread.operations, encoders);
    if lines.len() != target.len() || lines.iter().zip(target).any(|(a, b)| &a.text != b) {
        return refused("its content stream holds something this build cannot write back");
    }
    Ok(())
}

/// Pair the page's lines with the buffer's, refusing anything but a rewrite or
/// a deletion.
fn align(lines: &[Line], target: &[String]) -> Result<Vec<(usize, String)>, DocError> {
    let old: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
    let new: Vec<&str> = target.iter().map(String::as_str).collect();
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .diff_slices(&old, &new);

    let mut edits = Vec::new();
    for op in diff.ops() {
        match *op {
            DiffOp::Equal { .. } => {}
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for k in 0..old_len {
                    edits.push((old_index + k, String::new()));
                }
            }
            DiffOp::Insert { new_index, .. } => {
                return Err(DocError::Structure(format!(
                    "a line cannot be added to a PDF: {:?} has no place on the page. \
                     A PDF stores glyphs at fixed positions and does not re-flow, so text \
                     can be changed or removed here but not inserted",
                    truncate(new[new_index])
                )));
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                if new_len > old_len {
                    return Err(DocError::Structure(format!(
                        "a line cannot be added to a PDF: {:?} has no place on the page. \
                         A PDF stores glyphs at fixed positions and does not re-flow, so text \
                         can be changed or removed here but not inserted",
                        truncate(new[new_index + old_len])
                    )));
                }
                for k in 0..old_len {
                    let text = new.get(new_index + k).copied().unwrap_or("");
                    edits.push((old_index + k, text.to_string()));
                }
            }
        }
    }
    Ok(edits)
}

fn truncate(line: &str) -> String {
    let limit = 60;
    match line.chars().count() <= limit {
        true => line.to_string(),
        false => format!("{}…", line.chars().take(limit).collect::<String>()),
    }
}

/// Put `text` on the line.
///
/// Two ways, in order. **Keep every character in the run that drew it**, which
/// is what lets a line set in more than one font be edited at all: a subset
/// font carries only the glyphs its own words used, so a heading whose one bold
/// word came from a second font has no single font that can draw the whole line.
/// Where that does not work out, the line goes into its first run and the rest
/// are emptied.
fn set_line_text(
    operations: &mut [Operation],
    line: &Line,
    text: &str,
    encoders: &PageEncoders,
    warnings: &mut Vec<Warning>,
) -> Result<(), DocError> {
    if line.runs.is_empty() {
        return Ok(());
    }
    // Emptying a run whose glyphs could not be read would delete text nobody
    // can see in the buffer to put it back.
    for run in &line.runs {
        match encoders.get(&run.font) {
            Some(encoder) if encoder.is_writable() => {}
            Some(_) => {
                return Err(DocError::Structure(format!(
                    "{:?} is drawn with a font this build cannot map back to character \
                     codes, so it cannot be edited",
                    truncate(&line.text)
                )))
            }
            None => {
                return Err(DocError::Structure(format!(
                    "the font that drew {:?} is not among the page's resources, so the \
                     line cannot be rewritten",
                    truncate(&line.text)
                )))
            }
        }
    }

    if !text.is_empty() && text.chars().count() > line.text.chars().count() {
        push_once(
            warnings,
            Warning::new(
                "pdf.no_reflow",
                "a line got longer. A PDF places glyphs at fixed positions and does not \
                 re-wrap, so longer text runs past where the original ended",
            ),
        );
    }

    // Dealing the characters back to their own runs is the better answer when it
    // works. When it does not, its failure names the character that actually has
    // no home, so it is kept and preferred over whatever the fallback says.
    let mut dealt = None;
    if let Some(segments) = split_across_runs(line, text) {
        match encode_segments(line, &segments, encoders) {
            Ok(encoded) => {
                for (run, bytes) in line.runs.iter().zip(encoded) {
                    set_run(operations, run, bytes)?;
                }
                return Ok(());
            }
            Err(e) => dealt = Some(e),
        }
    }

    let first = &line.runs[0];
    let encoder = &encoders[&first.font];
    let show = encode_show(encoder, text).map_err(|e| match line.runs.len() > 1 {
        // Saying only "the font has no glyph for it" would send the reader
        // looking at the wrong thing: the line is set in more than one font, and
        // the reason it went through this one is that the change could not be
        // dealt back across them.
        true => DocError::Structure(format!(
            "{e} The line is drawn as {} separately positioned runs and the change \
             could not be split back across them, so it had to go through the font of \
             the first alone.",
            line.runs.len()
        )),
        false => e,
    });
    let show = match (show, dealt) {
        (Err(_), Some(dealt)) => return Err(dealt),
        (result, _) => result?,
    };
    if line.runs.len() > 1 && !text.is_empty() {
        push_once(
            warnings,
            Warning::new(
                "pdf.line_joined",
                "a rewritten line was drawn as several separately positioned runs and \
                 could not be split back across them; it is stored as one run, so its \
                 spacing may differ from the original",
            ),
        );
    }

    // Empty the rest of the line first: a gapped rewrite replaces its whole
    // operator, and every string in one `TJ` array belongs to one line.
    for run in &line.runs[1..] {
        set_run(operations, run, Vec::new())?;
    }
    match show {
        Show::Codes(bytes) => set_run(operations, first, bytes),
        Show::Gapped(items) => set_gapped(operations, first, items),
    }
}

/// Deal each character of `text` back to the run that drew the character it
/// replaced — one string per run, or `None` when the result would not read back
/// as `text`.
fn split_across_runs(line: &Line, text: &str) -> Option<Vec<String>> {
    if line.runs.len() < 2 {
        return None;
    }
    let new: Vec<char> = text.chars().collect();
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .diff_chars(&line.text, text);

    // `None` is undecided; `Some(None)` is a character that belongs to no run.
    let mut owner: Vec<Option<Option<usize>>> = vec![None; new.len()];
    for op in diff.ops() {
        if let DiffOp::Equal {
            old_index,
            new_index,
            len,
        } = *op
        {
            for k in 0..len {
                owner[new_index + k] = Some(*line.owners.get(old_index + k)?);
            }
        }
    }
    // Typed characters join the run before them, or the first one that follows.
    let mut last: Option<usize> = None;
    for slot in owner.iter_mut() {
        match slot {
            Some(Some(run)) => last = Some(*run),
            Some(None) => {}
            undecided => *undecided = Some(last),
        }
    }
    let mut next: Option<usize> = None;
    for slot in owner.iter_mut().rev() {
        match slot {
            Some(Some(run)) => next = Some(*run),
            Some(None) => {}
            slot => *slot = Some(next),
        }
    }

    let mut segments = vec![String::new(); line.runs.len()];
    let mut reached = 0usize;
    for (c, slot) in new.iter().zip(&owner) {
        // A space the page draws by moving the pen stays a movement.
        let Some(Some(run)) = slot else { continue };
        if *run < reached {
            return None;
        }
        reached = *run;
        segments[*run].push(*c);
    }
    (predict(line, &segments) == text).then_some(segments)
}

/// What [`scan`] would read back from a line drawn as these segments.
fn predict(line: &Line, segments: &[String]) -> String {
    let mut out = String::new();
    let mut gap = false;
    for (run, segment) in segments.iter().enumerate() {
        gap |= line.gap_before.get(run).copied().unwrap_or(false);
        if segment.is_empty() {
            continue;
        }
        if gap && !out.is_empty() && !out.ends_with(' ') && !segment.starts_with(' ') {
            out.push(' ');
        }
        out.push_str(segment);
        gap = false;
    }
    out
}

/// Encode each segment through the font of its own run.
fn encode_segments(
    line: &Line,
    segments: &[String],
    encoders: &PageEncoders,
) -> Result<Vec<Vec<u8>>, DocError> {
    line.runs
        .iter()
        .zip(segments)
        .map(|(run, segment)| {
            let encoder = encoders
                .get(&run.font)
                .ok_or_else(|| DocError::Structure("a run's font left the page".into()))?;
            encoder.encode(segment).map_err(|c| {
                DocError::Structure(format!(
                    "the change lands in a run drawn with a font that has no glyph for \
                     {c:?}; a PDF can only be given characters its own fonts carry"
                ))
            })
        })
        .collect()
}

/// How a line's new text is drawn.
enum Show {
    /// One string of character codes.
    Codes(Vec<u8>),
    /// Strings separated by positioning gaps, for a font with no space glyph.
    Gapped(Vec<Object>),
}

/// A word gap, in thousandths of an em — wide enough that reading the page back
/// sees a space there, which is what makes the round trip hold.
const GAP_WIDTH: i64 = -250;

/// Encode a line's text, falling back to positioning where a font has no space.
///
/// Subsetted text fonts routinely carry no space glyph: TeX and its
/// descendants set words apart by moving the pen, not by drawing anything. The
/// space in the buffer is something the *reader* put there, so writing it back
/// means putting the gap back, not demanding a glyph the document never had.
fn encode_show(encoder: &Encoder, text: &str) -> Result<Show, DocError> {
    let missing = |c: char| {
        DocError::Structure(format!(
            "the font that drew this line has no glyph for {c:?}; \
             a PDF can only be given characters its own fonts carry"
        ))
    };
    match encoder.encode(text) {
        Ok(bytes) => Ok(Show::Codes(bytes)),
        Err(' ') => {
            // A gap stands for exactly one space. Two in a row, or one at
            // either end, would not read back as what was written.
            if text.starts_with(' ') || text.ends_with(' ') || text.contains("  ") {
                return Err(missing(' '));
            }
            let mut items = Vec::new();
            for (index, word) in text.split(' ').enumerate() {
                if index > 0 {
                    items.push(Object::Integer(GAP_WIDTH));
                }
                let codes = encoder.encode(word).map_err(missing)?;
                items.push(Object::String(codes, lopdf::StringFormat::Hexadecimal));
            }
            Ok(Show::Gapped(items))
        }
        Err(c) => Err(missing(c)),
    }
}

/// Replace a run's whole operator with a `TJ` array of strings and gaps.
fn set_gapped(
    operations: &mut [Operation],
    run: &RunRef,
    items: Vec<Object>,
) -> Result<(), DocError> {
    let operation = operations
        .get_mut(run.operation)
        .ok_or_else(|| DocError::Structure("lost track of a text operator".into()))?;
    // `'` and `"` move to the next line as well as showing text; turning one
    // into a `TJ` would drop that movement and shift the rest of the page.
    if matches!(operation.operator.as_str(), "'" | "\"") {
        return Err(DocError::Structure(
            "this line's spaces are drawn as positioning rather than as a space \
             character, and the operator that drew it cannot carry them; the line \
             cannot be rewritten"
                .into(),
        ));
    }
    operation.operator = "TJ".to_string();
    operation.operands = vec![Object::Array(items)];
    Ok(())
}

fn set_run(operations: &mut [Operation], run: &RunRef, bytes: Vec<u8>) -> Result<(), DocError> {
    let operation = operations
        .get_mut(run.operation)
        .ok_or_else(|| DocError::Structure("lost track of a text operator".into()))?;
    let slot = match run.element {
        None => operation
            .operands
            .iter_mut()
            .rev()
            .find(|o| matches!(o, Object::String(..))),
        Some(element) => match operation.operands.first_mut() {
            Some(Object::Array(items)) => items.get_mut(element),
            _ => None,
        },
    };
    let slot = slot.ok_or_else(|| DocError::Structure("lost track of a text run".into()))?;
    *slot = Object::String(bytes, lopdf::StringFormat::Hexadecimal);
    Ok(())
}

/// Render a PDF document as the plain-text buffer the editor shows.
///
/// The same call [`crate::render`] makes, kept here so the write path can
/// compare a rewritten file against what it meant to store without depending on
/// the caller having picked the right syntax for the format.
fn render(document: &Document) -> String {
    markdown::to_plain_text(document)
}
