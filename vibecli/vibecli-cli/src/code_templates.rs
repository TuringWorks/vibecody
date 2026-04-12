#![allow(dead_code)]
//! Code generation templates — parameterized scaffolding for common patterns.
//! Matches GitHub Copilot Workspace v2's snippet/template feature.
//!
//! Templates support `{{variable}}` placeholders and nested template composition.
//! A `TemplateRegistry` holds named templates and can render them with a variable map.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single named template with a body containing `{{var}}` placeholders.
#[derive(Debug, Clone)]
pub struct Template {
    pub name: String,
    pub description: String,
    pub body: String,
    pub language: TemplateLanguage,
    pub variables: Vec<TemplateVar>,
}

/// Metadata for a template variable.
#[derive(Debug, Clone)]
pub struct TemplateVar {
    pub name: String,
    pub description: String,
    pub default: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateLanguage {
    Rust,
    TypeScript,
    Python,
    Go,
    Markdown,
    Any,
}

impl std::fmt::Display for TemplateLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateLanguage::Rust => write!(f, "rust"),
            TemplateLanguage::TypeScript => write!(f, "typescript"),
            TemplateLanguage::Python => write!(f, "python"),
            TemplateLanguage::Go => write!(f, "go"),
            TemplateLanguage::Markdown => write!(f, "markdown"),
            TemplateLanguage::Any => write!(f, "any"),
        }
    }
}

/// Result of rendering a template.
#[derive(Debug, Clone)]
pub struct RenderResult {
    pub content: String,
    pub missing_vars: Vec<String>,
    pub used_defaults: Vec<String>,
}

impl RenderResult {
    pub fn is_complete(&self) -> bool {
        self.missing_vars.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

/// Renders templates by substituting `{{variable}}` placeholders.
pub struct TemplateRenderer;

impl TemplateRenderer {
    /// Render `template` with `vars` map.
    pub fn render(template: &Template, vars: &HashMap<String, String>) -> RenderResult {
        let mut output = template.body.clone();
        let mut missing: Vec<String> = Vec::new();
        let mut used_defaults: Vec<String> = Vec::new();

        for var in &template.variables {
            let placeholder = format!("{{{{{}}}}}", var.name);
            if let Some(val) = vars.get(&var.name) {
                output = output.replace(&placeholder, val);
            } else if let Some(default) = &var.default {
                output = output.replace(&placeholder, default);
                used_defaults.push(var.name.clone());
            } else if var.required {
                missing.push(var.name.clone());
            } else {
                // Optional with no default: replace with empty string
                output = output.replace(&placeholder, "");
            }
        }

        RenderResult {
            content: output,
            missing_vars: missing,
            used_defaults,
        }
    }

    /// Extract all `{{var}}` placeholder names from a template body.
    pub fn extract_vars(body: &str) -> Vec<String> {
        let mut vars = Vec::new();
        let mut rest = body;
        while let Some(start) = rest.find("{{") {
            rest = &rest[start + 2..];
            if let Some(end) = rest.find("}}") {
                let name = rest[..end].trim().to_string();
                if !name.is_empty() && !vars.contains(&name) {
                    vars.push(name);
                }
                rest = &rest[end + 2..];
            } else {
                break;
            }
        }
        vars
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Holds a collection of named templates.
pub struct TemplateRegistry {
    templates: HashMap<String, Template>,
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        let mut reg = Self { templates: HashMap::new() };
        reg.load_builtins();
        reg
    }
}

impl TemplateRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn empty() -> Self {
        Self { templates: HashMap::new() }
    }

    pub fn register(&mut self, template: Template) {
        self.templates.insert(template.name.clone(), template);
    }

    pub fn get(&self, name: &str) -> Option<&Template> {
        self.templates.get(name)
    }

    pub fn list(&self) -> Vec<&Template> {
        let mut templates: Vec<&Template> = self.templates.values().collect();
        templates.sort_by(|a, b| a.name.cmp(&b.name));
        templates
    }

    pub fn list_by_language(&self, lang: &TemplateLanguage) -> Vec<&Template> {
        self.list().into_iter()
            .filter(|t| &t.language == lang || t.language == TemplateLanguage::Any)
            .collect()
    }

    /// Render a named template with the given variables.
    pub fn render(&self, name: &str, vars: &HashMap<String, String>) -> Result<RenderResult, String> {
        let template = self.get(name)
            .ok_or_else(|| format!("Template `{}` not found", name))?;
        Ok(TemplateRenderer::render(template, vars))
    }

    fn load_builtins(&mut self) {
        // Rust: new struct
        self.register(Template {
            name: "rust-struct".into(),
            description: "New Rust struct with optional derive macros".into(),
            language: TemplateLanguage::Rust,
            body: "#[derive({{derives}})]\npub struct {{name}} {\n    {{fields}}\n}\n".into(),
            variables: vec![
                TemplateVar { name: "name".into(), description: "Struct name".into(), default: Some("MyStruct".into()), required: true },
                TemplateVar { name: "derives".into(), description: "Derive macros".into(), default: Some("Debug, Clone".into()), required: false },
                TemplateVar { name: "fields".into(), description: "Struct fields".into(), default: Some("// fields here".into()), required: false },
            ],
        });

        // Rust: new enum
        self.register(Template {
            name: "rust-enum".into(),
            description: "New Rust enum".into(),
            language: TemplateLanguage::Rust,
            body: "#[derive({{derives}})]\npub enum {{name}} {\n    {{variants}}\n}\n".into(),
            variables: vec![
                TemplateVar { name: "name".into(), description: "Enum name".into(), default: Some("MyEnum".into()), required: true },
                TemplateVar { name: "derives".into(), description: "Derive macros".into(), default: Some("Debug, Clone, PartialEq".into()), required: false },
                TemplateVar { name: "variants".into(), description: "Enum variants".into(), default: Some("Variant1,\n    Variant2,".into()), required: false },
            ],
        });

        // Rust: Tauri command
        self.register(Template {
            name: "rust-tauri-command".into(),
            description: "New Tauri async command".into(),
            language: TemplateLanguage::Rust,
            body: "#[tauri::command]\npub async fn {{name}}({{params}}) -> Result<{{return_type}}, String> {\n    {{body}}\n}\n".into(),
            variables: vec![
                TemplateVar { name: "name".into(), description: "Command name".into(), default: None, required: true },
                TemplateVar { name: "params".into(), description: "Parameters".into(), default: Some("".into()), required: false },
                TemplateVar { name: "return_type".into(), description: "Return type".into(), default: Some("serde_json::Value".into()), required: false },
                TemplateVar { name: "body".into(), description: "Function body".into(), default: Some("todo!()".into()), required: false },
            ],
        });

        // Rust: test module
        self.register(Template {
            name: "rust-test-module".into(),
            description: "Rust test module scaffold".into(),
            language: TemplateLanguage::Rust,
            body: "#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn test_{{fn_name}}() {\n        // arrange\n        {{arrange}}\n        // act\n        {{act}}\n        // assert\n        {{assert}}\n    }\n}\n".into(),
            variables: vec![
                TemplateVar { name: "fn_name".into(), description: "Test function name".into(), default: Some("placeholder".into()), required: true },
                TemplateVar { name: "arrange".into(), description: "Arrange step".into(), default: Some("let _ = ();".into()), required: false },
                TemplateVar { name: "act".into(), description: "Act step".into(), default: Some("let result = todo!();".into()), required: false },
                TemplateVar { name: "assert".into(), description: "Assert step".into(), default: Some("assert!(true);".into()), required: false },
            ],
        });

        // TypeScript: React component
        self.register(Template {
            name: "ts-react-component".into(),
            description: "New React functional component".into(),
            language: TemplateLanguage::TypeScript,
            body: "import React from 'react';\n\ninterface {{name}}Props {\n    {{props}}\n}\n\nexport const {{name}}: React.FC<{{name}}Props> = ({ {{destructure}} }) => {\n    return (\n        <div className=\"{{class_name}}\">\n            {{children}}\n        </div>\n    );\n};\n\nexport default {{name}};\n".into(),
            variables: vec![
                TemplateVar { name: "name".into(), description: "Component name".into(), default: None, required: true },
                TemplateVar { name: "props".into(), description: "Props interface body".into(), default: Some("// props".into()), required: false },
                TemplateVar { name: "destructure".into(), description: "Destructured props".into(), default: Some("".into()), required: false },
                TemplateVar { name: "class_name".into(), description: "Root CSS class".into(), default: Some("container".into()), required: false },
                TemplateVar { name: "children".into(), description: "JSX content".into(), default: Some("{/* content */}".into()), required: false },
            ],
        });

        // TypeScript: async service function
        self.register(Template {
            name: "ts-async-service".into(),
            description: "Async TypeScript service function".into(),
            language: TemplateLanguage::TypeScript,
            body: "export async function {{name}}({{params}}): Promise<{{return_type}}> {\n    try {\n        {{body}}\n    } catch (error) {\n        throw new Error(`{{name}} failed: ${error}`);\n    }\n}\n".into(),
            variables: vec![
                TemplateVar { name: "name".into(), description: "Function name".into(), default: None, required: true },
                TemplateVar { name: "params".into(), description: "Parameters".into(), default: Some("".into()), required: false },
                TemplateVar { name: "return_type".into(), description: "Return type".into(), default: Some("void".into()), required: false },
                TemplateVar { name: "body".into(), description: "Function body".into(), default: Some("// implementation".into()), required: false },
            ],
        });

        // Markdown: ADR (Architecture Decision Record)
        self.register(Template {
            name: "md-adr".into(),
            description: "Architecture Decision Record".into(),
            language: TemplateLanguage::Markdown,
            body: "# ADR-{{number}}: {{title}}\n\n**Status**: {{status}}\n**Date**: {{date}}\n\n## Context\n\n{{context}}\n\n## Decision\n\n{{decision}}\n\n## Consequences\n\n{{consequences}}\n".into(),
            variables: vec![
                TemplateVar { name: "number".into(), description: "ADR number".into(), default: Some("001".into()), required: true },
                TemplateVar { name: "title".into(), description: "Decision title".into(), default: None, required: true },
                TemplateVar { name: "status".into(), description: "Proposed | Accepted | Deprecated".into(), default: Some("Proposed".into()), required: false },
                TemplateVar { name: "date".into(), description: "Decision date".into(), default: Some("2026-04-12".into()), required: false },
                TemplateVar { name: "context".into(), description: "Problem context".into(), default: Some("Describe the issue here.".into()), required: false },
                TemplateVar { name: "decision".into(), description: "Decision taken".into(), default: Some("We will...".into()), required: false },
                TemplateVar { name: "consequences".into(), description: "Trade-offs".into(), default: Some("Positive: ...\nNegative: ...".into()), required: false },
            ],
        });
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn test_extract_vars() {
        let body = "Hello {{name}}, you are {{age}} years old.";
        let found = TemplateRenderer::extract_vars(body);
        assert_eq!(found, vec!["name", "age"]);
    }

    #[test]
    fn test_render_all_provided() {
        let template = Template {
            name: "t".into(),
            description: "".into(),
            language: TemplateLanguage::Any,
            body: "fn {{name}}() -> {{return_type}} {}".into(),
            variables: vec![
                TemplateVar { name: "name".into(), description: "".into(), default: None, required: true },
                TemplateVar { name: "return_type".into(), description: "".into(), default: None, required: true },
            ],
        };
        let result = TemplateRenderer::render(&template, &vars(&[("name", "foo"), ("return_type", "i32")]));
        assert_eq!(result.content, "fn foo() -> i32 {}");
        assert!(result.is_complete());
    }

    #[test]
    fn test_render_uses_defaults() {
        let reg = TemplateRegistry::new();
        let result = reg.render("rust-struct", &vars(&[("name", "Config")])).unwrap();
        assert!(result.content.contains("pub struct Config"));
        assert!(result.content.contains("Debug, Clone")); // default derive
        assert!(!result.used_defaults.is_empty());
    }

    #[test]
    fn test_render_missing_required() {
        let template = Template {
            name: "t".into(),
            description: "".into(),
            language: TemplateLanguage::Any,
            body: "{{required_var}}".into(),
            variables: vec![
                TemplateVar { name: "required_var".into(), description: "".into(), default: None, required: true },
            ],
        };
        let result = TemplateRenderer::render(&template, &HashMap::new());
        assert!(!result.is_complete());
        assert!(result.missing_vars.contains(&"required_var".to_string()));
    }

    #[test]
    fn test_registry_builtin_count() {
        let reg = TemplateRegistry::new();
        assert!(reg.list().len() >= 7);
    }

    #[test]
    fn test_list_by_language() {
        let reg = TemplateRegistry::new();
        let rust = reg.list_by_language(&TemplateLanguage::Rust);
        assert!(rust.iter().all(|t| t.language == TemplateLanguage::Rust || t.language == TemplateLanguage::Any));
        assert!(rust.len() >= 4);
    }

    #[test]
    fn test_rust_tauri_command_template() {
        let reg = TemplateRegistry::new();
        let result = reg.render("rust-tauri-command", &vars(&[("name", "get_status")])).unwrap();
        assert!(result.content.contains("#[tauri::command]"));
        assert!(result.content.contains("pub async fn get_status"));
    }

    #[test]
    fn test_ts_react_component_template() {
        let reg = TemplateRegistry::new();
        let result = reg.render("ts-react-component", &vars(&[("name", "MyPanel")])).unwrap();
        assert!(result.content.contains("interface MyPanelProps"));
        assert!(result.content.contains("export const MyPanel"));
    }

    #[test]
    fn test_md_adr_template() {
        let reg = TemplateRegistry::new();
        let result = reg.render("md-adr", &vars(&[("number", "005"), ("title", "Use SQLite")])).unwrap();
        assert!(result.content.contains("ADR-005: Use SQLite"));
        assert!(result.content.contains("**Status**: Proposed"));
    }

    #[test]
    fn test_render_nonexistent_template() {
        let reg = TemplateRegistry::new();
        let result = reg.render("does-not-exist", &HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_register_custom() {
        let mut reg = TemplateRegistry::empty();
        reg.register(Template {
            name: "custom".into(),
            description: "test".into(),
            language: TemplateLanguage::Any,
            body: "hello {{who}}".into(),
            variables: vec![TemplateVar { name: "who".into(), description: "".into(), default: Some("world".into()), required: false }],
        });
        let result = reg.render("custom", &HashMap::new()).unwrap();
        assert_eq!(result.content, "hello world");
    }

    #[test]
    fn test_rust_test_module_template() {
        let reg = TemplateRegistry::new();
        let result = reg.render("rust-test-module", &vars(&[("fn_name", "my_function")])).unwrap();
        assert!(result.content.contains("fn test_my_function()"));
        assert!(result.content.contains("#[cfg(test)]"));
    }
}
