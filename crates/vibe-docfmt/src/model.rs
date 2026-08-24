//! The document model shared by every format in this crate.
//!
//! The model is deliberately small. It carries only what all three supported
//! formats can express *and* what a Markdown (or plain-text) buffer can carry
//! back without guessing: block structure, inline emphasis, links, and tables.
//! Anything a reader cannot represent here is reported as a [`Warning`] rather
//! than silently flattened — a document that loses a footnote must say so.

use serde::{Deserialize, Serialize};

/// A document format this crate can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocFormat {
    /// Office Open XML word processing document (`.docx`).
    Docx,
    /// EPUB 2/3 e-book (`.epub`).
    Epub,
    /// Apple Pages document or bundle (`.pages`).
    Pages,
}

impl DocFormat {
    /// Lowercase identifier used on the wire and in the UI.
    pub const fn as_str(self) -> &'static str {
        match self {
            DocFormat::Docx => "docx",
            DocFormat::Epub => "epub",
            DocFormat::Pages => "pages",
        }
    }

    /// Which text syntax the editable buffer for this format uses.
    ///
    /// Pages is plain text on purpose: its reader recovers paragraph text but
    /// not emphasis, so presenting the buffer as Markdown would invite the user
    /// to type `**bold**` and have it stored literally.
    pub const fn syntax(self) -> Syntax {
        match self {
            DocFormat::Docx | DocFormat::Epub => Syntax::Markdown,
            DocFormat::Pages => Syntax::PlainText,
        }
    }
}

/// The syntax of the editable text buffer for a format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Syntax {
    Markdown,
    PlainText,
}

impl Syntax {
    /// Monaco language id for this syntax.
    pub const fn monaco_language(self) -> &'static str {
        match self {
            Syntax::Markdown => "markdown",
            Syntax::PlainText => "plaintext",
        }
    }
}

/// Inline emphasis applied to a run of text.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanStyle {
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    /// Hyperlink target, when the run is a link.
    pub link: Option<String>,
}

impl SpanStyle {
    pub const fn plain() -> Self {
        SpanStyle { bold: false, italic: false, code: false, link: None }
    }

    pub fn is_plain(&self) -> bool {
        !self.bold && !self.italic && !self.code && self.link.is_none()
    }
}

/// A styled run of text inside a block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub text: String,
    pub style: SpanStyle,
}

impl Span {
    pub fn plain(text: impl Into<String>) -> Self {
        Span { text: text.into(), style: SpanStyle::plain() }
    }
}

/// A block-level element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Block {
    Heading { level: u8, spans: Vec<Span> },
    Paragraph { spans: Vec<Span> },
    ListItem { level: u8, ordered: bool, spans: Vec<Span> },
    /// Preformatted text; `text` keeps its own newlines.
    Code { text: String },
    /// First row is treated as the header row.
    Table { rows: Vec<Vec<Vec<Span>>> },
    Rule,
}

impl Block {
    /// The block's text with all styling dropped.
    pub fn plain_text(&self) -> String {
        match self {
            Block::Heading { spans, .. }
            | Block::Paragraph { spans }
            | Block::ListItem { spans, .. } => spans_text(spans),
            Block::Code { text } => text.clone(),
            Block::Table { rows } => rows
                .iter()
                .map(|row| {
                    row.iter().map(|cell| spans_text(cell)).collect::<Vec<_>>().join("\t")
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Block::Rule => String::new(),
        }
    }
}

/// Concatenated text of a span list.
pub fn spans_text(spans: &[Span]) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

/// One addressable part of a document.
///
/// DOCX has exactly one section. EPUB has one per spine item. Pages has one per
/// text storage found in the archive (body, header, footer, shape text …), which
/// is what makes write-back positional rather than a guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    /// Stable identifier used to route an edit back to the right container:
    /// the spine href for EPUB, the storage address for Pages, `"body"` for DOCX.
    pub id: String,
    pub title: Option<String>,
    pub blocks: Vec<Block>,
}

/// Something the reader or writer could not do faithfully.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warning {
    /// Short machine-readable code, e.g. `pages.style_runs_remapped`.
    pub code: String,
    pub message: String,
}

impl Warning {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Warning { code: code.into(), message: message.into() }
    }
}

/// A parsed document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub format: DocFormat,
    pub sections: Vec<Section>,
    pub warnings: Vec<Warning>,
}

impl Document {
    pub fn new(format: DocFormat) -> Self {
        Document { format, sections: Vec::new(), warnings: Vec::new() }
    }

    /// Total number of blocks across all sections.
    pub fn block_count(&self) -> usize {
        self.sections.iter().map(|s| s.blocks.len()).sum()
    }
}
