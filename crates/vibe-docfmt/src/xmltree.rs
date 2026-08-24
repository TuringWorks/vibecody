//! A tiny XML tree that round-trips byte-for-byte when nothing is modified.
//!
//! Rebuilding `word/document.xml` from a text model would be far less code —
//! and would delete every image, footnote, comment and page-setup element the
//! model does not know about. So instead the tree keeps the raw source of every
//! node it did not touch, and edits rewrite only the runs they mean to rewrite.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::error::DocError;

/// An XML node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Element(Element),
    /// Character data, stored in its original escaped form.
    Text(String),
    /// Comments, CDATA, processing instructions, doctype, declaration: kept
    /// verbatim, including their delimiters.
    Raw(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// Qualified name as written, e.g. `w:p`.
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
    /// True when the source wrote this as `<tag/>`.
    pub self_closing: bool,
    /// Verbatim open tag from the source (without `<`/`>`), used while the
    /// element's own name and attributes are untouched.
    raw_open: Option<String>,
}

impl Element {
    pub fn new(name: impl Into<String>) -> Self {
        Element {
            name: name.into(),
            attrs: Vec::new(),
            children: Vec::new(),
            self_closing: false,
            raw_open: None,
        }
    }

    /// Local name with any namespace prefix removed.
    pub fn local_name(&self) -> &str {
        match self.name.split_once(':') {
            Some((_, local)) => local,
            None => &self.name,
        }
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// Set (or replace) an attribute. Invalidates the verbatim open tag.
    pub fn set_attr(&mut self, name: &str, value: impl Into<String>) {
        self.raw_open = None;
        let value = value.into();
        match self.attrs.iter_mut().find(|(k, _)| k == name) {
            Some(slot) => slot.1 = value,
            None => self.attrs.push((name.to_string(), value)),
        }
    }

    /// Rename the element (e.g. `h1` → `h2`).
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.raw_open = None;
        self.name = name.into();
    }

    /// Direct children that are elements with the given local name.
    pub fn children_named<'a>(&'a self, local: &'a str) -> impl Iterator<Item = &'a Element> {
        self.children.iter().filter_map(move |n| match n {
            Node::Element(e) if e.local_name() == local => Some(e),
            _ => None,
        })
    }

    /// First descendant (depth-first) with the given local name.
    pub fn find_descendant(&self, local: &str) -> Option<&Element> {
        for child in &self.children {
            if let Node::Element(e) = child {
                if e.local_name() == local {
                    return Some(e);
                }
                if let Some(found) = e.find_descendant(local) {
                    return Some(found);
                }
            }
        }
        None
    }

    /// Concatenated text of this element and its descendants, unescaped.
    pub fn text_content(&self) -> String {
        let mut out = String::new();
        collect_text(self, &mut out);
        out
    }

    /// Replace all children with a single text node.
    pub fn set_text(&mut self, text: &str) {
        self.children = vec![Node::Text(escape_text(text))];
        self.self_closing = false;
    }
}

fn collect_text(el: &Element, out: &mut String) {
    for child in &el.children {
        match child {
            Node::Text(t) => out.push_str(&unescape_text(t)),
            Node::Element(e) => collect_text(e, out),
            Node::Raw(_) => {}
        }
    }
}

/// A parsed XML document: the prologue (declaration, doctype, comments) plus a
/// single root element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlDoc {
    pub prologue: Vec<Node>,
    pub root: Element,
    /// Trailing nodes after the root element (rare; usually a newline).
    pub epilogue: Vec<Node>,
}

/// Parse XML into a tree.
pub fn parse(xml: &str) -> Result<XmlDoc, DocError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    reader.config_mut().expand_empty_elements = false;
    reader.config_mut().check_end_names = false;

    let mut prologue: Vec<Node> = Vec::new();
    let mut stack: Vec<Element> = Vec::new();
    let mut root: Option<Element> = None;
    let mut epilogue: Vec<Node> = Vec::new();

    loop {
        let event = reader.read_event().map_err(|e| DocError::Parse(e.to_string()))?;
        match event {
            Event::Eof => break,
            Event::Start(e) => {
                stack.push(element_from(&e, false)?);
            }
            Event::Empty(e) => {
                let el = element_from(&e, true)?;
                // `<root/>` is a whole document: without this it would land in
                // the prologue and the parse would end with no root at all.
                if stack.is_empty() && root.is_none() {
                    root = Some(el);
                } else {
                    push_node(&mut stack, &mut prologue, &mut epilogue, &root, Node::Element(el));
                }
            }
            Event::End(_) => {
                let Some(finished) = stack.pop() else { continue };
                if stack.is_empty() {
                    root = Some(finished);
                } else if let Some(parent) = stack.last_mut() {
                    parent.children.push(Node::Element(finished));
                }
            }
            Event::Text(e) => {
                let raw = String::from_utf8_lossy(e.as_ref()).into_owned();
                push_node(&mut stack, &mut prologue, &mut epilogue, &root, Node::Text(raw));
            }
            Event::CData(e) => {
                let raw = String::from_utf8_lossy(e.as_ref()).into_owned();
                push_node(
                    &mut stack,
                    &mut prologue,
                    &mut epilogue,
                    &root,
                    Node::Raw(format!("<![CDATA[{raw}]]>")),
                );
            }
            Event::Comment(e) => {
                let raw = String::from_utf8_lossy(e.as_ref()).into_owned();
                push_node(
                    &mut stack,
                    &mut prologue,
                    &mut epilogue,
                    &root,
                    Node::Raw(format!("<!--{raw}-->")),
                );
            }
            Event::Decl(e) => {
                let raw = String::from_utf8_lossy(e.as_ref()).into_owned();
                push_node(
                    &mut stack,
                    &mut prologue,
                    &mut epilogue,
                    &root,
                    Node::Raw(format!("<?{raw}?>")),
                );
            }
            Event::PI(e) => {
                let raw = String::from_utf8_lossy(e.as_ref()).into_owned();
                push_node(
                    &mut stack,
                    &mut prologue,
                    &mut epilogue,
                    &root,
                    Node::Raw(format!("<?{raw}?>")),
                );
            }
            Event::DocType(e) => {
                let raw = String::from_utf8_lossy(e.as_ref()).into_owned();
                push_node(
                    &mut stack,
                    &mut prologue,
                    &mut epilogue,
                    &root,
                    Node::Raw(format!("<!DOCTYPE{raw}>")),
                );
            }
            Event::GeneralRef(e) => {
                let raw = String::from_utf8_lossy(e.as_ref()).into_owned();
                push_node(&mut stack, &mut prologue, &mut epilogue, &root, Node::Text(format!("&{raw};")));
            }
        }
    }

    let root = root.ok_or_else(|| DocError::Parse("XML has no root element".into()))?;
    Ok(XmlDoc { prologue, root, epilogue })
}

fn push_node(
    stack: &mut [Element],
    prologue: &mut Vec<Node>,
    epilogue: &mut Vec<Node>,
    root: &Option<Element>,
    node: Node,
) {
    match stack.last_mut() {
        Some(parent) => parent.children.push(node),
        None if root.is_none() => prologue.push(node),
        None => epilogue.push(node),
    }
}

fn element_from(e: &BytesStart<'_>, self_closing: bool) -> Result<Element, DocError> {
    let raw = String::from_utf8_lossy(e.as_ref()).into_owned();
    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
    let attrs = e
        .attributes()
        .with_checks(false)
        .map(|attr| {
            let attr = attr.map_err(|err| DocError::Parse(err.to_string()))?;
            let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
            let value = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
            Ok((key, value))
        })
        .collect::<Result<Vec<_>, DocError>>()?;
    Ok(Element { name, attrs, children: Vec::new(), self_closing, raw_open: Some(raw) })
}

/// Serialize a tree back to XML.
pub fn serialize(doc: &XmlDoc) -> String {
    let mut out = String::new();
    for node in &doc.prologue {
        write_node(node, &mut out);
    }
    write_element(&doc.root, &mut out);
    for node in &doc.epilogue {
        write_node(node, &mut out);
    }
    out
}

fn write_node(node: &Node, out: &mut String) {
    match node {
        Node::Text(t) => out.push_str(t),
        Node::Raw(r) => out.push_str(r),
        Node::Element(e) => write_element(e, out),
    }
}

fn write_element(el: &Element, out: &mut String) {
    let open = match &el.raw_open {
        Some(raw) => raw.clone(),
        None => {
            let attrs: String = el
                .attrs
                .iter()
                .map(|(k, v)| format!(" {k}=\"{}\"", escape_attr(v)))
                .collect();
            format!("{}{attrs}", el.name)
        }
    };
    if el.self_closing && el.children.is_empty() {
        out.push('<');
        out.push_str(&open);
        out.push_str("/>");
        return;
    }
    out.push('<');
    out.push_str(&open);
    out.push('>');
    for child in &el.children {
        write_node(child, out);
    }
    out.push_str("</");
    out.push_str(&el.name);
    out.push('>');
}

/// Escape text for an XML text node.
pub fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Escape a value for an XML attribute.
pub fn escape_attr(text: &str) -> String {
    escape_text(text).replace('"', "&quot;")
}

/// Resolve the XML entities this crate emits, plus the common named ones.
pub fn unescape_text(text: &str) -> String {
    // `quick_xml` has a full unescaper, but it errors on the unknown entities
    // that appear in real EPUBs (`&nbsp;` without a doctype). Falling back to a
    // literal copy keeps the character rather than failing the whole document.
    match quick_xml::escape::unescape(text) {
        Ok(cow) => cow.into_owned(),
        Err(_) => text
            .replace("&nbsp;", "\u{a0}")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&"),
    }
}

/// Parse XML from bytes, assuming UTF-8.
pub fn parse_bytes(bytes: &[u8]) -> Result<XmlDoc, DocError> {
    let text = String::from_utf8(bytes.to_vec())
        .map_err(|e| DocError::Parse(format!("XML part is not UTF-8: {e}")))?;
    parse(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_document_byte_for_byte() {
        let source = concat!(
            r#"<?xml version="1.0" encoding="UTF-8"?>"#,
            "\n",
            r#"<!-- a note --><root a="1" b="&amp;"><child/><t>text &lt;here&gt;</t>"#,
            "\n  <nested><deep>x</deep></nested>\n</root>\n",
        );
        let doc = parse(source).expect("parse");
        assert_eq!(serialize(&doc), source);
    }

    #[test]
    fn a_self_closing_root_is_still_a_root() {
        let doc = parse(r#"<Relationships xmlns="urn:x"/>"#).expect("parse");
        assert_eq!(doc.root.name, "Relationships");
        assert_eq!(doc.root.attr("xmlns"), Some("urn:x"));
    }

    #[test]
    fn text_content_unescapes_and_set_text_escapes() {
        let mut doc = parse("<p>a &amp; b</p>").expect("parse");
        assert_eq!(doc.root.text_content(), "a & b");
        doc.root.set_text("x < y");
        assert_eq!(serialize(&doc), "<p>x &lt; y</p>");
    }

    #[test]
    fn unknown_entities_survive_instead_of_failing_the_parse() {
        let doc = parse("<p>hard&nbsp;space</p>").expect("parse");
        assert_eq!(doc.root.text_content(), "hard\u{a0}space");
    }
}
