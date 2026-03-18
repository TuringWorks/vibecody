//! Auto-documentation wiki generator — analyze codebase and produce docs.
//!
//! Closes P1 Gap 6: Generate and maintain project documentation from
//! codebase analysis automatically (Devin Wiki style).
//!
//! # Features
//!
//! - Detect API endpoints, public interfaces, data models, configuration
//! - Generate markdown documentation with index page
//! - Track staleness and suggest updates on code changes
//! - Output to configurable directory (default: `docs/wiki/`)

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Document types
// ---------------------------------------------------------------------------

/// Type of documentation page.
#[derive(Debug, Clone, PartialEq)]
pub enum DocKind {
    /// Project overview / index
    Index,
    /// API endpoint documentation
    ApiEndpoint,
    /// Module / crate documentation
    Module,
    /// Data model / struct documentation
    DataModel,
    /// Configuration reference
    Configuration,
    /// Architecture overview
    Architecture,
    /// Getting started guide
    GettingStarted,
    /// CLI command reference
    CliReference,
    /// Custom page
    Custom(String),
}

impl DocKind {
    pub fn as_str(&self) -> &str {
        match self {
            DocKind::Index => "index",
            DocKind::ApiEndpoint => "api_endpoint",
            DocKind::Module => "module",
            DocKind::DataModel => "data_model",
            DocKind::Configuration => "configuration",
            DocKind::Architecture => "architecture",
            DocKind::GettingStarted => "getting_started",
            DocKind::CliReference => "cli_reference",
            DocKind::Custom(name) => name,
        }
    }
}

/// Staleness level of a documentation page.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Freshness {
    /// Docs match current code
    Fresh,
    /// Minor changes since last generation
    SlightlyStale,
    /// Significant changes — update recommended
    Stale,
    /// Source files deleted or heavily refactored
    Outdated,
}

impl Freshness {
    pub fn as_str(&self) -> &str {
        match self {
            Freshness::Fresh => "fresh",
            Freshness::SlightlyStale => "slightly_stale",
            Freshness::Stale => "stale",
            Freshness::Outdated => "outdated",
        }
    }
}

// ---------------------------------------------------------------------------
// Documentation page
// ---------------------------------------------------------------------------

/// A single documentation page.
#[derive(Debug, Clone)]
pub struct DocPage {
    pub title: String,
    pub slug: String,
    pub kind: DocKind,
    pub content: String,
    pub source_files: Vec<String>,
    pub generated_at: u64,
    pub freshness: Freshness,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub tags: Vec<String>,
}

impl DocPage {
    pub fn new(title: &str, kind: DocKind) -> Self {
        let slug = title
            .to_lowercase()
            .replace(' ', "-")
            .replace(|c: char| !c.is_alphanumeric() && c != '-', "");
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            title: title.to_string(),
            slug,
            kind,
            content: String::new(),
            source_files: Vec::new(),
            generated_at: ts,
            freshness: Freshness::Fresh,
            parent: None,
            children: Vec::new(),
            tags: Vec::new(),
        }
    }

    pub fn with_content(mut self, content: &str) -> Self {
        self.content = content.to_string();
        self
    }

    pub fn with_source(mut self, file: &str) -> Self {
        self.source_files.push(file.to_string());
        self
    }

    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    pub fn with_parent(mut self, parent: &str) -> Self {
        self.parent = Some(parent.to_string());
        self
    }

    pub fn add_child(&mut self, child_slug: &str) {
        self.children.push(child_slug.to_string());
    }

    /// Render as markdown file content.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# {}\n\n", self.title));
        if !self.tags.is_empty() {
            md.push_str(&format!(
                "**Tags**: {}\n\n",
                self.tags.join(", ")
            ));
        }
        md.push_str(&self.content);
        if !self.source_files.is_empty() {
            md.push_str("\n\n---\n\n**Source files**:\n");
            for f in &self.source_files {
                md.push_str(&format!("- `{}`\n", f));
            }
        }
        md
    }

    pub fn filename(&self) -> String {
        format!("{}.md", self.slug)
    }

    pub fn word_count(&self) -> usize {
        self.content.split_whitespace().count()
    }
}

// ---------------------------------------------------------------------------
// Detected code elements
// ---------------------------------------------------------------------------

/// An API endpoint detected in source code.
#[derive(Debug, Clone)]
pub struct DetectedEndpoint {
    pub method: String,
    pub path: String,
    pub handler: String,
    pub source_file: String,
    pub line: usize,
    pub description: Option<String>,
}

/// A public interface detected in source code.
#[derive(Debug, Clone)]
pub struct DetectedInterface {
    pub name: String,
    pub kind: String, // "trait", "struct", "class", "interface"
    pub methods: Vec<String>,
    pub source_file: String,
    pub line: usize,
    pub visibility: String,
}

/// A configuration option detected in source code.
#[derive(Debug, Clone)]
pub struct DetectedConfig {
    pub key: String,
    pub value_type: String,
    pub default_value: Option<String>,
    pub description: Option<String>,
    pub source_file: String,
}

// ---------------------------------------------------------------------------
// Wiki generator
// ---------------------------------------------------------------------------

/// Configuration for the documentation generator.
#[derive(Debug, Clone)]
pub struct DocGenConfig {
    pub output_dir: PathBuf,
    pub project_name: String,
    pub include_private: bool,
    pub generate_index: bool,
    pub detect_endpoints: bool,
    pub detect_models: bool,
    pub detect_config: bool,
}

impl Default for DocGenConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("docs/wiki"),
            project_name: "Project".to_string(),
            include_private: false,
            generate_index: true,
            detect_endpoints: true,
            detect_models: true,
            detect_config: true,
        }
    }
}

/// The wiki generator engine.
pub struct WikiGenerator {
    config: DocGenConfig,
    pages: Vec<DocPage>,
    endpoints: Vec<DetectedEndpoint>,
    interfaces: Vec<DetectedInterface>,
    configs: Vec<DetectedConfig>,
}

impl WikiGenerator {
    pub fn new(config: DocGenConfig) -> Self {
        Self {
            config,
            pages: Vec::new(),
            endpoints: Vec::new(),
            interfaces: Vec::new(),
            configs: Vec::new(),
        }
    }

    pub fn config(&self) -> &DocGenConfig {
        &self.config
    }

    // -- Element registration --

    pub fn add_endpoint(&mut self, endpoint: DetectedEndpoint) {
        self.endpoints.push(endpoint);
    }

    pub fn add_interface(&mut self, interface: DetectedInterface) {
        self.interfaces.push(interface);
    }

    pub fn add_config(&mut self, config: DetectedConfig) {
        self.configs.push(config);
    }

    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    pub fn interface_count(&self) -> usize {
        self.interfaces.len()
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }

    // -- Page management --

    pub fn add_page(&mut self, page: DocPage) {
        self.pages.push(page);
    }

    pub fn get_page(&self, slug: &str) -> Option<&DocPage> {
        self.pages.iter().find(|p| p.slug == slug)
    }

    pub fn list_pages(&self) -> &[DocPage] {
        &self.pages
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    // -- Generation --

    /// Generate documentation pages from detected elements.
    pub fn generate(&mut self) {
        // Generate index page
        if self.config.generate_index {
            let index = self.generate_index_page();
            self.pages.push(index);
        }

        // Generate API endpoints page
        if self.config.detect_endpoints && !self.endpoints.is_empty() {
            let api_page = self.generate_api_page();
            self.pages.push(api_page);
        }

        // Generate data models page
        if self.config.detect_models && !self.interfaces.is_empty() {
            let models_page = self.generate_models_page();
            self.pages.push(models_page);
        }

        // Generate configuration page
        if self.config.detect_config && !self.configs.is_empty() {
            let config_page = self.generate_config_page();
            self.pages.push(config_page);
        }
    }

    fn generate_index_page(&self) -> DocPage {
        let mut content = format!(
            "Welcome to the {} documentation wiki.\n\n## Pages\n\n",
            self.config.project_name
        );
        if !self.endpoints.is_empty() {
            content.push_str(&format!(
                "- [API Endpoints](api-endpoints.md) ({} endpoints)\n",
                self.endpoints.len()
            ));
        }
        if !self.interfaces.is_empty() {
            content.push_str(&format!(
                "- [Data Models](data-models.md) ({} interfaces)\n",
                self.interfaces.len()
            ));
        }
        if !self.configs.is_empty() {
            content.push_str(&format!(
                "- [Configuration](configuration.md) ({} options)\n",
                self.configs.len()
            ));
        }
        DocPage::new("Index", DocKind::Index).with_content(&content)
    }

    fn generate_api_page(&self) -> DocPage {
        let mut content = String::from("## API Endpoints\n\n");
        content.push_str("| Method | Path | Handler | Source |\n");
        content.push_str("|--------|------|---------|--------|\n");
        for ep in &self.endpoints {
            content.push_str(&format!(
                "| {} | `{}` | `{}` | `{}:{}` |\n",
                ep.method, ep.path, ep.handler, ep.source_file, ep.line
            ));
        }
        let source_files: Vec<String> = self
            .endpoints
            .iter()
            .map(|e| e.source_file.clone())
            .collect();
        let mut page = DocPage::new("API Endpoints", DocKind::ApiEndpoint)
            .with_content(&content)
            .with_tag("api");
        for f in &source_files {
            page = page.with_source(f);
        }
        page
    }

    fn generate_models_page(&self) -> DocPage {
        let mut content = String::from("## Data Models\n\n");
        for iface in &self.interfaces {
            content.push_str(&format!(
                "### {} ({})\n\n",
                iface.name, iface.kind
            ));
            content.push_str(&format!(
                "**Source**: `{}:{}`  \n**Visibility**: {}\n\n",
                iface.source_file, iface.line, iface.visibility
            ));
            if !iface.methods.is_empty() {
                content.push_str("**Methods**:\n");
                for m in &iface.methods {
                    content.push_str(&format!("- `{}`\n", m));
                }
            }
            content.push('\n');
        }
        DocPage::new("Data Models", DocKind::DataModel)
            .with_content(&content)
            .with_tag("models")
    }

    fn generate_config_page(&self) -> DocPage {
        let mut content = String::from("## Configuration Reference\n\n");
        content.push_str("| Key | Type | Default | Description |\n");
        content.push_str("|-----|------|---------|-------------|\n");
        for cfg in &self.configs {
            content.push_str(&format!(
                "| `{}` | {} | {} | {} |\n",
                cfg.key,
                cfg.value_type,
                cfg.default_value.as_deref().unwrap_or("-"),
                cfg.description.as_deref().unwrap_or("-"),
            ));
        }
        DocPage::new("Configuration", DocKind::Configuration)
            .with_content(&content)
            .with_tag("config")
    }

    // -- Staleness check --

    /// Check freshness of all pages based on source file modifications.
    pub fn check_freshness(&mut self, modified_files: &[String]) {
        for page in &mut self.pages {
            let affected = page
                .source_files
                .iter()
                .any(|f| modified_files.contains(f));
            if affected {
                page.freshness = Freshness::Stale;
            }
        }
    }

    /// Get stale pages that need regeneration.
    pub fn stale_pages(&self) -> Vec<&DocPage> {
        self.pages
            .iter()
            .filter(|p| p.freshness >= Freshness::Stale)
            .collect()
    }

    pub fn stats(&self) -> WikiStats {
        WikiStats {
            page_count: self.pages.len(),
            total_words: self.pages.iter().map(|p| p.word_count()).sum(),
            endpoint_count: self.endpoints.len(),
            interface_count: self.interfaces.len(),
            config_count: self.configs.len(),
            stale_count: self.stale_pages().len(),
        }
    }

    /// Get all generated file paths.
    pub fn output_files(&self) -> Vec<PathBuf> {
        self.pages
            .iter()
            .map(|p| self.config.output_dir.join(p.filename()))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct WikiStats {
    pub page_count: usize,
    pub total_words: usize,
    pub endpoint_count: usize,
    pub interface_count: usize,
    pub config_count: usize,
    pub stale_count: usize,
}

// ---------------------------------------------------------------------------
// Source file analyzer (simple patterns)
// ---------------------------------------------------------------------------

/// Extract API endpoints from source code (simple pattern matching).
pub fn extract_endpoints(source: &str, file_path: &str) -> Vec<DetectedEndpoint> {
    let mut endpoints = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        // Detect common patterns: .get("/path"), .post("/path"), route("/path")
        for method in &["get", "post", "put", "delete", "patch"] {
            let patterns = [
                format!(".{}(\"", method),
                format!(".{}(\"/", method),
            ];
            for pattern in &patterns {
                if let Some(idx) = trimmed.to_lowercase().find(pattern) {
                    let after = &trimmed[idx + pattern.len()..];
                    if let Some(end) = after.find('"') {
                        let path_part = &after[..end];
                        let full_path = if pattern.contains("/") {
                            format!("/{}", path_part)
                        } else {
                            path_part.to_string()
                        };
                        endpoints.push(DetectedEndpoint {
                            method: method.to_uppercase(),
                            path: full_path,
                            handler: format!("line_{}", i + 1),
                            source_file: file_path.to_string(),
                            line: i + 1,
                            description: None,
                        });
                    }
                }
            }
        }
    }
    endpoints
}

/// Extract public structs/traits/interfaces from source code.
pub fn extract_interfaces(source: &str, file_path: &str) -> Vec<DetectedInterface> {
    let mut interfaces = Vec::new();
    for (i, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        // Detect pub struct, pub trait, pub enum, pub fn
        for (keyword, kind) in &[
            ("pub struct ", "struct"),
            ("pub trait ", "trait"),
            ("pub enum ", "enum"),
            ("pub fn ", "function"),
            ("export interface ", "interface"),
            ("export class ", "class"),
        ] {
            if let Some(rest) = trimmed.strip_prefix(keyword) {
                let name_end = rest.find([' ', '{', '(', '<', ':']).unwrap_or(rest.len());
                let name = rest[..name_end].to_string();
                if !name.is_empty() {
                    interfaces.push(DetectedInterface {
                        name,
                        kind: kind.to_string(),
                        methods: Vec::new(),
                        source_file: file_path.to_string(),
                        line: i + 1,
                        visibility: "public".to_string(),
                    });
                }
            }
        }
    }
    interfaces
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_kind_as_str() {
        assert_eq!(DocKind::Index.as_str(), "index");
        assert_eq!(DocKind::ApiEndpoint.as_str(), "api_endpoint");
        assert_eq!(DocKind::Module.as_str(), "module");
        assert_eq!(DocKind::DataModel.as_str(), "data_model");
        assert_eq!(DocKind::Configuration.as_str(), "configuration");
        assert_eq!(DocKind::Architecture.as_str(), "architecture");
        assert_eq!(DocKind::CliReference.as_str(), "cli_reference");
        assert_eq!(DocKind::Custom("x".into()).as_str(), "x");
    }

    #[test]
    fn test_freshness_ordering() {
        assert!(Freshness::Fresh < Freshness::SlightlyStale);
        assert!(Freshness::SlightlyStale < Freshness::Stale);
        assert!(Freshness::Stale < Freshness::Outdated);
    }

    #[test]
    fn test_freshness_as_str() {
        assert_eq!(Freshness::Fresh.as_str(), "fresh");
        assert_eq!(Freshness::Stale.as_str(), "stale");
    }

    #[test]
    fn test_doc_page_new() {
        let page = DocPage::new("API Reference", DocKind::ApiEndpoint);
        assert_eq!(page.title, "API Reference");
        assert_eq!(page.slug, "api-reference");
        assert_eq!(page.kind, DocKind::ApiEndpoint);
        assert_eq!(page.freshness, Freshness::Fresh);
    }

    #[test]
    fn test_doc_page_slug_special_chars() {
        let page = DocPage::new("C++ Guide (v2)", DocKind::Module);
        assert_eq!(page.slug, "c-guide-v2");
    }

    #[test]
    fn test_doc_page_with_content() {
        let page = DocPage::new("Test", DocKind::Index)
            .with_content("Hello world")
            .with_source("main.rs")
            .with_tag("intro");
        assert_eq!(page.content, "Hello world");
        assert_eq!(page.source_files.len(), 1);
        assert_eq!(page.tags.len(), 1);
    }

    #[test]
    fn test_doc_page_with_parent() {
        let page = DocPage::new("Sub", DocKind::Module).with_parent("parent-slug");
        assert_eq!(page.parent.as_deref(), Some("parent-slug"));
    }

    #[test]
    fn test_doc_page_add_child() {
        let mut page = DocPage::new("Parent", DocKind::Index);
        page.add_child("child-1");
        page.add_child("child-2");
        assert_eq!(page.children.len(), 2);
    }

    #[test]
    fn test_doc_page_to_markdown() {
        let page = DocPage::new("Test Page", DocKind::Module)
            .with_content("Some content here.")
            .with_source("lib.rs")
            .with_tag("core");
        let md = page.to_markdown();
        assert!(md.contains("# Test Page"));
        assert!(md.contains("Some content here."));
        assert!(md.contains("`lib.rs`"));
        assert!(md.contains("core"));
    }

    #[test]
    fn test_doc_page_filename() {
        let page = DocPage::new("API Endpoints", DocKind::ApiEndpoint);
        assert_eq!(page.filename(), "api-endpoints.md");
    }

    #[test]
    fn test_doc_page_word_count() {
        let page = DocPage::new("Test", DocKind::Index)
            .with_content("one two three four five");
        assert_eq!(page.word_count(), 5);
    }

    #[test]
    fn test_docgen_config_default() {
        let cfg = DocGenConfig::default();
        assert_eq!(cfg.output_dir, PathBuf::from("docs/wiki"));
        assert!(!cfg.include_private);
        assert!(cfg.generate_index);
        assert!(cfg.detect_endpoints);
    }

    #[test]
    fn test_wiki_generator_new() {
        let gen = WikiGenerator::new(DocGenConfig::default());
        assert_eq!(gen.page_count(), 0);
        assert_eq!(gen.endpoint_count(), 0);
    }

    #[test]
    fn test_wiki_generator_add_elements() {
        let mut gen = WikiGenerator::new(DocGenConfig::default());
        gen.add_endpoint(DetectedEndpoint {
            method: "GET".into(),
            path: "/health".into(),
            handler: "health_check".into(),
            source_file: "serve.rs".into(),
            line: 10,
            description: None,
        });
        gen.add_interface(DetectedInterface {
            name: "Config".into(),
            kind: "struct".into(),
            methods: vec![],
            source_file: "config.rs".into(),
            line: 5,
            visibility: "pub".into(),
        });
        gen.add_config(DetectedConfig {
            key: "port".into(),
            value_type: "u16".into(),
            default_value: Some("7878".into()),
            description: Some("Server port".into()),
            source_file: "config.rs".into(),
        });
        assert_eq!(gen.endpoint_count(), 1);
        assert_eq!(gen.interface_count(), 1);
        assert_eq!(gen.config_count(), 1);
    }

    #[test]
    fn test_wiki_generator_generate() {
        let mut gen = WikiGenerator::new(DocGenConfig::default());
        gen.add_endpoint(DetectedEndpoint {
            method: "GET".into(), path: "/health".into(), handler: "h".into(),
            source_file: "s.rs".into(), line: 1, description: None,
        });
        gen.add_interface(DetectedInterface {
            name: "Config".into(), kind: "struct".into(), methods: vec!["new".into()],
            source_file: "c.rs".into(), line: 1, visibility: "pub".into(),
        });
        gen.add_config(DetectedConfig {
            key: "port".into(), value_type: "u16".into(),
            default_value: Some("8080".into()), description: Some("Port".into()),
            source_file: "c.rs".into(),
        });
        gen.generate();
        // index + api + models + config = 4 pages
        assert_eq!(gen.page_count(), 4);
        assert!(gen.get_page("index").is_some());
        assert!(gen.get_page("api-endpoints").is_some());
        assert!(gen.get_page("data-models").is_some());
        assert!(gen.get_page("configuration").is_some());
    }

    #[test]
    fn test_wiki_generator_generate_index_only() {
        let mut gen = WikiGenerator::new(DocGenConfig::default());
        gen.generate();
        assert_eq!(gen.page_count(), 1); // Just the index
    }

    #[test]
    fn test_check_freshness() {
        let mut gen = WikiGenerator::new(DocGenConfig::default());
        gen.add_page(
            DocPage::new("API", DocKind::ApiEndpoint).with_source("serve.rs"),
        );
        gen.add_page(
            DocPage::new("Config", DocKind::Configuration).with_source("config.rs"),
        );
        gen.check_freshness(&["serve.rs".to_string()]);
        assert_eq!(gen.get_page("api").unwrap().freshness, Freshness::Stale);
        assert_eq!(gen.get_page("config").unwrap().freshness, Freshness::Fresh);
    }

    #[test]
    fn test_stale_pages() {
        let mut gen = WikiGenerator::new(DocGenConfig::default());
        gen.add_page(DocPage::new("Fresh", DocKind::Module));
        let mut stale = DocPage::new("Stale", DocKind::Module);
        stale.freshness = Freshness::Stale;
        gen.add_page(stale);
        assert_eq!(gen.stale_pages().len(), 1);
    }

    #[test]
    fn test_stats() {
        let mut gen = WikiGenerator::new(DocGenConfig::default());
        gen.add_page(DocPage::new("Test", DocKind::Index).with_content("word1 word2"));
        gen.add_endpoint(DetectedEndpoint {
            method: "GET".into(), path: "/".into(), handler: "h".into(),
            source_file: "s.rs".into(), line: 1, description: None,
        });
        let stats = gen.stats();
        assert_eq!(stats.page_count, 1);
        assert_eq!(stats.total_words, 2);
        assert_eq!(stats.endpoint_count, 1);
    }

    #[test]
    fn test_output_files() {
        let mut gen = WikiGenerator::new(DocGenConfig::default());
        gen.add_page(DocPage::new("API Ref", DocKind::ApiEndpoint));
        let files = gen.output_files();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], PathBuf::from("docs/wiki/api-ref.md"));
    }

    #[test]
    fn test_extract_endpoints() {
        let source = r#"
            let app = Router::new()
                .get("/health", health_handler)
                .post("/chat", chat_handler)
                .put("/config", update_config);
        "#;
        let eps = extract_endpoints(source, "serve.rs");
        assert!(eps.len() >= 3);
    }

    #[test]
    fn test_extract_endpoints_empty() {
        let eps = extract_endpoints("let x = 42;", "main.rs");
        assert!(eps.is_empty());
    }

    #[test]
    fn test_extract_interfaces_rust() {
        let source = "pub struct Config {\n    pub port: u16,\n}\npub trait Provider {\n}\npub fn main() {}";
        let ifaces = extract_interfaces(source, "main.rs");
        assert!(ifaces.iter().any(|i| i.name == "Config" && i.kind == "struct"));
        assert!(ifaces.iter().any(|i| i.name == "Provider" && i.kind == "trait"));
        assert!(ifaces.iter().any(|i| i.name == "main" && i.kind == "function"));
    }

    #[test]
    fn test_extract_interfaces_typescript() {
        let source = "export interface User {\n  name: string;\n}\nexport class Service {}";
        let ifaces = extract_interfaces(source, "types.ts");
        assert!(ifaces.iter().any(|i| i.name == "User" && i.kind == "interface"));
        assert!(ifaces.iter().any(|i| i.name == "Service" && i.kind == "class"));
    }

    #[test]
    fn test_extract_interfaces_empty() {
        let ifaces = extract_interfaces("// just a comment", "file.rs");
        assert!(ifaces.is_empty());
    }

    #[test]
    fn test_api_page_contains_endpoints() {
        let mut gen = WikiGenerator::new(DocGenConfig::default());
        gen.add_endpoint(DetectedEndpoint {
            method: "GET".into(), path: "/health".into(), handler: "health_check".into(),
            source_file: "serve.rs".into(), line: 42, description: None,
        });
        gen.generate();
        let api_page = gen.get_page("api-endpoints").unwrap();
        assert!(api_page.content.contains("GET"));
        assert!(api_page.content.contains("/health"));
        assert!(api_page.content.contains("health_check"));
    }

    #[test]
    fn test_models_page_contains_interfaces() {
        let mut gen = WikiGenerator::new(DocGenConfig::default());
        gen.add_interface(DetectedInterface {
            name: "Config".into(), kind: "struct".into(), methods: vec!["new".into(), "load".into()],
            source_file: "config.rs".into(), line: 10, visibility: "public".into(),
        });
        gen.generate();
        let page = gen.get_page("data-models").unwrap();
        assert!(page.content.contains("Config"));
        assert!(page.content.contains("`new`"));
        assert!(page.content.contains("`load`"));
    }

    #[test]
    fn test_config_page_contains_options() {
        let mut gen = WikiGenerator::new(DocGenConfig::default());
        gen.add_config(DetectedConfig {
            key: "server.port".into(), value_type: "u16".into(),
            default_value: Some("7878".into()), description: Some("HTTP port".into()),
            source_file: "config.rs".into(),
        });
        gen.generate();
        let page = gen.get_page("configuration").unwrap();
        assert!(page.content.contains("server.port"));
        assert!(page.content.contains("7878"));
        assert!(page.content.contains("HTTP port"));
    }
}
