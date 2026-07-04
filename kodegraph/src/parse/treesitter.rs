//! Tree-sitter backbone parser — Tier 1, zero-config, always works.
//!
//! Walks the AST of a single file and extracts:
//! - **Symbols** (functions, structs, traits, enums, classes, interfaces, types, consts, modules).
//! - **Call edges** (caller = enclosing symbol, callee = call target tail).
//! - **Import edges** (target path + imported symbol tails).
//!
//! Edges are tagged `TreeSitter` / `Inferred` at confidence `0.7`. Cross-file
//! resolution of callees is best-effort here and upgraded by the LSP tier
//! (`EdgeProvider`) when available.

use std::path::Path;

use tree_sitter::{Node, Parser};

use crate::model::edge::{CallEdge, CallType, EdgeSource, ImportEdge, ImportType, Provenance,
                         TypeRelation};
use crate::model::symbol::{Language, Symbol, SymbolKind, Visibility};
use crate::parse::{language_of, ParsedFile, Parser as ParserTrait};

/// Tree-sitter node-kind → symbol mapping for one language.
struct KindSpec {
    kind: &'static str,
    symbol_kind: SymbolKind,
    /// Field name holding the symbol's name; if `None`, use the first identifier child.
    name_field: Option<&'static str>,
}

fn rust_specs() -> &'static [KindSpec] {
    &[
        KindSpec { kind: "function_item", symbol_kind: SymbolKind::Function, name_field: Some("name") },
        KindSpec { kind: "struct_item", symbol_kind: SymbolKind::Struct, name_field: Some("name") },
        KindSpec { kind: "enum_item", symbol_kind: SymbolKind::Enum, name_field: Some("name") },
        KindSpec { kind: "trait_item", symbol_kind: SymbolKind::Trait, name_field: Some("name") },
        KindSpec { kind: "type_item", symbol_kind: SymbolKind::TypeAlias, name_field: Some("name") },
        KindSpec { kind: "const_item", symbol_kind: SymbolKind::Constant, name_field: Some("name") },
        KindSpec { kind: "mod_item", symbol_kind: SymbolKind::Module, name_field: Some("name") },
        KindSpec { kind: "union_item", symbol_kind: SymbolKind::Struct, name_field: Some("name") },
    ]
}

fn ts_specs() -> &'static [KindSpec] {
    &[
        KindSpec { kind: "function_declaration", symbol_kind: SymbolKind::Function, name_field: Some("name") },
        KindSpec { kind: "class_declaration", symbol_kind: SymbolKind::Class, name_field: Some("name") },
        KindSpec { kind: "interface_declaration", symbol_kind: SymbolKind::Interface, name_field: Some("name") },
        KindSpec { kind: "enum_declaration", symbol_kind: SymbolKind::Enum, name_field: Some("name") },
        KindSpec { kind: "type_alias_declaration", symbol_kind: SymbolKind::TypeAlias, name_field: Some("name") },
        KindSpec { kind: "method_definition", symbol_kind: SymbolKind::Method, name_field: Some("name") },
        KindSpec { kind: "class", symbol_kind: SymbolKind::Class, name_field: None },
        KindSpec { kind: "function", symbol_kind: SymbolKind::Function, name_field: Some("name") },
    ]
}

fn py_specs() -> &'static [KindSpec] {
    &[
        KindSpec { kind: "function_definition", symbol_kind: SymbolKind::Function, name_field: Some("name") },
        KindSpec { kind: "class_definition", symbol_kind: SymbolKind::Class, name_field: Some("name") },
    ]
}

fn go_specs() -> &'static [KindSpec] {
    &[
        KindSpec { kind: "function_declaration", symbol_kind: SymbolKind::Function, name_field: Some("name") },
        KindSpec { kind: "method_declaration", symbol_kind: SymbolKind::Method, name_field: Some("name") },
    ]
}

fn specs_for(lang: Language) -> Option<&'static [KindSpec]> {
    match lang {
        Language::Rust => Some(rust_specs()),
        Language::TypeScript => Some(ts_specs()),
        Language::JavaScript => Some(ts_specs()),
        Language::Python => Some(py_specs()),
        Language::Go => Some(go_specs()),
        _ => None,
    }
}

fn call_kinds(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Rust => &["call_expression", "macro_invocation"],
        Language::TypeScript | Language::JavaScript => &["call_expression"],
        Language::Python => &["call"],
        Language::Go => &["call_expression"],
        _ => &[],
    }
}

fn import_kinds(lang: Language) -> &'static [&'static str] {
    match lang {
        Language::Rust => &["use_declaration"],
        Language::TypeScript | Language::JavaScript => &["import_statement"],
        Language::Python => &["import_statement", "import_from_statement"],
        Language::Go => &["import_declaration"],
        _ => &[],
    }
}

/// Tree-sitter parser for the v0.1 language set (Rust / TypeScript / Python / Go).
#[derive(Default, Debug, Clone)]
pub struct TreeSitterParser;

impl TreeSitterParser {
    /// Construct.
    pub fn new() -> Self {
        Self
    }

    fn ts_language(&self, lang: Language) -> Option<tree_sitter::Language> {
        // tree-sitter 0.23 grammars export a `LANGUAGE` const (LanguageFn) that
        // converts into `tree_sitter::Language` via `.into()`.
        match lang {
            Language::Rust => Some(tree_sitter_rust::LANGUAGE.into()),
            Language::TypeScript | Language::JavaScript => {
                Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            }
            Language::Python => Some(tree_sitter_python::LANGUAGE.into()),
            Language::Go => Some(tree_sitter_go::LANGUAGE.into()),
            _ => None,
        }
    }
}

impl ParserTrait for TreeSitterParser {
    fn supports(&self, lang: Language) -> bool {
        specs_for(lang).is_some()
    }

    fn parse_file(&self, path: &Path, src: &str, lang: Language) -> ParsedFile {
        let mut out = ParsedFile::default();
        if !lang.supported_by_treesitter() {
            return out;
        }
        let ts_lang = match self.ts_language(lang) {
            Some(l) => l,
            None => return out,
        };
        let mut parser = Parser::new();
        if parser.set_language(&ts_lang).is_err() {
            return out;
        }
        let tree = match parser.parse(src, None) {
            Some(t) => t,
            None => return out,
        };

        let root = tree.root_node();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("anon")
            .to_string();
        let prov = Provenance::from_source(EdgeSource::TreeSitter);
        let specs = specs_for(lang).unwrap_or(&[]);
        let calls = call_kinds(lang);
        let imports = import_kinds(lang);

        let mut ctx = WalkCtx {
            src,
            path_str: path.to_string_lossy().as_ref().to_string(),
            stem,
            lang,
            specs,
            call_kinds: calls,
            import_kinds: imports,
            provenance: prov,
            enclosing: Vec::new(),
            out: &mut out,
        };
        walk(&root, &mut ctx);
        out
    }
}

struct WalkCtx<'a> {
    src: &'a str,
    path_str: String,
    stem: String,
    lang: Language,
    specs: &'static [KindSpec],
    call_kinds: &'static [&'static str],
    import_kinds: &'static [&'static str],
    provenance: Provenance,
    enclosing: Vec<String>,
    out: &'a mut ParsedFile,
}

fn walk<'a>(node: &Node<'a>, ctx: &mut WalkCtx<'a>) {
    // Recurse into named children only (skip punctuation / keywords).
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        dispatch(&child, ctx);
    }
}

fn dispatch<'a>(node: &Node<'a>, ctx: &mut WalkCtx<'a>) {
    let kind = node.kind();

    // Symbol?
    if let Some(spec) = ctx.specs.iter().find(|s| s.kind == kind) {
        if let Some(name) = symbol_name(node, ctx.src, spec.name_field) {
            let line_start = node.start_position().row + 1;
            let line_end = node.end_position().row + 1;
            let qualified = qualified_name(ctx, &name);
            let visibility = infer_visibility(node, ctx.src, ctx.lang);
            let signature = extract_signature(node, ctx.src, ctx.lang);
            let sym = Symbol {
                name: name.clone(),
                kind: spec.symbol_kind,
                qualified_name: qualified.clone(),
                file_path: ctx.path_str.clone(),
                line_start,
                line_end,
                signature,
                doc_comment: None,
                visibility,
                language: ctx.lang,
            };
            ctx.out.symbols.push(sym);
            ctx.enclosing.push(qualified);
            walk(node, ctx);
            ctx.enclosing.pop();
            return;
        }
    }

    // Call?
    if ctx.call_kinds.contains(&kind) {
        if let Some(caller) = ctx.enclosing.last().cloned() {
            if let Some(callee) = call_target_tail(node, ctx.src, ctx.lang) {
                if !callee.is_empty() {
                    let line = node.start_position().row + 1;
                    let call_type = classify_call(node, ctx.src, ctx.lang);
                    ctx.out.calls.push(CallEdge {
                        caller,
                        callee,
                        file: ctx.path_str.clone(),
                        line,
                        call_type,
                        provenance: ctx.provenance,
                    });
                }
            }
        }
        walk(node, ctx);
        return;
    }

    // Import?
    if ctx.import_kinds.contains(&kind) {
        if let Some(imp) = extract_import(node, ctx.src, ctx.lang, &ctx.path_str, ctx.provenance) {
            ctx.out.imports.push(imp);
        }
        // imports have no interesting named children to recurse for symbols
        return;
    }

    // Default: descend.
    walk(node, ctx);
}

fn qualified_name(ctx: &WalkCtx<'_>, name: &str) -> String {
    if ctx.enclosing.is_empty() {
        format!("{}::{}", ctx.stem, name)
    } else {
        format!("{}::{}", ctx.enclosing.join("::"), name)
    }
}

fn symbol_name(node: &Node<'_>, src: &str, name_field: Option<&'static str>) -> Option<String> {
    if let Some(field) = name_field {
        if let Some(name_node) = node.child_by_field_name(field) {
            return Some(text_of(&name_node, src));
        }
    }
    // Fallback: first identifier / type_identifier child. Use the child inside the
    // loop (the cursor borrow is live only during iteration).
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let k = child.kind();
        if k == "identifier" || k == "type_identifier" || k == "property_identifier" {
            return Some(text_of(&child, src));
        }
    }
    None
}

fn text_of(node: &Node<'_>, src: &str) -> String {
    node.utf8_text(src.as_bytes())
        .map(|t| t.trim().to_string())
        .unwrap_or_default()
}

fn call_target_tail(node: &Node<'_>, src: &str, _lang: Language) -> Option<String> {
    // Primary: the `function` field (call_expression / call). Compute its text inside
    // the branch so no borrowed Node escapes its cursor.
    let raw = if let Some(func) = node.child_by_field_name("function") {
        text_of(&func, src)
    } else {
        // Fallback: first named child (e.g. macro_invocation). Use it inside the loop.
        let mut cursor = node.walk();
        let mut found = String::new();
        for child in node.named_children(&mut cursor) {
            found = text_of(&child, src);
            break;
        }
        found
    };
    if raw.is_empty() {
        return None;
    }
    // Take the last segment after `.` or `::`.
    let tail = raw
        .rsplit(['.', ':'])
        .next()
        .unwrap_or(&raw)
        .trim()
        .to_string();
    Some(tail)
}

fn classify_call(node: &Node<'_>, src: &str, lang: Language) -> CallType {
    if let Some(func) = node.child_by_field_name("function") {
        let raw = text_of(&func, src);
        if raw.contains('.') || raw.contains("::") {
            return CallType::Method;
        }
        // Heuristic: capitalized first char → constructor-ish.
        if raw.chars().next().map_or(false, |c| c.is_ascii_uppercase()) {
            return CallType::Constructor;
        }
    }
    if lang == Language::Rust && node.kind() == "macro_invocation" {
        return CallType::Direct;
    }
    CallType::Direct
}

fn infer_visibility(node: &Node<'_>, src: &str, lang: Language) -> Visibility {
    // Look at the small sibling span before the node for a `pub`/`private`/`export` keyword.
    let start = node.start_byte();
    let lookback = start.saturating_sub(32);
    let prefix = src.get(lookback..start).unwrap_or("");
    if lang == Language::Rust {
        if prefix.contains("pub(crate)") {
            return Visibility::Crate;
        }
        if prefix.contains("pub") {
            return Visibility::Public;
        }
        return Visibility::Private;
    }
    if lang == Language::TypeScript || lang == Language::JavaScript {
        if prefix.contains("export") {
            return Visibility::Public;
        }
        return Visibility::Internal;
    }
    if lang == Language::Python {
        if prefix.contains("__") {
            return Visibility::Private;
        }
        return Visibility::Public;
    }
    Visibility::Public
}

fn extract_signature(node: &Node<'_>, src: &str, lang: Language) -> Option<String> {
    // Best-effort: the first line of the declaration.
    let text = text_of(node, src);
    let first_line = text.lines().next()?.to_string();
    if lang == Language::Rust {
        // Trim the body `{ ... }`.
        if let Some(idx) = first_line.find('{') {
            return Some(first_line[..idx].trim_end().to_string());
        }
    }
    Some(first_line)
}

fn extract_import(
    node: &Node<'_>,
    src: &str,
    lang: Language,
    file: &str,
    prov: Provenance,
) -> Option<ImportEdge> {
    let raw = text_of(node, src);
    if raw.is_empty() {
        return None;
    }
    let (target, imported, import_type) = match lang {
        Language::Rust => parse_rust_use(&raw),
        Language::TypeScript | Language::JavaScript => parse_ts_import(&raw, node, src),
        Language::Python => parse_py_import(&raw),
        Language::Go => parse_go_import(&raw),
        _ => (raw.clone(), vec![], ImportType::Named),
    };
    Some(ImportEdge {
        source_file: file.to_string(),
        target,
        imported_symbols: imported,
        import_type,
        provenance: prov,
    })
}

fn parse_rust_use(raw: &str) -> (String, Vec<String>, ImportType) {
    let body = raw
        .trim_start_matches("use ")
        .trim_start_matches("pub use ")
        .trim_end_matches(';')
        .trim();
    let import_type = if body.contains("::*") {
        ImportType::Wildcard
    } else if body.contains(" as ") {
        ImportType::Reexport
    } else {
        ImportType::Named
    };
    let target = body.replace(' ', "");
    let tail = target.rsplit("::").next().unwrap_or(&target).to_string();
    (target, vec![tail], import_type)
}

fn parse_ts_import(raw: &str, node: &Node<'_>, src: &str) -> (String, Vec<String>, ImportType) {
    // Source string is in a child of kind `string`.
    let mut target = String::new();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "string" {
            target = text_of(&child, src).trim_matches(|c| c == '\'' || c == '"' || c == '`').to_string();
        }
    }
    if target.is_empty() {
        target = raw.to_string();
    }
    let import_type = if raw.contains("* as") {
        ImportType::Wildcard
    } else if raw.contains("default") || !raw.contains('{') {
        ImportType::Default
    } else {
        ImportType::Named
    };
    // Imported symbol tails: identifiers inside braces.
    let mut imported = Vec::new();
    if let Some(open) = raw.find('{') {
        if let Some(close) = raw[open..].find('}') {
            let inner = &raw[open + 1..open + close];
            for part in inner.split(',') {
                let name = part.split(" as ").next().unwrap_or(part).trim();
                if !name.is_empty() {
                    imported.push(name.to_string());
                }
            }
        }
    }
    (target, imported, import_type)
}

fn parse_py_import(raw: &str) -> (String, Vec<String>, ImportType) {
    let body = raw.trim_end_matches(';').trim();
    if let Some(rest) = body.strip_prefix("from ") {
        let (mod_part, _) = rest.split_once(" import ").unwrap_or((rest, ""));
        return (mod_part.trim().to_string(), vec![], ImportType::Named);
    }
    if let Some(rest) = body.strip_prefix("import ") {
        let name = rest.split(" as ").next().unwrap_or(rest).trim();
        return (name.to_string(), vec![name.to_string()], ImportType::Named);
    }
    (body.to_string(), vec![], ImportType::Named)
}

fn parse_go_import(raw: &str) -> (String, Vec<String>, ImportType) {
    // `import "path"` or `import ( ... )`.
    let quoted = raw.split('"').nth(1).unwrap_or("").to_string();
    if quoted.is_empty() {
        return (raw.trim().to_string(), vec![], ImportType::Named);
    }
    let tail = quoted.rsplit('/').next().unwrap_or(&quoted).to_string();
    (quoted, vec![tail], ImportType::Named)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_rust_function_and_call() {
        let p = TreeSitterParser::new();
        let src = r#"
pub fn build_temp_provider(name: &str) -> u32 {
    let x = helper(name);
    x + 1
}

fn helper(y: &str) -> u32 { 0 }
"#;
        let parsed = p.parse_file(&PathBuf::from("builder.rs"), src, Language::Rust);
        assert!(parsed.symbols.iter().any(|s| s.name == "build_temp_provider"));
        assert!(parsed.symbols.iter().any(|s| s.name == "helper"));
        assert!(parsed.calls.iter().any(|c| c.callee == "helper" && c.caller.contains("build_temp_provider")));
    }

    #[test]
    fn parse_rust_struct_trait_and_use() {
        let p = TreeSitterParser::new();
        let src = r#"
use std::collections::HashMap;

pub struct CodeGraph { nodes: usize }

pub trait Parser { fn parse_file(&self); }
"#;
        let parsed = p.parse_file(&PathBuf::from("model.rs"), src, Language::Rust);
        assert!(parsed.symbols.iter().any(|s| s.name == "CodeGraph" && s.kind == SymbolKind::Struct));
        assert!(parsed.symbols.iter().any(|s| s.name == "Parser" && s.kind == SymbolKind::Trait));
        assert!(parsed.imports.iter().any(|i| i.target.contains("HashMap")));
    }

    #[test]
    fn parse_typescript_class_and_call() {
        let p = TreeSitterParser::new();
        let src = r#"
export class Builder {
  build(): number {
    return this.helper();
  }
  private helper(): number { return 1; }
}
"#;
        let parsed = p.parse_file(&PathBuf::from("builder.ts"), src, Language::TypeScript);
        assert!(parsed.symbols.iter().any(|s| s.name == "Builder"));
        assert!(parsed.calls.iter().any(|c| c.callee == "helper"));
    }

    #[test]
    fn parse_python_function_and_call() {
        let p = TreeSitterParser::new();
        let src = r#"
def build(name):
    return helper(name)

def helper(y):
    return 0
"#;
        let parsed = p.parse_file(&PathBuf::from("builder.py"), src, Language::Python);
        assert!(parsed.symbols.iter().any(|s| s.name == "build"));
        assert!(parsed.calls.iter().any(|c| c.callee == "helper"));
    }

    #[test]
    fn parse_go_function_and_call() {
        let p = TreeSitterParser::new();
        let src = r#"
package main

import "fmt"

func Build(name string) int {
    fmt.Println(name)
    return helper(name)
}
"#;
        let parsed = p.parse_file(&PathBuf::from("builder.go"), src, Language::Go);
        assert!(parsed.symbols.iter().any(|s| s.name == "Build"));
        assert!(parsed.calls.iter().any(|c| c.callee == "Println" || c.callee == "helper"));
        assert!(parsed.imports.iter().any(|i| i.target == "fmt"));
    }

    #[test]
    fn unsupported_language_returns_empty() {
        let p = TreeSitterParser::new();
        let parsed = p.parse_file(&PathBuf::from("x.rb"), "def f; end", Language::Ruby);
        assert!(parsed.symbols.is_empty());
        assert!(!p.supports(Language::Ruby));
    }
}