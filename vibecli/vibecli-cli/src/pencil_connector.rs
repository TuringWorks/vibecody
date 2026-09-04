//! Pencil integration — Evolus Pencil .ep format + TuringWorks Pencil MCP bridge.
//!
//! Evolus Pencil (.ep files) are ZIP archives containing XML shape definitions.
//! The TuringWorks Pencil MCP server provides read/write access to .pen design files.
//! This module handles both, plus in-house wireframe generation.
//!
//! The document schema this module writes and reads back is the one
//! `skills/pencil-wireframe.md` documents: `<Document> → <Page> → <Shape>`,
//! stored as `content.xml` inside the `.ep` ZIP. Everything a template produces
//! round-trips through [`parse_ep_xml`] and is checked for XML well-formedness
//! in the tests — an export an editor cannot open is not an export.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::design_providers::{
    DesignComponent, DesignError, DesignFile, DesignFrame, DesignToken, DesignTokenType,
    ProviderKind,
};

// ─── Pencil shape types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PencilShapeKind {
    Rectangle,
    Ellipse,
    Text,
    Line,
    Arrow,
    Image,
    Button,
    Input,
    Checkbox,
    RadioButton,
    Dropdown,
    TextArea,
    Table,
    Browser,
    Mobile,
    Container,
    Custom,
}

impl PencilShapeKind {
    /// The `type="…"` attribute this kind is written as.
    ///
    /// Explicit, not `Debug`-derived: `RadioButton` debug-prints as
    /// `radiobutton`, which [`Self::from_tag`] does not recognise, so a radio
    /// button silently came back from its own XML as a rectangle.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Text => "text",
            Self::Line => "line",
            Self::Arrow => "arrow",
            Self::Image => "image",
            Self::Button => "button",
            Self::Input => "input",
            Self::Checkbox => "checkbox",
            Self::RadioButton => "radio",
            Self::Dropdown => "dropdown",
            Self::TextArea => "textarea",
            Self::Table => "table",
            Self::Browser => "browser",
            Self::Mobile => "mobile",
            Self::Container => "container",
            Self::Custom => "custom",
        }
    }

    /// Inverse of [`Self::tag`], accepting the aliases other tools emit.
    pub fn from_tag(tag: &str) -> Self {
        match tag.trim().to_lowercase().as_str() {
            "ellipse" | "circle" => Self::Ellipse,
            "text" | "label" => Self::Text,
            "line" => Self::Line,
            "arrow" => Self::Arrow,
            "image" | "img" => Self::Image,
            "button" => Self::Button,
            "input" | "textbox" => Self::Input,
            "checkbox" => Self::Checkbox,
            "radio" | "radiobutton" => Self::RadioButton,
            "dropdown" | "select" => Self::Dropdown,
            "textarea" => Self::TextArea,
            "table" => Self::Table,
            "browser" => Self::Browser,
            "mobile" => Self::Mobile,
            "container" | "group" => Self::Container,
            "custom" => Self::Custom,
            _ => Self::Rectangle,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PencilShape {
    pub id: String,
    pub kind: PencilShapeKind,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub label: String,
    pub style: PencilStyle,
    pub children: Vec<PencilShape>,
}

impl PencilShape {
    /// Every shape in the tree, parents before children.
    pub fn flatten(&self) -> Vec<&PencilShape> {
        std::iter::once(self)
            .chain(self.children.iter().flat_map(|c| c.flatten()))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PencilStyle {
    pub fill_color: Option<String>,
    pub stroke_color: Option<String>,
    pub stroke_width: Option<f64>,
    pub font_size: Option<f64>,
    pub font_weight: Option<String>,
    pub opacity: Option<f64>,
    pub border_radius: Option<f64>,
    pub text_align: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PencilPage {
    pub id: String,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub background: Option<String>,
    pub shapes: Vec<PencilShape>,
}

impl PencilPage {
    pub fn new(name: &str, width: f64, height: f64) -> Self {
        Self {
            id: next_id("page"),
            name: name.to_string(),
            width,
            height,
            background: None,
            shapes: Vec::new(),
        }
    }

    pub fn add_shape(&mut self, shape: PencilShape) {
        self.shapes.push(shape);
    }

    /// Shape count including nested children — what the panel reports per page.
    pub fn shape_count(&self) -> usize {
        self.shapes.iter().map(|s| s.flatten().len()).sum()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PencilDocument {
    pub id: String,
    pub name: String,
    pub pages: Vec<PencilPage>,
    pub metadata: HashMap<String, String>,
}

impl PencilDocument {
    pub fn new(name: &str) -> Self {
        Self {
            id: next_id("doc"),
            name: name.to_string(),
            pages: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn add_page(&mut self, page: PencilPage) {
        self.pages.push(page);
    }

    /// Serialize to Pencil EP XML format (the .ep ZIP inner content.xml)
    pub fn to_ep_xml(&self) -> String {
        let pages_xml: String = self
            .pages
            .iter()
            .map(page_to_xml)
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="http://www.evolus.vn/Namespace/Pencil"
          xmlns:p="http://www.evolus.vn/Namespace/Pencil"
          id="{id}"
          name="{name}"
          version="3.1.0">
{pages_xml}
</Document>"#,
            id = xml_escape(&self.id),
            name = xml_escape(&self.name),
        )
    }

    /// Package the document as a `.ep` archive: a ZIP whose `content.xml` is
    /// [`Self::to_ep_xml`].
    ///
    /// The panel used to hand the raw XML to the browser under a `.ep`
    /// filename. `.ep` is a ZIP, so the downloaded file was not one and
    /// nothing that reads `.ep` could open it.
    pub fn to_ep_archive(&self) -> Result<Vec<u8>, DesignError> {
        use std::io::Write as _;
        let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
        {
            let mut zw = zip::ZipWriter::new(&mut cursor);
            let opts: zip::write::SimpleFileOptions = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zw.start_file("content.xml", opts)
                .map_err(|e| DesignError::new("EP_ZIP", &format!("start content.xml: {e}")))?;
            zw.write_all(self.to_ep_xml().as_bytes())
                .map_err(|e| DesignError::new("EP_ZIP", &format!("write content.xml: {e}")))?;
            zw.finish()
                .map_err(|e| DesignError::new("EP_ZIP", &format!("finalise archive: {e}")))?;
        }
        Ok(cursor.into_inner())
    }

    /// Render the document as one standalone HTML page — a section per Pencil
    /// page, an absolutely positioned block per shape.
    ///
    /// Deterministic on purpose: the panel offers HTML as an export format, and
    /// an export must not depend on a provider being selected or a network
    /// being reachable.
    pub fn to_html(&self) -> String {
        let sections: String = self
            .pages
            .iter()
            .map(page_to_html)
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
  :root {{ color-scheme: light; }}
  body {{ margin: 0; padding: 24px; background: #f4f5f7; color: #111827;
         font: 14px/1.5 system-ui, -apple-system, "Segoe UI", sans-serif; }}
  h1 {{ font-size: 20px; margin: 0 0 16px; }}
  h2 {{ font-size: 14px; font-weight: 600; margin: 24px 0 8px; color: #4b5563; }}
  .wf-page {{ position: relative; background: #ffffff; overflow: hidden;
              border: 1px solid #d1d5db; border-radius: 6px;
              box-shadow: 0 1px 3px rgba(0,0,0,.08); }}
  .wf-scroll {{ overflow-x: auto; }}
  .wf-shape {{ position: absolute; box-sizing: border-box; overflow: hidden;
               display: flex; align-items: center; padding: 0 6px; }}
</style>
</head>
<body>
<h1>{title}</h1>
{sections}
</body>
</html>"#,
            title = html_escape(&self.name),
        )
    }

    /// Convert to a DesignFile for provider-agnostic usage
    pub fn to_design_file(&self) -> DesignFile {
        let frames: Vec<DesignFrame> = self
            .pages
            .iter()
            .map(|p| DesignFrame {
                id: p.id.clone(),
                name: p.name.clone(),
                width: p.width as u32,
                height: p.height as u32,
                thumbnail_url: None,
            })
            .collect();

        let components = extract_components_from_doc(self);
        let tokens = extract_tokens_from_doc(self);

        DesignFile {
            id: self.id.clone(),
            name: self.name.clone(),
            provider: ProviderKind::Pencil,
            last_modified: None,
            frames,
            components,
            tokens,
        }
    }
}

fn page_to_xml(page: &PencilPage) -> String {
    let shapes_xml: String = page
        .shapes
        .iter()
        .map(|s| shape_to_xml(s, 0))
        .collect::<Vec<_>>()
        .join("\n");
    let bg = page
        .background
        .as_deref()
        .map(|b| format!(" background=\"{}\"", xml_escape(b)))
        .unwrap_or_default();
    // A plain `format!`, not a raw string: the raw-string version of this
    // wrote a literal backslash-n into every document, so the shapes ended up
    // as one run-on line of text content instead of child elements.
    format!(
        "  <Page id=\"{id}\" name=\"{name}\" width=\"{w}\" height=\"{h}\"{bg}>\n{shapes_xml}\n  </Page>",
        id = xml_escape(&page.id),
        name = xml_escape(&page.name),
        w = num(page.width),
        h = num(page.height),
    )
}

fn shape_to_xml(shape: &PencilShape, depth: usize) -> String {
    let indent = "  ".repeat(depth + 2);
    let head = format!(
        "{indent}<Shape id=\"{id}\" type=\"{kind}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" label=\"{label}\"{style}",
        id = xml_escape(&shape.id),
        kind = shape.kind.tag(),
        x = num(shape.x),
        y = num(shape.y),
        w = num(shape.width),
        h = num(shape.height),
        label = xml_escape(&shape.label),
        style = style_to_attrs(&shape.style),
    );
    if shape.children.is_empty() {
        format!("{head} />")
    } else {
        let children: String = shape
            .children
            .iter()
            .map(|c| shape_to_xml(c, depth + 1))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{head}>\n{children}\n{indent}</Shape>")
    }
}

fn style_to_attrs(style: &PencilStyle) -> String {
    let mut attrs = String::new();
    if let Some(c) = &style.fill_color {
        attrs.push_str(&format!(" fill=\"{}\"", xml_escape(c)));
    }
    if let Some(c) = &style.stroke_color {
        attrs.push_str(&format!(" stroke=\"{}\"", xml_escape(c)));
    }
    if let Some(w) = style.stroke_width {
        attrs.push_str(&format!(" strokeWidth=\"{}\"", num(w)));
    }
    if let Some(fs) = style.font_size {
        attrs.push_str(&format!(" fontSize=\"{}\"", num(fs)));
    }
    if let Some(fw) = &style.font_weight {
        attrs.push_str(&format!(" fontWeight=\"{}\"", xml_escape(fw)));
    }
    if let Some(o) = style.opacity {
        attrs.push_str(&format!(" opacity=\"{}\"", num(o)));
    }
    if let Some(r) = style.border_radius {
        attrs.push_str(&format!(" borderRadius=\"{}\"", num(r)));
    }
    if let Some(a) = &style.text_align {
        attrs.push_str(&format!(" textAlign=\"{}\"", xml_escape(a)));
    }
    attrs
}

// ─── HTML rendering ───────────────────────────────────────────────────────────

fn page_to_html(page: &PencilPage) -> String {
    let shapes: String = page
        .shapes
        .iter()
        .map(shape_to_html)
        .collect::<Vec<_>>()
        .join("\n");
    let bg = page
        .background
        .as_deref()
        .map(|b| format!("background:{};", css_value(b)))
        .unwrap_or_default();
    format!(
        "<h2>{name} — {w}×{h}</h2>\n<div class=\"wf-scroll\"><div class=\"wf-page\" style=\"width:{w}px;height:{h}px;{bg}\">\n{shapes}\n</div></div>",
        name = html_escape(&page.name),
        w = num(page.width),
        h = num(page.height),
    )
}

fn shape_to_html(shape: &PencilShape) -> String {
    let s = &shape.style;
    let mut css = format!(
        "left:{x}px;top:{y}px;width:{w}px;height:{h}px;",
        x = num(shape.x),
        y = num(shape.y),
        w = num(shape.width),
        h = num(shape.height),
    );
    match &s.fill_color {
        Some(f) => css.push_str(&format!("background:{};", css_value(f))),
        // A text label with no fill would otherwise inherit nothing and read
        // as an invisible box; give the structural kinds a hairline instead.
        None if !matches!(shape.kind, PencilShapeKind::Text) => {
            css.push_str("border:1px dashed #cbd5e1;")
        }
        None => {}
    }
    if let Some(c) = &s.stroke_color {
        let w = s.stroke_width.unwrap_or(1.0);
        css.push_str(&format!("border:{}px solid {};", num(w), css_value(c)));
    }
    if let Some(r) = s.border_radius {
        css.push_str(&format!("border-radius:{}px;", num(r)));
    }
    if let Some(f) = s.font_size {
        css.push_str(&format!("font-size:{}px;", num(f)));
    }
    if let Some(fw) = &s.font_weight {
        css.push_str(&format!("font-weight:{};", css_value(fw)));
    }
    if let Some(o) = s.opacity {
        css.push_str(&format!("opacity:{};", num(o)));
    }
    if let Some(a) = &s.text_align {
        css.push_str(&format!("justify-content:{};", css_justify(a)));
    }
    if matches!(shape.kind, PencilShapeKind::Button) {
        css.push_str("color:#ffffff;font-weight:600;justify-content:center;");
    }
    let children: String = shape
        .children
        .iter()
        .map(shape_to_html)
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "<div class=\"wf-shape\" data-kind=\"{kind}\" data-id=\"{id}\" style=\"{css}\">{label}{children}</div>",
        kind = shape.kind.tag(),
        id = html_escape(&shape.id),
        label = html_escape(&shape.label),
    )
}

/// Colours and font weights reach CSS from a document that may not be ours.
/// Anything with a quote, semicolon, brace or angle bracket in it could close
/// the declaration and open a new one, so it does not get to be a value.
fn css_value(raw: &str) -> String {
    let safe = raw
        .chars()
        .filter(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '#' | '%' | '.' | ',' | '(' | ')' | ' ' | '-')
        })
        .collect::<String>();
    let trimmed = safe.trim();
    if trimmed.is_empty() {
        "transparent".to_string()
    } else {
        trimmed.to_string()
    }
}

fn css_justify(align: &str) -> &'static str {
    match align.trim().to_lowercase().as_str() {
        "center" => "center",
        "right" | "end" => "flex-end",
        _ => "flex-start",
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ─── EP file parsing ──────────────────────────────────────────────────────────

/// Parse a Pencil EP XML string (inner content.xml from .ep ZIP) into a PencilDocument.
/// Uses lightweight text extraction — no full XML parser required.
pub fn parse_ep_xml(xml: &str) -> Result<PencilDocument, DesignError> {
    if xml.trim().is_empty() {
        return Err(DesignError::new("EMPTY_EP", "Pencil EP XML is empty"));
    }

    let doc_block = top_level_blocks(xml, "Document")
        .into_iter()
        .next()
        .ok_or_else(|| {
            DesignError::new(
                "NO_DOCUMENT",
                "no <Document> element — this does not look like Pencil EP XML",
            )
        })?;
    let head = open_tag_of(&doc_block);

    let mut doc = PencilDocument {
        id: extract_attr_val(head, "id").unwrap_or_else(|| next_id("doc")),
        name: extract_attr_val(head, "name").unwrap_or_else(|| "Untitled".to_string()),
        pages: Vec::new(),
        metadata: HashMap::new(),
    };

    for page_chunk in top_level_blocks(&doc_block, "Page") {
        let phead = open_tag_of(&page_chunk);
        let mut page = PencilPage {
            id: extract_attr_val(phead, "id").unwrap_or_else(|| next_id("page")),
            name: extract_attr_val(phead, "name").unwrap_or_else(|| "Page".to_string()),
            width: attr_f64(phead, "width").unwrap_or(1280.0),
            height: attr_f64(phead, "height").unwrap_or(800.0),
            background: extract_attr_val(phead, "background"),
            shapes: Vec::new(),
        };
        if let Some(inner) = inner_of(&page_chunk, "Page") {
            page.shapes = top_level_blocks(inner, "Shape")
                .iter()
                .filter_map(|c| parse_shape_xml(c))
                .collect();
        }
        doc.pages.push(page);
    }

    Ok(doc)
}

fn parse_shape_xml(xml: &str) -> Option<PencilShape> {
    let head = open_tag_of(xml);
    let id = extract_attr_val(head, "id")?;
    let kind = PencilShapeKind::from_tag(&extract_attr_val(head, "type").unwrap_or_default());
    let inner = inner_of(xml, "Shape");
    // `label` is an attribute now. Documents written before that carried the
    // label as the element's text, so fall back to it rather than dropping it.
    let label = extract_attr_val(head, "label")
        .or_else(|| inner.map(leading_text).filter(|t| !t.is_empty()))
        .unwrap_or_default();
    let children = inner
        .map(|i| {
            top_level_blocks(i, "Shape")
                .iter()
                .filter_map(|c| parse_shape_xml(c))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some(PencilShape {
        id,
        kind,
        x: attr_f64(head, "x").unwrap_or(0.0),
        y: attr_f64(head, "y").unwrap_or(0.0),
        width: attr_f64(head, "width").unwrap_or(100.0),
        height: attr_f64(head, "height").unwrap_or(40.0),
        label,
        style: parse_style_attrs(head),
        children,
    })
}

fn parse_style_attrs(head: &str) -> PencilStyle {
    PencilStyle {
        fill_color: extract_attr_val(head, "fill"),
        stroke_color: extract_attr_val(head, "stroke"),
        stroke_width: attr_f64(head, "strokeWidth"),
        font_size: attr_f64(head, "fontSize"),
        font_weight: extract_attr_val(head, "fontWeight"),
        opacity: attr_f64(head, "opacity"),
        border_radius: attr_f64(head, "borderRadius"),
        text_align: extract_attr_val(head, "textAlign"),
    }
}

// ─── Wireframe templates ──────────────────────────────────────────────────────

/// Every template id the panel, the skill and `generate_template` agree on.
pub const TEMPLATE_IDS: [&str; 6] = [
    "landing_page",
    "dashboard",
    "mobile_app",
    "login_form",
    "settings_page",
    "data_table",
];

/// Build the document for a template id.
///
/// `sections` is the user's comma-separated list; each template that uses it
/// falls back to its own defaults when the list is empty. An unknown id is an
/// error — the panel previously fell through to a "dashboard with your
/// sections as pages", which quietly produced a different wireframe from the
/// one that was clicked.
pub fn generate_template(
    template_id: &str,
    title: &str,
    sections: &[String],
) -> Result<PencilDocument, DesignError> {
    let given: Vec<&str> = sections
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let or_default = |fallback: &[&'static str]| -> Vec<&str> {
        if given.is_empty() {
            fallback.to_vec()
        } else {
            given.clone()
        }
    };

    match template_id {
        "landing_page" => Ok(template_landing_page(title)),
        "dashboard" => Ok(template_dashboard(
            title,
            &or_default(&["Overview", "Analytics", "Settings", "Users"]),
        )),
        "mobile_app" => Ok(template_mobile_app(
            title,
            &or_default(&["Home", "Search", "Profile"]),
        )),
        "login_form" => Ok(template_login_form(title)),
        "settings_page" => Ok(template_settings_page(
            title,
            &or_default(&["Account", "Notifications", "Privacy", "Advanced"]),
        )),
        "data_table" => Ok(template_data_table(
            title,
            &or_default(&["Name", "Status", "Owner", "Updated"]),
        )),
        other => Err(DesignError::new(
            "UNKNOWN_TEMPLATE",
            &format!(
                "unknown wireframe template `{other}` — expected one of {}",
                TEMPLATE_IDS.join(", ")
            ),
        )),
    }
}

/// Generate a landing page wireframe
pub fn template_landing_page(title: &str) -> PencilDocument {
    let mut doc = PencilDocument::new(title);
    let mut page = PencilPage::new("Landing Page", 1440.0, 900.0);

    // Nav bar
    page.add_shape(make_rect(
        "nav",
        0.0,
        0.0,
        1440.0,
        64.0,
        Some("#f0f0f0"),
        "Navigation",
    ));
    page.add_shape(make_rect(
        "logo",
        20.0,
        12.0,
        120.0,
        40.0,
        Some("#cccccc"),
        "Logo",
    ));
    page.add_shape(make_text(
        "nav-links",
        800.0,
        22.0,
        400.0,
        24.0,
        "Home  About  Features  Pricing  Contact",
    ));
    page.add_shape(make_button(
        "nav-cta",
        1300.0,
        16.0,
        120.0,
        32.0,
        "Get Started",
    ));

    // Hero
    page.add_shape(make_rect(
        "hero",
        0.0,
        64.0,
        1440.0,
        500.0,
        Some("#e8f4fd"),
        "",
    ));
    page.add_shape(make_text(
        "hero-title",
        200.0,
        180.0,
        700.0,
        60.0,
        "Your Amazing Product Headline",
    ));
    page.add_shape(make_text(
        "hero-sub",
        200.0,
        260.0,
        600.0,
        30.0,
        "A compelling subtitle that explains the value",
    ));
    page.add_shape(make_button(
        "hero-cta-1",
        200.0,
        330.0,
        160.0,
        50.0,
        "Start Free Trial",
    ));
    page.add_shape(make_button(
        "hero-cta-2",
        380.0,
        330.0,
        140.0,
        50.0,
        "Learn More",
    ));
    page.add_shape(make_rect(
        "hero-img",
        900.0,
        100.0,
        460.0,
        380.0,
        Some("#cccccc"),
        "Product Screenshot",
    ));

    // Features section
    page.add_shape(make_text(
        "feat-title",
        540.0,
        600.0,
        360.0,
        40.0,
        "Key Features",
    ));
    for (i, feature) in ["Feature One", "Feature Two", "Feature Three"]
        .iter()
        .enumerate()
    {
        let x = 160.0 + (i as f64) * 380.0;
        page.add_shape(make_rect(
            &format!("feat-icon-{}", i),
            x + 100.0,
            660.0,
            60.0,
            60.0,
            Some("#3b82f6"),
            "",
        ));
        page.add_shape(make_text(
            &format!("feat-name-{}", i),
            x,
            740.0,
            260.0,
            28.0,
            feature,
        ));
        page.add_shape(make_text(
            &format!("feat-desc-{}", i),
            x,
            775.0,
            260.0,
            40.0,
            "Feature description goes here with key benefits",
        ));
    }

    doc.add_page(page);
    doc
}

/// Generate a dashboard wireframe
pub fn template_dashboard(title: &str, sections: &[&str]) -> PencilDocument {
    let mut doc = PencilDocument::new(title);
    let mut page = PencilPage::new("Dashboard", 1440.0, 900.0);

    // Sidebar
    page.add_shape(make_rect(
        "sidebar",
        0.0,
        0.0,
        240.0,
        900.0,
        Some("#1e293b"),
        "",
    ));
    page.add_shape(make_text(
        "sidebar-logo",
        20.0,
        20.0,
        200.0,
        40.0,
        "VibeCody Dashboard",
    ));
    for (i, section) in sections.iter().enumerate() {
        let sy = 80.0 + (i as f64) * 48.0;
        page.add_shape(make_rect(
            &format!("nav-item-{}", i),
            8.0,
            sy,
            224.0,
            40.0,
            Some("#334155"),
            section,
        ));
    }

    // Header
    page.add_shape(make_rect(
        "header",
        240.0,
        0.0,
        1200.0,
        60.0,
        Some("#f8fafc"),
        "",
    ));
    page.add_shape(make_text(
        "header-title",
        260.0,
        15.0,
        400.0,
        30.0,
        "Dashboard Overview",
    ));

    // Stats row
    for (i, label) in ["Total Users", "Active Today", "Revenue", "Conversion"]
        .iter()
        .enumerate()
    {
        let x = 260.0 + (i as f64) * 290.0;
        page.add_shape(make_rect(
            &format!("stat-{}", i),
            x,
            80.0,
            270.0,
            100.0,
            Some("#ffffff"),
            "",
        ));
        page.add_shape(make_text(
            &format!("stat-val-{}", i),
            x + 20.0,
            100.0,
            200.0,
            36.0,
            "—",
        ));
        page.add_shape(make_text(
            &format!("stat-lbl-{}", i),
            x + 20.0,
            145.0,
            200.0,
            20.0,
            label,
        ));
    }

    // Main chart
    page.add_shape(make_rect(
        "chart-area",
        260.0,
        200.0,
        780.0,
        380.0,
        Some("#f8fafc"),
        "Chart Placeholder",
    ));
    // Right panel
    page.add_shape(make_rect(
        "right-panel",
        1060.0,
        200.0,
        360.0,
        680.0,
        Some("#f8fafc"),
        "Recent Activity",
    ));

    doc.add_page(page);
    doc
}

/// Generate a mobile app wireframe
pub fn template_mobile_app(title: &str, screens: &[&str]) -> PencilDocument {
    let mut doc = PencilDocument::new(title);
    for (i, screen_name) in screens.iter().enumerate() {
        let mut page = PencilPage::new(screen_name, 390.0, 844.0);
        let offset_x = 0.0;

        // Status bar
        page.add_shape(make_rect(
            &format!("status-{}", i),
            offset_x,
            0.0,
            390.0,
            44.0,
            Some("#f0f0f0"),
            "Status Bar",
        ));
        // Navigation bar
        page.add_shape(make_rect(
            &format!("navbar-{}", i),
            offset_x,
            44.0,
            390.0,
            56.0,
            Some("#ffffff"),
            "",
        ));
        page.add_shape(make_text(
            &format!("navbar-title-{}", i),
            offset_x + 130.0,
            58.0,
            130.0,
            28.0,
            screen_name,
        ));
        // Content area
        page.add_shape(make_rect(
            &format!("content-{}", i),
            offset_x,
            100.0,
            390.0,
            688.0,
            Some("#f8f9fa"),
            "Content",
        ));
        // Tab bar
        page.add_shape(make_rect(
            &format!("tabbar-{}", i),
            offset_x,
            795.0,
            390.0,
            49.0,
            Some("#ffffff"),
            "Tab Bar",
        ));
        for (ti, tab) in ["Home", "Search", "Profile", "Settings"].iter().enumerate() {
            let tx = offset_x + (ti as f64) * 97.5 + 20.0;
            page.add_shape(make_text(
                &format!("tab-{}-{}", i, ti),
                tx,
                808.0,
                60.0,
                24.0,
                tab,
            ));
        }
        doc.add_page(page);
    }
    doc
}

/// Generate a login form wireframe — email/password with social auth.
pub fn template_login_form(title: &str) -> PencilDocument {
    let mut doc = PencilDocument::new(title);
    let mut page = PencilPage::new("Login", 1440.0, 900.0);
    let (cx, cy, cw) = (510.0, 140.0, 420.0);

    page.add_shape(make_rect(
        "backdrop",
        0.0,
        0.0,
        1440.0,
        900.0,
        Some("#f1f5f9"),
        "",
    ));
    page.add_shape(make_rect(
        "card",
        cx,
        cy,
        cw,
        620.0,
        Some("#ffffff"),
        "",
    ));
    page.add_shape(make_rect(
        "brand-mark",
        cx + 180.0,
        cy + 40.0,
        60.0,
        60.0,
        Some("#3b82f6"),
        "",
    ));
    page.add_shape(make_text(
        "form-title",
        cx + 40.0,
        cy + 120.0,
        340.0,
        34.0,
        "Sign in to your account",
    ));
    page.add_shape(make_text(
        "form-sub",
        cx + 40.0,
        cy + 158.0,
        340.0,
        24.0,
        "Welcome back — enter your details",
    ));

    page.add_shape(make_text(
        "email-label",
        cx + 40.0,
        cy + 208.0,
        340.0,
        20.0,
        "Email",
    ));
    page.add_shape(make_input(
        "email-input",
        cx + 40.0,
        cy + 232.0,
        340.0,
        44.0,
        "you@example.com",
    ));
    page.add_shape(make_text(
        "password-label",
        cx + 40.0,
        cy + 292.0,
        340.0,
        20.0,
        "Password",
    ));
    page.add_shape(make_input(
        "password-input",
        cx + 40.0,
        cy + 316.0,
        340.0,
        44.0,
        "••••••••",
    ));

    page.add_shape(make_checkbox(
        "remember-me",
        cx + 40.0,
        cy + 376.0,
        160.0,
        20.0,
        "Remember me",
    ));
    page.add_shape(make_text(
        "forgot-link",
        cx + 240.0,
        cy + 376.0,
        140.0,
        20.0,
        "Forgot password?",
    ));

    page.add_shape(make_button(
        "submit",
        cx + 40.0,
        cy + 414.0,
        340.0,
        46.0,
        "Sign in",
    ));
    page.add_shape(make_text(
        "divider",
        cx + 40.0,
        cy + 476.0,
        340.0,
        20.0,
        "— or continue with —",
    ));
    for (i, provider) in ["Google", "GitHub"].iter().enumerate() {
        page.add_shape(make_rect(
            &format!("social-{}", i),
            cx + 40.0 + (i as f64) * 176.0,
            cy + 506.0,
            164.0,
            44.0,
            Some("#ffffff"),
            provider,
        ));
    }
    page.add_shape(make_text(
        "signup-link",
        cx + 40.0,
        cy + 566.0,
        340.0,
        20.0,
        "No account? Create one",
    ));

    doc.add_page(page);
    doc
}

/// Generate a settings page wireframe — grouped settings with toggles.
pub fn template_settings_page(title: &str, groups: &[&str]) -> PencilDocument {
    let mut doc = PencilDocument::new(title);
    let mut page = PencilPage::new("Settings", 1440.0, 900.0);

    page.add_shape(make_rect(
        "settings-nav",
        0.0,
        0.0,
        260.0,
        900.0,
        Some("#f8fafc"),
        "",
    ));
    page.add_shape(make_text(
        "settings-nav-title",
        24.0,
        24.0,
        200.0,
        30.0,
        "Settings",
    ));
    for (i, group) in groups.iter().enumerate() {
        page.add_shape(make_rect(
            &format!("settings-nav-{}", i),
            12.0,
            72.0 + (i as f64) * 44.0,
            236.0,
            36.0,
            Some(if i == 0 { "#e2e8f0" } else { "#f8fafc" }),
            group,
        ));
    }

    page.add_shape(make_text(
        "settings-title",
        300.0,
        32.0,
        500.0,
        34.0,
        groups.first().copied().unwrap_or("Settings"),
    ));
    page.add_shape(make_text(
        "settings-sub",
        300.0,
        70.0,
        700.0,
        22.0,
        "Manage how this workspace behaves for everyone on it",
    ));

    // Each group is a card of three rows; a row is a label, a description and
    // a toggle rendered as a checkbox shape.
    let rows = ["Enabled", "Email alerts", "Weekly digest"];
    for (gi, group) in groups.iter().enumerate() {
        let gy = 120.0 + (gi as f64) * 180.0;
        page.add_shape(make_rect(
            &format!("group-{}", gi),
            300.0,
            gy,
            1100.0,
            160.0,
            Some("#ffffff"),
            "",
        ));
        page.add_shape(make_text(
            &format!("group-title-{}", gi),
            324.0,
            gy + 16.0,
            400.0,
            26.0,
            group,
        ));
        for (ri, row) in rows.iter().enumerate() {
            let ry = gy + 54.0 + (ri as f64) * 34.0;
            page.add_shape(make_text(
                &format!("row-label-{}-{}", gi, ri),
                324.0,
                ry,
                420.0,
                22.0,
                row,
            ));
            page.add_shape(make_checkbox(
                &format!("row-toggle-{}-{}", gi, ri),
                1330.0,
                ry,
                46.0,
                24.0,
                "",
            ));
        }
    }

    page.add_shape(make_button(
        "settings-save",
        1240.0,
        830.0,
        160.0,
        44.0,
        "Save changes",
    ));

    doc.add_page(page);
    doc
}

/// Generate a data table wireframe — filter bar, sortable columns, pagination.
pub fn template_data_table(title: &str, columns: &[&str]) -> PencilDocument {
    let mut doc = PencilDocument::new(title);
    let mut page = PencilPage::new("Data Table", 1440.0, 900.0);
    const ROWS: usize = 8;
    let col_w = (1360.0 / columns.len().max(1) as f64).floor();

    page.add_shape(make_text("table-title", 40.0, 28.0, 500.0, 34.0, title));
    page.add_shape(make_input(
        "table-search",
        40.0,
        80.0,
        360.0,
        40.0,
        "Search…",
    ));
    page.add_shape(make_dropdown(
        "table-filter",
        416.0,
        80.0,
        200.0,
        40.0,
        "All statuses",
    ));
    page.add_shape(make_button(
        "table-new",
        1240.0,
        80.0,
        160.0,
        40.0,
        "New record",
    ));

    // Header row: one cell per column, each a sortable header.
    page.add_shape(make_rect(
        "table-head",
        40.0,
        140.0,
        1360.0,
        44.0,
        Some("#f1f5f9"),
        "",
    ));
    for (ci, col) in columns.iter().enumerate() {
        page.add_shape(make_text(
            &format!("col-{}", ci),
            52.0 + (ci as f64) * col_w,
            152.0,
            col_w - 24.0,
            22.0,
            &format!("{col} ▲"),
        ));
    }

    // Body rows, alternating fill so the grid reads as a table.
    for ri in 0..ROWS {
        let ry = 184.0 + (ri as f64) * 52.0;
        page.add_shape(make_rect(
            &format!("row-{}", ri),
            40.0,
            ry,
            1360.0,
            52.0,
            Some(if ri % 2 == 0 { "#ffffff" } else { "#f8fafc" }),
            "",
        ));
        for ci in 0..columns.len() {
            page.add_shape(make_text(
                &format!("cell-{}-{}", ri, ci),
                52.0 + (ci as f64) * col_w,
                ry + 16.0,
                col_w - 24.0,
                22.0,
                "—",
            ));
        }
    }

    let footer_y = 184.0 + (ROWS as f64) * 52.0 + 16.0;
    page.add_shape(make_text(
        "table-count",
        40.0,
        footer_y + 10.0,
        320.0,
        22.0,
        &format!("Showing {ROWS} of — records"),
    ));
    for (i, label) in ["Previous", "1", "2", "3", "Next"].iter().enumerate() {
        page.add_shape(make_rect(
            &format!("page-btn-{}", i),
            1120.0 + (i as f64) * 58.0,
            footer_y,
            54.0,
            36.0,
            Some("#ffffff"),
            label,
        ));
    }

    doc.add_page(page);
    doc
}

fn make_rect(
    id: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: Option<&str>,
    label: &str,
) -> PencilShape {
    PencilShape {
        id: id.to_string(),
        kind: PencilShapeKind::Rectangle,
        x,
        y,
        width: w,
        height: h,
        label: label.to_string(),
        style: PencilStyle {
            fill_color: fill.map(|s| s.to_string()),
            ..Default::default()
        },
        children: Vec::new(),
    }
}

fn make_text(id: &str, x: f64, y: f64, w: f64, h: f64, label: &str) -> PencilShape {
    PencilShape {
        id: id.to_string(),
        kind: PencilShapeKind::Text,
        x,
        y,
        width: w,
        height: h,
        label: label.to_string(),
        style: PencilStyle::default(),
        children: Vec::new(),
    }
}

fn make_button(id: &str, x: f64, y: f64, w: f64, h: f64, label: &str) -> PencilShape {
    PencilShape {
        id: id.to_string(),
        kind: PencilShapeKind::Button,
        x,
        y,
        width: w,
        height: h,
        label: label.to_string(),
        style: PencilStyle {
            fill_color: Some("#3b82f6".to_string()),
            border_radius: Some(6.0),
            ..Default::default()
        },
        children: Vec::new(),
    }
}

fn make_input(id: &str, x: f64, y: f64, w: f64, h: f64, placeholder: &str) -> PencilShape {
    PencilShape {
        id: id.to_string(),
        kind: PencilShapeKind::Input,
        x,
        y,
        width: w,
        height: h,
        label: placeholder.to_string(),
        style: PencilStyle {
            fill_color: Some("#ffffff".to_string()),
            stroke_color: Some("#cbd5e1".to_string()),
            stroke_width: Some(1.0),
            border_radius: Some(6.0),
            ..Default::default()
        },
        children: Vec::new(),
    }
}

fn make_dropdown(id: &str, x: f64, y: f64, w: f64, h: f64, label: &str) -> PencilShape {
    PencilShape {
        kind: PencilShapeKind::Dropdown,
        ..make_input(id, x, y, w, h, label)
    }
}

fn make_checkbox(id: &str, x: f64, y: f64, w: f64, h: f64, label: &str) -> PencilShape {
    PencilShape {
        id: id.to_string(),
        kind: PencilShapeKind::Checkbox,
        x,
        y,
        width: w,
        height: h,
        label: label.to_string(),
        style: PencilStyle {
            fill_color: Some("#ffffff".to_string()),
            stroke_color: Some("#94a3b8".to_string()),
            stroke_width: Some(1.0),
            border_radius: Some(4.0),
            ..Default::default()
        },
        children: Vec::new(),
    }
}

// ─── TuringWorks Pencil MCP bridge ───────────────────────────────────────────

/// MCP tool descriptor for the TuringWorks Pencil server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PencilMcpOp {
    pub tool: String,
    pub params: serde_json::Value,
}

impl PencilMcpOp {
    /// Get editor state (active .pen file)
    pub fn get_editor_state() -> Self {
        Self {
            tool: "get_editor_state".to_string(),
            params: serde_json::json!({ "include_schema": false }),
        }
    }

    /// Open a .pen file
    pub fn open_document(path: &str) -> Self {
        Self {
            tool: "open_document".to_string(),
            params: serde_json::json!({ "filePathOrNew": path }),
        }
    }

    /// Batch read nodes
    pub fn batch_get(patterns: &[&str]) -> Self {
        Self {
            tool: "batch_get".to_string(),
            params: serde_json::json!({ "patterns": patterns, "nodeIds": [] }),
        }
    }

    /// Batch design operations
    pub fn batch_design(operations: &str) -> Self {
        Self {
            tool: "batch_design".to_string(),
            params: serde_json::json!({ "operations": operations }),
        }
    }

    /// Get design guidelines
    pub fn get_guidelines(category: Option<&str>) -> Self {
        let mut p = serde_json::json!({});
        if let Some(c) = category {
            p["category"] = serde_json::Value::String(c.to_string());
        }
        Self {
            tool: "get_guidelines".to_string(),
            params: p,
        }
    }

    /// Capture the current canvas.
    pub fn get_screenshot() -> Self {
        Self {
            tool: "get_screenshot".to_string(),
            params: serde_json::json!({}),
        }
    }

    /// Build the op the panel names, so an unimplemented button is an error
    /// rather than a fabricated `status: "ok"`.
    pub fn for_operation(operation: &str, argument: Option<&str>) -> Result<Self, DesignError> {
        match operation {
            "get_editor_state" => Ok(Self::get_editor_state()),
            "open_document" => argument
                .map(|p| p.trim())
                .filter(|p| !p.is_empty())
                .map(Self::open_document)
                .ok_or_else(|| {
                    DesignError::new("MISSING_PATH", "open_document needs a .pen file path")
                }),
            "batch_get" => {
                let pattern = argument.map(|p| p.trim()).filter(|p| !p.is_empty());
                Ok(Self::batch_get(&[pattern.unwrap_or("**")]))
            }
            "batch_design" => argument
                .map(|o| o.trim())
                .filter(|o| !o.is_empty())
                .map(Self::batch_design)
                .ok_or_else(|| {
                    DesignError::new("MISSING_OPS", "batch_design needs an operations script")
                }),
            "get_guidelines" => Ok(Self::get_guidelines(
                argument.map(|c| c.trim()).filter(|c| !c.is_empty()),
            )),
            "get_screenshot" => Ok(Self::get_screenshot()),
            other => Err(DesignError::new(
                "UNKNOWN_MCP_OP",
                &format!("unknown Pencil MCP operation `{other}`"),
            )),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

// ─── Design file extraction helpers ──────────────────────────────────────────

fn extract_components_from_doc(doc: &PencilDocument) -> Vec<DesignComponent> {
    let mut components = Vec::new();
    for page in &doc.pages {
        for shape in page.shapes.iter().flat_map(|s| s.flatten()) {
            if matches!(
                shape.kind,
                PencilShapeKind::Button
                    | PencilShapeKind::Input
                    | PencilShapeKind::Dropdown
                    | PencilShapeKind::TextArea
                    | PencilShapeKind::Table
            ) {
                components.push(DesignComponent {
                    id: shape.id.clone(),
                    name: if shape.label.is_empty() {
                        format!("{:?}", shape.kind)
                    } else {
                        shape.label.clone()
                    },
                    description: format!("{:?} at ({}, {})", shape.kind, shape.x, shape.y),
                    category: "ui".to_string(),
                    props: {
                        let mut m = HashMap::new();
                        m.insert("width".to_string(), shape.width.to_string());
                        m.insert("height".to_string(), shape.height.to_string());
                        m
                    },
                });
            }
        }
    }
    components
}

fn extract_tokens_from_doc(doc: &PencilDocument) -> Vec<DesignToken> {
    let mut seen = std::collections::HashSet::new();
    let mut tokens = Vec::new();
    for page in &doc.pages {
        for shape in page.shapes.iter().flat_map(|s| s.flatten()) {
            if let Some(fill) = &shape.style.fill_color {
                if !fill.is_empty() && seen.insert(fill.clone()) {
                    tokens.push(DesignToken {
                        name: format!("color-{}", fill.trim_start_matches('#')),
                        token_type: DesignTokenType::Color,
                        value: fill.clone(),
                        description: None,
                        provider: ProviderKind::Pencil,
                    });
                }
            }
        }
    }
    tokens
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn xml_unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Format a coordinate for an attribute: `1440`, not `1440` with a trailing
/// `.0` or an exponent. Non-finite values are never written — an `inf` width
/// is not a measurement.
fn num(v: f64) -> String {
    if !v.is_finite() {
        return "0".to_string();
    }
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// The opening tag of an element block, `<Shape …>` or `<Shape … />`.
///
/// Attributes are read from this, never from the whole block: a parent whose
/// child also carries `id="…"` would otherwise be at the mercy of which one
/// `find` reached first.
fn open_tag_of(block: &str) -> &str {
    match block.find('>') {
        Some(i) => &block[..=i],
        None => block,
    }
}

/// The content between an element's opening and closing tag, or `None` when it
/// is self-closing.
fn inner_of<'a>(block: &'a str, tag: &str) -> Option<&'a str> {
    let head_end = block.find('>')? + 1;
    if block[..head_end].trim_end().ends_with("/>") {
        return None;
    }
    let close_at = block.rfind(&format!("</{tag}"))?;
    (close_at >= head_end).then(|| &block[head_end..close_at])
}

/// Text before the first child element — where pre-`label` documents kept the
/// shape's label.
fn leading_text(inner: &str) -> String {
    let text = match inner.find('<') {
        Some(i) => &inner[..i],
        None => inner,
    };
    xml_unescape(text.trim())
}

fn extract_attr_val(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!("{attr}=\"");
    let bytes = xml.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = xml[from..].find(&pattern) {
        let at = from + rel;
        let val_start = at + pattern.len();
        // `id="` must not match the tail of `grid="`; require a tag or
        // whitespace boundary before the attribute name.
        let boundary = at == 0 || bytes[at - 1].is_ascii_whitespace() || bytes[at - 1] == b'<';
        if boundary {
            let rest = &xml[val_start..];
            let end = rest.find('"')?;
            return Some(xml_unescape(&rest[..end]));
        }
        from = val_start;
    }
    None
}

fn attr_f64(xml: &str, attr: &str) -> Option<f64> {
    extract_attr_val(xml, attr)?
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

/// The outermost `<tag …>…</tag>` (and `<tag … />`) blocks in `xml`.
///
/// Depth-aware: a `<Shape>` nested in a `<Shape>` stays inside its parent's
/// block. The previous `find("</Shape>")` cut the parent short at its first
/// child's closing tag, and a self-closing shape (no closing tag at all) made
/// it swallow the rest of the document.
fn top_level_blocks(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}");
    let bytes = xml.as_bytes();
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut i = 0usize;

    while i < xml.len() {
        if !xml.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let rest = &xml[i..];
        if rest.starts_with(&close) && ends_tag_name(bytes, i + close.len()) {
            let end = rest.find('>').map(|e| i + e + 1).unwrap_or(xml.len());
            if depth > 0 {
                depth -= 1;
                if depth == 0 {
                    out.push(xml[start..end].to_string());
                }
            }
            i = end;
            continue;
        }
        if rest.starts_with(&open) && ends_tag_name(bytes, i + open.len()) {
            let end = rest.find('>').map(|e| i + e + 1).unwrap_or(xml.len());
            let self_closing = xml[i..end].trim_end().ends_with("/>");
            if depth == 0 {
                start = i;
            }
            if self_closing {
                if depth == 0 {
                    out.push(xml[i..end].to_string());
                }
            } else {
                depth += 1;
            }
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// True when byte `pos` ends a tag name — so `<Page` does not match `<Pages`.
fn ends_tag_name(bytes: &[u8], pos: usize) -> bool {
    bytes
        .get(pos)
        .map(|c| c.is_ascii_whitespace() || *c == b'>' || *c == b'/')
        .unwrap_or(false)
}

/// A short, process-unique id.
///
/// The old version was the wall clock in microseconds, so every shape built in
/// the same tick shared an id — and two documents generated within the same
/// microsecond collided outright. A monotonic counter cannot.
fn next_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{:x}-{:x}", t.as_secs(), n)
}

/// Kept for callers outside this module that want an opaque id.
pub fn uuid_short() -> String {
    next_id("id")
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Fails the test with the reader's own message rather than a bare
    /// `assert!(is_ok())`, so a malformed export says *where* it broke.
    fn assert_well_formed(xml: &str) {
        let mut reader = quick_xml::Reader::from_str(xml);
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => buf.clear(),
                Err(e) => panic!("EP XML is not well-formed at {}: {e}", reader.buffer_position()),
            }
        }
    }

    #[test]
    fn document_to_ep_xml_has_pages() {
        let mut doc = PencilDocument::new("TestDoc");
        let page = PencilPage::new("Page1", 1280.0, 800.0);
        doc.add_page(page);
        let xml = doc.to_ep_xml();
        assert!(xml.contains("<?xml"));
        assert!(xml.contains("Page1"));
        assert!(xml.contains("TestDoc"));
    }

    #[test]
    fn ep_xml_puts_shapes_on_their_own_lines_not_a_literal_backslash_n() {
        // The regression this guards: `page_to_xml` used a raw string whose
        // `\n` stayed a literal backslash-n, so every shape landed inside the
        // page as run-on text.
        let doc = template_landing_page("Lit");
        let xml = doc.to_ep_xml();
        assert!(!xml.contains(r"\n"), "literal \\n leaked into the EP XML");
        assert!(xml.contains("\n    <Shape "));
    }

    #[test]
    fn every_template_emits_well_formed_xml() {
        let sections = vec!["Alpha".to_string(), "Beta & Co".to_string()];
        for id in TEMPLATE_IDS {
            let doc = generate_template(id, "Quotes \" & <angles>", &sections)
                .unwrap_or_else(|e| panic!("{id}: {e}"));
            assert!(!doc.pages.is_empty(), "{id} produced no pages");
            assert!(
                doc.pages.iter().all(|p| p.shape_count() > 0),
                "{id} produced an empty page"
            );
            assert_well_formed(&doc.to_ep_xml());
        }
    }

    #[test]
    fn templates_round_trip_through_the_parser() {
        for id in TEMPLATE_IDS {
            let doc = generate_template(id, "Round Trip", &[]).unwrap();
            let back = parse_ep_xml(&doc.to_ep_xml()).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_eq!(back.name, doc.name, "{id}: name");
            assert_eq!(back.pages.len(), doc.pages.len(), "{id}: page count");
            for (a, b) in doc.pages.iter().zip(back.pages.iter()) {
                assert_eq!(a.name, b.name, "{id}: page name");
                assert_eq!(a.shape_count(), b.shape_count(), "{id}: shape count");
                assert_eq!(a.width, b.width, "{id}: page width");
            }
        }
    }

    #[test]
    fn round_trip_preserves_shape_kind_label_and_style() {
        let doc = generate_template("login_form", "Sign in", &[]).unwrap();
        let back = parse_ep_xml(&doc.to_ep_xml()).unwrap();
        let submit = back.pages[0]
            .shapes
            .iter()
            .find(|s| s.id == "submit")
            .expect("submit button survived the round trip");
        assert_eq!(submit.kind, PencilShapeKind::Button);
        assert_eq!(submit.label, "Sign in");
        assert_eq!(submit.style.fill_color.as_deref(), Some("#3b82f6"));
        assert_eq!(submit.style.border_radius, Some(6.0));
    }

    #[test]
    fn radio_button_kind_survives_its_own_serialisation() {
        // `format!("{:?}")` produced `radiobutton`, which the parser did not
        // know, so the shape came back as a rectangle.
        assert_eq!(
            PencilShapeKind::from_tag(PencilShapeKind::RadioButton.tag()),
            PencilShapeKind::RadioButton
        );
        assert_eq!(
            PencilShapeKind::from_tag(PencilShapeKind::TextArea.tag()),
            PencilShapeKind::TextArea
        );
    }

    #[test]
    fn ep_archive_is_a_zip_containing_content_xml() {
        let doc = generate_template("dashboard", "Ops", &[]).unwrap();
        let bytes = doc.to_ep_archive().expect("archive");
        assert_eq!(&bytes[..2], b"PK", "a .ep must be a ZIP");
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("readable zip");
        let mut file = zip.by_name("content.xml").expect("content.xml entry");
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut file, &mut xml).unwrap();
        assert_eq!(xml, doc.to_ep_xml());
        assert_well_formed(&xml);
    }

    #[test]
    fn html_export_renders_every_shape() {
        let doc = generate_template("data_table", "Records", &[]).unwrap();
        let html = doc.to_html();
        assert!(html.starts_with("<!DOCTYPE html>"));
        let expected: usize = doc.pages.iter().map(|p| p.shape_count()).sum();
        assert_eq!(html.matches("class=\"wf-shape\"").count(), expected);
        assert!(html.contains("Records"));
    }

    #[test]
    fn html_export_escapes_labels_and_refuses_css_injection() {
        let mut doc = PencilDocument::new("X");
        let mut page = PencilPage::new("P", 100.0, 100.0);
        page.add_shape(PencilShape {
            id: "s".into(),
            kind: PencilShapeKind::Rectangle,
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
            label: "<script>alert(1)</script>".into(),
            style: PencilStyle {
                fill_color: Some("red;} body{display:none".into()),
                ..Default::default()
            },
            children: Vec::new(),
        });
        doc.add_page(page);
        let html = doc.to_html();
        assert!(!html.contains("<script>"));
        assert!(!html.contains("body{display:none"));
    }

    #[test]
    fn unknown_template_is_an_error_not_a_different_wireframe() {
        let err = generate_template("nope", "T", &[]).unwrap_err();
        assert_eq!(err.code, "UNKNOWN_TEMPLATE");
    }

    #[test]
    fn template_landing_page_has_nav() {
        let doc = template_landing_page("MyProduct");
        assert_eq!(doc.pages.len(), 1);
        let page = &doc.pages[0];
        assert!(page.shapes.iter().any(|s| s.id == "nav"));
        assert!(page.shapes.iter().any(|s| s.id == "hero"));
    }

    #[test]
    fn template_dashboard_has_sidebar() {
        let sections = vec!["Overview", "Analytics", "Settings"];
        let doc = template_dashboard("App Dashboard", &sections);
        assert_eq!(doc.pages.len(), 1);
        let page = &doc.pages[0];
        assert!(page.shapes.iter().any(|s| s.id == "sidebar"));
    }

    #[test]
    fn template_mobile_app_creates_screens() {
        let screens = ["Home", "Profile", "Settings"];
        let doc = template_mobile_app("MyApp", &screens);
        assert_eq!(doc.pages.len(), 3);
    }

    #[test]
    fn template_login_form_has_credentials_and_social_auth() {
        let doc = template_login_form("Sign in");
        let page = &doc.pages[0];
        for id in ["email-input", "password-input", "submit", "social-0"] {
            assert!(page.shapes.iter().any(|s| s.id == id), "missing {id}");
        }
    }

    #[test]
    fn template_settings_page_has_a_toggle_per_group_row() {
        let doc = template_settings_page("Prefs", &["Account", "Privacy"]);
        let page = &doc.pages[0];
        let toggles = page
            .shapes
            .iter()
            .filter(|s| s.kind == PencilShapeKind::Checkbox)
            .count();
        assert_eq!(toggles, 6, "2 groups × 3 rows");
    }

    #[test]
    fn template_data_table_has_a_header_cell_per_column() {
        let cols = ["Name", "Status", "Owner"];
        let doc = template_data_table("Users", &cols);
        let page = &doc.pages[0];
        for (i, c) in cols.iter().enumerate() {
            let header = page
                .shapes
                .iter()
                .find(|s| s.id == format!("col-{i}"))
                .unwrap_or_else(|| panic!("missing col-{i}"));
            assert!(header.label.starts_with(c));
        }
        assert!(page.shapes.iter().any(|s| s.id == "cell-7-2"));
    }

    #[test]
    fn generate_template_falls_back_to_defaults_on_empty_sections() {
        let blank = vec!["  ".to_string(), String::new()];
        let doc = generate_template("mobile_app", "App", &blank).unwrap();
        assert_eq!(doc.pages.len(), 3, "default screens");
    }

    #[test]
    fn to_design_file_maps_frames() {
        let mut doc = PencilDocument::new("D");
        doc.add_page(PencilPage::new("P1", 390.0, 844.0));
        doc.add_page(PencilPage::new("P2", 1440.0, 900.0));
        let df = doc.to_design_file();
        assert_eq!(df.frames.len(), 2);
        assert_eq!(df.provider, ProviderKind::Pencil);
    }

    #[test]
    fn parse_ep_xml_empty_returns_err() {
        assert!(parse_ep_xml("").is_err());
    }

    #[test]
    fn parse_ep_xml_without_a_document_element_is_an_error() {
        assert_eq!(
            parse_ep_xml("<html><body>not a wireframe</body></html>")
                .unwrap_err()
                .code,
            "NO_DOCUMENT"
        );
    }

    #[test]
    fn parse_ep_xml_basic() {
        let xml = r#"<?xml version="1.0"?>
<Document name="MyDoc" id="doc1">
  <Page name="Page1" id="p1" width="1280" height="800">
    <Shape id="s1" type="rectangle" x="10" y="20" width="100" height="40"></Shape>
  </Page>
</Document>"#;
        let doc = parse_ep_xml(xml).unwrap();
        assert_eq!(doc.name, "MyDoc");
        assert_eq!(doc.pages.len(), 1);
        assert_eq!(doc.pages[0].shapes.len(), 1);
    }

    #[test]
    fn parse_ep_xml_keeps_nested_shapes_as_children() {
        let xml = r#"<Document name="D" id="d">
  <Page name="P" id="p" width="100" height="100">
    <Shape id="outer" type="container" x="0" y="0" width="90" height="90">
      <Shape id="inner" type="button" x="5" y="5" width="20" height="10" label="Go" />
    </Shape>
    <Shape id="after" type="text" x="0" y="95" width="10" height="4" label="Tail" />
  </Page>
</Document>"#;
        let doc = parse_ep_xml(xml).unwrap();
        let page = &doc.pages[0];
        assert_eq!(page.shapes.len(), 2, "outer and after are both top level");
        assert_eq!(page.shapes[0].children.len(), 1);
        assert_eq!(page.shapes[0].children[0].id, "inner");
        assert_eq!(page.shapes[0].children[0].label, "Go");
        assert_eq!(page.shapes[1].id, "after");
        assert_eq!(page.shape_count(), 3);
    }

    #[test]
    fn parse_ep_xml_reads_legacy_text_labels() {
        let xml = r#"<Document name="D" id="d"><Page name="P" id="p" width="10" height="10">
<Shape id="s1" type="text" x="0" y="0" width="5" height="5">Hello &amp; welcome</Shape>
</Page></Document>"#;
        let doc = parse_ep_xml(xml).unwrap();
        assert_eq!(doc.pages[0].shapes[0].label, "Hello & welcome");
    }

    #[test]
    fn attribute_lookup_respects_name_boundaries() {
        // `id="` used to match the tail of `grid="`.
        let head = r#"<Shape grid="8" id="real" data-width="9" width="120">"#;
        assert_eq!(extract_attr_val(head, "id").as_deref(), Some("real"));
        assert_eq!(attr_f64(head, "width"), Some(120.0));
    }

    #[test]
    fn page_split_does_not_match_a_pages_wrapper() {
        let xml = r#"<Document name="D" id="d"><Pages><Page id="p" name="Only" width="10" height="10"></Page></Pages></Document>"#;
        let doc = parse_ep_xml(xml).unwrap();
        assert_eq!(doc.pages.len(), 1);
        assert_eq!(doc.pages[0].name, "Only");
    }

    #[test]
    fn pencil_mcp_op_serialises() {
        let op = PencilMcpOp::get_editor_state();
        let json = op.to_json();
        assert!(json.contains("get_editor_state"));
    }

    #[test]
    fn mcp_op_for_operation_requires_the_arguments_it_needs() {
        assert!(PencilMcpOp::for_operation("open_document", None).is_err());
        assert!(PencilMcpOp::for_operation("open_document", Some("  ")).is_err());
        assert!(PencilMcpOp::for_operation("open_document", Some("/a.pen")).is_ok());
        assert!(PencilMcpOp::for_operation("batch_get", None).is_ok());
        assert_eq!(
            PencilMcpOp::for_operation("nope", None).unwrap_err().code,
            "UNKNOWN_MCP_OP"
        );
    }

    #[test]
    fn make_button_has_fill_color() {
        let btn = make_button("b1", 10.0, 20.0, 120.0, 40.0, "Submit");
        assert_eq!(btn.kind, PencilShapeKind::Button);
        assert!(btn.style.fill_color.is_some());
    }

    #[test]
    fn generated_ids_are_unique_within_a_tick() {
        let ids: std::collections::HashSet<String> = (0..1000).map(|_| next_id("x")).collect();
        assert_eq!(ids.len(), 1000);
    }
}
