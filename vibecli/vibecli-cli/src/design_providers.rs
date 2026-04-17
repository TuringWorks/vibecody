//! Design provider abstraction — multi-tool interop layer.
//!
//! Supports Figma, Penpot, Pencil (Evolus + TuringWorks), Draw.io, and in-house generation.
//! All providers expose a common trait surface so UI panels and agent tools are provider-agnostic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Provider kind ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Figma,
    Penpot,
    Pencil,
    DrawIo,
    Mermaid,
    PlantUml,
    C4Model,
    Inhouse,
}

impl ProviderKind {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Figma => "Figma",
            Self::Penpot => "Penpot",
            Self::Pencil => "Pencil",
            Self::DrawIo => "Draw.io",
            Self::Mermaid => "Mermaid",
            Self::PlantUml => "PlantUML",
            Self::C4Model => "C4 Model",
            Self::Inhouse => "VibeCody Built-in",
        }
    }

    pub fn supports_editing(&self) -> bool {
        matches!(self, Self::Figma | Self::Penpot | Self::Pencil | Self::DrawIo | Self::Inhouse)
    }

    pub fn supports_export(&self) -> bool {
        true
    }
}

// ─── Diagram format ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagramFormat {
    DrawIoXml,
    MermaidMd,
    PlantUml,
    C4Dsl,
    SvgMarkup,
    PngBytes,
    Json,
}

impl DiagramFormat {
    pub fn file_extension(&self) -> &str {
        match self {
            Self::DrawIoXml => "drawio",
            Self::MermaidMd => "md",
            Self::PlantUml => "puml",
            Self::C4Dsl => "dsl",
            Self::SvgMarkup => "svg",
            Self::PngBytes => "png",
            Self::Json => "json",
        }
    }
    pub fn mime_type(&self) -> &str {
        match self {
            Self::DrawIoXml => "application/xml",
            Self::MermaidMd => "text/markdown",
            Self::PlantUml => "text/plain",
            Self::C4Dsl => "text/plain",
            Self::SvgMarkup => "image/svg+xml",
            Self::PngBytes => "image/png",
            Self::Json => "application/json",
        }
    }
}

// ─── Diagram type ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagramKind {
    Flowchart,
    Sequence,
    ClassDiagram,
    EntityRelationship,
    ComponentDiagram,
    DeploymentDiagram,
    C4Context,
    C4Container,
    C4Component,
    C4Code,
    Architecture,
    StateMachine,
    MindMap,
    Gantt,
    UserJourney,
    Wireframe,
    NetworkTopology,
}

impl DiagramKind {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Flowchart => "Flowchart",
            Self::Sequence => "Sequence Diagram",
            Self::ClassDiagram => "Class Diagram",
            Self::EntityRelationship => "Entity-Relationship Diagram",
            Self::ComponentDiagram => "Component Diagram",
            Self::DeploymentDiagram => "Deployment Diagram",
            Self::C4Context => "C4 Context",
            Self::C4Container => "C4 Container",
            Self::C4Component => "C4 Component",
            Self::C4Code => "C4 Code",
            Self::Architecture => "Architecture Diagram",
            Self::StateMachine => "State Machine",
            Self::MindMap => "Mind Map",
            Self::Gantt => "Gantt Chart",
            Self::UserJourney => "User Journey",
            Self::Wireframe => "Wireframe",
            Self::NetworkTopology => "Network Topology",
        }
    }

    /// Best format for this diagram kind
    pub fn preferred_format(&self) -> DiagramFormat {
        match self {
            Self::Flowchart | Self::Sequence | Self::ClassDiagram
            | Self::StateMachine | Self::Gantt | Self::UserJourney
            | Self::MindMap => DiagramFormat::MermaidMd,
            Self::C4Context | Self::C4Container | Self::C4Component | Self::C4Code => {
                DiagramFormat::C4Dsl
            }
            Self::EntityRelationship | Self::ComponentDiagram
            | Self::DeploymentDiagram | Self::Architecture
            | Self::NetworkTopology | Self::Wireframe => DiagramFormat::DrawIoXml,
        }
    }
}

// ─── Design file / frame / component ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignFrame {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignComponent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub props: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignToken {
    pub name: String,
    pub token_type: DesignTokenType,
    pub value: String,
    pub description: Option<String>,
    pub provider: ProviderKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignTokenType {
    Color,
    Typography,
    Spacing,
    BorderRadius,
    Shadow,
    Animation,
    Breakpoint,
    ZIndex,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignFile {
    pub id: String,
    pub name: String,
    pub provider: ProviderKind,
    pub last_modified: Option<String>,
    pub frames: Vec<DesignFrame>,
    pub components: Vec<DesignComponent>,
    pub tokens: Vec<DesignToken>,
}

// ─── Diagram document ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramDoc {
    pub id: String,
    pub title: String,
    pub kind: DiagramKind,
    pub format: DiagramFormat,
    pub content: String,
    pub provider: ProviderKind,
    pub created_at_ms: u64,
    pub metadata: HashMap<String, String>,
}

impl DiagramDoc {
    pub fn new(title: &str, kind: DiagramKind, content: String, provider: ProviderKind) -> Self {
        let format = kind.preferred_format();
        Self {
            id: format!("diag-{}", uuid_short()),
            title: title.to_string(),
            kind,
            format,
            content,
            provider,
            created_at_ms: epoch_ms(),
            metadata: HashMap::new(),
        }
    }
}

// ─── Provider trait ───────────────────────────────────────────────────────────

/// Synchronous design provider interface.
/// For async operations, providers return descriptors that can be resolved via agents.
pub trait DesignProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn display_name(&self) -> &str;

    /// List available projects / files
    fn list_files(&self) -> Result<Vec<DesignFile>, DesignError>;

    /// Import a design file by URL or ID
    fn import_file(&self, url_or_id: &str, token: Option<&str>) -> Result<DesignFile, DesignError>;

    /// Export a component to source code
    fn export_component(
        &self,
        component: &DesignComponent,
        framework: &str,
    ) -> Result<String, DesignError>;

    /// Extract design tokens from a file
    fn extract_tokens(&self, file: &DesignFile) -> Vec<DesignToken>;
}

// ─── Provider registry ────────────────────────────────────────────────────────

pub struct DesignProviderRegistry {
    providers: HashMap<ProviderKind, Box<dyn DesignProvider>>,
}

impl std::fmt::Debug for DesignProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DesignProviderRegistry({} providers)", self.providers.len())
    }
}

impl DesignProviderRegistry {
    pub fn new() -> Self {
        Self { providers: HashMap::new() }
    }

    pub fn register(&mut self, provider: Box<dyn DesignProvider>) {
        self.providers.insert(provider.kind(), provider);
    }

    pub fn get(&self, kind: &ProviderKind) -> Option<&dyn DesignProvider> {
        self.providers.get(kind).map(|b| b.as_ref())
    }

    pub fn available(&self) -> Vec<ProviderKind> {
        self.providers.keys().cloned().collect()
    }
}

impl Default for DesignProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Error ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignError {
    pub code: String,
    pub message: String,
    pub provider: Option<ProviderKind>,
}

impl DesignError {
    pub fn new(code: &str, msg: &str) -> Self {
        Self { code: code.to_string(), message: msg.to_string(), provider: None }
    }
    pub fn for_provider(code: &str, msg: &str, provider: ProviderKind) -> Self {
        Self { code: code.to_string(), message: msg.to_string(), provider: Some(provider) }
    }
}

impl std::fmt::Display for DesignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{:x}{:04x}", t.as_secs(), t.subsec_micros() & 0xffff)
}

fn epoch_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Code generation helpers ──────────────────────────────────────────────────

/// Convert a list of DesignTokens to CSS custom properties
pub fn tokens_to_css(tokens: &[DesignToken]) -> String {
    let mut css = String::from(":root {\n");
    for t in tokens {
        let var_name = t.name.to_lowercase().replace([' ', '/'], "-");
        css.push_str(&format!("  --{}: {};\n", var_name, t.value));
    }
    css.push('}');
    css
}

/// Convert tokens to a TypeScript design token object
pub fn tokens_to_ts(tokens: &[DesignToken]) -> String {
    let mut ts = String::from("export const tokens = {\n");
    let mut categories: HashMap<String, Vec<&DesignToken>> = HashMap::new();
    for t in tokens {
        categories
            .entry(format!("{:?}", t.token_type).to_lowercase())
            .or_default()
            .push(t);
    }
    for (cat, toks) in &categories {
        ts.push_str(&format!("  {}: {{\n", cat));
        for t in toks {
            let key = t.name.replace(['-', ' '], "_");
            ts.push_str(&format!("    {}: \"{}\",\n", key, t.value));
        }
        ts.push_str("  },\n");
    }
    ts.push_str("} as const;\n");
    ts
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_kind_display() {
        assert_eq!(ProviderKind::Figma.display_name(), "Figma");
        assert_eq!(ProviderKind::DrawIo.display_name(), "Draw.io");
        assert_eq!(ProviderKind::Penpot.display_name(), "Penpot");
    }

    #[test]
    fn diagram_kind_preferred_format() {
        assert_eq!(DiagramKind::Flowchart.preferred_format(), DiagramFormat::MermaidMd);
        assert_eq!(DiagramKind::C4Context.preferred_format(), DiagramFormat::C4Dsl);
        assert_eq!(DiagramKind::Architecture.preferred_format(), DiagramFormat::DrawIoXml);
    }

    #[test]
    fn tokens_to_css_roundtrip() {
        let tokens = vec![
            DesignToken {
                name: "Primary Blue".to_string(),
                token_type: DesignTokenType::Color,
                value: "#3b82f6".to_string(),
                description: None,
                provider: ProviderKind::Inhouse,
            },
        ];
        let css = tokens_to_css(&tokens);
        assert!(css.contains("--primary-blue: #3b82f6;"));
    }

    #[test]
    fn diagram_doc_has_id() {
        let doc = DiagramDoc::new(
            "Test Flow",
            DiagramKind::Flowchart,
            "flowchart TD\n  A-->B".to_string(),
            ProviderKind::Mermaid,
        );
        assert!(doc.id.starts_with("diag-"));
        assert_eq!(doc.title, "Test Flow");
    }

    #[test]
    fn registry_register_and_retrieve() {
        let registry = DesignProviderRegistry::new();
        assert!(registry.get(&ProviderKind::Figma).is_none());
        assert!(registry.available().is_empty());
    }

    #[test]
    fn design_error_display() {
        let e = DesignError::new("NOT_FOUND", "file not found");
        assert_eq!(e.to_string(), "[NOT_FOUND] file not found");
    }
}
