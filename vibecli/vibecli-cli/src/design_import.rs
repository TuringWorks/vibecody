//! Design-to-code pipeline for VibeCody.
//!
//! Import Figma designs, sketches/wireframes, and SVG mockups to generate
//! framework-specific UI code. Closes the gap vs Bolt.new (Figma import) and
//! Replit (sketch-to-code).
//!
//! REPL commands: `/design-import figma|svg|image|history|config`

// === Enums ===

#[derive(Debug, Clone, PartialEq)]
pub enum FrameworkTarget {
    React,
    Vue,
    Svelte,
    Angular,
    Html,
}

impl std::fmt::Display for FrameworkTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::React => write!(f, "react"),
            Self::Vue => write!(f, "vue"),
            Self::Svelte => write!(f, "svelte"),
            Self::Angular => write!(f, "angular"),
            Self::Html => write!(f, "html"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CssStrategy {
    TailwindClasses,
    CssModules,
    StyledComponents,
    InlineStyles,
    CssVariables,
}

impl std::fmt::Display for CssStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TailwindClasses => write!(f, "tailwind"),
            Self::CssModules => write!(f, "css-modules"),
            Self::StyledComponents => write!(f, "styled-components"),
            Self::InlineStyles => write!(f, "inline-styles"),
            Self::CssVariables => write!(f, "css-variables"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DesignSource {
    FigmaUrl(String),
    ImageFile(String),
    SvgFile(String),
    PdfFile(String),
}

impl std::fmt::Display for DesignSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FigmaUrl(url) => write!(f, "figma:{}", url),
            Self::ImageFile(path) => write!(f, "image:{}", path),
            Self::SvgFile(path) => write!(f, "svg:{}", path),
            Self::PdfFile(path) => write!(f, "pdf:{}", path),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FigmaNodeType {
    Frame,
    Text,
    Rectangle,
    Image,
    Vector,
    Component,
    Instance,
    Group,
}

impl std::fmt::Display for FigmaNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Frame => write!(f, "FRAME"),
            Self::Text => write!(f, "TEXT"),
            Self::Rectangle => write!(f, "RECTANGLE"),
            Self::Image => write!(f, "IMAGE"),
            Self::Vector => write!(f, "VECTOR"),
            Self::Component => write!(f, "COMPONENT"),
            Self::Instance => write!(f, "INSTANCE"),
            Self::Group => write!(f, "GROUP"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutMode {
    None,
    Horizontal,
    Vertical,
}

impl std::fmt::Display for LayoutMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Horizontal => write!(f, "horizontal"),
            Self::Vertical => write!(f, "vertical"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportError {
    FigmaApiError(String),
    InvalidUrl(String),
    FileNotFound(String),
    UnsupportedFormat(String),
    ParseError(String),
    CodeGenError(String),
    ConfigError(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FigmaApiError(msg) => write!(f, "Figma API error: {}", msg),
            Self::InvalidUrl(msg) => write!(f, "invalid URL: {}", msg),
            Self::FileNotFound(msg) => write!(f, "file not found: {}", msg),
            Self::UnsupportedFormat(msg) => write!(f, "unsupported format: {}", msg),
            Self::ParseError(msg) => write!(f, "parse error: {}", msg),
            Self::CodeGenError(msg) => write!(f, "code generation error: {}", msg),
            Self::ConfigError(msg) => write!(f, "config error: {}", msg),
        }
    }
}

// === Data Structures ===

#[derive(Debug, Clone)]
pub struct ImportConfig {
    pub figma_api_token: Option<String>,
    pub output_dir: String,
    pub framework: FrameworkTarget,
    pub css_strategy: CssStrategy,
    pub include_responsive: bool,
}

impl Default for ImportConfig {
    fn default() -> Self {
        Self {
            figma_api_token: None,
            output_dir: "src/components".to_string(),
            framework: FrameworkTarget::React,
            css_strategy: CssStrategy::TailwindClasses,
            include_responsive: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Spacing {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Spacing {
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self { top, right, bottom, left }
    }

    pub fn uniform(value: f32) -> Self {
        Self { top: value, right: value, bottom: value, left: value }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeStyles {
    pub fill_color: Option<String>,
    pub stroke_color: Option<String>,
    pub font_family: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<u16>,
    pub text_content: Option<String>,
    pub border_radius: Option<f32>,
    pub opacity: Option<f32>,
    pub padding: Option<Spacing>,
    pub gap: Option<f32>,
    pub layout_mode: Option<LayoutMode>,
}

#[derive(Debug, Clone)]
pub struct FigmaNode {
    pub id: String,
    pub name: String,
    pub node_type: FigmaNodeType,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub styles: NodeStyles,
    pub children: Vec<FigmaNode>,
}

#[derive(Debug, Clone)]
pub struct FigmaFrame {
    pub id: String,
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub background_color: Option<String>,
    pub children: Vec<FigmaNode>,
}

#[derive(Debug, Clone)]
pub struct ComponentProp {
    pub name: String,
    pub prop_type: String,
    pub default_value: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct ExtractedComponent {
    pub name: String,
    pub html_structure: String,
    pub css_styles: String,
    pub props: Vec<ComponentProp>,
    pub children_slots: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GeneratedCode {
    pub filename: String,
    pub code: String,
    pub language: String,
    pub component_name: String,
    pub imports: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImportResult {
    pub source: DesignSource,
    pub components: Vec<GeneratedCode>,
    pub styles: Option<GeneratedCode>,
    pub warnings: Vec<String>,
    pub timestamp: u64,
}

// === DesignImporter ===

pub struct DesignImporter {
    config: ImportConfig,
    history: Vec<ImportResult>,
}

impl DesignImporter {
    pub fn new(config: ImportConfig) -> Self {
        Self {
            config,
            history: Vec::new(),
        }
    }

    /// Top-level import dispatcher.
    pub fn import(&mut self, source: DesignSource) -> Result<ImportResult, ImportError> {
        let result = match &source {
            DesignSource::FigmaUrl(url) => {
                let (_file_key, _node_id) = Self::parse_figma_url(url)?;
                if self.config.figma_api_token.is_none() {
                    return Err(ImportError::ConfigError(
                        "figma_api_token is required for Figma imports".to_string(),
                    ));
                }
                // In a real implementation we would call the Figma API here.
                // Return a stub result for now.
                ImportResult {
                    source: source.clone(),
                    components: Vec::new(),
                    styles: None,
                    warnings: vec!["Figma API integration requires network access".to_string()],
                    timestamp: Self::now_ts(),
                }
            }
            DesignSource::SvgFile(path) => {
                if !path.ends_with(".svg") {
                    return Err(ImportError::UnsupportedFormat(
                        "expected .svg file".to_string(),
                    ));
                }
                // In production we would read the file; here we return a placeholder.
                ImportResult {
                    source: source.clone(),
                    components: Vec::new(),
                    styles: None,
                    warnings: Vec::new(),
                    timestamp: Self::now_ts(),
                }
            }
            DesignSource::ImageFile(path) => {
                let lower = path.to_lowercase();
                if !lower.ends_with(".png")
                    && !lower.ends_with(".jpg")
                    && !lower.ends_with(".jpeg")
                    && !lower.ends_with(".webp")
                {
                    return Err(ImportError::UnsupportedFormat(
                        "expected .png, .jpg, .jpeg, or .webp image".to_string(),
                    ));
                }
                ImportResult {
                    source: source.clone(),
                    components: Vec::new(),
                    styles: None,
                    warnings: vec!["Image-to-code requires AI vision model".to_string()],
                    timestamp: Self::now_ts(),
                }
            }
            DesignSource::PdfFile(path) => {
                if !path.to_lowercase().ends_with(".pdf") {
                    return Err(ImportError::UnsupportedFormat(
                        "expected .pdf file".to_string(),
                    ));
                }
                ImportResult {
                    source: source.clone(),
                    components: Vec::new(),
                    styles: None,
                    warnings: Vec::new(),
                    timestamp: Self::now_ts(),
                }
            }
        };
        self.history.push(result.clone());
        Ok(result)
    }

    /// Parse a Figma URL into (file_key, node_id).
    /// Accepted forms:
    ///   https://www.figma.com/file/FILEKEY/Title?node-id=NODEID
    ///   https://www.figma.com/design/FILEKEY/Title?node-id=NODEID
    pub fn parse_figma_url(url: &str) -> Result<(String, String), ImportError> {
        let url = url.trim();
        if !url.contains("figma.com/") {
            return Err(ImportError::InvalidUrl(
                "URL must be a figma.com link".to_string(),
            ));
        }

        // Extract file key from path segments: /file/KEY/ or /design/KEY/
        let after_domain = url
            .split("figma.com/")
            .nth(1)
            .ok_or_else(|| ImportError::InvalidUrl("malformed Figma URL".to_string()))?;

        let segments: Vec<&str> = after_domain.split('/').collect();
        if segments.len() < 2 {
            return Err(ImportError::InvalidUrl(
                "URL must contain file key".to_string(),
            ));
        }
        let kind = segments[0];
        if kind != "file" && kind != "design" {
            return Err(ImportError::InvalidUrl(format!(
                "expected /file/ or /design/ in URL, got /{}/ ",
                kind
            )));
        }
        // The file key may have query params attached if there is no title segment
        let file_key = segments[1].split('?').next().unwrap_or(segments[1]).to_string();
        if file_key.is_empty() {
            return Err(ImportError::InvalidUrl(
                "file key is empty".to_string(),
            ));
        }

        // Extract node-id from query string (optional)
        let node_id = if let Some(query_start) = url.find('?') {
            let query = &url[query_start + 1..];
            query
                .split('&')
                .find_map(|pair| {
                    let (key, val) = pair.split_once('=')?;
                    
                    
                    if key == "node-id" {
                        Some(val.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or_default()
        } else {
            String::new()
        };

        Ok((file_key, node_id))
    }

    /// Traverse a FigmaFrame tree and extract components from top-level children.
    pub fn extract_components_from_frame(frame: &FigmaFrame) -> Vec<ExtractedComponent> {
        let mut components = Vec::new();
        for child in &frame.children {
            components.push(Self::node_to_component(child));
        }
        components
    }

    /// Copy styles already present on a node (identity helper for uniformity).
    pub fn extract_styles_from_node(node: &FigmaNode) -> NodeStyles {
        node.styles.clone()
    }

    /// Recursively convert a FigmaNode tree into an ExtractedComponent.
    pub fn node_to_component(node: &FigmaNode) -> ExtractedComponent {
        let name = Self::sanitize_component_name(&node.name);
        let mut css_parts: Vec<String> = Vec::new();
        let mut props: Vec<ComponentProp> = Vec::new();
        let mut children_slots: Vec<String> = Vec::new();

        // Build CSS from styles
        let css = Self::generate_css_from_styles(&node.styles);
        if !css.is_empty() {
            css_parts.push(css);
        }

        // Dimensions
        css_parts.push(format!("width: {}px;", node.width));
        css_parts.push(format!("height: {}px;", node.height));

        // Text nodes expose a `text` prop
        if node.node_type == FigmaNodeType::Text {
            if let Some(ref content) = node.styles.text_content {
                props.push(ComponentProp {
                    name: "text".to_string(),
                    prop_type: "string".to_string(),
                    default_value: Some(content.clone()),
                    required: false,
                });
            }
        }

        // Build child HTML
        let mut child_html = String::new();
        for child in &node.children {
            let child_name = Self::sanitize_component_name(&child.name);
            children_slots.push(child_name.clone());
            child_html.push_str(&format!("  <{child_name} />\n"));
        }

        let tag = Self::tag_for_node_type(&node.node_type);
        let html_structure = if child_html.is_empty() {
            format!("<{tag} className=\"{name}\" />", tag = tag, name = name)
        } else {
            format!(
                "<{tag} className=\"{name}\">\n{children}</{tag}>",
                tag = tag,
                name = name,
                children = child_html
            )
        };

        ExtractedComponent {
            name,
            html_structure,
            css_styles: css_parts.join("\n"),
            props,
            children_slots,
        }
    }

    // --- Code generation per framework ---

    pub fn generate_component(&self, comp: &ExtractedComponent) -> GeneratedCode {
        match self.config.framework {
            FrameworkTarget::React => Self::generate_react_component(comp),
            FrameworkTarget::Vue => Self::generate_vue_component(comp),
            FrameworkTarget::Svelte => Self::generate_svelte_component(comp),
            FrameworkTarget::Angular | FrameworkTarget::Html => {
                Self::generate_html_component(comp)
            }
        }
    }

    pub fn generate_react_component(comp: &ExtractedComponent) -> GeneratedCode {
        let mut imports = vec!["import React from 'react';".to_string()];
        let props_interface = Self::build_ts_props_interface(comp);

        let mut code = String::new();
        code.push_str(&imports.join("\n"));
        code.push('\n');
        if !props_interface.is_empty() {
            code.push('\n');
            code.push_str(&props_interface);
            code.push('\n');
        }
        code.push_str(&format!(
            "\nexport const {name}: React.FC<{name}Props> = ({{ {params} }}) => {{\n  return (\n    {html}\n  );\n}};\n",
            name = comp.name,
            params = Self::props_destructure(comp),
            html = comp.html_structure,
        ));

        // Append style block if using CSS modules
        let filename = format!("{}.tsx", comp.name);
        imports.push(format!("import styles from './{}.module.css';", comp.name));

        GeneratedCode {
            filename,
            code,
            language: "tsx".to_string(),
            component_name: comp.name.clone(),
            imports,
        }
    }

    pub fn generate_vue_component(comp: &ExtractedComponent) -> GeneratedCode {
        let mut code = String::new();
        code.push_str("<template>\n");
        code.push_str(&format!("  {}\n", comp.html_structure));
        code.push_str("</template>\n\n");
        code.push_str("<script setup lang=\"ts\">\n");
        for prop in &comp.props {
            let required = if prop.required { "required: true" } else { "required: false" };
            code.push_str(&format!(
                "defineProps<{{ {}: {} }}>()\n",
                prop.name, prop.prop_type
            ));
            let _ = required; // used for documentation only in this stub
        }
        code.push_str("</script>\n\n");
        code.push_str("<style scoped>\n");
        code.push_str(&comp.css_styles);
        code.push_str("\n</style>\n");

        let filename = format!("{}.vue", comp.name);
        GeneratedCode {
            filename,
            code,
            language: "vue".to_string(),
            component_name: comp.name.clone(),
            imports: Vec::new(),
        }
    }

    pub fn generate_svelte_component(comp: &ExtractedComponent) -> GeneratedCode {
        let mut code = String::new();
        code.push_str("<script lang=\"ts\">\n");
        for prop in &comp.props {
            let default = prop
                .default_value
                .as_deref()
                .map(|v| format!(" = '{}'", v))
                .unwrap_or_default();
            code.push_str(&format!("  export let {}: {}{};\n", prop.name, prop.prop_type, default));
        }
        code.push_str("</script>\n\n");
        code.push_str(&comp.html_structure);
        code.push('\n');
        code.push_str("\n<style>\n");
        code.push_str(&comp.css_styles);
        code.push_str("\n</style>\n");

        let filename = format!("{}.svelte", comp.name);
        GeneratedCode {
            filename,
            code,
            language: "svelte".to_string(),
            component_name: comp.name.clone(),
            imports: Vec::new(),
        }
    }

    pub fn generate_html_component(comp: &ExtractedComponent) -> GeneratedCode {
        let mut code = String::new();
        code.push_str("<!DOCTYPE html>\n<html>\n<head>\n");
        code.push_str(&format!("  <title>{}</title>\n", comp.name));
        code.push_str("  <style>\n");
        code.push_str(&format!("    .{} {{\n      {}\n    }}\n", comp.name, comp.css_styles));
        code.push_str("  </style>\n");
        code.push_str("</head>\n<body>\n");
        code.push_str(&format!("  {}\n", comp.html_structure));
        code.push_str("</body>\n</html>\n");

        let filename = format!("{}.html", comp.name);
        GeneratedCode {
            filename,
            code,
            language: "html".to_string(),
            component_name: comp.name.clone(),
            imports: Vec::new(),
        }
    }

    // --- CSS helpers ---

    /// Generate a CSS rule body from NodeStyles.
    pub fn generate_css(comp: &ExtractedComponent) -> String {
        format!(".{} {{\n  {}\n}}", comp.name, comp.css_styles)
    }

    /// Generate Tailwind utility classes from NodeStyles.
    pub fn generate_tailwind_classes(styles: &NodeStyles) -> String {
        let mut classes: Vec<String> = Vec::new();

        if let Some(ref color) = styles.fill_color {
            classes.push(format!("bg-[{}]", Self::color_to_css(color)));
        }
        if let Some(ref color) = styles.stroke_color {
            classes.push(format!("border-[{}]", Self::color_to_css(color)));
        }
        if let Some(size) = styles.font_size {
            let rounded = size.round() as u32;
            classes.push(format!("text-[{}px]", rounded));
        }
        if let Some(weight) = styles.font_weight {
            let tw = match weight {
                0..=199 => "font-thin",
                200..=299 => "font-extralight",
                300..=399 => "font-light",
                400..=499 => "font-normal",
                500..=599 => "font-medium",
                600..=699 => "font-semibold",
                700..=799 => "font-bold",
                800..=899 => "font-extrabold",
                _ => "font-black",
            };
            classes.push(tw.to_string());
        }
        if let Some(radius) = styles.border_radius {
            if radius > 0.0 {
                classes.push(format!("rounded-[{}px]", radius.round() as u32));
            }
        }
        if let Some(opacity) = styles.opacity {
            if (opacity - 1.0).abs() > f32::EPSILON {
                let pct = (opacity * 100.0).round() as u32;
                classes.push(format!("opacity-{}", pct));
            }
        }
        if let Some(ref padding) = styles.padding {
            classes.push(format!("p-[{}px]", padding.top.round() as u32));
        }
        if let Some(gap) = styles.gap {
            classes.push(format!("gap-[{}px]", gap.round() as u32));
        }
        if let Some(ref layout) = styles.layout_mode {
            match layout {
                LayoutMode::Horizontal => classes.push("flex flex-row".to_string()),
                LayoutMode::Vertical => classes.push("flex flex-col".to_string()),
                LayoutMode::None => {}
            }
        }

        classes.join(" ")
    }

    /// Normalize a color string to a CSS-compatible value.
    pub fn color_to_css(color: &str) -> String {
        let trimmed = color.trim();
        // Already a hex color
        if trimmed.starts_with('#') {
            return trimmed.to_lowercase();
        }
        // Already an rgb/rgba/hsl value
        if trimmed.starts_with("rgb") || trimmed.starts_with("hsl") {
            return trimmed.to_string();
        }
        // Bare 6- or 8-char hex without #
        if (trimmed.len() == 6 || trimmed.len() == 8)
            && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
                return format!("#{}", trimmed.to_lowercase());
            }
        // Named CSS color — pass through
        trimmed.to_lowercase()
    }

    /// Parse SVG content and extract components from top-level groups/elements.
    pub fn parse_svg(content: &str) -> Result<Vec<ExtractedComponent>, ImportError> {
        let content = content.trim();
        if !content.contains("<svg") {
            return Err(ImportError::ParseError(
                "content does not contain an <svg> element".to_string(),
            ));
        }

        let mut components: Vec<ExtractedComponent> = Vec::new();

        // Lightweight parser: extract top-level <g>, <rect>, <circle>, <text>, <path> elements
        let tag_names = ["g", "rect", "circle", "text", "path", "ellipse", "line", "polygon"];
        for tag in &tag_names {
            let open = format!("<{}", tag);
            let mut search_start = 0;
            let mut index = 0;
            while let Some(pos) = content[search_start..].find(&open) {
                let abs_pos = search_start + pos;
                // Find the end of this element (self-closing or closing tag)
                let snippet_end = content[abs_pos..]
                    .find('>')
                    .map(|p| abs_pos + p + 1)
                    .unwrap_or(content.len());
                let snippet = &content[abs_pos..snippet_end];

                let name = Self::extract_svg_id(snippet)
                    .unwrap_or_else(|| format!("Svg{}_{}", Self::capitalize(tag), index));

                components.push(ExtractedComponent {
                    name,
                    html_structure: snippet.to_string(),
                    css_styles: String::new(),
                    props: Vec::new(),
                    children_slots: Vec::new(),
                });

                index += 1;
                search_start = snippet_end;
            }
        }

        Ok(components)
    }

    /// Return import history.
    pub fn get_import_history(&self) -> Vec<&ImportResult> {
        self.history.iter().collect()
    }

    /// Estimate complexity of a FigmaFrame by counting all nodes recursively.
    pub fn estimate_complexity(frame: &FigmaFrame) -> u32 {
        fn count_nodes(nodes: &[FigmaNode]) -> u32 {
            let mut total = 0u32;
            for node in nodes {
                total += 1;
                total += count_nodes(&node.children);
            }
            total
        }
        count_nodes(&frame.children)
    }

    // --- Private helpers ---

    fn generate_css_from_styles(styles: &NodeStyles) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(ref color) = styles.fill_color {
            parts.push(format!("background-color: {};", Self::color_to_css(color)));
        }
        if let Some(ref color) = styles.stroke_color {
            parts.push(format!("border-color: {};", Self::color_to_css(color)));
        }
        if let Some(ref family) = styles.font_family {
            parts.push(format!("font-family: '{}', sans-serif;", family));
        }
        if let Some(size) = styles.font_size {
            parts.push(format!("font-size: {}px;", size));
        }
        if let Some(weight) = styles.font_weight {
            parts.push(format!("font-weight: {};", weight));
        }
        if let Some(radius) = styles.border_radius {
            parts.push(format!("border-radius: {}px;", radius));
        }
        if let Some(opacity) = styles.opacity {
            parts.push(format!("opacity: {};", opacity));
        }
        if let Some(ref padding) = styles.padding {
            parts.push(format!(
                "padding: {}px {}px {}px {}px;",
                padding.top, padding.right, padding.bottom, padding.left
            ));
        }
        if let Some(gap) = styles.gap {
            parts.push(format!("gap: {}px;", gap));
        }
        if let Some(ref layout) = styles.layout_mode {
            match layout {
                LayoutMode::Horizontal => {
                    parts.push("display: flex;".to_string());
                    parts.push("flex-direction: row;".to_string());
                }
                LayoutMode::Vertical => {
                    parts.push("display: flex;".to_string());
                    parts.push("flex-direction: column;".to_string());
                }
                LayoutMode::None => {}
            }
        }
        parts.join("\n")
    }

    fn sanitize_component_name(name: &str) -> String {
        let cleaned: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        // Ensure PascalCase start
        if cleaned.is_empty() {
            return "Component".to_string();
        }
        let mut chars = cleaned.chars();
        let first = chars.next().expect("non-empty string");
        if first.is_ascii_digit() {
            format!("C{}", cleaned)
        } else {
            let upper: String = first.to_uppercase().collect();
            format!("{}{}", upper, chars.collect::<String>())
        }
    }

    fn tag_for_node_type(nt: &FigmaNodeType) -> &'static str {
        match nt {
            FigmaNodeType::Text => "span",
            FigmaNodeType::Image => "img",
            FigmaNodeType::Vector => "svg",
            _ => "div",
        }
    }

    fn build_ts_props_interface(comp: &ExtractedComponent) -> String {
        let mut lines = vec![format!("interface {}Props {{", comp.name)];
        for prop in &comp.props {
            let optional = if prop.required { "" } else { "?" };
            lines.push(format!("  {}{}: {};", prop.name, optional, prop.prop_type));
        }
        lines.push("}".to_string());
        lines.join("\n")
    }

    fn props_destructure(comp: &ExtractedComponent) -> String {
        comp.props
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn extract_svg_id(snippet: &str) -> Option<String> {
        let id_start = snippet.find("id=\"")?;
        let rest = &snippet[id_start + 4..];
        let id_end = rest.find('"')?;
        let raw = &rest[..id_end];
        if raw.is_empty() {
            None
        } else {
            Some(Self::sanitize_component_name(raw))
        }
    }

    fn capitalize(s: &str) -> String {
        let mut c = s.chars();
        match c.next() {
            None => String::new(),
            Some(f) => {
                let upper: String = f.to_uppercase().collect();
                format!("{}{}", upper, c.collect::<String>())
            }
        }
    }

    fn now_ts() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    // --- Config defaults ---

    #[test]
    fn test_config_defaults() {
        let cfg = ImportConfig::default();
        assert_eq!(cfg.output_dir, "src/components");
        assert_eq!(cfg.framework, FrameworkTarget::React);
        assert_eq!(cfg.css_strategy, CssStrategy::TailwindClasses);
        assert!(cfg.include_responsive);
        assert!(cfg.figma_api_token.is_none());
    }

    #[test]
    fn test_config_custom() {
        let cfg = ImportConfig {
            figma_api_token: Some("figd_xxx".to_string()),
            output_dir: "lib/ui".to_string(),
            framework: FrameworkTarget::Vue,
            css_strategy: CssStrategy::CssModules,
            include_responsive: false,
        };
        assert_eq!(cfg.framework, FrameworkTarget::Vue);
        assert!(!cfg.include_responsive);
    }

    // --- Figma URL parsing ---

    #[test]
    fn test_parse_figma_url_valid_file() {
        let (key, node) =
            DesignImporter::parse_figma_url("https://www.figma.com/file/abc123/MyDesign?node-id=1-2")
                .unwrap();
        assert_eq!(key, "abc123");
        assert_eq!(node, "1-2");
    }

    #[test]
    fn test_parse_figma_url_valid_design() {
        let (key, node) =
            DesignImporter::parse_figma_url("https://www.figma.com/design/xyz789/Title")
                .unwrap();
        assert_eq!(key, "xyz789");
        assert!(node.is_empty());
    }

    #[test]
    fn test_parse_figma_url_with_node_id() {
        let (key, node) = DesignImporter::parse_figma_url(
            "https://www.figma.com/file/KEY123/Name?type=design&node-id=42-99&mode=dev",
        )
        .unwrap();
        assert_eq!(key, "KEY123");
        assert_eq!(node, "42-99");
    }

    #[test]
    fn test_parse_figma_url_invalid_not_figma() {
        let err = DesignImporter::parse_figma_url("https://example.com/file/abc").unwrap_err();
        assert!(matches!(err, ImportError::InvalidUrl(_)));
    }

    #[test]
    fn test_parse_figma_url_invalid_no_key() {
        let err =
            DesignImporter::parse_figma_url("https://www.figma.com/file/").unwrap_err();
        assert!(matches!(err, ImportError::InvalidUrl(_)));
    }

    #[test]
    fn test_parse_figma_url_invalid_wrong_segment() {
        let err =
            DesignImporter::parse_figma_url("https://www.figma.com/proto/abc/Title").unwrap_err();
        assert!(matches!(err, ImportError::InvalidUrl(_)));
    }

    // --- Helper: build a test node ---

    fn make_node(name: &str, node_type: FigmaNodeType) -> FigmaNode {
        FigmaNode {
            id: "n1".to_string(),
            name: name.to_string(),
            node_type,
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
            styles: NodeStyles::default(),
            children: Vec::new(),
        }
    }

    fn make_frame(name: &str, children: Vec<FigmaNode>) -> FigmaFrame {
        FigmaFrame {
            id: "f1".to_string(),
            name: name.to_string(),
            width: 1440.0,
            height: 900.0,
            background_color: Some("#ffffff".to_string()),
            children,
        }
    }

    // --- Component extraction ---

    #[test]
    fn test_extract_components_from_frame_empty() {
        let frame = make_frame("Empty", vec![]);
        let comps = DesignImporter::extract_components_from_frame(&frame);
        assert!(comps.is_empty());
    }

    #[test]
    fn test_extract_components_from_frame_multiple() {
        let frame = make_frame(
            "Page",
            vec![
                make_node("Header", FigmaNodeType::Frame),
                make_node("Body", FigmaNodeType::Frame),
                make_node("Footer", FigmaNodeType::Frame),
            ],
        );
        let comps = DesignImporter::extract_components_from_frame(&frame);
        assert_eq!(comps.len(), 3);
        assert_eq!(comps[0].name, "Header");
        assert_eq!(comps[2].name, "Footer");
    }

    // --- Style extraction ---

    #[test]
    fn test_extract_styles_from_node() {
        let mut node = make_node("Card", FigmaNodeType::Rectangle);
        node.styles.fill_color = Some("#ff0000".to_string());
        node.styles.border_radius = Some(8.0);
        let styles = DesignImporter::extract_styles_from_node(&node);
        assert_eq!(styles.fill_color.as_deref(), Some("#ff0000"));
        assert_eq!(styles.border_radius, Some(8.0));
    }

    #[test]
    fn test_node_to_component_text_node() {
        let mut node = make_node("Title", FigmaNodeType::Text);
        node.styles.text_content = Some("Hello World".to_string());
        node.styles.font_size = Some(24.0);
        let comp = DesignImporter::node_to_component(&node);
        assert_eq!(comp.name, "Title");
        assert_eq!(comp.props.len(), 1);
        assert_eq!(comp.props[0].name, "text");
        assert_eq!(comp.props[0].default_value.as_deref(), Some("Hello World"));
    }

    #[test]
    fn test_node_to_component_with_children() {
        let mut parent = make_node("Container", FigmaNodeType::Frame);
        parent.children.push(make_node("Child1", FigmaNodeType::Rectangle));
        parent.children.push(make_node("Child2", FigmaNodeType::Text));
        let comp = DesignImporter::node_to_component(&parent);
        assert_eq!(comp.children_slots.len(), 2);
        assert!(comp.html_structure.contains("Child1"));
        assert!(comp.html_structure.contains("Child2"));
    }

    // --- React code generation ---

    #[test]
    fn test_generate_react_component() {
        let comp = ExtractedComponent {
            name: "Button".to_string(),
            html_structure: "<button className=\"Button\">Click</button>".to_string(),
            css_styles: "background-color: blue;".to_string(),
            props: vec![ComponentProp {
                name: "label".to_string(),
                prop_type: "string".to_string(),
                default_value: None,
                required: true,
            }],
            children_slots: Vec::new(),
        };
        let gen = DesignImporter::generate_react_component(&comp);
        assert_eq!(gen.language, "tsx");
        assert!(gen.filename.ends_with(".tsx"));
        assert!(gen.code.contains("React.FC"));
        assert!(gen.code.contains("ButtonProps"));
        assert_eq!(gen.component_name, "Button");
    }

    // --- Vue code generation ---

    #[test]
    fn test_generate_vue_component() {
        let comp = ExtractedComponent {
            name: "Card".to_string(),
            html_structure: "<div class=\"Card\"></div>".to_string(),
            css_styles: "padding: 16px;".to_string(),
            props: Vec::new(),
            children_slots: Vec::new(),
        };
        let gen = DesignImporter::generate_vue_component(&comp);
        assert_eq!(gen.language, "vue");
        assert!(gen.code.contains("<template>"));
        assert!(gen.code.contains("<style scoped>"));
        assert!(gen.filename.ends_with(".vue"));
    }

    // --- Svelte code generation ---

    #[test]
    fn test_generate_svelte_component() {
        let comp = ExtractedComponent {
            name: "Badge".to_string(),
            html_structure: "<span>badge</span>".to_string(),
            css_styles: "color: red;".to_string(),
            props: vec![ComponentProp {
                name: "count".to_string(),
                prop_type: "number".to_string(),
                default_value: Some("0".to_string()),
                required: false,
            }],
            children_slots: Vec::new(),
        };
        let gen = DesignImporter::generate_svelte_component(&comp);
        assert_eq!(gen.language, "svelte");
        assert!(gen.code.contains("export let count"));
        assert!(gen.code.contains("<style>"));
    }

    // --- HTML code generation ---

    #[test]
    fn test_generate_html_component() {
        let comp = ExtractedComponent {
            name: "Hero".to_string(),
            html_structure: "<div>Hero Section</div>".to_string(),
            css_styles: "text-align: center;".to_string(),
            props: Vec::new(),
            children_slots: Vec::new(),
        };
        let gen = DesignImporter::generate_html_component(&comp);
        assert_eq!(gen.language, "html");
        assert!(gen.code.contains("<!DOCTYPE html>"));
        assert!(gen.code.contains("<style>"));
    }

    // --- Tailwind class generation ---

    #[test]
    fn test_generate_tailwind_classes_basic() {
        let styles = NodeStyles {
            fill_color: Some("#3b82f6".to_string()),
            font_size: Some(16.0),
            font_weight: Some(700),
            border_radius: Some(8.0),
            ..Default::default()
        };
        let classes = DesignImporter::generate_tailwind_classes(&styles);
        assert!(classes.contains("bg-[#3b82f6]"));
        assert!(classes.contains("text-[16px]"));
        assert!(classes.contains("font-bold"));
        assert!(classes.contains("rounded-[8px]"));
    }

    #[test]
    fn test_generate_tailwind_classes_layout() {
        let styles = NodeStyles {
            layout_mode: Some(LayoutMode::Horizontal),
            gap: Some(12.0),
            ..Default::default()
        };
        let classes = DesignImporter::generate_tailwind_classes(&styles);
        assert!(classes.contains("flex flex-row"));
        assert!(classes.contains("gap-[12px]"));
    }

    #[test]
    fn test_generate_tailwind_classes_opacity() {
        let styles = NodeStyles {
            opacity: Some(0.5),
            ..Default::default()
        };
        let classes = DesignImporter::generate_tailwind_classes(&styles);
        assert!(classes.contains("opacity-50"));
    }

    #[test]
    fn test_generate_tailwind_classes_empty() {
        let styles = NodeStyles::default();
        let classes = DesignImporter::generate_tailwind_classes(&styles);
        assert!(classes.is_empty());
    }

    // --- CSS generation ---

    #[test]
    fn test_generate_css() {
        let comp = ExtractedComponent {
            name: "Panel".to_string(),
            html_structure: String::new(),
            css_styles: "background-color: #fff;\npadding: 16px;".to_string(),
            props: Vec::new(),
            children_slots: Vec::new(),
        };
        let css = DesignImporter::generate_css(&comp);
        assert!(css.starts_with(".Panel {"));
        assert!(css.contains("background-color"));
    }

    // --- Color normalization ---

    #[test]
    fn test_color_to_css_hex() {
        assert_eq!(DesignImporter::color_to_css("#FF0000"), "#ff0000");
    }

    #[test]
    fn test_color_to_css_bare_hex() {
        assert_eq!(DesignImporter::color_to_css("ff00ff"), "#ff00ff");
    }

    #[test]
    fn test_color_to_css_rgb() {
        assert_eq!(
            DesignImporter::color_to_css("rgb(255, 0, 0)"),
            "rgb(255, 0, 0)"
        );
    }

    #[test]
    fn test_color_to_css_named() {
        assert_eq!(DesignImporter::color_to_css("Red"), "red");
    }

    // --- SVG parsing ---

    #[test]
    fn test_parse_svg_valid() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect id="bg" x="0" y="0" width="100" height="100" />
            <circle id="dot" cx="50" cy="50" r="25" />
        </svg>"#;
        let comps = DesignImporter::parse_svg(svg).unwrap();
        assert!(comps.len() >= 2);
        let names: Vec<&str> = comps.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"Bg"));
        assert!(names.contains(&"Dot"));
    }

    #[test]
    fn test_parse_svg_no_svg_element() {
        let err = DesignImporter::parse_svg("<div>not svg</div>").unwrap_err();
        assert!(matches!(err, ImportError::ParseError(_)));
    }

    #[test]
    fn test_parse_svg_empty_content() {
        let comps = DesignImporter::parse_svg("<svg></svg>").unwrap();
        assert!(comps.is_empty());
    }

    // --- Complexity estimation ---

    #[test]
    fn test_estimate_complexity_empty() {
        let frame = make_frame("Empty", vec![]);
        assert_eq!(DesignImporter::estimate_complexity(&frame), 0);
    }

    #[test]
    fn test_estimate_complexity_flat() {
        let frame = make_frame(
            "Flat",
            vec![
                make_node("A", FigmaNodeType::Rectangle),
                make_node("B", FigmaNodeType::Text),
            ],
        );
        assert_eq!(DesignImporter::estimate_complexity(&frame), 2);
    }

    #[test]
    fn test_estimate_complexity_nested() {
        let mut parent = make_node("Parent", FigmaNodeType::Frame);
        parent.children.push(make_node("Child1", FigmaNodeType::Text));
        parent.children.push(make_node("Child2", FigmaNodeType::Rectangle));
        let frame = make_frame("Nested", vec![parent]);
        // 1 parent + 2 children = 3
        assert_eq!(DesignImporter::estimate_complexity(&frame), 3);
    }

    // --- Import history ---

    #[test]
    fn test_import_history() {
        let mut importer = DesignImporter::new(ImportConfig::default());
        assert!(importer.get_import_history().is_empty());
        let _ = importer.import(DesignSource::SvgFile("test.svg".to_string()));
        assert_eq!(importer.get_import_history().len(), 1);
    }

    // --- Error cases ---

    #[test]
    fn test_import_figma_without_token() {
        let mut importer = DesignImporter::new(ImportConfig::default());
        let err = importer
            .import(DesignSource::FigmaUrl(
                "https://www.figma.com/file/abc/Title".to_string(),
            ))
            .unwrap_err();
        assert!(matches!(err, ImportError::ConfigError(_)));
    }

    #[test]
    fn test_import_unsupported_image_format() {
        let mut importer = DesignImporter::new(ImportConfig::default());
        let err = importer
            .import(DesignSource::ImageFile("sketch.bmp".to_string()))
            .unwrap_err();
        assert!(matches!(err, ImportError::UnsupportedFormat(_)));
    }

    #[test]
    fn test_import_unsupported_svg_extension() {
        let mut importer = DesignImporter::new(ImportConfig::default());
        let err = importer
            .import(DesignSource::SvgFile("drawing.xml".to_string()))
            .unwrap_err();
        assert!(matches!(err, ImportError::UnsupportedFormat(_)));
    }

    // --- Display impls ---

    #[test]
    fn test_framework_display() {
        assert_eq!(format!("{}", FrameworkTarget::Svelte), "svelte");
        assert_eq!(format!("{}", FrameworkTarget::Angular), "angular");
    }

    #[test]
    fn test_css_strategy_display() {
        assert_eq!(format!("{}", CssStrategy::StyledComponents), "styled-components");
    }

    #[test]
    fn test_design_source_display() {
        let src = DesignSource::FigmaUrl("https://figma.com/file/x/Y".to_string());
        assert!(format!("{}", src).starts_with("figma:"));
    }

    #[test]
    fn test_import_error_display() {
        let err = ImportError::FileNotFound("missing.svg".to_string());
        assert!(format!("{}", err).contains("file not found"));
    }

    // --- generate_component dispatch ---

    #[test]
    fn test_generate_component_dispatches_to_framework() {
        let comp = ExtractedComponent {
            name: "Test".to_string(),
            html_structure: "<div />".to_string(),
            css_styles: String::new(),
            props: Vec::new(),
            children_slots: Vec::new(),
        };

        let importer = DesignImporter::new(ImportConfig {
            framework: FrameworkTarget::Svelte,
            ..Default::default()
        });
        let gen = importer.generate_component(&comp);
        assert_eq!(gen.language, "svelte");

        let importer2 = DesignImporter::new(ImportConfig {
            framework: FrameworkTarget::Html,
            ..Default::default()
        });
        let gen2 = importer2.generate_component(&comp);
        assert_eq!(gen2.language, "html");
    }

    // --- Sanitize component name ---

    #[test]
    fn test_sanitize_component_name_special_chars() {
        let name = DesignImporter::sanitize_component_name("my-button group");
        assert_eq!(name, "My_button_group");
    }

    #[test]
    fn test_sanitize_component_name_digit_start() {
        let name = DesignImporter::sanitize_component_name("123card");
        assert_eq!(name, "C123card");
    }

    #[test]
    fn test_sanitize_component_name_empty() {
        let name = DesignImporter::sanitize_component_name("");
        assert_eq!(name, "Component");
    }

    // --- Spacing ---

    #[test]
    fn test_spacing_uniform() {
        let s = Spacing::uniform(16.0);
        assert_eq!(s.top, 16.0);
        assert_eq!(s.right, 16.0);
        assert_eq!(s.bottom, 16.0);
        assert_eq!(s.left, 16.0);
    }
}
