//! Pencil integration — Evolus Pencil .ep format + TuringWorks Pencil MCP bridge.
//!
//! Evolus Pencil (.ep files) are ZIP archives containing XML shape definitions.
//! The TuringWorks Pencil MCP server provides read/write access to .pen design files.
//! This module handles both, plus in-house wireframe generation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::design_providers::{DesignComponent, DesignError, DesignFile, DesignFrame, DesignToken, DesignTokenType, ProviderKind};

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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
            id: uuid_short(),
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
            id: uuid_short(),
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
        let pages_xml: String = self.pages.iter().map(|p| page_to_xml(p)).collect::<Vec<_>>().join("\n");
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="http://www.evolus.vn/Namespace/Pencil"
          xmlns:p="http://www.evolus.vn/Namespace/Pencil"
          id="{}"
          name="{}"
          version="3.1.0">
{}
</Document>"#,
            self.id, xml_escape(&self.name), pages_xml
        )
    }

    /// Convert to a DesignFile for provider-agnostic usage
    pub fn to_design_file(&self) -> DesignFile {
        let frames: Vec<DesignFrame> = self.pages.iter().map(|p| DesignFrame {
            id: p.id.clone(),
            name: p.name.clone(),
            width: p.width as u32,
            height: p.height as u32,
            thumbnail_url: None,
        }).collect();

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
    let shapes_xml: String = page.shapes.iter().map(|s| shape_to_xml(s, 0)).collect::<Vec<_>>().join("\n");
    let bg = page.background.as_deref().map(|b| format!(" background=\"{}\"", b)).unwrap_or_default();
    format!(
        r#"  <Page id="{}" name="{}" width="{}" height="{}"{}>\n{}\n  </Page>"#,
        page.id, xml_escape(&page.name), page.width, page.height, bg, shapes_xml
    )
}

fn shape_to_xml(shape: &PencilShape, depth: usize) -> String {
    let indent = "  ".repeat(depth + 2);
    let style_str = style_to_attrs(&shape.style);
    let children_xml = if shape.children.is_empty() {
        String::new()
    } else {
        let c: String = shape.children.iter().map(|c| shape_to_xml(c, depth + 1)).collect::<Vec<_>>().join("\n");
        format!("\n{}\n{}", c, indent)
    };
    format!(
        r#"{}<Shape id="{}" type="{}" x="{}" y="{}" width="{}" height="{}"{}>{}</Shape>"#,
        indent, shape.id, format!("{:?}", shape.kind).to_lowercase(),
        shape.x, shape.y, shape.width, shape.height,
        style_str,
        if shape.label.is_empty() && children_xml.is_empty() {
            String::new()
        } else {
            format!("{}{}", xml_escape(&shape.label), children_xml)
        }
    )
}

fn style_to_attrs(style: &PencilStyle) -> String {
    let mut attrs = String::new();
    if let Some(c) = &style.fill_color { attrs.push_str(&format!(" fill=\"{}\"", c)); }
    if let Some(c) = &style.stroke_color { attrs.push_str(&format!(" stroke=\"{}\"", c)); }
    if let Some(w) = style.stroke_width { attrs.push_str(&format!(" strokeWidth=\"{}\"", w)); }
    if let Some(fs) = style.font_size { attrs.push_str(&format!(" fontSize=\"{}\"", fs)); }
    if let Some(r) = style.border_radius { attrs.push_str(&format!(" borderRadius=\"{}\"", r)); }
    attrs
}

// ─── EP file parsing ──────────────────────────────────────────────────────────

/// Parse a Pencil EP XML string (inner content.xml from .ep ZIP) into a PencilDocument.
/// Uses lightweight text extraction — no full XML parser required.
pub fn parse_ep_xml(xml: &str) -> Result<PencilDocument, DesignError> {
    if xml.trim().is_empty() {
        return Err(DesignError::new("EMPTY_EP", "Pencil EP XML is empty"));
    }

    let name = extract_attr_val(xml, "name").unwrap_or_else(|| "Untitled".to_string());
    let id = extract_attr_val(xml, "id").unwrap_or_else(uuid_short);
    let mut doc = PencilDocument { id, name, pages: Vec::new(), metadata: HashMap::new() };

    for page_chunk in split_tag_blocks(xml, "<Page", "</Page>") {
        let pname = extract_attr_val(&page_chunk, "name").unwrap_or_else(|| "Page".to_string());
        let pid = extract_attr_val(&page_chunk, "id").unwrap_or_else(uuid_short);
        let pw: f64 = extract_attr_val(&page_chunk, "width")
            .and_then(|v| v.parse().ok()).unwrap_or(1280.0);
        let ph: f64 = extract_attr_val(&page_chunk, "height")
            .and_then(|v| v.parse().ok()).unwrap_or(800.0);
        let mut page = PencilPage { id: pid, name: pname, width: pw, height: ph, background: None, shapes: Vec::new() };

        for shape_chunk in split_tag_blocks(&page_chunk, "<Shape", "</Shape>") {
            if let Some(shape) = parse_shape_xml(&shape_chunk) {
                page.shapes.push(shape);
            }
        }
        doc.pages.push(page);
    }

    Ok(doc)
}

fn parse_shape_xml(xml: &str) -> Option<PencilShape> {
    let id = extract_attr_val(xml, "id")?;
    let kind_str = extract_attr_val(xml, "type").unwrap_or_else(|| "rectangle".to_string());
    let kind = match kind_str.to_lowercase().as_str() {
        "ellipse" | "circle" => PencilShapeKind::Ellipse,
        "text" | "label" => PencilShapeKind::Text,
        "line" => PencilShapeKind::Line,
        "arrow" => PencilShapeKind::Arrow,
        "image" | "img" => PencilShapeKind::Image,
        "button" => PencilShapeKind::Button,
        "input" | "textbox" => PencilShapeKind::Input,
        "checkbox" => PencilShapeKind::Checkbox,
        "radio" => PencilShapeKind::RadioButton,
        "dropdown" | "select" => PencilShapeKind::Dropdown,
        "textarea" => PencilShapeKind::TextArea,
        "table" => PencilShapeKind::Table,
        "browser" => PencilShapeKind::Browser,
        "mobile" => PencilShapeKind::Mobile,
        "container" | "group" => PencilShapeKind::Container,
        _ => PencilShapeKind::Rectangle,
    };
    let x: f64 = extract_attr_val(xml, "x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let y: f64 = extract_attr_val(xml, "y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let w: f64 = extract_attr_val(xml, "width").and_then(|v| v.parse().ok()).unwrap_or(100.0);
    let h: f64 = extract_attr_val(xml, "height").and_then(|v| v.parse().ok()).unwrap_or(40.0);
    Some(PencilShape { id, kind, x, y, width: w, height: h, label: String::new(), style: PencilStyle::default(), children: Vec::new() })
}

// ─── Wireframe templates ──────────────────────────────────────────────────────

/// Generate a landing page wireframe
pub fn template_landing_page(title: &str) -> PencilDocument {
    let mut doc = PencilDocument::new(title);
    let mut page = PencilPage::new("Landing Page", 1440.0, 900.0);

    // Nav bar
    page.add_shape(make_rect("nav", 0.0, 0.0, 1440.0, 64.0, Some("#f0f0f0"), "Navigation"));
    page.add_shape(make_rect("logo", 20.0, 12.0, 120.0, 40.0, Some("#cccccc"), "Logo"));
    page.add_shape(make_text("nav-links", 800.0, 22.0, 400.0, 24.0, "Home  About  Features  Pricing  Contact"));
    page.add_shape(make_button("nav-cta", 1300.0, 16.0, 120.0, 32.0, "Get Started"));

    // Hero
    page.add_shape(make_rect("hero", 0.0, 64.0, 1440.0, 500.0, Some("#e8f4fd"), ""));
    page.add_shape(make_text("hero-title", 200.0, 180.0, 700.0, 60.0, "Your Amazing Product Headline"));
    page.add_shape(make_text("hero-sub", 200.0, 260.0, 600.0, 30.0, "A compelling subtitle that explains the value"));
    page.add_shape(make_button("hero-cta-1", 200.0, 330.0, 160.0, 50.0, "Start Free Trial"));
    page.add_shape(make_button("hero-cta-2", 380.0, 330.0, 140.0, 50.0, "Learn More"));
    page.add_shape(make_rect("hero-img", 900.0, 100.0, 460.0, 380.0, Some("#cccccc"), "Product Screenshot"));

    // Features section
    page.add_shape(make_text("feat-title", 540.0, 600.0, 360.0, 40.0, "Key Features"));
    for (i, feature) in ["Feature One", "Feature Two", "Feature Three"].iter().enumerate() {
        let x = 160.0 + (i as f64) * 380.0;
        page.add_shape(make_rect(&format!("feat-icon-{}", i), x + 100.0, 660.0, 60.0, 60.0, Some("#3b82f6"), ""));
        page.add_shape(make_text(&format!("feat-name-{}", i), x, 740.0, 260.0, 28.0, feature));
        page.add_shape(make_text(&format!("feat-desc-{}", i), x, 775.0, 260.0, 40.0, "Feature description goes here with key benefits"));
    }

    doc.add_page(page);
    doc
}

/// Generate a dashboard wireframe
pub fn template_dashboard(title: &str, sections: &[&str]) -> PencilDocument {
    let mut doc = PencilDocument::new(title);
    let mut page = PencilPage::new("Dashboard", 1440.0, 900.0);

    // Sidebar
    page.add_shape(make_rect("sidebar", 0.0, 0.0, 240.0, 900.0, Some("#1e293b"), ""));
    page.add_shape(make_text("sidebar-logo", 20.0, 20.0, 200.0, 40.0, "VibeCody Dashboard"));
    for (i, section) in sections.iter().enumerate() {
        let sy = 80.0 + (i as f64) * 48.0;
        page.add_shape(make_rect(&format!("nav-item-{}", i), 8.0, sy, 224.0, 40.0, Some("#334155"), section));
    }

    // Header
    page.add_shape(make_rect("header", 240.0, 0.0, 1200.0, 60.0, Some("#f8fafc"), ""));
    page.add_shape(make_text("header-title", 260.0, 15.0, 400.0, 30.0, "Dashboard Overview"));

    // Stats row
    for (i, label) in ["Total Users", "Active Today", "Revenue", "Conversion"].iter().enumerate() {
        let x = 260.0 + (i as f64) * 290.0;
        page.add_shape(make_rect(&format!("stat-{}", i), x, 80.0, 270.0, 100.0, Some("#ffffff"), ""));
        page.add_shape(make_text(&format!("stat-val-{}", i), x + 20.0, 100.0, 200.0, 36.0, "—"));
        page.add_shape(make_text(&format!("stat-lbl-{}", i), x + 20.0, 145.0, 200.0, 20.0, label));
    }

    // Main chart
    page.add_shape(make_rect("chart-area", 260.0, 200.0, 780.0, 380.0, Some("#f8fafc"), "Chart Placeholder"));
    // Right panel
    page.add_shape(make_rect("right-panel", 1060.0, 200.0, 360.0, 680.0, Some("#f8fafc"), "Recent Activity"));

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
        page.add_shape(make_rect(&format!("status-{}", i), offset_x, 0.0, 390.0, 44.0, Some("#f0f0f0"), "Status Bar"));
        // Navigation bar
        page.add_shape(make_rect(&format!("navbar-{}", i), offset_x, 44.0, 390.0, 56.0, Some("#ffffff"), ""));
        page.add_shape(make_text(&format!("navbar-title-{}", i), offset_x + 130.0, 58.0, 130.0, 28.0, screen_name));
        // Content area
        page.add_shape(make_rect(&format!("content-{}", i), offset_x, 100.0, 390.0, 688.0, Some("#f8f9fa"), "Content"));
        // Tab bar
        page.add_shape(make_rect(&format!("tabbar-{}", i), offset_x, 795.0, 390.0, 49.0, Some("#ffffff"), "Tab Bar"));
        for (ti, tab) in ["Home", "Search", "Profile", "Settings"].iter().enumerate() {
            let tx = offset_x + (ti as f64) * 97.5 + 20.0;
            page.add_shape(make_text(&format!("tab-{}-{}", i, ti), tx, 808.0, 60.0, 24.0, tab));
        }
        doc.add_page(page);
    }
    doc
}

fn make_rect(id: &str, x: f64, y: f64, w: f64, h: f64, fill: Option<&str>, label: &str) -> PencilShape {
    PencilShape {
        id: id.to_string(),
        kind: PencilShapeKind::Rectangle,
        x, y, width: w, height: h,
        label: label.to_string(),
        style: PencilStyle { fill_color: fill.map(|s| s.to_string()), ..Default::default() },
        children: Vec::new(),
    }
}

fn make_text(id: &str, x: f64, y: f64, w: f64, h: f64, label: &str) -> PencilShape {
    PencilShape {
        id: id.to_string(), kind: PencilShapeKind::Text,
        x, y, width: w, height: h, label: label.to_string(),
        style: PencilStyle::default(), children: Vec::new(),
    }
}

fn make_button(id: &str, x: f64, y: f64, w: f64, h: f64, label: &str) -> PencilShape {
    PencilShape {
        id: id.to_string(), kind: PencilShapeKind::Button,
        x, y, width: w, height: h, label: label.to_string(),
        style: PencilStyle { fill_color: Some("#3b82f6".to_string()), border_radius: Some(6.0), ..Default::default() },
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
        Self { tool: "get_editor_state".to_string(), params: serde_json::json!({ "include_schema": false }) }
    }

    /// Open a .pen file
    pub fn open_document(path: &str) -> Self {
        Self { tool: "open_document".to_string(), params: serde_json::json!({ "filePathOrNew": path }) }
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
        Self { tool: "batch_design".to_string(), params: serde_json::json!({ "operations": operations }) }
    }

    /// Get design guidelines
    pub fn get_guidelines(category: Option<&str>) -> Self {
        let mut p = serde_json::json!({});
        if let Some(c) = category { p["category"] = serde_json::Value::String(c.to_string()); }
        Self { tool: "get_guidelines".to_string(), params: p }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

// ─── Design file extraction helpers ──────────────────────────────────────────

fn extract_components_from_doc(doc: &PencilDocument) -> Vec<DesignComponent> {
    let mut components = Vec::new();
    for page in &doc.pages {
        for shape in &page.shapes {
            if matches!(shape.kind, PencilShapeKind::Button | PencilShapeKind::Input |
                PencilShapeKind::Dropdown | PencilShapeKind::TextArea | PencilShapeKind::Table)
            {
                components.push(DesignComponent {
                    id: shape.id.clone(),
                    name: if shape.label.is_empty() { format!("{:?}", shape.kind) } else { shape.label.clone() },
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
        for shape in &page.shapes {
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
    s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;")
}

fn extract_attr_val(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    let start = xml.find(&pattern)? + pattern.len();
    let rest = &xml[start..];
    let end = rest.find('"')?;
    Some(rest[..end].replace("&amp;", "&").replace("&quot;", "\""))
}

fn split_tag_blocks(xml: &str, open_tag: &str, close_tag: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut remaining = xml;
    while let Some(start) = remaining.find(open_tag) {
        let from = &remaining[start..];
        let end = from.find(close_tag).map(|e| e + close_tag.len()).unwrap_or(from.len());
        results.push(from[..end].to_string());
        remaining = &from[end..];
    }
    results
}

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{:x}{:04x}", t.as_secs(), t.subsec_micros() & 0xffff)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
    fn pencil_mcp_op_serialises() {
        let op = PencilMcpOp::get_editor_state();
        let json = op.to_json();
        assert!(json.contains("get_editor_state"));
    }

    #[test]
    fn make_button_has_fill_color() {
        let btn = make_button("b1", 10.0, 20.0, 120.0, 40.0, "Submit");
        assert_eq!(btn.kind, PencilShapeKind::Button);
        assert!(btn.style.fill_color.is_some());
    }
}
