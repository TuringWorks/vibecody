//! Sketch canvas — draw shapes, recognize UI components, infer layouts,
//! and generate framework-specific code from hand-drawn wireframes.
//!
//! Gap 20 — Converts freehand sketches into React, HTML, or SwiftUI components.

use serde::{Deserialize, Serialize};
/// Bounding box for positioned elements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BoundingBox {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    pub fn area(&self) -> f64 {
        self.width * self.height
    }

    pub fn center(&self) -> (f64, f64) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn overlaps(&self, other: &BoundingBox) -> bool {
        self.x < other.x + other.width
            && self.x + self.width > other.x
            && self.y < other.y + other.height
            && self.y + self.height > other.y
    }
}

/// A canvas drawing element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CanvasElement {
    Rectangle { x: f64, y: f64, width: f64, height: f64, fill: Option<String> },
    Circle { cx: f64, cy: f64, radius: f64, fill: Option<String> },
    Line { x1: f64, y1: f64, x2: f64, y2: f64 },
    Text { x: f64, y: f64, content: String, font_size: f64 },
    Arrow { x1: f64, y1: f64, x2: f64, y2: f64 },
    Freehand { points: Vec<(f64, f64)> },
}

impl CanvasElement {
    pub fn bounds(&self) -> BoundingBox {
        match self {
            Self::Rectangle { x, y, width, height, .. } => BoundingBox::new(*x, *y, *width, *height),
            Self::Circle { cx, cy, radius, .. } => BoundingBox::new(cx - radius, cy - radius, radius * 2.0, radius * 2.0),
            Self::Line { x1, y1, x2, y2 } | Self::Arrow { x1, y1, x2, y2 } => {
                let min_x = x1.min(*x2);
                let min_y = y1.min(*y2);
                BoundingBox::new(min_x, min_y, (x2 - x1).abs(), (y2 - y1).abs())
            }
            Self::Text { x, y, content, font_size } => {
                BoundingBox::new(*x, *y, content.len() as f64 * font_size * 0.6, *font_size)
            }
            Self::Freehand { points } => {
                if points.is_empty() {
                    return BoundingBox::new(0.0, 0.0, 0.0, 0.0);
                }
                let min_x = points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
                let max_x = points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
                let min_y = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
                let max_y = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
                BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y)
            }
        }
    }
}

/// Recognized UI component type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UiComponent {
    Button,
    Input,
    Card,
    List,
    Navbar,
    Sidebar,
    Modal,
    Table,
    Image,
    Divider,
}

impl UiComponent {
    pub fn name(&self) -> &str {
        match self {
            Self::Button => "Button",
            Self::Input => "Input",
            Self::Card => "Card",
            Self::List => "List",
            Self::Navbar => "Navbar",
            Self::Sidebar => "Sidebar",
            Self::Modal => "Modal",
            Self::Table => "Table",
            Self::Image => "Image",
            Self::Divider => "Divider",
        }
    }
}

/// A shape recognized as a UI component.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecognizedShape {
    pub element_index: usize,
    pub recognized_as: UiComponent,
    pub confidence: f64,
    pub bounds: BoundingBox,
}

/// Layout type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayoutType {
    Flex,
    Grid,
    Absolute,
}

/// Layout direction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayoutDirection {
    Row,
    Column,
    Wrap,
}

/// Inferred layout from recognized shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutInference {
    pub layout_type: LayoutType,
    pub direction: LayoutDirection,
    pub children: Vec<String>,
}

/// Target framework for code generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Framework {
    React,
    Html,
    SwiftUI,
}

impl Framework {
    pub fn name(&self) -> &str {
        match self {
            Self::React => "React",
            Self::Html => "HTML",
            Self::SwiftUI => "SwiftUI",
        }
    }
}

/// Generated code output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedCode {
    pub framework: Framework,
    pub code: String,
    pub imports: Vec<String>,
}

/// Code generation engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentGenerator {
    pub framework: Framework,
}

impl ComponentGenerator {
    pub fn new(framework: Framework) -> Self {
        Self { framework }
    }

    pub fn generate(&self, recognized: &[RecognizedShape], layout: &LayoutInference) -> GeneratedCode {
        match self.framework {
            Framework::React => self.generate_react(recognized, layout),
            Framework::Html => self.generate_html(recognized, layout),
            Framework::SwiftUI => self.generate_swiftui(recognized, layout),
        }
    }

    fn generate_react(&self, recognized: &[RecognizedShape], layout: &LayoutInference) -> GeneratedCode {
        let mut imports = vec!["import React from 'react';".to_string()];
        let direction = match layout.direction {
            LayoutDirection::Row => "row",
            LayoutDirection::Column => "column",
            LayoutDirection::Wrap => "row",
        };

        let mut children = String::new();
        for shape in recognized {
            let comp = match &shape.recognized_as {
                UiComponent::Button => "      <button className=\"btn\">Click me</button>".to_string(),
                UiComponent::Input => "      <input className=\"input\" placeholder=\"Type here...\" />".to_string(),
                UiComponent::Card => "      <div className=\"card\"><p>Card content</p></div>".to_string(),
                UiComponent::Navbar => "      <nav className=\"navbar\"><span>Logo</span></nav>".to_string(),
                UiComponent::Sidebar => "      <aside className=\"sidebar\">Sidebar</aside>".to_string(),
                UiComponent::Image => "      <img src=\"placeholder.png\" alt=\"image\" />".to_string(),
                UiComponent::Divider => "      <hr />".to_string(),
                _ => format!("      <div className=\"{}\">{}</div>", shape.recognized_as.name().to_lowercase(), shape.recognized_as.name()),
            };
            children.push_str(&comp);
            children.push('\n');
        }

        let code = format!(
            "export default function SketchComponent() {{\n  return (\n    <div style={{{{ display: 'flex', flexDirection: '{}' }}}}>\n{}\n    </div>\n  );\n}}",
            direction, children.trim_end()
        );
        imports.push("import './styles.css';".to_string());
        GeneratedCode { framework: Framework::React, code, imports }
    }

    fn generate_html(&self, recognized: &[RecognizedShape], layout: &LayoutInference) -> GeneratedCode {
        let direction = match layout.direction {
            LayoutDirection::Row => "row",
            LayoutDirection::Column => "column",
            LayoutDirection::Wrap => "row",
        };

        let mut children = String::new();
        for shape in recognized {
            let tag = match &shape.recognized_as {
                UiComponent::Button => "  <button>Click me</button>".to_string(),
                UiComponent::Input => "  <input placeholder=\"Type here...\" />".to_string(),
                UiComponent::Card => "  <div class=\"card\"><p>Card content</p></div>".to_string(),
                UiComponent::Divider => "  <hr />".to_string(),
                _ => format!("  <div class=\"{}\">{}</div>", shape.recognized_as.name().to_lowercase(), shape.recognized_as.name()),
            };
            children.push_str(&tag);
            children.push('\n');
        }

        let code = format!(
            "<div style=\"display:flex;flex-direction:{}\">\n{}</div>",
            direction, children
        );
        GeneratedCode { framework: Framework::Html, code, imports: Vec::new() }
    }

    fn generate_swiftui(&self, recognized: &[RecognizedShape], layout: &LayoutInference) -> GeneratedCode {
        let container = match layout.direction {
            LayoutDirection::Row => "HStack",
            LayoutDirection::Column => "VStack",
            LayoutDirection::Wrap => "LazyVGrid(columns: columns)",
        };

        let mut children = String::new();
        for shape in recognized {
            let view = match &shape.recognized_as {
                UiComponent::Button => "    Button(\"Click me\") {}".to_string(),
                UiComponent::Input => "    TextField(\"Type here...\", text: $text)".to_string(),
                UiComponent::Card => "    VStack { Text(\"Card content\") }.padding()".to_string(),
                UiComponent::Divider => "    Divider()".to_string(),
                UiComponent::Image => "    Image(\"placeholder\")".to_string(),
                _ => format!("    Text(\"{}\")", shape.recognized_as.name()),
            };
            children.push_str(&view);
            children.push('\n');
        }

        let code = format!(
            "struct SketchView: View {{\n  var body: some View {{\n    {} {{\n{}\n    }}\n  }}\n}}",
            container, children.trim_end()
        );
        GeneratedCode {
            framework: Framework::SwiftUI,
            code,
            imports: vec!["import SwiftUI".to_string()],
        }
    }
}

/// The main sketch canvas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SketchCanvas {
    pub elements: Vec<CanvasElement>,
    pub recognized: Vec<RecognizedShape>,
    pub layouts: Vec<LayoutInference>,
    pub config: CanvasConfig,
}

/// Canvas configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanvasConfig {
    pub width: f64,
    pub height: f64,
    pub confidence_threshold: f64,
    pub default_framework: Framework,
}

impl Default for CanvasConfig {
    fn default() -> Self {
        Self {
            width: 1024.0,
            height: 768.0,
            confidence_threshold: 0.5,
            default_framework: Framework::React,
        }
    }
}

impl SketchCanvas {
    pub fn new(config: CanvasConfig) -> Self {
        Self {
            elements: Vec::new(),
            recognized: Vec::new(),
            layouts: Vec::new(),
            config,
        }
    }

    /// Add an element to the canvas.
    pub fn add_element(&mut self, element: CanvasElement) {
        self.elements.push(element);
    }

    /// Recognize shapes as UI components (heuristic-based).
    pub fn recognize_shapes(&mut self) -> &[RecognizedShape] {
        self.recognized.clear();

        for (i, elem) in self.elements.iter().enumerate() {
            let bounds = elem.bounds();
            let (component, confidence) = match elem {
                CanvasElement::Rectangle { width, height, .. } => {
                    let aspect = width / height.max(0.001);
                    if *height < 60.0 && *width < 200.0 && aspect < 4.0 {
                        (UiComponent::Button, 0.85)
                    } else if *height < 50.0 && aspect > 2.0 {
                        (UiComponent::Input, 0.8)
                    } else if aspect > 4.0 && bounds.y < 80.0 {
                        (UiComponent::Navbar, 0.7)
                    } else if *width < 120.0 && *height > 300.0 {
                        (UiComponent::Sidebar, 0.65)
                    } else {
                        (UiComponent::Card, 0.6)
                    }
                }
                CanvasElement::Circle { radius, .. } => {
                    if *radius < 30.0 {
                        (UiComponent::Button, 0.5)
                    } else {
                        (UiComponent::Image, 0.55)
                    }
                }
                CanvasElement::Line { x1, y1, x2, y2 } => {
                    let dx = (x2 - x1).abs();
                    let dy = (y2 - y1).abs();
                    if dx > dy * 5.0 {
                        (UiComponent::Divider, 0.9)
                    } else {
                        (UiComponent::Divider, 0.4)
                    }
                }
                CanvasElement::Text { content, .. } => {
                    if content.to_lowercase().contains("button") || content.to_lowercase().contains("submit") {
                        (UiComponent::Button, 0.9)
                    } else if content.to_lowercase().contains("nav") || content.to_lowercase().contains("menu") {
                        (UiComponent::Navbar, 0.8)
                    } else {
                        (UiComponent::Card, 0.3)
                    }
                }
                CanvasElement::Arrow { .. } => {
                    (UiComponent::Divider, 0.3)
                }
                CanvasElement::Freehand { points } => {
                    if points.len() < 5 {
                        (UiComponent::Divider, 0.2)
                    } else {
                        let bb = elem.bounds();
                        let aspect = bb.width / bb.height.max(0.001);
                        if aspect > 0.7 && aspect < 1.5 {
                            (UiComponent::Button, 0.4)
                        } else {
                            (UiComponent::Card, 0.3)
                        }
                    }
                }
            };

            if confidence >= self.config.confidence_threshold {
                self.recognized.push(RecognizedShape {
                    element_index: i,
                    recognized_as: component,
                    confidence,
                    bounds,
                });
            }
        }

        &self.recognized
    }

    /// Infer layout from recognized shapes.
    pub fn infer_layout(&mut self) -> LayoutInference {
        if self.recognized.is_empty() {
            let layout = LayoutInference {
                layout_type: LayoutType::Flex,
                direction: LayoutDirection::Column,
                children: Vec::new(),
            };
            self.layouts.push(layout.clone());
            return layout;
        }

        // Determine if children are arranged horizontally or vertically
        let mut sorted = self.recognized.clone();
        sorted.sort_by(|a, b| a.bounds.y.partial_cmp(&b.bounds.y).unwrap_or(std::cmp::Ordering::Equal));

        let y_spread: f64 = if sorted.len() > 1 {
            sorted.last().expect("non-empty").bounds.y - sorted.first().expect("non-empty").bounds.y
        } else {
            0.0
        };

        let mut sorted_x = self.recognized.clone();
        sorted_x.sort_by(|a, b| a.bounds.x.partial_cmp(&b.bounds.x).unwrap_or(std::cmp::Ordering::Equal));
        let x_spread: f64 = if sorted_x.len() > 1 {
            sorted_x.last().expect("non-empty").bounds.x - sorted_x.first().expect("non-empty").bounds.x
        } else {
            0.0
        };

        let direction = if x_spread > y_spread * 1.5 {
            LayoutDirection::Row
        } else if y_spread > x_spread * 1.5 {
            LayoutDirection::Column
        } else {
            LayoutDirection::Wrap
        };

        let children: Vec<String> = self.recognized.iter()
            .map(|r| r.recognized_as.name().to_string())
            .collect();

        let layout_type = if self.recognized.len() > 6 {
            LayoutType::Grid
        } else {
            LayoutType::Flex
        };

        let layout = LayoutInference {
            layout_type,
            direction,
            children,
        };
        self.layouts.push(layout.clone());
        layout
    }

    /// Generate code from the current recognized state.
    pub fn generate_code(&self, framework: &Framework) -> Result<GeneratedCode, String> {
        if self.recognized.is_empty() {
            return Err("No shapes recognized — call recognize_shapes first".to_string());
        }
        let layout = self.layouts.last()
            .ok_or_else(|| "No layout inferred — call infer_layout first".to_string())?;
        let gen = ComponentGenerator::new(framework.clone());
        Ok(gen.generate(&self.recognized, layout))
    }

    /// Export the canvas as SVG.
    pub fn export_svg(&self) -> String {
        let mut svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\">\n",
            self.config.width, self.config.height
        );

        for elem in &self.elements {
            match elem {
                CanvasElement::Rectangle { x, y, width, height, fill } => {
                    let f = fill.as_deref().unwrap_or("none");
                    svg.push_str(&format!(
                        "  <rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"black\" />\n",
                        x, y, width, height, f
                    ));
                }
                CanvasElement::Circle { cx, cy, radius, fill } => {
                    let f = fill.as_deref().unwrap_or("none");
                    svg.push_str(&format!(
                        "  <circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"black\" />\n",
                        cx, cy, radius, f
                    ));
                }
                CanvasElement::Line { x1, y1, x2, y2 } => {
                    svg.push_str(&format!(
                        "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" />\n",
                        x1, y1, x2, y2
                    ));
                }
                CanvasElement::Text { x, y, content, font_size } => {
                    svg.push_str(&format!(
                        "  <text x=\"{}\" y=\"{}\" font-size=\"{}\">{}</text>\n",
                        x, y, font_size, content
                    ));
                }
                CanvasElement::Arrow { x1, y1, x2, y2 } => {
                    svg.push_str(&format!(
                        "  <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"black\" marker-end=\"url(#arrow)\" />\n",
                        x1, y1, x2, y2
                    ));
                }
                CanvasElement::Freehand { points } => {
                    if !points.is_empty() {
                        let d: Vec<String> = points.iter().enumerate().map(|(i, (x, y))| {
                            if i == 0 { format!("M {} {}", x, y) } else { format!("L {} {}", x, y) }
                        }).collect();
                        svg.push_str(&format!(
                            "  <path d=\"{}\" fill=\"none\" stroke=\"black\" />\n",
                            d.join(" ")
                        ));
                    }
                }
            }
        }

        svg.push_str("</svg>");
        svg
    }

    /// Clear all canvas state.
    pub fn clear(&mut self) {
        self.elements.clear();
        self.recognized.clear();
        self.layouts.clear();
    }

    /// Total element count.
    pub fn element_count(&self) -> usize {
        self.elements.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas() -> SketchCanvas {
        SketchCanvas::new(CanvasConfig::default())
    }

    #[test]
    fn test_bounding_box_new() {
        let bb = BoundingBox::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(bb.area(), 5000.0);
    }

    #[test]
    fn test_bounding_box_center() {
        let bb = BoundingBox::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(bb.center(), (50.0, 25.0));
    }

    #[test]
    fn test_bounding_box_overlaps() {
        let a = BoundingBox::new(0.0, 0.0, 100.0, 100.0);
        let b = BoundingBox::new(50.0, 50.0, 100.0, 100.0);
        assert!(a.overlaps(&b));
    }

    #[test]
    fn test_bounding_box_no_overlap() {
        let a = BoundingBox::new(0.0, 0.0, 50.0, 50.0);
        let b = BoundingBox::new(100.0, 100.0, 50.0, 50.0);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn test_canvas_new() {
        let c = canvas();
        assert!(c.elements.is_empty());
        assert!(c.recognized.is_empty());
    }

    #[test]
    fn test_add_element() {
        let mut c = canvas();
        c.add_element(CanvasElement::Rectangle { x: 0.0, y: 0.0, width: 100.0, height: 40.0, fill: None });
        assert_eq!(c.element_count(), 1);
    }

    #[test]
    fn test_clear() {
        let mut c = canvas();
        c.add_element(CanvasElement::Circle { cx: 50.0, cy: 50.0, radius: 25.0, fill: None });
        c.clear();
        assert!(c.elements.is_empty());
    }

    #[test]
    fn test_recognize_button() {
        let mut c = canvas();
        c.add_element(CanvasElement::Rectangle { x: 10.0, y: 10.0, width: 120.0, height: 45.0, fill: None });
        c.recognize_shapes();
        assert!(!c.recognized.is_empty());
        assert_eq!(c.recognized[0].recognized_as, UiComponent::Button);
    }

    #[test]
    fn test_recognize_input() {
        let mut c = canvas();
        c.add_element(CanvasElement::Rectangle { x: 10.0, y: 10.0, width: 300.0, height: 35.0, fill: None });
        c.recognize_shapes();
        assert!(c.recognized.iter().any(|r| r.recognized_as == UiComponent::Input));
    }

    #[test]
    fn test_recognize_divider() {
        let mut c = canvas();
        c.add_element(CanvasElement::Line { x1: 0.0, y1: 100.0, x2: 500.0, y2: 100.0 });
        c.recognize_shapes();
        assert!(c.recognized.iter().any(|r| r.recognized_as == UiComponent::Divider));
    }

    #[test]
    fn test_recognize_text_button() {
        let mut c = canvas();
        c.add_element(CanvasElement::Text { x: 10.0, y: 10.0, content: "Submit Button".to_string(), font_size: 14.0 });
        c.recognize_shapes();
        assert!(c.recognized.iter().any(|r| r.recognized_as == UiComponent::Button));
    }

    #[test]
    fn test_recognize_navbar() {
        let mut c = canvas();
        c.add_element(CanvasElement::Rectangle { x: 0.0, y: 0.0, width: 1000.0, height: 60.0, fill: None });
        c.recognize_shapes();
        assert!(c.recognized.iter().any(|r| r.recognized_as == UiComponent::Navbar));
    }

    #[test]
    fn test_recognize_confidence_filter() {
        let mut c = SketchCanvas::new(CanvasConfig {
            confidence_threshold: 0.95,
            ..Default::default()
        });
        c.add_element(CanvasElement::Freehand { points: vec![(0.0, 0.0), (1.0, 1.0)] });
        c.recognize_shapes();
        assert!(c.recognized.is_empty()); // low confidence filtered
    }

    #[test]
    fn test_infer_layout_empty() {
        let mut c = canvas();
        let layout = c.infer_layout();
        assert_eq!(layout.direction, LayoutDirection::Column);
        assert!(layout.children.is_empty());
    }

    #[test]
    fn test_infer_layout_column() {
        let mut c = canvas();
        c.add_element(CanvasElement::Rectangle { x: 10.0, y: 10.0, width: 120.0, height: 45.0, fill: None });
        c.add_element(CanvasElement::Rectangle { x: 10.0, y: 100.0, width: 120.0, height: 45.0, fill: None });
        c.add_element(CanvasElement::Rectangle { x: 10.0, y: 200.0, width: 120.0, height: 45.0, fill: None });
        c.recognize_shapes();
        let layout = c.infer_layout();
        assert_eq!(layout.direction, LayoutDirection::Column);
    }

    #[test]
    fn test_infer_layout_row() {
        let mut c = canvas();
        c.add_element(CanvasElement::Rectangle { x: 10.0, y: 10.0, width: 120.0, height: 45.0, fill: None });
        c.add_element(CanvasElement::Rectangle { x: 200.0, y: 10.0, width: 120.0, height: 45.0, fill: None });
        c.add_element(CanvasElement::Rectangle { x: 400.0, y: 10.0, width: 120.0, height: 45.0, fill: None });
        c.recognize_shapes();
        let layout = c.infer_layout();
        assert_eq!(layout.direction, LayoutDirection::Row);
    }

    #[test]
    fn test_generate_code_react() {
        let mut c = canvas();
        c.add_element(CanvasElement::Rectangle { x: 10.0, y: 10.0, width: 120.0, height: 45.0, fill: None });
        c.recognize_shapes();
        c.infer_layout();
        let code = c.generate_code(&Framework::React).unwrap();
        assert_eq!(code.framework, Framework::React);
        assert!(code.code.contains("SketchComponent"));
        assert!(!code.imports.is_empty());
    }

    #[test]
    fn test_generate_code_html() {
        let mut c = canvas();
        c.add_element(CanvasElement::Rectangle { x: 10.0, y: 10.0, width: 120.0, height: 45.0, fill: None });
        c.recognize_shapes();
        c.infer_layout();
        let code = c.generate_code(&Framework::Html).unwrap();
        assert!(code.code.contains("<div"));
    }

    #[test]
    fn test_generate_code_swiftui() {
        let mut c = canvas();
        c.add_element(CanvasElement::Rectangle { x: 10.0, y: 10.0, width: 120.0, height: 45.0, fill: None });
        c.recognize_shapes();
        c.infer_layout();
        let code = c.generate_code(&Framework::SwiftUI).unwrap();
        assert!(code.code.contains("SketchView"));
        assert!(code.imports.contains(&"import SwiftUI".to_string()));
    }

    #[test]
    fn test_generate_code_no_shapes() {
        let c = canvas();
        assert!(c.generate_code(&Framework::React).is_err());
    }

    #[test]
    fn test_generate_code_no_layout() {
        let mut c = canvas();
        c.add_element(CanvasElement::Rectangle { x: 10.0, y: 10.0, width: 120.0, height: 45.0, fill: None });
        c.recognize_shapes();
        assert!(c.generate_code(&Framework::React).is_err());
    }

    #[test]
    fn test_export_svg_empty() {
        let c = canvas();
        let svg = c.export_svg();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_export_svg_rect() {
        let mut c = canvas();
        c.add_element(CanvasElement::Rectangle { x: 10.0, y: 20.0, width: 100.0, height: 50.0, fill: Some("red".to_string()) });
        let svg = c.export_svg();
        assert!(svg.contains("<rect"));
        assert!(svg.contains("red"));
    }

    #[test]
    fn test_export_svg_circle() {
        let mut c = canvas();
        c.add_element(CanvasElement::Circle { cx: 50.0, cy: 50.0, radius: 25.0, fill: None });
        let svg = c.export_svg();
        assert!(svg.contains("<circle"));
    }

    #[test]
    fn test_export_svg_line() {
        let mut c = canvas();
        c.add_element(CanvasElement::Line { x1: 0.0, y1: 0.0, x2: 100.0, y2: 100.0 });
        let svg = c.export_svg();
        assert!(svg.contains("<line"));
    }

    #[test]
    fn test_export_svg_text() {
        let mut c = canvas();
        c.add_element(CanvasElement::Text { x: 10.0, y: 20.0, content: "Hello".to_string(), font_size: 16.0 });
        let svg = c.export_svg();
        assert!(svg.contains("<text"));
        assert!(svg.contains("Hello"));
    }

    #[test]
    fn test_export_svg_arrow() {
        let mut c = canvas();
        c.add_element(CanvasElement::Arrow { x1: 0.0, y1: 0.0, x2: 100.0, y2: 50.0 });
        let svg = c.export_svg();
        assert!(svg.contains("marker-end"));
    }

    #[test]
    fn test_export_svg_freehand() {
        let mut c = canvas();
        c.add_element(CanvasElement::Freehand { points: vec![(0.0, 0.0), (10.0, 10.0), (20.0, 5.0)] });
        let svg = c.export_svg();
        assert!(svg.contains("<path"));
    }

    #[test]
    fn test_element_bounds_rectangle() {
        let e = CanvasElement::Rectangle { x: 10.0, y: 20.0, width: 100.0, height: 50.0, fill: None };
        let b = e.bounds();
        assert_eq!(b.x, 10.0);
        assert_eq!(b.width, 100.0);
    }

    #[test]
    fn test_element_bounds_circle() {
        let e = CanvasElement::Circle { cx: 50.0, cy: 50.0, radius: 25.0, fill: None };
        let b = e.bounds();
        assert_eq!(b.x, 25.0);
        assert_eq!(b.width, 50.0);
    }

    #[test]
    fn test_element_bounds_freehand_empty() {
        let e = CanvasElement::Freehand { points: vec![] };
        let b = e.bounds();
        assert_eq!(b.width, 0.0);
    }

    #[test]
    fn test_ui_component_name() {
        assert_eq!(UiComponent::Button.name(), "Button");
        assert_eq!(UiComponent::Sidebar.name(), "Sidebar");
    }

    #[test]
    fn test_framework_name() {
        assert_eq!(Framework::React.name(), "React");
        assert_eq!(Framework::SwiftUI.name(), "SwiftUI");
    }

    #[test]
    fn test_canvas_config_default() {
        let cfg = CanvasConfig::default();
        assert_eq!(cfg.width, 1024.0);
        assert_eq!(cfg.confidence_threshold, 0.5);
    }

    #[test]
    fn test_recognized_shape_serde() {
        let r = RecognizedShape {
            element_index: 0,
            recognized_as: UiComponent::Button,
            confidence: 0.85,
            bounds: BoundingBox::new(0.0, 0.0, 100.0, 40.0),
        };
        let json = serde_json::to_string(&r).unwrap();
        let de: RecognizedShape = serde_json::from_str(&json).unwrap();
        assert_eq!(r, de);
    }

    #[test]
    fn test_canvas_element_serde() {
        let e = CanvasElement::Rectangle { x: 1.0, y: 2.0, width: 3.0, height: 4.0, fill: None };
        let json = serde_json::to_string(&e).unwrap();
        let de: CanvasElement = serde_json::from_str(&json).unwrap();
        assert_eq!(e, de);
    }
}
