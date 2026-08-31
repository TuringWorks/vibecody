//! Markdown <-> [`Document`] conversion.
//!
//! This is the only place that decides what the editable buffer looks like, so
//! both directions live together and are tested as a pair: `to_markdown` after
//! `from_markdown` must be a fixed point, which is what lets `write` verify a
//! rewritten file by comparing canonical Markdown instead of trusting the
//! format writer.

use crate::model::{Block, DocFormat, Document, Section, Span, SpanStyle};

/// Marker emitted before each section when a document has more than one.
///
/// It is a Markdown comment so the buffer still renders as Markdown, and it is
/// what routes an edited region back to the right EPUB chapter / Pages storage.
const SECTION_PREFIX: &str = "<!-- vibedoc:section ";

// ── Emit ─────────────────────────────────────────────────────────────

/// Render a document as Markdown.
pub fn to_markdown(doc: &Document) -> String {
    let multi = doc.sections.len() > 1;
    let mut out = String::new();
    for section in &doc.sections {
        if multi {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&section_marker(section));
            out.push_str("\n\n");
        }
        out.push_str(&blocks_to_markdown(&section.blocks));
    }
    // Exactly one trailing newline: a text buffer people edit should not carry
    // an invisible tail that flips `isDirty` on open.
    let trimmed = out.trim_end_matches('\n');
    let mut result = trimmed.to_string();
    if !result.is_empty() {
        result.push('\n');
    }
    result
}

fn section_marker(section: &Section) -> String {
    let title = section.title.as_deref().unwrap_or("");
    format!(
        "{SECTION_PREFIX}id=\"{}\" title=\"{}\" -->",
        attr_escape(&section.id),
        attr_escape(title)
    )
}

fn attr_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
}

fn attr_unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some(next) => out.push(next),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn blocks_to_markdown(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in blocks {
        match block {
            Block::Heading { level, spans } => {
                let hashes = "#".repeat((*level).clamp(1, 6) as usize);
                out.push_str(&format!("{hashes} {}\n\n", spans_to_markdown(spans)));
            }
            Block::Paragraph { spans } => {
                out.push_str(&format!(
                    "{}\n\n",
                    escape_block_start(&spans_to_markdown(spans))
                ));
            }
            Block::ListItem {
                level,
                ordered,
                spans,
            } => {
                let indent = "  ".repeat(*level as usize);
                let bullet = if *ordered { "1." } else { "-" };
                out.push_str(&format!("{indent}{bullet} {}\n", spans_to_markdown(spans)));
            }
            Block::Code { text } => {
                // A buffer is split on `\n` and a `\r` before one is dropped by
                // the split, so a block ending in CRLF came back a line short.
                // The block's own line endings are normalised on the way out.
                let text = text.replace("\r\n", "\n").replace('\r', "\n");
                let fence = code_fence_for(&text);
                out.push_str(&format!(
                    "{fence}\n{}\n{fence}\n\n",
                    text.trim_end_matches('\n')
                ));
            }
            Block::Table { rows } => {
                out.push_str(&table_to_markdown(rows));
            }
            Block::Rule => out.push_str("---\n\n"),
        }
    }
    normalize_list_spacing(&out)
}

/// Insert the blank line that ends a run of list items.
///
/// Fenced blocks are stepped over: a code sample containing a line that opens
/// with `- ` is not a list, and the blank line inserted after it landed inside
/// the block and shifted everything below it by a line.
fn normalize_list_spacing(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = String::new();
    let mut fence: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        out.push_str(line);
        out.push('\n');
        match fence {
            Some(open) => {
                if closes_fence(line, open) {
                    fence = None;
                }
                continue;
            }
            None => {
                if let Some(open) = opening_fence(line.trim()) {
                    fence = Some(open);
                    continue;
                }
            }
        }
        // A run of list items reads as one block; it ends when the next line
        // is not an item, or is an item of the other kind (bullets then
        // numbers need the blank line or they read as one list).
        let kind = list_kind(line);
        let next_kind = lines.get(i + 1).and_then(|l| list_kind(l));
        let run_ends = kind.is_some() && kind != next_kind;
        let next_is_blank = lines.get(i + 1).map(|l| l.is_empty()).unwrap_or(true);
        if run_ends && !next_is_blank {
            out.push('\n');
        }
    }
    out
}

/// `Some(true)` for an ordered item, `Some(false)` for a bullet, `None`
/// for anything else.
fn list_kind(line: &str) -> Option<bool> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("- ") {
        return Some(false);
    }
    ordered_marker_len(trimmed).map(|_| true)
}

/// Length of a `12. ` style ordered-list marker at the start of `text`.
fn ordered_marker_len(text: &str) -> Option<usize> {
    let digits = text.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let rest = &text[digits..];
    if rest.starts_with(". ") {
        Some(digits + 2)
    } else {
        None
    }
}

/// The fence for a code block: longer than any run of backticks that opens a
/// line inside it.
///
/// Books about Markdown quote Markdown, so a preformatted block really does
/// contain a ``` line. Fenced with three, it closed on that line and the rest
/// of the block became prose — text that no longer read back as itself.
fn code_fence_for(text: &str) -> String {
    let longest = text
        .lines()
        .map(|line| line.trim_start().chars().take_while(|c| *c == '`').count())
        .max()
        .unwrap_or(0);
    "`".repeat(longest.saturating_add(1).max(3))
}

fn table_to_markdown(rows: &[Vec<Vec<Span>>]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut out = String::new();
    for (i, row) in rows.iter().enumerate() {
        let cells: Vec<String> = (0..width)
            .map(|c| {
                row.get(c)
                    .map(|spans| spans_to_markdown(spans).replace('|', "\\|"))
                    .unwrap_or_default()
            })
            .collect();
        out.push_str(&format!("| {} |\n", cells.join(" | ")));
        if i == 0 {
            let seps: Vec<&str> = (0..width).map(|_| "---").collect();
            out.push_str(&format!("| {} |\n", seps.join(" | ")));
        }
    }
    out.push('\n');
    out
}

/// Render styled runs as Markdown inline syntax.
///
/// Emphasis is *factored*: a style shared by a whole run is opened once around
/// it rather than repeated per span. Wrapping each span on its own produced
/// runs like `**bold ****bold italic*** more**`, whose markers merge into an
/// ambiguous `****` that no parser — including this one — reads back as what
/// was written. Factoring gives `**bold *bold italic* more**`, which
/// [`parse_spans`] recovers exactly, and that is what lets a save be verified.
pub fn spans_to_markdown(spans: &[Span]) -> String {
    let normalized = canonical_spans(spans);
    let live: Vec<&Span> = normalized.iter().collect();
    // `*` and `**` written next to each other merge: an italic run followed by
    // a bold one emits `*x***y**`, which reads back as neither. Wherever a
    // block mixes the two, italic is written with underscores, which cannot
    // run together with an asterisk.
    let applied = Applied {
        underscore_italic: live.iter().any(|s| s.style.bold),
        ..Applied::default()
    };
    emit_spans(&live, applied)
}

/// The emphases already opened around the spans being emitted, and how italic
/// is spelled in this block.
#[derive(Clone, Copy, Default)]
struct Applied {
    link: bool,
    bold: bool,
    italic: bool,
    underscore_italic: bool,
}

fn emit_spans(spans: &[&Span], applied: Applied) -> String {
    let Some(head) = spans.first() else {
        return String::new();
    };

    if !applied.link {
        if let Some(href) = shared_link(spans) {
            let inner = emit_spans(
                spans,
                Applied {
                    link: true,
                    ..applied
                },
            );
            return format!("[{inner}]({})", emit_href(href));
        }
    }
    if !applied.bold && spans.iter().all(|s| s.style.bold) {
        let inner = emit_spans(
            spans,
            Applied {
                bold: true,
                ..applied
            },
        );
        return format!("**{inner}**");
    }
    if !applied.italic && spans.iter().all(|s| s.style.italic) {
        let marker = match applied.underscore_italic {
            true => "_",
            false => "*",
        };
        let inner = emit_spans(
            spans,
            Applied {
                italic: true,
                ..applied
            },
        );
        return format!("{marker}{inner}{marker}");
    }

    // Nothing is shared by the whole run: emit the longest prefix that shares
    // an emphasis with its first span, then the rest. The prefix is always
    // shorter than the run — a prefix covering everything would have been
    // caught above — so this terminates.
    let split = shared_prefix_len(spans, applied);
    match split {
        0 => format!("{}{}", leaf(head), emit_spans(&spans[1..], applied)),
        n => format!(
            "{}{}",
            emit_spans(&spans[..n], applied),
            emit_spans(&spans[n..], applied)
        ),
    }
}

/// A single span's text, with only the emphasis no wrapper can carry: code.
fn leaf(span: &Span) -> String {
    match span.style.code {
        true => code_span(&span.text),
        false => escape_inline(&span.text),
    }
}

/// Write a code run, choosing a fence the run's own backticks cannot close.
///
/// Programming books are full of `` ` `` inside code — the backtick operator,
/// a nested span, a shell substitution. A single-backtick fence around them
/// closes early, so the text read back was not the text written and the book
/// could not be saved. One more backtick than the longest run inside, and a
/// space of padding when the run starts or ends with one, is what CommonMark
/// specifies and what `parse_spans` undoes.
fn code_span(text: &str) -> String {
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest + 1);
    // Padding also protects a leading or trailing space, which `unpad_code`
    // would otherwise strip as if this writer had put it there. Content that is
    // nothing but spaces needs none: it is never stripped.
    let edge = |c: char| c == '`' || c == ' ';
    let pad = match !text.trim().is_empty() && (text.starts_with(edge) || text.ends_with(edge)) {
        true => " ",
        false => "",
    };
    format!("{fence}{pad}{text}{pad}{fence}")
}

/// The spans as they are written: one line each, nothing empty, and neighbours
/// that look alike joined.
///
/// Two code runs side by side emit one fence against another, and a run holding
/// only a stray carriage return emits an empty pair — markers that read back as
/// a different set of spans than the ones written, so the buffer disagreed with
/// its own parse. Joining them first makes what is emitted the shape the parser
/// returns.
fn canonical_spans(spans: &[Span]) -> Vec<Span> {
    spans
        .iter()
        .map(|span| Span {
            text: one_line(&span.text),
            style: span.style.clone(),
        })
        .filter(|span| !span.text.is_empty())
        .fold(Vec::new(), |mut acc: Vec<Span>, span| {
            match acc.last_mut() {
                Some(last) if last.style == span.style => last.text.push_str(&span.text),
                _ => acc.push(span),
            }
            acc
        })
}

/// A link target on one line.
///
/// XHTML wraps long attribute values, so an EPUB's `href` reaches the model
/// with a newline and an indent inside it. Emitted verbatim that splits the
/// buffer line in two and the link no longer parses — the reason a book of
/// references could be opened but never saved. A URL cannot contain raw
/// whitespace, so removing it is the unwrap, not a guess.
fn emit_href(href: &str) -> String {
    href.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .replace(')', "%29")
}

/// Inline text with its line breaks flattened.
///
/// One block is one line in the buffer. A span carrying a newline would open a
/// second line that parses as a block of its own, so the text and its own parse
/// could never agree.
fn one_line(text: &str) -> String {
    match text.contains(['\n', '\r']) {
        true => text.replace('\r', "").replace('\n', " "),
        false => text.to_string(),
    }
}

/// The one link target every span carries, when they all carry the same one.
fn shared_link<'a>(spans: &[&'a Span]) -> Option<&'a str> {
    let first = spans.first()?.style.link.as_deref()?;
    spans
        .iter()
        .all(|s| s.style.link.as_deref() == Some(first))
        .then_some(first)
}

/// How many leading spans share the first span's outermost unopened emphasis.
fn shared_prefix_len(spans: &[&Span], applied: Applied) -> usize {
    let head = spans[0];
    let same: fn(&Span, &Span) -> bool = if !applied.link && head.style.link.is_some() {
        |a, b| a.style.link == b.style.link
    } else if !applied.bold && head.style.bold {
        |_, b| b.style.bold
    } else if !applied.italic && head.style.italic {
        |_, b| b.style.italic
    } else {
        return 0;
    };
    spans.iter().take_while(|s| same(head, s)).count()
}

/// Escape a paragraph's first character when the line would otherwise re-parse
/// as a heading, list item, rule, table row or code fence.
///
/// A DOCX paragraph whose text is literally "2. Map your first 90 days" is a
/// paragraph, not the second item of a list — it was numbered by hand in Word.
/// Emitting it unescaped made `from_markdown` read it back as an ordered item,
/// which `to_markdown` then renumbered to "1.". The buffer and its own parse
/// disagreed, so every save of such a document failed verification.
fn escape_block_start(line: &str) -> String {
    let indent = line.len() - line.trim_start().len();
    let (indent, rest) = line.split_at(indent);
    let escaped = match rest.chars().next() {
        Some('#') | Some('-') | Some('|') => format!("\\{rest}"),
        Some(c) if c.is_ascii_digit() => {
            let digits = rest.chars().take_while(char::is_ascii_digit).count();
            match rest[digits..].starts_with('.') {
                true => format!("{}\\{}", &rest[..digits], &rest[digits..]),
                false => rest.to_string(),
            }
        }
        _ => rest.to_string(),
    };
    format!("{indent}{escaped}")
}

fn escape_inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '*' | '_' | '`' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

// ── Parse ────────────────────────────────────────────────────────────

/// Parse a Markdown buffer back into a document of the given format.
pub fn from_markdown(format: DocFormat, markdown: &str) -> Document {
    let mut doc = Document::new(format);
    for raw_section in split_sections(markdown) {
        doc.sections.push(Section {
            id: raw_section.id,
            title: raw_section.title,
            blocks: parse_blocks(&raw_section.body),
        });
    }
    if doc.sections.is_empty() {
        doc.sections.push(Section {
            id: String::new(),
            title: None,
            blocks: Vec::new(),
        });
    }
    doc
}

struct RawSection {
    id: String,
    title: Option<String>,
    body: String,
}

fn split_sections(markdown: &str) -> Vec<RawSection> {
    let mut sections: Vec<RawSection> = Vec::new();
    let mut current: Option<RawSection> = None;
    for line in markdown.lines() {
        if let Some((id, title)) = parse_section_marker(line) {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            current = Some(RawSection {
                id,
                title,
                body: String::new(),
            });
            continue;
        }
        match current.as_mut() {
            Some(section) => {
                section.body.push_str(line);
                section.body.push('\n');
            }
            None => {
                // Content before the first marker (or a marker-less buffer)
                // belongs to an implicit leading section.
                current = Some(RawSection {
                    id: String::new(),
                    title: None,
                    body: format!("{line}\n"),
                });
            }
        }
    }
    if let Some(section) = current {
        sections.push(section);
    }
    sections
}

fn parse_section_marker(line: &str) -> Option<(String, Option<String>)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix(SECTION_PREFIX)?;
    let rest = rest.strip_suffix("-->")?.trim();
    let id = attr_value(rest, "id").unwrap_or_default();
    let title = attr_value(rest, "title").filter(|t| !t.is_empty());
    Some((id, title))
}

fn attr_value(text: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=\"");
    let start = text.find(&needle)? + needle.len();
    // Walk by character, not by byte: a non-ASCII id (a chapter filename with
    // an accent) must come back the way it went in.
    let mut value = String::new();
    let mut chars = text[start..].chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                value.push(c);
                match chars.next() {
                    Some(escaped) => value.push(escaped),
                    None => break,
                }
            }
            '"' => break,
            _ => value.push(c),
        }
    }
    Some(attr_unescape(&value))
}

fn parse_blocks(body: &str) -> Vec<Block> {
    let lines: Vec<&str> = body.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    let mut paragraph: Vec<String> = Vec::new();

    // A paragraph is only known to be finished when a structural line or a
    // blank line arrives, so flushing is a closure over the accumulator.
    //
    // A single line is kept byte for byte. `to_markdown` never wraps, so one
    // line is what a document that came from a file looks like, and its leading
    // and trailing whitespace is part of the text — ASCII diagrams are indented,
    // and Word paragraphs routinely end in a non-breaking space, which `trim`
    // eats because U+00A0 is Unicode whitespace. Trimming here made the buffer
    // disagree with its own parse and failed every save.
    macro_rules! flush_paragraph {
        () => {
            if !paragraph.is_empty() {
                let text = match paragraph.len() {
                    1 => paragraph.remove(0),
                    _ => paragraph
                        .iter()
                        .map(|line| line.trim())
                        .collect::<Vec<_>>()
                        .join(" "),
                };
                paragraph.clear();
                if !text.trim().is_empty() {
                    blocks.push(Block::Paragraph {
                        spans: parse_spans(&text),
                    });
                }
            }
        };
    }

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        // Structural markers are matched against the line without its indent;
        // what follows a marker keeps its trailing whitespace.
        let start = line.trim_start();

        if trimmed.is_empty() {
            flush_paragraph!();
            i += 1;
            continue;
        }

        if let Some(fence) = opening_fence(trimmed) {
            flush_paragraph!();
            let mut text = String::new();
            i += 1;
            while i < lines.len() && !closes_fence(lines[i], fence) {
                text.push_str(lines[i]);
                text.push('\n');
                i += 1;
            }
            i += 1; // closing fence (or end of buffer)
            blocks.push(Block::Code {
                text: text.trim_end_matches('\n').to_string(),
            });
            continue;
        }

        if trimmed.chars().all(|c| c == '-') && trimmed.len() >= 3 {
            flush_paragraph!();
            blocks.push(Block::Rule);
            i += 1;
            continue;
        }

        if let Some(level) = heading_level(trimmed) {
            flush_paragraph!();
            // Skip the hashes and the single space `to_markdown` writes after
            // them; everything else, trailing whitespace included, is the text.
            let text = &start[level as usize + 1..];
            blocks.push(Block::Heading {
                level,
                spans: parse_spans(text),
            });
            i += 1;
            continue;
        }

        if trimmed.starts_with("| ") || trimmed.starts_with("|") && trimmed.ends_with("|") {
            flush_paragraph!();
            let mut rows = Vec::new();
            while i < lines.len() && lines[i].trim().starts_with('|') {
                let row = lines[i].trim();
                if !is_table_separator(row) {
                    rows.push(parse_table_row(row));
                }
                i += 1;
            }
            blocks.push(Block::Table { rows });
            continue;
        }

        let indent = line.len() - start.len();
        let level = (indent / 2) as u8;
        if let Some(rest) = start.strip_prefix("- ") {
            flush_paragraph!();
            blocks.push(Block::ListItem {
                level,
                ordered: false,
                spans: parse_spans(rest),
            });
            i += 1;
            continue;
        }
        if let Some(marker) = ordered_marker_len(start) {
            flush_paragraph!();
            blocks.push(Block::ListItem {
                level,
                ordered: true,
                spans: parse_spans(&start[marker..]),
            });
            i += 1;
            continue;
        }

        paragraph.push(line.to_string());
        i += 1;
    }
    flush_paragraph!();
    blocks
}

/// The length of the fence a line opens a code block with, if it opens one.
///
/// A paragraph holding a code run whose own text contains backticks is written
/// with a long fence too, so the line alone is not the answer. CommonMark
/// settles it: what follows an opening fence is an info string, which carries
/// neither whitespace nor a backtick.
fn opening_fence(trimmed: &str) -> Option<usize> {
    let fence = trimmed.chars().take_while(|c| *c == '`').count();
    let info = &trimmed[fence..];
    (fence >= 3 && !info.contains('`') && info.trim() == info).then_some(fence)
}

/// Whether a line closes a block opened with `fence` backticks: at least as
/// many, and nothing else on the line.
fn closes_fence(line: &str, fence: usize) -> bool {
    let line = line.trim_start();
    let run = line.chars().take_while(|c| *c == '`').count();
    run >= fence && line[run..].trim().is_empty()
}

fn heading_level(trimmed: &str) -> Option<u8> {
    let hashes = trimmed.chars().take_while(|c| *c == '#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    if trimmed[hashes..].starts_with(' ') {
        Some(hashes as u8)
    } else {
        None
    }
}

fn is_table_separator(row: &str) -> bool {
    row.trim_matches('|').split('|').all(|cell| {
        !cell.trim().is_empty()
            && cell
                .trim()
                .chars()
                .all(|c| c == '-' || c == ':' || c == ' ')
    })
}

fn parse_table_row(row: &str) -> Vec<Vec<Span>> {
    let inner = row.trim().trim_start_matches('|').trim_end_matches('|');
    split_unescaped_pipes(inner)
        .into_iter()
        .map(|cell| parse_spans(unpad_cell(&cell)))
        .collect()
}

/// Remove the one space `table_to_markdown` puts on each side of a cell — and
/// only that one, so a cell whose own text ends in a space (or the non-breaking
/// space Word litters tables with) survives the round trip.
fn unpad_cell(cell: &str) -> &str {
    let cell = cell.strip_prefix(' ').unwrap_or(cell);
    cell.strip_suffix(' ').unwrap_or(cell)
}

fn split_unescaped_pipes(text: &str) -> Vec<String> {
    let mut cells = vec![String::new()];
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                chars.next();
                if let Some(last) = cells.last_mut() {
                    last.push('|');
                }
            }
            '|' => cells.push(String::new()),
            _ => {
                if let Some(last) = cells.last_mut() {
                    last.push(c);
                }
            }
        }
    }
    cells
}

/// Parse Markdown inline syntax into styled runs.
pub fn parse_spans(text: &str) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut buf = String::new();
    let mut i = 0;

    let push_buf = |buf: &mut String, spans: &mut Vec<Span>| {
        if !buf.is_empty() {
            spans.push(Span::plain(std::mem::take(buf)));
        }
    };

    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if i + 1 < chars.len() => {
                buf.push(chars[i + 1]);
                i += 2;
            }
            '`' => match code_span_at(&chars, i) {
                Some((from, to, next)) => {
                    push_buf(&mut buf, &mut spans);
                    let inner: String = chars[from..to].iter().collect();
                    spans.push(Span {
                        text: unpad_code(&inner).to_string(),
                        style: SpanStyle {
                            code: true,
                            ..SpanStyle::plain()
                        },
                    });
                    i = next;
                }
                None => {
                    buf.push(c);
                    i += 1;
                }
            },
            '*' | '_' => {
                let double = chars.get(i + 1) == Some(&c);
                let marker: String = if double {
                    format!("{c}{c}")
                } else {
                    c.to_string()
                };
                match find_close_str(&chars, i + marker.len(), &marker) {
                    Some(end) => {
                        push_buf(&mut buf, &mut spans);
                        let inner: String = chars[i + marker.len()..end].iter().collect();
                        let mut inner_spans = parse_spans(&inner);
                        for span in inner_spans.iter_mut() {
                            if double {
                                span.style.bold = true;
                            } else {
                                span.style.italic = true;
                            }
                        }
                        spans.extend(inner_spans);
                        i = end + marker.len();
                    }
                    None => {
                        buf.push(c);
                        i += 1;
                    }
                }
            }
            '[' => match parse_link(&chars, i) {
                Some((label, href, next)) => {
                    push_buf(&mut buf, &mut spans);
                    let mut inner = parse_spans(&label);
                    for span in inner.iter_mut() {
                        span.style.link = Some(href.clone());
                    }
                    spans.extend(inner);
                    i = next;
                }
                None => {
                    buf.push(c);
                    i += 1;
                }
            },
            _ => {
                buf.push(c);
                i += 1;
            }
        }
    }
    push_buf(&mut buf, &mut spans);
    spans
}

/// Where a code run starting at `i` keeps its content, and what follows it.
///
/// A code span binds tighter than emphasis and than a link label, so every
/// scanner below steps over one whole rather than reading the `*`, `_` or `]`
/// inside it as markup — `` `*an_array@10` `` is a name, not the start of an
/// italic run.
fn code_span_at(chars: &[char], i: usize) -> Option<(usize, usize, usize)> {
    if chars.get(i) != Some(&'`') {
        return None;
    }
    let fence = backtick_run(chars, i);
    let end = find_fence_close(chars, i + fence, fence)?;
    Some((i + fence, end, end + fence))
}

fn backtick_run(chars: &[char], from: usize) -> usize {
    chars[from..].iter().take_while(|c| **c == '`').count()
}

/// The next run of *exactly* `fence` backticks — a longer run is content.
fn find_fence_close(chars: &[char], from: usize, fence: usize) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i] != '`' {
            i += 1;
            continue;
        }
        let run = backtick_run(chars, i);
        if run == fence {
            return Some(i);
        }
        i += run;
    }
    None
}

/// Undo the padding [`code_span`] adds around content that begins or ends with
/// a backtick: one space each side, and only when both are there.
fn unpad_code(text: &str) -> &str {
    match text.starts_with(' ') && text.ends_with(' ') && !text.trim().is_empty() {
        true => &text[1..text.len() - 1],
        false => text,
    }
}

fn find_close(chars: &[char], from: usize, needle: char) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if let Some((_, _, next)) = code_span_at(chars, i) {
            i = next;
            continue;
        }
        if chars[i] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_close_str(chars: &[char], from: usize, needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    let mut i = from;
    while i + needle.len() <= chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if let Some((_, _, next)) = code_span_at(chars, i) {
            i = next;
            continue;
        }
        // A link is one unit too. `_` is legal in a URL and Packt's chapter
        // hrefs are full of them; read as emphasis they closed a run that had
        // opened in the prose before the link, and the link stopped parsing.
        if chars[i] == '[' {
            if let Some((_, _, next)) = parse_link(chars, i) {
                i = next;
                continue;
            }
        }
        if chars[i..i + needle.len()] == needle[..] {
            // `*` must not match the first half of a `**` closer.
            let next = chars.get(i + needle.len());
            if needle.len() == 1 && next == Some(&needle[0]) {
                i += 1;
                continue;
            }
            return Some(i);
        }
        i += 1;
    }
    None
}

fn parse_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    let close = find_close(chars, start + 1, ']')?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = find_close(chars, close + 2, ')')?;
    let label: String = chars[start + 1..close].iter().collect();
    let href: String = chars[close + 2..end].iter().collect();
    Some((label, href.replace("%29", ")"), end + 1))
}

// ── Plain text ───────────────────────────────────────────────────────

/// Render a document as plain text, one line per block.
///
/// Used for Pages, whose reader recovers paragraph text only.
pub fn to_plain_text(doc: &Document) -> String {
    let multi = doc.sections.len() > 1;
    let mut out = String::new();
    for section in &doc.sections {
        if multi {
            out.push_str(&format!("{}\n", storage_marker(&section.id)));
        }
        for block in &section.blocks {
            out.push_str(&block.plain_text());
            out.push('\n');
        }
    }
    out
}

/// Marker separating Pages text storages in a plain-text buffer.
pub fn storage_marker(id: &str) -> String {
    format!("<<< vibedoc:storage {id} >>>")
}

/// Parse the id out of a storage marker line.
pub fn parse_storage_marker(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("<<< vibedoc:storage ")?;
    let id = rest.strip_suffix(">>>")?.trim();
    Some(id.to_string())
}

/// Parse a plain-text buffer (the Pages editing view) back into a document.
///
/// Storage markers, when present, split the buffer; a buffer with none is a
/// single storage.
pub fn from_plain_text(format: DocFormat, text: &str) -> Document {
    let mut doc = Document::new(format);
    let mut current: Option<Section> = None;
    for line in text.lines() {
        match parse_storage_marker(line) {
            Some(id) => {
                if let Some(section) = current.take() {
                    doc.sections.push(section);
                }
                current = Some(Section {
                    id,
                    title: None,
                    blocks: Vec::new(),
                });
            }
            None => {
                let section = current.get_or_insert_with(|| Section {
                    id: String::new(),
                    title: None,
                    blocks: Vec::new(),
                });
                section.blocks.push(Block::Paragraph {
                    spans: vec![Span::plain(line)],
                });
            }
        }
    }
    if let Some(section) = current {
        doc.sections.push(section);
    }
    if doc.sections.is_empty() {
        doc.sections.push(Section {
            id: String::new(),
            title: None,
            blocks: Vec::new(),
        });
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::spans_text;

    fn doc(sections: Vec<Section>) -> Document {
        Document {
            format: DocFormat::Epub,
            sections,
            warnings: Vec::new(),
        }
    }

    /// The invariant the whole crate rests on: what a buffer says, its own parse
    /// must say back. `write_text` verifies a save by comparing the rewritten
    /// file's Markdown against the Markdown of the text that was saved, so any
    /// text this pair does not agree on is a document that cannot be saved.
    fn is_a_fixed_point(source: &str) {
        let rendered = to_markdown(&from_markdown(DocFormat::Docx, source));
        assert_eq!(
            to_markdown(&from_markdown(DocFormat::Docx, &rendered)),
            rendered,
            "render(parse(render(parse(x)))) == render(parse(x))"
        );
    }

    #[test]
    fn markdown_is_a_fixed_point() {
        // Italic is written with underscores here because the paragraph also
        // holds bold: `*` beside `**` merges into a marker neither meant.
        let source = "# Heading\n\nsome **bold**, _italic_, `code`, and a [link](https://x.dev)\n\n\
                      - one\n- two\n\n1. first\n\n```\ncode()\n```\n\n---\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n";
        let parsed = from_markdown(DocFormat::Docx, source);
        let rendered = to_markdown(&parsed);
        assert_eq!(rendered.trim(), source.trim(), "render(parse(x)) == x");
        is_a_fixed_point(source);
    }

    #[test]
    fn italic_alone_still_uses_asterisks() {
        let source = "plain *italic* text\n";
        assert_eq!(
            to_markdown(&from_markdown(DocFormat::Docx, source)).trim(),
            source.trim()
        );
    }

    // ── What real documents broke on ─────────────────────────────────
    //
    // Every case below came from a Word file or an EPUB that opened in the
    // editor and then refused to save, because the buffer and its own parse
    // disagreed. Each is the smallest text that reproduces one of them.

    #[test]
    fn a_trailing_non_breaking_space_is_part_of_the_text() {
        // Word ends paragraphs with U+00A0 constantly, and `str::trim` eats it
        // because Unicode calls it whitespace.
        let doc = doc(vec![Section {
            id: String::new(),
            title: None,
            blocks: vec![Block::Paragraph {
                spans: vec![Span::plain("a line\u{a0}")],
            }],
        }]);
        let rendered = to_markdown(&doc);
        let reparsed = from_markdown(DocFormat::Docx, &rendered);
        assert_eq!(spans_text(block_spans(&reparsed)), "a line\u{a0}");
        is_a_fixed_point(&rendered);
    }

    #[test]
    fn a_paragraphs_indentation_is_part_of_the_text() {
        // ASCII diagrams pasted into Word are one indented paragraph each.
        let source = "        │  Commit early    │\n";
        let parsed = from_markdown(DocFormat::Docx, source);
        assert_eq!(
            spans_text(block_spans(&parsed)),
            "        │  Commit early    │"
        );
        is_a_fixed_point(source);
    }

    #[test]
    fn a_hand_numbered_paragraph_stays_a_paragraph() {
        // "2. Map your first 90 days" typed into Word is a paragraph. Read back
        // as the second item of a list, it was renumbered to "1." on the way
        // out and the buffer no longer matched itself.
        let doc = doc(vec![Section {
            id: String::new(),
            title: None,
            blocks: vec![Block::Paragraph {
                spans: vec![Span::plain("2. Map your first 90 days")],
            }],
        }]);
        let rendered = to_markdown(&doc);
        let reparsed = from_markdown(DocFormat::Docx, &rendered);
        assert!(
            matches!(reparsed.sections[0].blocks[0], Block::Paragraph { .. }),
            "a numbered paragraph must not become a list item: {rendered:?}"
        );
        assert_eq!(
            spans_text(block_spans(&reparsed)),
            "2. Map your first 90 days"
        );
    }

    #[test]
    fn a_paragraph_that_opens_like_a_block_is_escaped() {
        for text in [
            "# not a heading",
            "- not a bullet",
            "| not a table",
            "3. not a list",
        ] {
            let doc = doc(vec![Section {
                id: String::new(),
                title: None,
                blocks: vec![Block::Paragraph {
                    spans: vec![Span::plain(text)],
                }],
            }]);
            let rendered = to_markdown(&doc);
            let reparsed = from_markdown(DocFormat::Docx, &rendered);
            assert!(
                matches!(reparsed.sections[0].blocks[0], Block::Paragraph { .. }),
                "{text:?} became {:?}",
                reparsed.sections[0].blocks[0]
            );
            assert_eq!(spans_text(block_spans(&reparsed)), text);
        }
    }

    #[test]
    fn emphasis_markers_never_run_together() {
        let spans = vec![
            Span {
                text: "States when ".into(),
                style: SpanStyle {
                    bold: true,
                    ..SpanStyle::plain()
                },
            },
            Span {
                text: "not".into(),
                style: SpanStyle {
                    bold: true,
                    italic: true,
                    ..SpanStyle::plain()
                },
            },
            Span {
                text: " to use it".into(),
                style: SpanStyle {
                    bold: true,
                    ..SpanStyle::plain()
                },
            },
        ];
        let rendered = spans_to_markdown(&spans);
        assert_eq!(rendered, "**States when _not_ to use it**");
        assert_eq!(spans_to_markdown(&parse_spans(&rendered)), rendered);
    }

    #[test]
    fn an_italic_run_next_to_a_bold_one_stays_readable() {
        let spans = vec![
            Span {
                text: "problem".into(),
                style: SpanStyle {
                    italic: true,
                    ..SpanStyle::plain()
                },
            },
            Span {
                text: " ".into(),
                style: SpanStyle {
                    bold: true,
                    ..SpanStyle::plain()
                },
            },
        ];
        let rendered = spans_to_markdown(&spans);
        assert_eq!(spans_to_markdown(&parse_spans(&rendered)), rendered);
    }

    #[test]
    fn a_code_run_may_hold_backticks_and_edge_spaces() {
        for text in ["`", "echo `date`", " D(y, x1) ", "a``b", "  ", "x"] {
            let spans = vec![Span {
                text: text.to_string(),
                style: SpanStyle {
                    code: true,
                    ..SpanStyle::plain()
                },
            }];
            let rendered = spans_to_markdown(&spans);
            let reparsed = parse_spans(&rendered);
            assert_eq!(spans_text(&reparsed), text, "rendered as {rendered:?}");
            assert!(reparsed.iter().all(|s| s.style.code), "{rendered:?}");
        }
    }

    #[test]
    fn a_code_run_hides_its_emphasis_markers() {
        // `*an_array@10` is a name. Scanning past the code run is what keeps its
        // asterisk from opening an italic span that swallows the rest.
        let source = "see *`*an_array@10`*` [GDB]` now\n";
        is_a_fixed_point(source);
        let parsed = from_markdown(DocFormat::Docx, source);
        assert_eq!(
            spans_text(block_spans(&parsed)),
            "see *an_array@10 [GDB] now"
        );
    }

    #[test]
    fn a_paragraph_opening_with_a_long_code_fence_is_not_a_code_block() {
        let spans = vec![Span {
            text: "a``b".into(),
            style: SpanStyle {
                code: true,
                ..SpanStyle::plain()
            },
        }];
        let rendered = format!("{}\n", spans_to_markdown(&spans));
        assert!(rendered.starts_with("```"), "{rendered:?}");
        let parsed = from_markdown(DocFormat::Docx, &rendered);
        assert!(
            matches!(parsed.sections[0].blocks[0], Block::Paragraph { .. }),
            "{:?}",
            parsed.sections[0].blocks[0]
        );
        is_a_fixed_point(&rendered);
    }

    #[test]
    fn a_wrapped_href_comes_back_on_one_line() {
        // XHTML wraps long attribute values; the newline landed in the buffer.
        let spans = vec![Span {
            text: "the paper".into(),
            style: SpanStyle {
                link: Some("https://example.dev/a\n            /very/long".into()),
                ..SpanStyle::plain()
            },
        }];
        let rendered = spans_to_markdown(&spans);
        assert_eq!(rendered, "[the paper](https://example.dev/a/very/long)");
        assert_eq!(spans_to_markdown(&parse_spans(&rendered)), rendered);
    }

    fn block_spans(doc: &Document) -> &[Span] {
        match &doc.sections[0].blocks[0] {
            Block::Heading { spans, .. }
            | Block::Paragraph { spans }
            | Block::ListItem { spans, .. } => spans,
            other => panic!("not a text block: {other:?}"),
        }
    }

    #[test]
    fn a_non_ascii_section_id_survives_the_marker() {
        let source = to_markdown(&doc(vec![
            Section {
                id: "OEBPS/châpitre-1.xhtml".into(),
                title: Some("Café".into()),
                blocks: vec![Block::Paragraph {
                    spans: vec![Span::plain("un")],
                }],
            },
            Section {
                id: "OEBPS/ch2.xhtml".into(),
                title: None,
                blocks: vec![Block::Paragraph {
                    spans: vec![Span::plain("two")],
                }],
            },
        ]));
        let parsed = from_markdown(DocFormat::Epub, &source);
        assert_eq!(parsed.sections[0].id, "OEBPS/châpitre-1.xhtml");
        assert_eq!(parsed.sections[0].title.as_deref(), Some("Café"));
        assert_eq!(parsed.sections[1].id, "OEBPS/ch2.xhtml");
    }

    #[test]
    fn a_quote_in_a_title_does_not_end_the_marker() {
        let source = to_markdown(&doc(vec![
            Section {
                id: "a".into(),
                title: Some(r#"He said "go""#.into()),
                blocks: vec![],
            },
            Section {
                id: "b".into(),
                title: None,
                blocks: vec![],
            },
        ]));
        let parsed = from_markdown(DocFormat::Epub, &source);
        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(parsed.sections[0].title.as_deref(), Some(r#"He said "go""#));
    }

    #[test]
    fn escaped_markdown_characters_stay_literal() {
        let spans = parse_spans(r"a \*not italic\* b");
        assert_eq!(spans_text(&spans), "a *not italic* b");
        let rendered = spans_to_markdown(&spans);
        assert_eq!(spans_text(&parse_spans(&rendered)), "a *not italic* b");
    }

    #[test]
    fn plain_text_round_trips_storage_markers() {
        let document = Document {
            format: DocFormat::Pages,
            sections: vec![
                Section {
                    id: "f.iwa:1:0".into(),
                    title: None,
                    blocks: vec![Block::Paragraph {
                        spans: vec![Span::plain("body")],
                    }],
                },
                Section {
                    id: "f.iwa:2:0".into(),
                    title: None,
                    blocks: vec![Block::Paragraph {
                        spans: vec![Span::plain("header")],
                    }],
                },
            ],
            warnings: Vec::new(),
        };
        let text = to_plain_text(&document);
        let parsed = from_plain_text(DocFormat::Pages, &text);
        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(parsed.sections[0].id, "f.iwa:1:0");
        assert_eq!(parsed.sections[1].blocks[0].plain_text(), "header");
    }

    #[test]
    fn a_pages_buffer_without_markers_is_one_storage() {
        let parsed = from_plain_text(DocFormat::Pages, "line one\nline two\n");
        assert_eq!(parsed.sections.len(), 1);
        assert_eq!(parsed.sections[0].blocks.len(), 2);
    }
}
