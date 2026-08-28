//! EPUB 2/3 e-books.
//!
//! A section is one spine item. Reading flattens each chapter's XHTML body into
//! blocks; writing edits that same XHTML in place, so stylesheets, images,
//! metadata, the OPF and the navigation document are carried over untouched.

use crate::error::DocError;
use crate::model::{Block, DocFormat, Document, Section, Span, SpanStyle, Warning};
use crate::surgical::{self, BlockAdapter, Slot};
use crate::xmltree::{self, Element, Node};
use crate::zipedit::{self, ZipEntry};

const CONTAINER_PART: &str = "META-INF/container.xml";

/// The result of rewriting an e-book.
#[derive(Debug)]
pub struct Rewrite {
    pub bytes: Vec<u8>,
    pub effective: Document,
    pub warnings: Vec<Warning>,
}

// ── Spine discovery ──────────────────────────────────────────────────

/// One readable chapter: the zip entry that holds it, in spine order.
#[derive(Debug, Clone)]
struct SpineItem {
    /// Full path of the entry inside the container.
    path: String,
}

/// Container path of the OPF package document. Shared with [`crate::epub_view`],
/// which reads the same package for display rather than for editing.
pub fn opf_path_of(entries: &[ZipEntry]) -> Result<String, DocError> {
    opf_path(entries)
}

/// The `<body>` of a chapter document, wherever it sits in the tree.
pub fn body_of_root(root: &Element) -> Option<&Element> {
    body_of(root)
}

fn opf_path(entries: &[ZipEntry]) -> Result<String, DocError> {
    let container = zipedit::find(entries, CONTAINER_PART)
        .ok_or_else(|| DocError::Parse(format!("{CONTAINER_PART} is missing")))?;
    let xml = xmltree::parse_bytes(&container.data)?;
    xml.root
        .find_descendant("rootfile")
        .and_then(|rf| rf.attr("full-path"))
        .map(str::to_string)
        .ok_or_else(|| DocError::Parse("container.xml names no rootfile".into()))
}

/// Resolve an href relative to the directory holding the OPF.
fn resolve(base_dir: &str, href: &str) -> String {
    let href = href.split('#').next().unwrap_or(href);
    let decoded = percent_decode(href);
    if base_dir.is_empty() {
        return decoded;
    }
    format!("{base_dir}{decoded}")
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            match hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                Some(byte) => {
                    out.push(byte);
                    i += 3;
                    continue;
                }
                None => {}
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn spine(entries: &[ZipEntry]) -> Result<Vec<SpineItem>, DocError> {
    let opf = opf_path(entries)?;
    let base_dir = match opf.rfind('/') {
        Some(i) => opf[..=i].to_string(),
        None => String::new(),
    };
    let part = zipedit::find(entries, &opf)
        .ok_or_else(|| DocError::Parse(format!("{opf} named by container.xml is missing")))?;
    let xml = xmltree::parse_bytes(&part.data)?;

    let manifest = xml
        .root
        .children_named("manifest")
        .next()
        .ok_or_else(|| DocError::Parse("the OPF has no <manifest>".into()))?;
    let items: Vec<(String, String, String)> = manifest
        .children_named("item")
        .filter_map(|item| {
            Some((
                item.attr("id")?.to_string(),
                item.attr("href")?.to_string(),
                item.attr("media-type").unwrap_or("").to_string(),
            ))
        })
        .collect();

    let spine_el = xml
        .root
        .children_named("spine")
        .next()
        .ok_or_else(|| DocError::Parse("the OPF has no <spine>".into()))?;

    let chapters: Vec<SpineItem> = spine_el
        .children_named("itemref")
        .filter_map(|itemref| {
            let idref = itemref.attr("idref")?;
            let (_, href, media) = items.iter().find(|(id, _, _)| id == idref)?;
            // Only XHTML documents carry readable text; an SVG cover in the
            // spine is skipped rather than rendered as an empty chapter.
            let is_xhtml =
                media.contains("xhtml") || media.contains("text/html") || media.is_empty();
            is_xhtml.then(|| SpineItem {
                path: resolve(&base_dir, href),
            })
        })
        .collect();

    if chapters.is_empty() {
        return Err(DocError::Parse(
            "this EPUB's spine has no XHTML chapters".into(),
        ));
    }
    Ok(chapters)
}

// ── Read ─────────────────────────────────────────────────────────────

/// Parse an `.epub` file into the document model.
pub fn read(bytes: &[u8]) -> Result<Document, DocError> {
    let entries = zipedit::read_entries(bytes)?;
    let chapters = spine(&entries)?;
    let mut warnings = Vec::new();

    let sections = chapters
        .iter()
        .map(|chapter| {
            let part = zipedit::find(&entries, &chapter.path).ok_or_else(|| {
                DocError::Parse(format!(
                    "spine item {} is not in the container",
                    chapter.path
                ))
            })?;
            let xml = xmltree::parse_bytes(&part.data)?;
            let body = body_of(&xml.root)
                .ok_or_else(|| DocError::Parse(format!("{} has no <body>", chapter.path)))?;
            let slots = collect_slots(body, &mut warnings);
            Ok(Section {
                id: chapter.path.clone(),
                title: chapter_title(&xml.root, &slots),
                blocks: slots.into_iter().map(|s| s.block).collect(),
            })
        })
        .collect::<Result<Vec<_>, DocError>>()?;

    Ok(Document {
        format: DocFormat::Epub,
        sections,
        warnings,
    })
}

fn body_of(root: &Element) -> Option<&Element> {
    if root.local_name() == "body" {
        return Some(root);
    }
    root.children_named("body")
        .next()
        .or_else(|| root.find_descendant("body"))
}

fn body_of_mut(root: &mut Element) -> Option<&mut Element> {
    if root.local_name() == "body" {
        return Some(root);
    }
    fn walk(el: &mut Element) -> Option<&mut Element> {
        let found = el
            .children
            .iter()
            .position(|n| matches!(n, Node::Element(e) if e.local_name() == "body"));
        if let Some(i) = found {
            return match el.children.get_mut(i) {
                Some(Node::Element(e)) => Some(e),
                _ => None,
            };
        }
        for child in el.children.iter_mut() {
            if let Node::Element(e) = child {
                if let Some(found) = walk(e) {
                    return Some(found);
                }
            }
        }
        None
    }
    walk(root)
}

fn chapter_title(root: &Element, slots: &[Slot]) -> Option<String> {
    let head_title = root
        .find_descendant("title")
        .map(|t| normalize_ws(&t.text_content()))
        .filter(|t| !t.is_empty());
    head_title.or_else(|| {
        slots.iter().find_map(|slot| match &slot.block {
            Block::Heading { spans, .. } => {
                let text = crate::model::spans_text(spans);
                (!text.trim().is_empty()).then(|| text.trim().to_string())
            }
            _ => None,
        })
    })
}

/// Block-level elements this crate models.
fn block_of(el: &Element, warnings: &mut Vec<Warning>) -> Option<Block> {
    // A text-less element is not a block: an image-only paragraph or a spacer
    // has no representation in the buffer, so treating it as an empty block
    // would make the next save delete it.
    let spans_if_text = |el: &Element, warnings: &mut Vec<Warning>| {
        let spans = inline_spans(el, warnings);
        (!spans.iter().all(|s| s.text.trim().is_empty())).then_some(spans)
    };
    match el.local_name() {
        "p" => Some(Block::Paragraph {
            spans: spans_if_text(el, warnings)?,
        }),
        name if is_heading(name) => {
            let level = name[1..].parse().unwrap_or(1);
            Some(Block::Heading {
                level,
                spans: spans_if_text(el, warnings)?,
            })
        }
        "li" => {
            let ordered = false; // corrected by the caller, which knows the parent
            Some(Block::ListItem {
                level: 0,
                ordered,
                spans: spans_if_text(el, warnings)?,
            })
        }
        "pre" => Some(Block::Code {
            text: el.text_content().trim_end_matches('\n').to_string(),
        }),
        "hr" => Some(Block::Rule),
        "table" => Some(table_block(el, warnings)),
        _ => None,
    }
}

fn is_heading(name: &str) -> bool {
    matches!(name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

/// Containers we descend into looking for blocks.
fn is_container(name: &str) -> bool {
    matches!(
        name,
        "div"
            | "section"
            | "article"
            | "main"
            | "blockquote"
            | "ul"
            | "ol"
            | "nav"
            | "aside"
            | "header"
            | "footer"
            | "figure"
    )
}

fn collect_slots(body: &Element, warnings: &mut Vec<Warning>) -> Vec<Slot> {
    let mut slots = Vec::new();
    walk_slots(
        body,
        &mut Vec::new(),
        &ListCtx::default(),
        warnings,
        &mut slots,
    );
    slots
}

#[derive(Default, Clone)]
struct ListCtx {
    depth: u8,
    ordered: bool,
}

fn walk_slots(
    el: &Element,
    path: &mut Vec<usize>,
    list: &ListCtx,
    warnings: &mut Vec<Warning>,
    out: &mut Vec<Slot>,
) {
    for (i, node) in el.children.iter().enumerate() {
        let Node::Element(child) = node else { continue };
        path.push(i);
        match block_of(child, warnings) {
            Some(Block::ListItem { spans, .. }) => out.push(Slot {
                path: path.clone(),
                block: Block::ListItem {
                    level: list.depth.saturating_sub(1),
                    ordered: list.ordered,
                    spans,
                },
            }),
            Some(block) => out.push(Slot {
                path: path.clone(),
                block,
            }),
            None if is_container(child.local_name()) => {
                let name = child.local_name();
                let nested = match name {
                    "ul" | "ol" => ListCtx {
                        depth: list.depth + 1,
                        ordered: name == "ol",
                    },
                    _ => list.clone(),
                };
                walk_slots(child, path, &nested, warnings, out);
            }
            None => {}
        }
        path.pop();
    }
}

fn table_block(el: &Element, warnings: &mut Vec<Warning>) -> Block {
    let mut rows = Vec::new();
    collect_rows(el, warnings, &mut rows);
    Block::Table { rows }
}

fn collect_rows(el: &Element, warnings: &mut Vec<Warning>, rows: &mut Vec<Vec<Vec<Span>>>) {
    for child in &el.children {
        let Node::Element(child) = child else {
            continue;
        };
        match child.local_name() {
            "tr" => {
                let cells = child
                    .children
                    .iter()
                    .filter_map(|n| match n {
                        Node::Element(cell) if matches!(cell.local_name(), "td" | "th") => {
                            Some(inline_spans(cell, warnings))
                        }
                        _ => None,
                    })
                    .collect();
                rows.push(cells);
            }
            "thead" | "tbody" | "tfoot" => collect_rows(child, warnings, rows),
            _ => {}
        }
    }
}

fn inline_spans(el: &Element, warnings: &mut Vec<Warning>) -> Vec<Span> {
    let mut spans = Vec::new();
    collect_inline(el, &SpanStyle::plain(), warnings, &mut spans);
    let merged = merge_spans(spans);
    trim_edges(merged)
}

fn collect_inline(
    el: &Element,
    style: &SpanStyle,
    warnings: &mut Vec<Warning>,
    out: &mut Vec<Span>,
) {
    for node in &el.children {
        match node {
            Node::Text(raw) => {
                let text = normalize_ws_keep_edges(&xmltree::unescape_text(raw));
                if !text.is_empty() {
                    out.push(Span {
                        text,
                        style: style.clone(),
                    });
                }
            }
            Node::Raw(_) => {}
            Node::Element(child) => {
                let mut nested = style.clone();
                match child.local_name() {
                    "b" | "strong" => nested.bold = true,
                    "i" | "em" => nested.italic = true,
                    "code" | "kbd" | "samp" | "tt" => nested.code = true,
                    "a" => {
                        if let Some(href) = child.attr("href") {
                            nested.link = Some(href.to_string());
                        }
                    }
                    "br" => {
                        out.push(Span {
                            text: " ".to_string(),
                            style: style.clone(),
                        });
                        continue;
                    }
                    "img" | "image" | "svg" => {
                        push_once(
                            warnings,
                            Warning::new(
                                "epub.inline_image",
                                "this chapter has inline images; they are not shown in the \
                                 text buffer and are kept at the end of their paragraph when saved",
                            ),
                        );
                        continue;
                    }
                    _ => {}
                }
                collect_inline(child, &nested, warnings, out);
            }
        }
    }
}

fn push_once(warnings: &mut Vec<Warning>, warning: Warning) {
    if !warnings.iter().any(|w| w.code == warning.code) {
        warnings.push(warning);
    }
}

fn merge_spans(spans: Vec<Span>) -> Vec<Span> {
    spans.into_iter().fold(Vec::new(), |mut acc, span| {
        match acc.last_mut() {
            Some(last) if last.style == span.style => last.text.push_str(&span.text),
            _ => acc.push(span),
        }
        acc
    })
}

/// Trim the whitespace XHTML indentation adds at the edges of a block.
fn trim_edges(mut spans: Vec<Span>) -> Vec<Span> {
    if let Some(first) = spans.first_mut() {
        first.text = first.text.trim_start().to_string();
    }
    if let Some(last) = spans.last_mut() {
        last.text = last.text.trim_end().to_string();
    }
    spans.retain(|s| !s.text.is_empty());
    spans
}

/// Collapse HTML whitespace runs to single spaces.
fn normalize_ws_keep_edges(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_ws = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            out.push(c);
            in_ws = false;
        }
    }
    out
}

fn normalize_ws(text: &str) -> String {
    normalize_ws_keep_edges(text).trim().to_string()
}

// ── Write ────────────────────────────────────────────────────────────

/// Rewrite an `.epub`, replacing chapter text with `target`.
pub fn write(original: &[u8], target: &Document) -> Result<Rewrite, DocError> {
    let mut entries = zipedit::read_entries(original)?;
    let chapters = spine(&entries)?;

    if target.sections.len() != chapters.len() {
        return Err(DocError::Structure(format!(
            "this EPUB has {} chapters but the edited text has {}; \
             adding or removing chapters is not supported — keep every \
             `<!-- vibedoc:section … -->` marker in place",
            chapters.len(),
            target.sections.len()
        )));
    }

    let mut warnings = Vec::new();
    let mut effective_sections = Vec::new();

    for (chapter, section) in chapters.iter().zip(target.sections.iter()) {
        let part = zipedit::find(&entries, &chapter.path).ok_or_else(|| {
            DocError::Parse(format!(
                "spine item {} is not in the container",
                chapter.path
            ))
        })?;
        let mut xml = xmltree::parse_bytes(&part.data)?;
        let body = body_of(&xml.root)
            .ok_or_else(|| DocError::Parse(format!("{} has no <body>", chapter.path)))?;
        let mut read_warnings = Vec::new();
        let slots = collect_slots(body, &mut read_warnings);
        let original_title = chapter_title(&xml.root, &slots);

        if section.title.is_some() && section.title != original_title {
            push_once(
                &mut warnings,
                Warning::new(
                    "epub.section_marker_ignored",
                    "a chapter title in a section marker was changed; chapter titles come \
                     from the EPUB itself and are not written back",
                ),
            );
        }

        let mut adapter = XhtmlAdapter {
            warnings: Vec::new(),
        };
        let body_mut = body_of_mut(&mut xml.root)
            .ok_or_else(|| DocError::Parse(format!("{} has no <body>", chapter.path)))?;
        warnings.extend(surgical::apply(
            body_mut,
            &slots,
            &section.blocks,
            &mut adapter,
        )?);
        warnings.extend(adapter.warnings);

        let data = xmltree::serialize(&xml).into_bytes();
        if !zipedit::replace(&mut entries, &chapter.path, data) {
            return Err(DocError::Structure(format!(
                "{} vanished mid-write",
                chapter.path
            )));
        }

        effective_sections.push(Section {
            id: chapter.path.clone(),
            title: original_title,
            blocks: section.blocks.clone(),
        });
    }

    let bytes = zipedit::write_entries(&entries)?;
    Ok(Rewrite {
        bytes,
        effective: Document {
            format: DocFormat::Epub,
            sections: effective_sections,
            warnings: Vec::new(),
        },
        warnings,
    })
}

struct XhtmlAdapter {
    warnings: Vec<Warning>,
}

impl XhtmlAdapter {
    /// Replace an element's inline content, keeping media children.
    fn set_inline(&mut self, el: &mut Element, spans: &[Span]) {
        let kept: Vec<Node> = el
            .children
            .iter()
            .filter(|n| matches!(n, Node::Element(e) if matches!(e.local_name(), "img" | "image" | "svg")))
            .cloned()
            .collect();
        if !kept.is_empty() {
            push_once(
                &mut self.warnings,
                Warning::new(
                    "epub.inline_image_moved",
                    "inline images were kept at the end of their paragraph, \
                     because the text buffer does not record where they sat",
                ),
            );
        }
        el.children = spans.iter().map(span_node).collect();
        el.children.extend(kept);
        el.self_closing = false;
    }
}

fn span_node(span: &Span) -> Node {
    let text = Node::Text(xmltree::escape_text(&span.text));
    let mut node = text;
    let wrap = |inner: Node, name: &str| {
        let mut el = Element::new(name);
        el.children.push(inner);
        Node::Element(el)
    };
    if span.style.code {
        node = wrap(node, "code");
    }
    if span.style.bold {
        node = wrap(node, "strong");
    }
    if span.style.italic {
        node = wrap(node, "em");
    }
    if let Some(href) = &span.style.link {
        let mut el = Element::new("a");
        el.set_attr("href", href.clone());
        el.children.push(node);
        node = Node::Element(el);
    }
    node
}

impl BlockAdapter for XhtmlAdapter {
    fn rewrite(&mut self, el: &mut Element, block: &Block) -> Result<Vec<Warning>, DocError> {
        match block {
            Block::Paragraph { spans } => {
                if is_heading(el.local_name()) || el.local_name() == "li" {
                    // A heading demoted to a paragraph keeps its place in the
                    // flow; a list item demoted to a paragraph stays inside its
                    // list, which is what the reader will report back.
                    if is_heading(el.local_name()) {
                        el.set_name("p");
                    }
                }
                self.set_inline(el, spans);
                Ok(Vec::new())
            }
            Block::Heading { level, spans } => {
                let name = format!("h{}", (*level).clamp(1, 6));
                if el.local_name() != name {
                    el.set_name(name);
                }
                self.set_inline(el, spans);
                Ok(Vec::new())
            }
            Block::ListItem { spans, .. } => {
                if el.local_name() != "li" {
                    return Err(DocError::Structure(format!(
                        "cannot turn <{}> into a list item; add the item inside \
                         an existing list instead",
                        el.local_name()
                    )));
                }
                self.set_inline(el, spans);
                Ok(Vec::new())
            }
            Block::Code { text } => {
                if el.local_name() != "pre" {
                    el.set_name("pre");
                }
                el.set_text(text);
                Ok(Vec::new())
            }
            Block::Rule => {
                if el.local_name() != "hr" {
                    el.set_name("hr");
                }
                el.children.clear();
                el.self_closing = true;
                Ok(Vec::new())
            }
            Block::Table { rows } => self.rewrite_table(el, rows),
        }
    }

    fn build(&mut self, template: Option<&Element>, block: &Block) -> Result<Element, DocError> {
        match block {
            Block::ListItem { spans, ordered, .. } => {
                let mut li = Element::new("li");
                self.set_inline(&mut li, spans);
                match template.map(Element::local_name) {
                    Some("li") => Ok(li),
                    _ => {
                        // No list to join: wrap the item in its own list rather
                        // than emitting an <li> with no parent list.
                        push_once(
                            &mut self.warnings,
                            Warning::new(
                                "epub.list_wrapped",
                                "a new list item was written as its own single-item list, \
                                 because there was no list at that position",
                            ),
                        );
                        let mut list = Element::new(if *ordered { "ol" } else { "ul" });
                        list.children.push(Node::Element(li));
                        Ok(list)
                    }
                }
            }
            other => {
                let name = match other {
                    Block::Heading { level, .. } => format!("h{}", (*level).clamp(1, 6)),
                    Block::Code { .. } => "pre".to_string(),
                    Block::Rule => "hr".to_string(),
                    _ => "p".to_string(),
                };
                let mut el = Element::new(name);
                self.rewrite(&mut el, other)?;
                Ok(el)
            }
        }
    }

    fn check_insert(&self, block: &Block, _template: Option<&Element>) -> Result<(), DocError> {
        match block {
            Block::Table { .. } => Err(DocError::Structure(
                "adding a table to an EPUB chapter is not supported".into(),
            )),
            _ => Ok(()),
        }
    }
}

impl XhtmlAdapter {
    fn rewrite_table(
        &mut self,
        el: &mut Element,
        rows: &[Vec<Vec<Span>>],
    ) -> Result<Vec<Warning>, DocError> {
        let mut row_paths: Vec<Vec<usize>> = Vec::new();
        collect_row_paths(el, &mut Vec::new(), &mut row_paths);
        if row_paths.len() != rows.len() {
            return Err(DocError::Structure(format!(
                "this table has {} rows but the edited text has {}; \
                 adding or removing table rows is not supported",
                row_paths.len(),
                rows.len()
            )));
        }
        for (row_index, path) in row_paths.iter().enumerate() {
            let Some(tr) = surgical::get_mut(el, path) else {
                continue;
            };
            let cell_indices: Vec<usize> = tr
                .children
                .iter()
                .enumerate()
                .filter_map(|(i, n)| match n {
                    Node::Element(e) if matches!(e.local_name(), "td" | "th") => Some(i),
                    _ => None,
                })
                .collect();
            if cell_indices.len() != rows[row_index].len() {
                return Err(DocError::Structure(format!(
                    "row {} has {} cells but the edited text has {}; \
                     adding or removing columns is not supported",
                    row_index + 1,
                    cell_indices.len(),
                    rows[row_index].len()
                )));
            }
            for (cell_index, node_index) in cell_indices.into_iter().enumerate() {
                let Some(Node::Element(cell)) = tr.children.get_mut(node_index) else {
                    continue;
                };
                let spans = rows[row_index][cell_index].clone();
                self.set_inline(cell, &spans);
            }
        }
        Ok(Vec::new())
    }
}

fn collect_row_paths(el: &Element, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    for (i, node) in el.children.iter().enumerate() {
        let Node::Element(child) = node else { continue };
        path.push(i);
        match child.local_name() {
            "tr" => out.push(path.clone()),
            "thead" | "tbody" | "tfoot" => collect_row_paths(child, path, out),
            _ => {}
        }
        path.pop();
    }
}
