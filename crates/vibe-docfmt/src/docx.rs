//! Office Open XML word processing documents (`.docx`).
//!
//! Reading walks `word/document.xml`; writing edits that same tree in place and
//! puts every other part of the package back untouched, so images, footnotes,
//! headers, comments and page setup survive a save even though the text model
//! knows nothing about them.

use crate::error::DocError;
use crate::model::{Block, DocFormat, Document, Section, Span, SpanStyle, Warning};
use crate::surgical::{self, BlockAdapter, Slot};
use crate::xmltree::{self, Element, Node};
use crate::zipedit::{self, ZipEntry};

const DOCUMENT_PART: &str = "word/document.xml";
const RELS_PART: &str = "word/_rels/document.xml.rels";
const NUMBERING_PART: &str = "word/numbering.xml";
const STYLES_PART: &str = "word/styles.xml";
const CONTENT_TYPES_PART: &str = "[Content_Types].xml";

const HYPERLINK_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";
const NUMBERING_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering";
const W_NS: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

/// Fonts that mean "this run is code" in both directions.
const MONO_FONTS: [&str; 5] = ["Consolas", "Courier New", "Menlo", "Monaco", "SF Mono"];

/// The result of rewriting a document.
#[derive(Debug)]
pub struct Rewrite {
    pub bytes: Vec<u8>,
    /// What the writer actually stored — the target after any degradation it
    /// reported. Verification compares the re-read file against this, so a
    /// declared degradation is not mistaken for corruption.
    pub effective: Document,
    pub warnings: Vec<Warning>,
}

// ── Read ─────────────────────────────────────────────────────────────

/// Parse a `.docx` file into the document model.
pub fn read(bytes: &[u8]) -> Result<Document, DocError> {
    let entries = zipedit::read_entries(bytes)?;
    let part = zipedit::find(&entries, DOCUMENT_PART)
        .ok_or_else(|| DocError::Parse(format!("{DOCUMENT_PART} is missing")))?;
    let xml = xmltree::parse_bytes(&part.data)?;
    let body = find_body(&xml.root)
        .ok_or_else(|| DocError::Parse("word/document.xml has no <w:body>".into()))?;

    let rels = read_rels(&entries)?;
    let numbering = read_numbering(&entries)?;
    let ctx = ReadCtx { rels, numbering };

    let mut warnings = Vec::new();
    let slots = collect_slots(body, &ctx, &mut warnings);
    let blocks = slots.into_iter().map(|s| s.block).collect();

    Ok(Document {
        format: DocFormat::Docx,
        sections: vec![Section {
            id: "body".to_string(),
            title: None,
            blocks,
        }],
        warnings,
    })
}

struct ReadCtx {
    /// Relationship id → target, for hyperlinks.
    rels: Vec<(String, String)>,
    /// numId → is-ordered.
    numbering: Vec<(String, bool)>,
}

impl ReadCtx {
    fn link_target(&self, id: &str) -> Option<&str> {
        self.rels
            .iter()
            .find(|(k, _)| k == id)
            .map(|(_, v)| v.as_str())
    }

    fn is_ordered(&self, num_id: &str) -> bool {
        self.numbering
            .iter()
            .find(|(k, _)| k == num_id)
            .map(|(_, ordered)| *ordered)
            // An unknown numId is far more often a bullet list than a numbered
            // one, but this is a guess and the warning at the call site says so.
            .unwrap_or(false)
    }
}

fn find_body(root: &Element) -> Option<&Element> {
    root.children_named("body").next()
}

fn find_body_mut(root: &mut Element) -> Option<&mut Element> {
    root.children.iter_mut().find_map(|n| match n {
        Node::Element(e) if e.local_name() == "body" => Some(e),
        _ => None,
    })
}

fn read_rels(entries: &[ZipEntry]) -> Result<Vec<(String, String)>, DocError> {
    let Some(part) = zipedit::find(entries, RELS_PART) else {
        return Ok(Vec::new());
    };
    let xml = xmltree::parse_bytes(&part.data)?;
    Ok(xml
        .root
        .children_named("Relationship")
        .filter(|rel| rel.attr("Type") == Some(HYPERLINK_REL))
        .filter_map(|rel| Some((rel.attr("Id")?.to_string(), rel.attr("Target")?.to_string())))
        .collect())
}

fn read_numbering(entries: &[ZipEntry]) -> Result<Vec<(String, bool)>, DocError> {
    let Some(part) = zipedit::find(entries, NUMBERING_PART) else {
        return Ok(Vec::new());
    };
    let xml = xmltree::parse_bytes(&part.data)?;

    // abstractNumId → level-0 format
    let abstract_fmt: Vec<(String, String)> = xml
        .root
        .children_named("abstractNum")
        .filter_map(|an| {
            let id = an.attr("w:abstractNumId")?.to_string();
            let fmt = an
                .children_named("lvl")
                .find(|lvl| lvl.attr("w:ilvl").unwrap_or("0") == "0")
                .and_then(|lvl| lvl.children_named("numFmt").next())
                .and_then(|f| f.attr("w:val"))
                .unwrap_or("bullet")
                .to_string();
            Some((id, fmt))
        })
        .collect();

    Ok(xml
        .root
        .children_named("num")
        .filter_map(|num| {
            let num_id = num.attr("w:numId")?.to_string();
            let abstract_id = num
                .children_named("abstractNumId")
                .next()
                .and_then(|a| a.attr("w:val"))?;
            let fmt = abstract_fmt
                .iter()
                .find(|(id, _)| id == abstract_id)
                .map(|(_, f)| f.as_str())
                .unwrap_or("bullet");
            Some((num_id, fmt != "bullet"))
        })
        .collect())
}

fn collect_slots(body: &Element, ctx: &ReadCtx, warnings: &mut Vec<Warning>) -> Vec<Slot> {
    body.children
        .iter()
        .enumerate()
        .filter_map(|(i, node)| {
            let Node::Element(el) = node else { return None };
            let block = match el.local_name() {
                "p" => paragraph_block(el, ctx, warnings)?,
                "tbl" => table_block(el, ctx, warnings),
                _ => return None,
            };
            Some(Slot {
                path: vec![i],
                block,
            })
        })
        .collect()
}

/// Convert a `w:p` to a block. Returns `None` for paragraphs that carry no
/// text — spacer paragraphs stay out of the buffer so a save cannot delete them.
fn paragraph_block(el: &Element, ctx: &ReadCtx, warnings: &mut Vec<Warning>) -> Option<Block> {
    let spans = paragraph_spans(el, ctx);
    let props = el.children_named("pPr").next();

    if spans.iter().all(|s| s.text.trim().is_empty()) {
        let has_rule = props
            .and_then(|p| p.children_named("pBdr").next())
            .map(|b| b.children_named("bottom").next().is_some())
            .unwrap_or(false);
        return has_rule.then_some(Block::Rule);
    }

    let style = props
        .and_then(|p| p.children_named("pStyle").next())
        .and_then(|s| s.attr("w:val"))
        .unwrap_or("");

    if let Some(level) = heading_level_of(style) {
        return Some(Block::Heading { level, spans });
    }

    if let Some(num_pr) = props.and_then(|p| p.children_named("numPr").next()) {
        let level = num_pr
            .children_named("ilvl")
            .next()
            .and_then(|l| l.attr("w:val"))
            .and_then(|v| v.parse::<u8>().ok())
            .unwrap_or(0);
        let num_id = num_pr
            .children_named("numId")
            .next()
            .and_then(|n| n.attr("w:val"))
            .unwrap_or("");
        if !num_id.is_empty() && !ctx.numbering.iter().any(|(k, _)| k == num_id) {
            push_once(
                warnings,
                Warning::new(
                    "docx.unknown_numbering",
                    format!(
                        "list numId {num_id} is not defined in word/numbering.xml; \
                         it is shown as a bullet list"
                    ),
                ),
            );
        }
        return Some(Block::ListItem {
            level,
            ordered: ctx.is_ordered(num_id),
            spans,
        });
    }

    Some(Block::Paragraph { spans })
}

fn heading_level_of(style: &str) -> Option<u8> {
    let lower = style.to_ascii_lowercase();
    let digits = lower.strip_prefix("heading")?.trim_start_matches(' ');
    let level: u8 = digits.parse().ok()?;
    (1..=6).contains(&level).then_some(level)
}

fn paragraph_spans(el: &Element, ctx: &ReadCtx) -> Vec<Span> {
    let mut spans = Vec::new();
    collect_spans(el, ctx, None, &mut spans);
    merge_spans(spans)
}

fn collect_spans(el: &Element, ctx: &ReadCtx, link: Option<&str>, out: &mut Vec<Span>) {
    for child in &el.children {
        let Node::Element(child) = child else {
            continue;
        };
        match child.local_name() {
            "hyperlink" => {
                let target = child
                    .attr("r:id")
                    .and_then(|id| ctx.link_target(id))
                    .map(str::to_string)
                    .or_else(|| child.attr("w:anchor").map(|a| format!("#{a}")));
                collect_spans(child, ctx, target.as_deref(), out);
            }
            "r" => {
                let style = run_style(child, link);
                for run_child in &child.children {
                    let Node::Element(rc) = run_child else {
                        continue;
                    };
                    match rc.local_name() {
                        "t" => out.push(Span {
                            text: rc.text_content(),
                            style: style.clone(),
                        }),
                        "tab" => out.push(Span {
                            text: "\t".to_string(),
                            style: style.clone(),
                        }),
                        "br" => out.push(Span {
                            text: " ".to_string(),
                            style: style.clone(),
                        }),
                        _ => {}
                    }
                }
            }
            // Structured document tags and smart tags wrap runs.
            "sdt" | "sdtContent" | "smartTag" | "ins" => collect_spans(child, ctx, link, out),
            _ => {}
        }
    }
}

fn run_style(run: &Element, link: Option<&str>) -> SpanStyle {
    let props = run.children_named("rPr").next();
    let has = |name: &str| {
        props
            .map(|p| {
                p.children_named(name)
                    .next()
                    .map(|e| !matches!(e.attr("w:val"), Some("0") | Some("false")))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    };
    let mono = props
        .and_then(|p| p.children_named("rFonts").next())
        .and_then(|f| f.attr("w:ascii"))
        .map(|font| MONO_FONTS.contains(&font))
        .unwrap_or(false);
    SpanStyle {
        bold: has("b"),
        italic: has("i"),
        code: mono,
        link: link.map(str::to_string),
    }
}

fn merge_spans(spans: Vec<Span>) -> Vec<Span> {
    spans.into_iter().fold(Vec::new(), |mut acc, span| {
        match acc.last_mut() {
            Some(last) if last.style == span.style => last.text.push_str(&span.text),
            _ => {
                if !span.text.is_empty() {
                    acc.push(span)
                }
            }
        }
        acc
    })
}

fn table_block(el: &Element, ctx: &ReadCtx, warnings: &mut Vec<Warning>) -> Block {
    let rows = el
        .children_named("tr")
        .map(|tr| {
            tr.children_named("tc")
                .map(|tc| {
                    let paragraphs: Vec<Vec<Span>> = tc
                        .children_named("p")
                        .map(|p| paragraph_spans(p, ctx))
                        .collect();
                    if paragraphs.len() > 1 {
                        push_once(
                            warnings,
                            Warning::new(
                                "docx.multi_paragraph_cell",
                                "a table cell holds more than one paragraph; \
                                 the cell is shown as one line and saving rewrites \
                                 only its first paragraph",
                            ),
                        );
                    }
                    paragraphs.into_iter().next().unwrap_or_default()
                })
                .collect()
        })
        .collect();
    Block::Table { rows }
}

fn push_once(warnings: &mut Vec<Warning>, warning: Warning) {
    if !warnings.iter().any(|w| w.code == warning.code) {
        warnings.push(warning);
    }
}

// ── Write ────────────────────────────────────────────────────────────

/// Rewrite a `.docx`, replacing its text with `target` and keeping every other
/// part of the package.
pub fn write(original: &[u8], target: &Document) -> Result<Rewrite, DocError> {
    let mut entries = zipedit::read_entries(original)?;
    let part = zipedit::find(&entries, DOCUMENT_PART)
        .ok_or_else(|| DocError::Parse(format!("{DOCUMENT_PART} is missing")))?;
    let mut xml = xmltree::parse_bytes(&part.data)?;

    let rels = read_rels(&entries)?;
    let numbering = read_numbering(&entries)?;
    let ctx = ReadCtx { rels, numbering };

    let body = find_body(&xml.root)
        .ok_or_else(|| DocError::Parse("word/document.xml has no <w:body>".into()))?;
    let mut read_warnings = Vec::new();
    let slots = collect_slots(body, &ctx, &mut read_warnings);

    let target_blocks: Vec<Block> = target
        .sections
        .iter()
        .flat_map(|s| s.blocks.iter().cloned())
        .collect();
    let (effective_blocks, mut warnings) = degrade(target_blocks);

    let mut adapter = DocxAdapter {
        ctx,
        new_rels: Vec::new(),
        bullet_num_id: None,
        ordered_num_id: None,
        needs_numbering: false,
        used_heading_levels: Vec::new(),
    };
    let body_mut = find_body_mut(&mut xml.root)
        .ok_or_else(|| DocError::Parse("word/document.xml has no <w:body>".into()))?;
    warnings.extend(surgical::apply(
        body_mut,
        &slots,
        &effective_blocks,
        &mut adapter,
    )?);

    let document_xml = xmltree::serialize(&xml);
    if !zipedit::replace(&mut entries, DOCUMENT_PART, document_xml.into_bytes()) {
        return Err(DocError::Structure(format!(
            "{DOCUMENT_PART} vanished mid-write"
        )));
    }

    apply_new_rels(&mut entries, &adapter.new_rels)?;
    if adapter.needs_numbering {
        ensure_numbering_part(&mut entries, &mut adapter)?;
    }
    ensure_heading_styles(&mut entries, &adapter.used_heading_levels)?;

    let bytes = zipedit::write_entries(&entries)?;
    Ok(Rewrite {
        bytes,
        effective: Document {
            format: DocFormat::Docx,
            sections: vec![Section {
                id: "body".to_string(),
                title: None,
                blocks: effective_blocks,
            }],
            warnings: Vec::new(),
        },
        warnings,
    })
}

/// Turn blocks the format cannot hold into ones it can, reporting each change.
fn degrade(blocks: Vec<Block>) -> (Vec<Block>, Vec<Warning>) {
    let mut warnings = Vec::new();
    let out = blocks
        .into_iter()
        .flat_map(|block| match block {
            Block::Code { text } => {
                push_once(
                    &mut warnings,
                    Warning::new(
                        "docx.code_block_flattened",
                        "DOCX has no code-block element; fenced code was stored as \
                         monospaced paragraphs and will read back as paragraphs",
                    ),
                );
                text.lines()
                    .map(|line| Block::Paragraph {
                        spans: vec![Span {
                            text: line.to_string(),
                            style: SpanStyle {
                                code: true,
                                ..SpanStyle::plain()
                            },
                        }],
                    })
                    .collect::<Vec<_>>()
            }
            other => vec![other],
        })
        .collect();
    (out, warnings)
}

struct DocxAdapter {
    ctx: ReadCtx,
    /// Relationships to append: (id, target).
    new_rels: Vec<(String, String)>,
    bullet_num_id: Option<String>,
    ordered_num_id: Option<String>,
    needs_numbering: bool,
    used_heading_levels: Vec<u8>,
}

impl DocxAdapter {
    /// Resolve (or mint) the relationship id for a hyperlink target.
    fn rel_for(&mut self, target: &str) -> String {
        if let Some((id, _)) = self.ctx.rels.iter().find(|(_, t)| t == target) {
            return id.clone();
        }
        if let Some((id, _)) = self.new_rels.iter().find(|(_, t)| t == target) {
            return id.clone();
        }
        let id = format!("rIdVibedoc{}", self.new_rels.len() + 1);
        self.new_rels.push((id.clone(), target.to_string()));
        id
    }

    fn num_id_for(&mut self, ordered: bool) -> String {
        let slot = if ordered {
            &mut self.ordered_num_id
        } else {
            &mut self.bullet_num_id
        };
        if let Some(id) = slot {
            return id.clone();
        }
        let found = self
            .ctx
            .numbering
            .iter()
            .find(|(_, is_ordered)| *is_ordered == ordered)
            .map(|(id, _)| id.clone());
        let id = match found {
            Some(id) => id,
            None => {
                self.needs_numbering = true;
                // Ids well above anything Word generates by hand; the numbering
                // part is written with exactly these.
                if ordered {
                    "9002".to_string()
                } else {
                    "9001".to_string()
                }
            }
        };
        *slot = Some(id.clone());
        id
    }
}

impl BlockAdapter for DocxAdapter {
    fn rewrite(&mut self, el: &mut Element, block: &Block) -> Result<Vec<Warning>, DocError> {
        match (el.local_name(), block) {
            ("tbl", Block::Table { rows }) => rewrite_table(el, rows, self),
            ("tbl", _) | (_, Block::Table { .. }) => Err(DocError::Structure(
                "a table cannot be replaced by text (or text by a table) in DOCX; \
                 edit the cells instead"
                    .into(),
            )),
            ("p", _) => rewrite_paragraph(el, block, self),
            (other, _) => Err(DocError::Structure(format!(
                "unexpected <w:{other}> in the document body"
            ))),
        }
    }

    fn build(&mut self, template: Option<&Element>, block: &Block) -> Result<Element, DocError> {
        let mut el = match template {
            // Clone the neighbour so the new paragraph inherits its formatting,
            // then strip the parts `rewrite` is about to set.
            Some(t) if t.local_name() == "p" => {
                let mut clone = t.clone();
                clone
                    .children
                    .retain(|n| matches!(n, Node::Element(e) if e.local_name() == "pPr"));
                clone
            }
            _ => {
                let mut fresh = Element::new("w:p");
                fresh.set_attr("xmlns:w", W_NS);
                fresh
            }
        };
        rewrite_paragraph(&mut el, block, self)?;
        Ok(el)
    }

    fn check_insert(&self, block: &Block, _template: Option<&Element>) -> Result<(), DocError> {
        match block {
            Block::Table { .. } => Err(DocError::Structure(
                "adding a table to a DOCX is not supported; add rows in Word instead".into(),
            )),
            _ => Ok(()),
        }
    }
}

fn rewrite_table(
    el: &mut Element,
    rows: &[Vec<Vec<Span>>],
    adapter: &mut DocxAdapter,
) -> Result<Vec<Warning>, DocError> {
    let existing_rows: Vec<usize> = el
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, n)| match n {
            Node::Element(e) if e.local_name() == "tr" => Some(i),
            _ => None,
        })
        .collect();
    if existing_rows.len() != rows.len() {
        return Err(DocError::Structure(format!(
            "this DOCX table has {} rows but the edited text has {}; \
             adding or removing table rows is not supported",
            existing_rows.len(),
            rows.len()
        )));
    }
    let mut warnings = Vec::new();
    for (row_index, tr_index) in existing_rows.into_iter().enumerate() {
        let Some(Node::Element(tr)) = el.children.get_mut(tr_index) else {
            continue;
        };
        let cell_indices: Vec<usize> = tr
            .children
            .iter()
            .enumerate()
            .filter_map(|(i, n)| match n {
                Node::Element(e) if e.local_name() == "tc" => Some(i),
                _ => None,
            })
            .collect();
        let row = &rows[row_index];
        if cell_indices.len() != row.len() {
            return Err(DocError::Structure(format!(
                "row {} of this DOCX table has {} cells but the edited text has {}; \
                 adding or removing columns is not supported",
                row_index + 1,
                cell_indices.len(),
                row.len()
            )));
        }
        for (cell_index, tc_index) in cell_indices.into_iter().enumerate() {
            let Some(Node::Element(tc)) = tr.children.get_mut(tc_index) else {
                continue;
            };
            let first_p = tc.children.iter_mut().find_map(|n| match n {
                Node::Element(e) if e.local_name() == "p" => Some(e),
                _ => None,
            });
            match first_p {
                Some(p) => {
                    let block = Block::Paragraph {
                        spans: row[cell_index].clone(),
                    };
                    warnings.extend(rewrite_paragraph(p, &block, adapter)?);
                }
                None => {
                    let mut p = Element::new("w:p");
                    let block = Block::Paragraph {
                        spans: row[cell_index].clone(),
                    };
                    rewrite_paragraph(&mut p, &block, adapter)?;
                    tc.children.push(Node::Element(p));
                }
            }
        }
    }
    Ok(warnings)
}

fn rewrite_paragraph(
    el: &mut Element,
    block: &Block,
    adapter: &mut DocxAdapter,
) -> Result<Vec<Warning>, DocError> {
    let spans: Vec<Span> = match block {
        Block::Heading { spans, .. }
        | Block::Paragraph { spans }
        | Block::ListItem { spans, .. } => spans.clone(),
        Block::Rule => Vec::new(),
        Block::Code { text } => vec![Span {
            text: text.clone(),
            style: SpanStyle {
                code: true,
                ..SpanStyle::plain()
            },
        }],
        Block::Table { .. } => {
            return Err(DocError::Structure(
                "a table cannot be written as a paragraph".into(),
            ))
        }
    };

    apply_paragraph_props(el, block, adapter);

    // Find the run template before mutating: the first run that carries text is
    // the closest thing to "how this paragraph is meant to look".
    let template_run = el
        .children
        .iter()
        .find_map(|n| match n {
            Node::Element(e) if e.local_name() == "r" && e.find_descendant("t").is_some() => {
                Some(e.clone())
            }
            _ => None,
        })
        .or_else(|| {
            el.find_descendant("hyperlink")
                .and_then(|h| h.children_named("r").next().cloned())
        });

    // Remove the text-bearing children; anything else (drawings, bookmarks,
    // comment ranges, field codes) stays exactly where it was.
    let text_indices: Vec<usize> = el
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, n)| match n {
            Node::Element(e) if is_text_carrier(e) => Some(i),
            _ => None,
        })
        .collect();
    let insert_at = text_indices.first().copied().unwrap_or(el.children.len());
    for index in text_indices.into_iter().rev() {
        el.children.remove(index);
    }

    let nodes: Vec<Node> = spans
        .iter()
        .filter(|s| !s.text.is_empty())
        .map(|span| Node::Element(build_run(span, template_run.as_ref(), adapter)))
        .collect();
    let at = insert_at.min(el.children.len());
    for (k, node) in nodes.into_iter().enumerate() {
        el.children.insert(at + k, node);
    }
    Ok(Vec::new())
}

/// A run (or hyperlink) whose only contribution is text.
fn is_text_carrier(el: &Element) -> bool {
    match el.local_name() {
        "r" => {
            el.find_descendant("drawing").is_none()
                && el.find_descendant("object").is_none()
                && el.find_descendant("pict").is_none()
        }
        "hyperlink" => true,
        _ => false,
    }
}

fn build_run(span: &Span, template: Option<&Element>, adapter: &mut DocxAdapter) -> Element {
    let mut run = Element::new("w:r");
    let mut props = template
        .and_then(|t| t.children_named("rPr").next().cloned())
        .unwrap_or_else(|| Element::new("w:rPr"));
    // Start from the template's look, then set exactly what the model knows.
    props.children.retain(
        |n| !matches!(n, Node::Element(e) if matches!(e.local_name(), "b" | "bCs" | "i" | "iCs")),
    );
    if span.style.bold {
        props.children.push(Node::Element(empty("w:b")));
    }
    if span.style.italic {
        props.children.push(Node::Element(empty("w:i")));
    }
    if span.style.code {
        props
            .children
            .retain(|n| !matches!(n, Node::Element(e) if e.local_name() == "rFonts"));
        let mut fonts = Element::new("w:rFonts");
        fonts.set_attr("w:ascii", MONO_FONTS[0]);
        fonts.set_attr("w:hAnsi", MONO_FONTS[0]);
        fonts.self_closing = true;
        props.children.insert(0, Node::Element(fonts));
    }
    if !props.children.is_empty() {
        run.children.push(Node::Element(props));
    }

    let mut text = Element::new("w:t");
    text.set_attr("xml:space", "preserve");
    text.set_text(&span.text);
    run.children.push(Node::Element(text));

    match &span.style.link {
        Some(href) if !href.starts_with('#') => {
            let id = adapter.rel_for(href);
            let mut link = Element::new("w:hyperlink");
            link.set_attr("r:id", id);
            link.children.push(Node::Element(run));
            link
        }
        Some(anchor) => {
            let mut link = Element::new("w:hyperlink");
            link.set_attr("w:anchor", anchor.trim_start_matches('#'));
            link.children.push(Node::Element(run));
            link
        }
        None => run,
    }
}

fn empty(name: &str) -> Element {
    let mut el = Element::new(name);
    el.self_closing = true;
    el
}

fn apply_paragraph_props(el: &mut Element, block: &Block, adapter: &mut DocxAdapter) {
    let props_index = el
        .children
        .iter()
        .position(|n| matches!(n, Node::Element(e) if e.local_name() == "pPr"));
    let mut props = match props_index {
        Some(i) => match el.children.remove(i) {
            Node::Element(e) => e,
            other => {
                el.children.insert(i, other);
                Element::new("w:pPr")
            }
        },
        None => Element::new("w:pPr"),
    };

    // Clear the properties this model owns, then set them from the block.
    props.children.retain(
        |n| !matches!(n, Node::Element(e) if matches!(e.local_name(), "pStyle" | "numPr" | "pBdr")),
    );

    match block {
        Block::Heading { level, .. } => {
            let mut style = Element::new("w:pStyle");
            style.set_attr("w:val", format!("Heading{level}"));
            style.self_closing = true;
            props.children.insert(0, Node::Element(style));
            if !adapter.used_heading_levels.contains(level) {
                adapter.used_heading_levels.push(*level);
            }
        }
        Block::ListItem { level, ordered, .. } => {
            let mut style = Element::new("w:pStyle");
            style.set_attr("w:val", "ListParagraph");
            style.self_closing = true;
            let mut num_pr = Element::new("w:numPr");
            let mut ilvl = Element::new("w:ilvl");
            ilvl.set_attr("w:val", level.to_string());
            ilvl.self_closing = true;
            let mut num_id = Element::new("w:numId");
            num_id.set_attr("w:val", adapter.num_id_for(*ordered));
            num_id.self_closing = true;
            num_pr.children.push(Node::Element(ilvl));
            num_pr.children.push(Node::Element(num_id));
            props.children.insert(0, Node::Element(num_pr));
            props.children.insert(0, Node::Element(style));
        }
        Block::Rule => {
            let mut bdr = Element::new("w:pBdr");
            let mut bottom = Element::new("w:bottom");
            bottom.set_attr("w:val", "single");
            bottom.set_attr("w:sz", "6");
            bottom.set_attr("w:space", "1");
            bottom.set_attr("w:color", "auto");
            bottom.self_closing = true;
            bdr.children.push(Node::Element(bottom));
            props.children.insert(0, Node::Element(bdr));
        }
        _ => {}
    }

    if !props.children.is_empty() {
        el.children.insert(0, Node::Element(props));
    }
}

// ── Package parts written on demand ──────────────────────────────────

fn apply_new_rels(entries: &mut [ZipEntry], new_rels: &[(String, String)]) -> Result<(), DocError> {
    if new_rels.is_empty() {
        return Ok(());
    }
    let part = zipedit::find(entries, RELS_PART).ok_or_else(|| {
        DocError::Structure(format!("{RELS_PART} is missing; cannot add hyperlinks"))
    })?;
    let mut xml = xmltree::parse_bytes(&part.data)?;
    for (id, target) in new_rels {
        let mut rel = Element::new("Relationship");
        rel.set_attr("Id", id.clone());
        rel.set_attr("Type", HYPERLINK_REL);
        rel.set_attr("Target", target.clone());
        rel.set_attr("TargetMode", "External");
        rel.self_closing = true;
        xml.root.children.push(Node::Element(rel));
    }
    let data = xmltree::serialize(&xml).into_bytes();
    zipedit::replace(entries, RELS_PART, data);
    Ok(())
}

/// Add a numbering part (and its relationship / content type) when the document
/// has none and the edit introduced a list.
fn ensure_numbering_part(
    entries: &mut Vec<ZipEntry>,
    adapter: &mut DocxAdapter,
) -> Result<(), DocError> {
    if zipedit::find(entries, NUMBERING_PART).is_some() {
        return Ok(());
    }
    entries.push(ZipEntry {
        name: NUMBERING_PART.to_string(),
        data: numbering_xml().into_bytes(),
        compression: zip::CompressionMethod::Deflated,
        is_dir: false,
    });

    // Relationship from the document to the numbering part.
    if let Some(part) = zipedit::find(entries, RELS_PART) {
        let mut xml = xmltree::parse_bytes(&part.data)?;
        let already = xml
            .root
            .children_named("Relationship")
            .any(|r| r.attr("Type") == Some(NUMBERING_REL));
        if !already {
            let mut rel = Element::new("Relationship");
            rel.set_attr("Id", "rIdVibedocNumbering");
            rel.set_attr("Type", NUMBERING_REL);
            rel.set_attr("Target", "numbering.xml");
            rel.self_closing = true;
            xml.root.children.push(Node::Element(rel));
            let data = xmltree::serialize(&xml).into_bytes();
            zipedit::replace(entries, RELS_PART, data);
        }
    }

    // Content-type override, or Word refuses to open the package.
    if let Some(part) = zipedit::find(entries, CONTENT_TYPES_PART) {
        let mut xml = xmltree::parse_bytes(&part.data)?;
        let already = xml
            .root
            .children_named("Override")
            .any(|o| o.attr("PartName") == Some("/word/numbering.xml"));
        if !already {
            let mut over = Element::new("Override");
            over.set_attr("PartName", "/word/numbering.xml");
            over.set_attr(
                "ContentType",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml",
            );
            over.self_closing = true;
            xml.root.children.push(Node::Element(over));
            let data = xmltree::serialize(&xml).into_bytes();
            zipedit::replace(entries, CONTENT_TYPES_PART, data);
        }
    }

    adapter.needs_numbering = false;
    Ok(())
}

fn numbering_xml() -> String {
    let level = |ilvl: u8, fmt: &str, text: &str| {
        format!(
            r#"<w:lvl w:ilvl="{ilvl}"><w:start w:val="1"/><w:numFmt w:val="{fmt}"/>\
<w:lvlText w:val="{text}"/><w:lvlJc w:val="left"/>\
<w:pPr><w:ind w:left="{}" w:hanging="360"/></w:pPr></w:lvl>"#,
            720 + 360 * ilvl as u32
        )
        .replace("\\\n", "")
    };
    let bullet_levels: String = (0..6).map(|i| level(i, "bullet", "\u{2022}")).collect();
    let ordered_levels: String = (0..6).map(|i| level(i, "decimal", "%1.")).collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="{W_NS}">\
<w:abstractNum w:abstractNumId="9001">{bullet_levels}</w:abstractNum>\
<w:abstractNum w:abstractNumId="9002">{ordered_levels}</w:abstractNum>\
<w:num w:numId="9001"><w:abstractNumId w:val="9001"/></w:num>\
<w:num w:numId="9002"><w:abstractNumId w:val="9002"/></w:num>\
</w:numbering>"#
    )
    .replace("\\\n", "")
}

/// Define any heading style the edit introduced that the package lacks, so Word
/// renders `# Title` as a heading rather than body text.
fn ensure_heading_styles(entries: &mut [ZipEntry], levels: &[u8]) -> Result<(), DocError> {
    if levels.is_empty() {
        return Ok(());
    }
    let Some(part) = zipedit::find(entries, STYLES_PART) else {
        return Ok(());
    };
    let mut xml = xmltree::parse_bytes(&part.data)?;
    let mut added = false;
    for level in levels {
        let style_id = format!("Heading{level}");
        let exists = xml.root.children_named("style").any(|s| {
            s.attr("w:styleId")
                .map(|v| v.eq_ignore_ascii_case(&style_id))
                .unwrap_or(false)
        });
        if exists {
            continue;
        }
        let size = 40 - (*level as u32).min(6) * 4;
        let xml_text = format!(
            r#"<w:style w:type="paragraph" w:styleId="{style_id}"><w:name w:val="heading {level}"/>\
<w:basedOn w:val="Normal"/><w:qFormat/><w:pPr><w:outlineLvl w:val="{}"/>\
<w:spacing w:before="240" w:after="120"/></w:pPr>\
<w:rPr><w:b/><w:sz w:val="{size}"/></w:rPr></w:style>"#,
            level - 1
        )
        .replace("\\\n", "");
        let fragment = xmltree::parse(&xml_text)?;
        xml.root.children.push(Node::Element(fragment.root));
        added = true;
    }
    if added {
        let data = xmltree::serialize(&xml).into_bytes();
        zipedit::replace(entries, STYLES_PART, data);
    }
    Ok(())
}
