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
    value.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ")
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
                out.push_str(&format!("{}\n\n", spans_to_markdown(spans)));
            }
            Block::ListItem { level, ordered, spans } => {
                let indent = "  ".repeat(*level as usize);
                let bullet = if *ordered { "1." } else { "-" };
                out.push_str(&format!("{indent}{bullet} {}\n", spans_to_markdown(spans)));
            }
            Block::Code { text } => {
                out.push_str("```\n");
                out.push_str(text.trim_end_matches('\n'));
                out.push_str("\n```\n\n");
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
fn normalize_list_spacing(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        out.push_str(line);
        out.push('\n');
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
pub fn spans_to_markdown(spans: &[Span]) -> String {
    let mut out = String::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        let mut text = if span.style.code {
            format!("`{}`", span.text)
        } else {
            escape_inline(&span.text)
        };
        if span.style.bold {
            text = format!("**{text}**");
        }
        if span.style.italic {
            text = format!("*{text}*");
        }
        if let Some(href) = &span.style.link {
            text = format!("[{text}]({})", href.replace(')', "%29"));
        }
        out.push_str(&text);
    }
    out
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
        doc.sections.push(Section { id: String::new(), title: None, blocks: Vec::new() });
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
            current = Some(RawSection { id, title, body: String::new() });
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
    macro_rules! flush_paragraph {
        () => {
            if !paragraph.is_empty() {
                let text = paragraph.join(" ");
                paragraph.clear();
                if !text.trim().is_empty() {
                    blocks.push(Block::Paragraph { spans: parse_spans(text.trim()) });
                }
            }
        };
    }

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        if trimmed.is_empty() {
            flush_paragraph!();
            i += 1;
            continue;
        }

        if let Some(fence) = trimmed.strip_prefix("```") {
            flush_paragraph!();
            let _lang = fence.trim();
            let mut text = String::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                text.push_str(lines[i]);
                text.push('\n');
                i += 1;
            }
            i += 1; // closing fence (or end of buffer)
            blocks.push(Block::Code { text: text.trim_end_matches('\n').to_string() });
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
            let text = trimmed[level as usize..].trim();
            blocks.push(Block::Heading { level, spans: parse_spans(text) });
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

        let indent = line.len() - line.trim_start().len();
        let level = (indent / 2) as u8;
        if let Some(rest) = trimmed.strip_prefix("- ") {
            flush_paragraph!();
            blocks.push(Block::ListItem { level, ordered: false, spans: parse_spans(rest.trim()) });
            i += 1;
            continue;
        }
        if let Some(marker) = ordered_marker_len(trimmed) {
            flush_paragraph!();
            let rest = trimmed[marker..].trim();
            blocks.push(Block::ListItem { level, ordered: true, spans: parse_spans(rest) });
            i += 1;
            continue;
        }

        paragraph.push(trimmed.to_string());
        i += 1;
    }
    flush_paragraph!();
    blocks
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
    row.trim_matches('|')
        .split('|')
        .all(|cell| !cell.trim().is_empty() && cell.trim().chars().all(|c| c == '-' || c == ':' || c == ' '))
}

fn parse_table_row(row: &str) -> Vec<Vec<Span>> {
    let inner = row.trim().trim_start_matches('|').trim_end_matches('|');
    split_unescaped_pipes(inner)
        .into_iter()
        .map(|cell| parse_spans(cell.trim()))
        .collect()
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
            '`' => match find_close(&chars, i + 1, '`') {
                Some(end) => {
                    push_buf(&mut buf, &mut spans);
                    let inner: String = chars[i + 1..end].iter().collect();
                    spans.push(Span {
                        text: inner,
                        style: SpanStyle { code: true, ..SpanStyle::plain() },
                    });
                    i = end + 1;
                }
                None => {
                    buf.push(c);
                    i += 1;
                }
            },
            '*' | '_' => {
                let double = chars.get(i + 1) == Some(&c);
                let marker: String = if double { format!("{c}{c}") } else { c.to_string() };
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

fn find_close(chars: &[char], from: usize, needle: char) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == '\\' {
            i += 2;
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
pub fn from_plain_text(text: &str) -> Document {
    let mut doc = Document::new(DocFormat::Pages);
    let mut current: Option<Section> = None;
    for line in text.lines() {
        match parse_storage_marker(line) {
            Some(id) => {
                if let Some(section) = current.take() {
                    doc.sections.push(section);
                }
                current = Some(Section { id, title: None, blocks: Vec::new() });
            }
            None => {
                let section = current.get_or_insert_with(|| Section {
                    id: String::new(),
                    title: None,
                    blocks: Vec::new(),
                });
                section.blocks.push(Block::Paragraph { spans: vec![Span::plain(line)] });
            }
        }
    }
    if let Some(section) = current {
        doc.sections.push(section);
    }
    if doc.sections.is_empty() {
        doc.sections.push(Section { id: String::new(), title: None, blocks: Vec::new() });
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::spans_text;

    fn doc(sections: Vec<Section>) -> Document {
        Document { format: DocFormat::Epub, sections, warnings: Vec::new() }
    }

    #[test]
    fn markdown_is_a_fixed_point() {
        let source = "# Heading\n\nsome **bold**, *italic*, `code`, and a [link](https://x.dev)\n\n\
                      - one\n- two\n\n1. first\n\n```\ncode()\n```\n\n---\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n";
        let parsed = from_markdown(DocFormat::Docx, source);
        let rendered = to_markdown(&parsed);
        assert_eq!(rendered.trim(), source.trim(), "render(parse(x)) == x");
        assert_eq!(to_markdown(&from_markdown(DocFormat::Docx, &rendered)), rendered);
    }

    #[test]
    fn a_non_ascii_section_id_survives_the_marker() {
        let source = to_markdown(&doc(vec![
            Section { id: "OEBPS/châpitre-1.xhtml".into(), title: Some("Café".into()), blocks: vec![Block::Paragraph { spans: vec![Span::plain("un")] }] },
            Section { id: "OEBPS/ch2.xhtml".into(), title: None, blocks: vec![Block::Paragraph { spans: vec![Span::plain("two")] }] },
        ]));
        let parsed = from_markdown(DocFormat::Epub, &source);
        assert_eq!(parsed.sections[0].id, "OEBPS/châpitre-1.xhtml");
        assert_eq!(parsed.sections[0].title.as_deref(), Some("Café"));
        assert_eq!(parsed.sections[1].id, "OEBPS/ch2.xhtml");
    }

    #[test]
    fn a_quote_in_a_title_does_not_end_the_marker() {
        let source = to_markdown(&doc(vec![
            Section { id: "a".into(), title: Some(r#"He said "go""#.into()), blocks: vec![] },
            Section { id: "b".into(), title: None, blocks: vec![] },
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
                Section { id: "f.iwa:1:0".into(), title: None, blocks: vec![Block::Paragraph { spans: vec![Span::plain("body")] }] },
                Section { id: "f.iwa:2:0".into(), title: None, blocks: vec![Block::Paragraph { spans: vec![Span::plain("header")] }] },
            ],
            warnings: Vec::new(),
        };
        let text = to_plain_text(&document);
        let parsed = from_plain_text(&text);
        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(parsed.sections[0].id, "f.iwa:1:0");
        assert_eq!(parsed.sections[1].blocks[0].plain_text(), "header");
    }

    #[test]
    fn a_pages_buffer_without_markers_is_one_storage() {
        let parsed = from_plain_text("line one\nline two\n");
        assert_eq!(parsed.sections.len(), 1);
        assert_eq!(parsed.sections[0].blocks.len(), 2);
    }
}
