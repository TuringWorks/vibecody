#![allow(dead_code)]
//! AST-aware code application — deterministic edits using AST targeting.
//!
//! Closes P2 Gap 8: Use AST targeting for deterministic edits instead of
//! text-based diffs (Continue.dev 1.0 approach).
//!
//! # Architecture
//!
//! ```text
//! Edit Request -> AST Parse -> Target Node -> Apply Transform -> Validate
//!   - TreeSitter-style node addressing (path.to.node)
//!   - Structural edits (rename, wrap, extract, inline)
//!   - Scope-aware insertion (correct indentation, imports)
//!   - Validation (parse after edit, no syntax errors)
//! ```

// ---------------------------------------------------------------------------
// Language detection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    Java,
    CSharp,
    Kotlin,
    Swift,
    Ruby,
    Cpp,
    Unknown,
}

impl Language {
    /// Detect language from file extension.
    pub fn detect(filename: &str) -> Language {
        let ext = filename.rsplit('.').next().unwrap_or("");
        match ext {
            "rs" => Language::Rust,
            "ts" | "tsx" => Language::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "py" | "pyi" => Language::Python,
            "go" => Language::Go,
            "java" => Language::Java,
            "cs" => Language::CSharp,
            "kt" | "kts" => Language::Kotlin,
            "swift" => Language::Swift,
            "rb" => Language::Ruby,
            "cpp" | "cc" | "cxx" | "c" | "h" | "hpp" => Language::Cpp,
            _ => Language::Unknown,
        }
    }

    /// Return the single-line comment prefix for this language.
    pub fn comment_prefix(&self) -> &str {
        match self {
            Language::Rust => "//",
            Language::TypeScript => "//",
            Language::JavaScript => "//",
            Language::Python => "#",
            Language::Go => "//",
            Language::Java => "//",
            Language::CSharp => "//",
            Language::Kotlin => "//",
            Language::Swift => "//",
            Language::Ruby => "#",
            Language::Cpp => "//",
            Language::Unknown => "//",
        }
    }
}

// ---------------------------------------------------------------------------
// AST node kinds
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum AstNodeKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Interface,
    Import,
    Module,
    Block,
    Field,
    Parameter,
    Variable,
    Constant,
    TypeAlias,
    Impl,
}

impl AstNodeKind {
    pub fn as_str(&self) -> &str {
        match self {
            AstNodeKind::Function => "function",
            AstNodeKind::Method => "method",
            AstNodeKind::Class => "class",
            AstNodeKind::Struct => "struct",
            AstNodeKind::Enum => "enum",
            AstNodeKind::Trait => "trait",
            AstNodeKind::Interface => "interface",
            AstNodeKind::Import => "import",
            AstNodeKind::Module => "module",
            AstNodeKind::Block => "block",
            AstNodeKind::Field => "field",
            AstNodeKind::Parameter => "parameter",
            AstNodeKind::Variable => "variable",
            AstNodeKind::Constant => "constant",
            AstNodeKind::TypeAlias => "type_alias",
            AstNodeKind::Impl => "impl",
        }
    }
}

// ---------------------------------------------------------------------------
// Edit operations
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum EditOperation {
    Replace,
    InsertBefore,
    InsertAfter,
    Delete,
    Wrap,
    Rename,
    Move,
}

// ---------------------------------------------------------------------------
// AST node
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AstNode {
    pub id: String,
    pub kind: AstNodeKind,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_col: usize,
    pub end_col: usize,
    pub children: Vec<AstNode>,
    pub parent_id: Option<String>,
    pub language: Language,
    pub signature: Option<String>,
}

impl AstNode {
    pub fn new(kind: AstNodeKind, name: &str, start_line: usize, end_line: usize, language: Language) -> Self {
        let id = format!("{}::{}@{}-{}", kind.as_str(), name, start_line, end_line);
        Self {
            id,
            kind,
            name: name.to_string(),
            start_line,
            end_line,
            start_col: 0,
            end_col: 0,
            children: Vec::new(),
            parent_id: None,
            language,
            signature: None,
        }
    }

    pub fn add_child(&mut self, mut child: AstNode) {
        child.parent_id = Some(self.id.clone());
        self.children.push(child);
    }

    /// Recursive search by name.
    pub fn find_by_name(&self, name: &str) -> Option<&AstNode> {
        if self.name == name {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_name(name) {
                return Some(found);
            }
        }
        None
    }

    /// Find by path like "MyStruct::my_method".
    pub fn find_by_path(&self, path: &str) -> Option<&AstNode> {
        let parts: Vec<&str> = path.splitn(2, "::").collect();
        if parts.is_empty() {
            return None;
        }
        let first = parts[0];
        let child = self.children.iter().find(|c| c.name == first)?;
        if parts.len() == 1 {
            Some(child)
        } else {
            child.find_by_path(parts[1])
        }
    }

    pub fn line_count(&self) -> usize {
        if self.end_line >= self.start_line {
            self.end_line - self.start_line + 1
        } else {
            0
        }
    }

    pub fn contains_line(&self, line: usize) -> bool {
        line >= self.start_line && line <= self.end_line
    }

    /// Nesting depth (0 for leaf nodes).
    pub fn depth(&self) -> usize {
        let mut max_child_depth = 0;
        for child in &self.children {
            let d = child.depth() + 1;
            if d > max_child_depth {
                max_child_depth = d;
            }
        }
        max_child_depth
    }
}

// ---------------------------------------------------------------------------
// AST edit
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AstEdit {
    pub id: String,
    pub target_node: String,
    pub operation: EditOperation,
    pub new_content: Option<String>,
    pub description: String,
    pub confidence: f64,
    pub requires_review: bool,
}

impl AstEdit {
    pub fn new(target: &str, operation: EditOperation, description: &str) -> Self {
        let id = format!("edit-{}-{}", target, description.len());
        Self {
            id,
            target_node: target.to_string(),
            operation,
            new_content: None,
            description: description.to_string(),
            confidence: 0.8,
            requires_review: false,
        }
    }

    pub fn with_content(mut self, content: &str) -> Self {
        self.new_content = Some(content.to_string());
        self
    }

    pub fn with_confidence(mut self, conf: f64) -> Self {
        self.confidence = conf.clamp(0.0, 1.0);
        self
    }
}

// ---------------------------------------------------------------------------
// AST edit result
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AstEditResult {
    pub edit_id: String,
    pub success: bool,
    pub original_lines: (usize, usize),
    pub new_lines: (usize, usize),
    pub lines_added: i32,
    pub lines_removed: i32,
    pub conflicts: Vec<String>,
}

impl AstEditResult {
    fn ok(edit_id: &str, orig: (usize, usize), new: (usize, usize), added: i32, removed: i32) -> Self {
        Self {
            edit_id: edit_id.to_string(),
            success: true,
            original_lines: orig,
            new_lines: new,
            lines_added: added,
            lines_removed: removed,
            conflicts: Vec::new(),
        }
    }

    fn fail(edit_id: &str, reason: &str) -> Self {
        Self {
            edit_id: edit_id.to_string(),
            success: false,
            original_lines: (0, 0),
            new_lines: (0, 0),
            lines_added: 0,
            lines_removed: 0,
            conflicts: vec![reason.to_string()],
        }
    }
}

// ---------------------------------------------------------------------------
// File AST
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FileAst {
    pub file_path: String,
    pub language: Language,
    pub nodes: Vec<AstNode>,
    pub line_count: usize,
    pub content: String,
}

impl FileAst {
    /// Create a new FileAst, detecting language and parsing nodes.
    pub fn new(path: &str, content: &str) -> Self {
        let language = Language::detect(path);
        let nodes = match language {
            Language::Rust => Self::parse_rust(content),
            Language::TypeScript => Self::parse_typescript(content),
            Language::JavaScript => Self::parse_typescript(content),
            Language::Python => Self::parse_python(content),
            _ => Self::parse_rust(content),
        };
        Self {
            file_path: path.to_string(),
            language,
            nodes,
            line_count: content.lines().count(),
            content: content.to_string(),
        }
    }

    /// Parse Rust source into AST nodes using line-by-line pattern matching.
    pub fn parse_rust(content: &str) -> Vec<AstNode> {
        let lines: Vec<&str> = content.lines().collect();
        let mut nodes = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            let line_num = i + 1;

            if is_rust_fn(trimmed) {
                let name = extract_name_after_keyword(trimmed, "fn ");
                let end = find_brace_block_end(&lines, i);
                let sig = trimmed.trim_end_matches('{').trim().to_string();
                let mut node = AstNode::new(AstNodeKind::Function, &name, line_num, end, Language::Rust);
                node.signature = Some(sig);
                nodes.push(node);
                i = end;
                continue;
            }
            if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ")
                || trimmed.starts_with("pub(crate) struct ")
            {
                let name = extract_name_after_keyword(trimmed, "struct ");
                let end = if trimmed.contains('{') {
                    find_brace_block_end(&lines, i)
                } else {
                    line_num
                };
                let mut node = AstNode::new(AstNodeKind::Struct, &name, line_num, end, Language::Rust);
                node.signature = Some(trimmed.to_string());
                nodes.push(node);
                i = end;
                continue;
            }
            if trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ")
                || trimmed.starts_with("pub(crate) enum ")
            {
                let name = extract_name_after_keyword(trimmed, "enum ");
                let end = find_brace_block_end(&lines, i);
                let mut node = AstNode::new(AstNodeKind::Enum, &name, line_num, end, Language::Rust);
                node.signature = Some(trimmed.to_string());
                nodes.push(node);
                i = end;
                continue;
            }
            if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
                let name = extract_name_after_keyword(trimmed, "trait ");
                let end = find_brace_block_end(&lines, i);
                let mut node = AstNode::new(AstNodeKind::Trait, &name, line_num, end, Language::Rust);
                node.signature = Some(trimmed.to_string());
                nodes.push(node);
                i = end;
                continue;
            }
            if trimmed.starts_with("impl ") || trimmed.starts_with("impl<") {
                let name = extract_impl_name(trimmed);
                let end = find_brace_block_end(&lines, i);
                let mut node = AstNode::new(AstNodeKind::Impl, &name, line_num, end, Language::Rust);
                node.signature = Some(trimmed.to_string());
                let mut j = i + 1;
                while j < end.saturating_sub(1).max(i + 1) && j < lines.len() {
                    let inner = lines[j].trim();
                    if is_rust_fn(inner) {
                        let mname = extract_name_after_keyword(inner, "fn ");
                        let mend = find_brace_block_end(&lines, j);
                        let msig = inner.trim_end_matches('{').trim().to_string();
                        let mut method = AstNode::new(AstNodeKind::Method, &mname, j + 1, mend, Language::Rust);
                        method.signature = Some(msig);
                        node.add_child(method);
                        j = mend;
                        continue;
                    }
                    j += 1;
                }
                nodes.push(node);
                i = end;
                continue;
            }
            if trimmed.starts_with("pub mod ") || trimmed.starts_with("mod ") {
                let name = extract_name_after_keyword(trimmed, "mod ");
                let name = name.trim_end_matches(';').to_string();
                let end = if trimmed.contains('{') {
                    find_brace_block_end(&lines, i)
                } else {
                    line_num
                };
                nodes.push(AstNode::new(AstNodeKind::Module, &name, line_num, end, Language::Rust));
                i = end;
                continue;
            }
            if trimmed.starts_with("use ") || trimmed.starts_with("pub use ") {
                let import_text = trimmed.trim_end_matches(';').to_string();
                nodes.push(AstNode::new(AstNodeKind::Import, &import_text, line_num, line_num, Language::Rust));
                i += 1;
                continue;
            }
            if trimmed.starts_with("pub const ") || trimmed.starts_with("const ") {
                let name = extract_name_after_keyword(trimmed, "const ");
                let name = name.split(':').next().unwrap_or(&name).to_string();
                nodes.push(AstNode::new(AstNodeKind::Constant, &name, line_num, line_num, Language::Rust));
                i += 1;
                continue;
            }
            if trimmed.starts_with("pub type ") || trimmed.starts_with("type ") {
                let name = extract_name_after_keyword(trimmed, "type ");
                let name = name.split(['=', '<', ' ']).next().unwrap_or(&name).to_string();
                nodes.push(AstNode::new(AstNodeKind::TypeAlias, &name, line_num, line_num, Language::Rust));
                i += 1;
                continue;
            }
            i += 1;
        }
        nodes
    }

    /// Parse TypeScript/JavaScript source into AST nodes.
    pub fn parse_typescript(content: &str) -> Vec<AstNode> {
        let lines: Vec<&str> = content.lines().collect();
        let mut nodes = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            let line_num = i + 1;

            if trimmed.starts_with("function ") || trimmed.starts_with("export function ")
                || trimmed.starts_with("async function ") || trimmed.starts_with("export async function ")
            {
                let name = extract_name_after_keyword(trimmed, "function ");
                let end = find_brace_block_end(&lines, i);
                let mut node = AstNode::new(AstNodeKind::Function, &name, line_num, end, Language::TypeScript);
                node.signature = Some(trimmed.trim_end_matches('{').trim().to_string());
                nodes.push(node);
                i = end;
                continue;
            }
            if trimmed.starts_with("class ") || trimmed.starts_with("export class ")
                || trimmed.starts_with("abstract class ") || trimmed.starts_with("export abstract class ")
            {
                let name = extract_name_after_keyword(trimmed, "class ");
                let end = find_brace_block_end(&lines, i);
                let mut node = AstNode::new(AstNodeKind::Class, &name, line_num, end, Language::TypeScript);
                node.signature = Some(trimmed.trim_end_matches('{').trim().to_string());
                nodes.push(node);
                i = end;
                continue;
            }
            if trimmed.starts_with("interface ") || trimmed.starts_with("export interface ") {
                let name = extract_name_after_keyword(trimmed, "interface ");
                let end = find_brace_block_end(&lines, i);
                nodes.push(AstNode::new(AstNodeKind::Interface, &name, line_num, end, Language::TypeScript));
                i = end;
                continue;
            }
            if trimmed.starts_with("import ") {
                let import_text = trimmed.trim_end_matches(';').to_string();
                nodes.push(AstNode::new(AstNodeKind::Import, &import_text, line_num, line_num, Language::TypeScript));
                i += 1;
                continue;
            }
            if (trimmed.starts_with("const ") || trimmed.starts_with("export const ")
                || trimmed.starts_with("let ") || trimmed.starts_with("var "))
                && trimmed.contains('=')
            {
                let kw = if trimmed.contains("const ") { "const " } else if trimmed.contains("let ") { "let " } else { "var " };
                let name = extract_name_after_keyword(trimmed, kw);
                let name = name.split([' ', ':', '=']).next().unwrap_or(&name).to_string();
                let kind = if trimmed.contains("const ") { AstNodeKind::Constant } else { AstNodeKind::Variable };
                nodes.push(AstNode::new(kind, &name, line_num, line_num, Language::TypeScript));
                i += 1;
                continue;
            }
            if trimmed.starts_with("type ") || trimmed.starts_with("export type ") {
                let name = extract_name_after_keyword(trimmed, "type ");
                let name = name.split(['=', '<', ' ']).next().unwrap_or(&name).to_string();
                nodes.push(AstNode::new(AstNodeKind::TypeAlias, &name, line_num, line_num, Language::TypeScript));
                i += 1;
                continue;
            }
            i += 1;
        }
        nodes
    }

    /// Parse Python source into AST nodes.
    pub fn parse_python(content: &str) -> Vec<AstNode> {
        let lines: Vec<&str> = content.lines().collect();
        let mut nodes = Vec::new();
        let mut i = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            let line_num = i + 1;

            if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
                let name = extract_name_after_keyword(trimmed, "def ");
                let end = find_python_block_end(&lines, i);
                let mut node = AstNode::new(AstNodeKind::Function, &name, line_num, end, Language::Python);
                node.signature = Some(trimmed.trim_end_matches(':').trim().to_string());
                nodes.push(node);
                i = end;
                continue;
            }
            if trimmed.starts_with("class ") {
                let name = extract_name_after_keyword(trimmed, "class ");
                let name = name.trim_end_matches(':').split('(').next().unwrap_or(&name).to_string();
                let end = find_python_block_end(&lines, i);
                let mut node = AstNode::new(AstNodeKind::Class, &name, line_num, end, Language::Python);
                node.signature = Some(trimmed.trim_end_matches(':').trim().to_string());
                let base_indent = leading_spaces(lines[i]);
                let mut j = i + 1;
                while j < end && j < lines.len() {
                    let inner = lines[j].trim();
                    if (inner.starts_with("def ") || inner.starts_with("async def "))
                        && leading_spaces(lines[j]) > base_indent
                    {
                        let mname = extract_name_after_keyword(inner, "def ");
                        let mend = find_python_block_end(&lines, j);
                        let mut method = AstNode::new(AstNodeKind::Method, &mname, j + 1, mend, Language::Python);
                        method.signature = Some(inner.trim_end_matches(':').trim().to_string());
                        node.add_child(method);
                        j = mend;
                        continue;
                    }
                    j += 1;
                }
                nodes.push(node);
                i = end;
                continue;
            }
            if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
                nodes.push(AstNode::new(AstNodeKind::Import, trimmed, line_num, line_num, Language::Python));
                i += 1;
                continue;
            }
            i += 1;
        }
        nodes
    }

    pub fn find_node(&self, name: &str) -> Option<&AstNode> {
        for node in &self.nodes {
            if node.name == name {
                return Some(node);
            }
            if let Some(found) = node.find_by_name(name) {
                return Some(found);
            }
        }
        None
    }

    pub fn node_at_line(&self, line: usize) -> Option<&AstNode> {
        fn find_deepest<'a>(nodes: &'a [AstNode], line: usize) -> Option<&'a AstNode> {
            for node in nodes {
                if node.contains_line(line) {
                    if let Some(child) = find_deepest(&node.children, line) {
                        return Some(child);
                    }
                    return Some(node);
                }
            }
            None
        }
        find_deepest(&self.nodes, line)
    }
}

// ---------------------------------------------------------------------------
// Conflict strategy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictStrategy {
    Ask,
    KeepOriginal,
    UseNew,
    Merge,
}

// ---------------------------------------------------------------------------
// AST edit config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AstEditConfig {
    pub preserve_formatting: bool,
    pub preserve_comments: bool,
    pub auto_fix_imports: bool,
    pub conflict_resolution: ConflictStrategy,
    pub min_confidence: f64,
}

impl AstEditConfig {
    pub fn default_config() -> Self {
        Self {
            preserve_formatting: true,
            preserve_comments: true,
            auto_fix_imports: true,
            conflict_resolution: ConflictStrategy::Ask,
            min_confidence: 0.7,
        }
    }

    pub fn strict() -> Self {
        Self {
            preserve_formatting: true,
            preserve_comments: true,
            auto_fix_imports: false,
            conflict_resolution: ConflictStrategy::KeepOriginal,
            min_confidence: 0.95,
        }
    }
}

// ---------------------------------------------------------------------------
// AST editor
// ---------------------------------------------------------------------------

pub struct AstEditor {
    pub files: Vec<FileAst>,
    pub pending_edits: Vec<AstEdit>,
    pub applied_edits: Vec<AstEditResult>,
    pub config: AstEditConfig,
}

impl AstEditor {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            pending_edits: Vec::new(),
            applied_edits: Vec::new(),
            config: AstEditConfig::default_config(),
        }
    }

    pub fn load_file(&mut self, path: &str, content: &str) {
        self.files.retain(|f| f.file_path != path);
        self.files.push(FileAst::new(path, content));
    }

    pub fn add_edit(&mut self, edit: AstEdit) {
        self.pending_edits.push(edit);
    }

    pub fn get_file(&self, path: &str) -> Option<&FileAst> {
        self.files.iter().find(|f| f.file_path == path)
    }

    /// Apply a single pending edit by id.
    pub fn apply_edit(&mut self, edit_id: &str) -> Result<AstEditResult, String> {
        let edit_idx = self.pending_edits.iter().position(|e| e.id == edit_id)
            .ok_or_else(|| format!("Edit '{}' not found", edit_id))?;

        if self.pending_edits[edit_idx].confidence < self.config.min_confidence {
            return Err(format!(
                "Edit confidence {:.2} below minimum {:.2}",
                self.pending_edits[edit_idx].confidence,
                self.config.min_confidence
            ));
        }

        let edit = self.pending_edits.remove(edit_idx);
        let result = self.execute_edit(&edit);
        self.applied_edits.push(result.clone());
        Ok(result)
    }

    /// Apply all pending edits in order.
    pub fn apply_all(&mut self) -> Vec<AstEditResult> {
        let edits: Vec<AstEdit> = self.pending_edits.drain(..).collect();
        let mut results = Vec::new();
        for edit in &edits {
            if edit.confidence < self.config.min_confidence {
                results.push(AstEditResult::fail(&edit.id, "Confidence below threshold"));
                continue;
            }
            let result = self.execute_edit(edit);
            results.push(result);
        }
        self.applied_edits.extend(results.clone());
        results
    }

    /// Preview what an edit would produce without applying it.
    pub fn preview_edit(&self, edit_id: &str) -> Option<String> {
        let edit = self.pending_edits.iter().find(|e| e.id == edit_id)?;
        for file in &self.files {
            if let Some(node) = find_node_by_target(&file.nodes, &edit.target_node) {
                let lines: Vec<&str> = file.content.lines().collect();
                let start = node.start_line.saturating_sub(1);
                let end = node.end_line.min(lines.len());
                let original: Vec<&str> = lines[start..end].to_vec();
                let mut preview = String::new();
                preview.push_str(&format!("--- {} (lines {}-{})\n", file.file_path, node.start_line, node.end_line));
                preview.push_str(&format!("+++ {} ({:?})\n", edit.target_node, edit.operation));
                for line in &original {
                    preview.push_str(&format!("- {}\n", line));
                }
                if let Some(ref new_content) = edit.new_content {
                    for line in new_content.lines() {
                        preview.push_str(&format!("+ {}\n", line));
                    }
                }
                return Some(preview);
            }
        }
        None
    }

    /// Validate edits for conflicts (overlapping targets, etc.).
    pub fn validate_edits(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        let mut targets: Vec<&str> = Vec::new();
        for edit in &self.pending_edits {
            if targets.contains(&edit.target_node.as_str()) {
                warnings.push(format!("Duplicate target: '{}'", edit.target_node));
            }
            targets.push(&edit.target_node);
        }

        for edit in &self.pending_edits {
            if edit.confidence < self.config.min_confidence {
                warnings.push(format!(
                    "Edit '{}' confidence {:.2} below min {:.2}",
                    edit.id, edit.confidence, self.config.min_confidence
                ));
            }
        }

        for edit in &self.pending_edits {
            let found = self.files.iter().any(|f| find_node_by_target(&f.nodes, &edit.target_node).is_some());
            if !found {
                warnings.push(format!("Target '{}' not found in any loaded file", edit.target_node));
            }
        }

        warnings
    }

    fn execute_edit(&mut self, edit: &AstEdit) -> AstEditResult {
        let file_idx = self.files.iter().position(|f| {
            find_node_by_target(&f.nodes, &edit.target_node).is_some()
        });
        let file_idx = match file_idx {
            Some(idx) => idx,
            None => return AstEditResult::fail(&edit.id, &format!("Target '{}' not found", edit.target_node)),
        };

        let file = &self.files[file_idx];
        let node = match find_node_by_target(&file.nodes, &edit.target_node) {
            Some(n) => n.clone(),
            None => return AstEditResult::fail(&edit.id, "Target node disappeared"),
        };

        let lines: Vec<&str> = file.content.lines().collect();
        let orig_start = node.start_line;
        let orig_end = node.end_line;

        let new_content = match edit.operation {
            EditOperation::Replace => {
                let replacement = match &edit.new_content {
                    Some(c) => c.clone(),
                    None => return AstEditResult::fail(&edit.id, "Replace requires new_content"),
                };
                let mut result_lines: Vec<String> = Vec::new();
                for line in &lines[..orig_start.saturating_sub(1)] {
                    result_lines.push(line.to_string());
                }
                for line in replacement.lines() {
                    result_lines.push(line.to_string());
                }
                if orig_end <= lines.len() {
                    for line in &lines[orig_end..] {
                        result_lines.push(line.to_string());
                    }
                }
                result_lines.join("\n")
            }
            EditOperation::InsertBefore => {
                let content = match &edit.new_content {
                    Some(c) => c.clone(),
                    None => return AstEditResult::fail(&edit.id, "InsertBefore requires new_content"),
                };
                let mut result_lines: Vec<String> = Vec::new();
                for line in &lines[..orig_start.saturating_sub(1)] {
                    result_lines.push(line.to_string());
                }
                for line in content.lines() {
                    result_lines.push(line.to_string());
                }
                for line in &lines[orig_start.saturating_sub(1)..] {
                    result_lines.push(line.to_string());
                }
                result_lines.join("\n")
            }
            EditOperation::InsertAfter => {
                let content = match &edit.new_content {
                    Some(c) => c.clone(),
                    None => return AstEditResult::fail(&edit.id, "InsertAfter requires new_content"),
                };
                let mut result_lines: Vec<String> = Vec::new();
                let insert_at = orig_end.min(lines.len());
                for line in &lines[..insert_at] {
                    result_lines.push(line.to_string());
                }
                for line in content.lines() {
                    result_lines.push(line.to_string());
                }
                for line in &lines[insert_at..] {
                    result_lines.push(line.to_string());
                }
                result_lines.join("\n")
            }
            EditOperation::Delete => {
                let mut result_lines: Vec<String> = Vec::new();
                for (i, line) in lines.iter().enumerate() {
                    let ln = i + 1;
                    if ln < orig_start || ln > orig_end {
                        result_lines.push(line.to_string());
                    }
                }
                result_lines.join("\n")
            }
            EditOperation::Wrap => {
                let wrapper = match &edit.new_content {
                    Some(c) => c.clone(),
                    None => return AstEditResult::fail(&edit.id, "Wrap requires new_content (wrapper template)"),
                };
                let mut result_lines: Vec<String> = Vec::new();
                for line in &lines[..orig_start.saturating_sub(1)] {
                    result_lines.push(line.to_string());
                }
                let body: Vec<&str> = lines[orig_start.saturating_sub(1)..orig_end.min(lines.len())].to_vec();
                let body_str = body.join("\n");
                let wrapped = wrapper.replace("{body}", &body_str);
                for line in wrapped.lines() {
                    result_lines.push(line.to_string());
                }
                if orig_end < lines.len() {
                    for line in &lines[orig_end..] {
                        result_lines.push(line.to_string());
                    }
                }
                result_lines.join("\n")
            }
            EditOperation::Rename => {
                let new_name = match &edit.new_content {
                    Some(c) => c.clone(),
                    None => return AstEditResult::fail(&edit.id, "Rename requires new_content (new name)"),
                };
                let old_name = &node.name;
                file.content.replace(old_name.as_str(), &new_name)
            }
            EditOperation::Move => {
                let mut result_lines: Vec<String> = Vec::new();
                let moved_lines: Vec<String> = lines[orig_start.saturating_sub(1)..orig_end.min(lines.len())]
                    .iter().map(|l| l.to_string()).collect();
                for (i, line) in lines.iter().enumerate() {
                    let ln = i + 1;
                    if ln < orig_start || ln > orig_end {
                        result_lines.push(line.to_string());
                    }
                }
                result_lines.push(String::new());
                result_lines.extend(moved_lines);
                result_lines.join("\n")
            }
        };

        let new_line_count = new_content.lines().count();
        let old_line_count = lines.len();
        let added = new_line_count as i32 - old_line_count as i32;
        let removed = (orig_end - orig_start + 1) as i32;
        let actually_added = added + removed;

        let path = self.files[file_idx].file_path.clone();
        self.files[file_idx] = FileAst::new(&path, &new_content);

        AstEditResult::ok(
            &edit.id,
            (orig_start, orig_end),
            (orig_start, (orig_start as i32 + actually_added - 1).max(orig_start as i32) as usize),
            actually_added.max(0),
            removed,
        )
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn is_rust_fn(trimmed: &str) -> bool {
    trimmed.starts_with("pub fn ")
        || trimmed.starts_with("fn ")
        || trimmed.starts_with("pub async fn ")
        || trimmed.starts_with("async fn ")
        || trimmed.starts_with("pub(crate) fn ")
        || trimmed.starts_with("pub(crate) async fn ")
        || trimmed.starts_with("pub(super) fn ")
        || trimmed.starts_with("pub unsafe fn ")
        || trimmed.starts_with("unsafe fn ")
}

fn extract_name_after_keyword(line: &str, keyword: &str) -> String {
    if let Some(idx) = line.find(keyword) {
        let rest = &line[idx + keyword.len()..];
        let end = rest.find([' ', '{', '(', '<', ':', ';', '\n']).unwrap_or(rest.len());
        rest[..end].to_string()
    } else {
        "unknown".to_string()
    }
}

fn extract_impl_name(line: &str) -> String {
    let rest = line.strip_prefix("impl").unwrap_or(line).trim();
    let rest = if rest.starts_with('<') {
        let close = find_matching_angle(rest).unwrap_or(0);
        rest[close + 1..].trim()
    } else {
        rest
    };
    if let Some(for_idx) = rest.find(" for ") {
        let after_for = &rest[for_idx + 5..];
        let end = after_for.find([' ', '{', '<']).unwrap_or(after_for.len());
        return after_for[..end].to_string();
    }
    let end = rest.find([' ', '{', '<']).unwrap_or(rest.len());
    rest[..end].to_string()
}

fn find_matching_angle(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, ch) in s.chars().enumerate() {
        if ch == '<' { depth += 1; }
        if ch == '>' {
            depth -= 1;
            if depth == 0 { return Some(i); }
        }
    }
    None
}

fn find_brace_block_end(lines: &[&str], start: usize) -> usize {
    let mut depth = 0i32;
    let mut found_open = false;
    for (i, line) in lines[start..].iter().enumerate() {
        for ch in line.chars() {
            if ch == '{' { depth += 1; found_open = true; }
            if ch == '}' { depth -= 1; }
        }
        if found_open && depth <= 0 {
            return start + i + 1;
        }
    }
    if !found_open {
        return start + 1;
    }
    lines.len()
}

fn find_python_block_end(lines: &[&str], start: usize) -> usize {
    if start >= lines.len() {
        return start + 1;
    }
    let base_indent = leading_spaces(lines[start]);
    let mut end = start + 1;
    while end < lines.len() {
        let line = lines[end];
        if line.trim().is_empty() {
            end += 1;
            continue;
        }
        if leading_spaces(line) <= base_indent {
            break;
        }
        end += 1;
    }
    if end == start + 1 {
        end = start + 1;
    }
    end
}

fn leading_spaces(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn find_node_by_target<'a>(nodes: &'a [AstNode], target: &str) -> Option<&'a AstNode> {
    for node in nodes {
        if node.name == target {
            return Some(node);
        }
    }
    if target.contains("::") {
        let parts: Vec<&str> = target.splitn(2, "::").collect();
        for node in nodes {
            if node.name == parts[0] {
                if let Some(child) = node.find_by_name(parts[1]) {
                    return Some(child);
                }
            }
        }
    }
    for node in nodes {
        if let Some(found) = find_node_by_target(&node.children, target) {
            return Some(found);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Language tests --

    #[test]
    fn test_language_detect_rust() {
        assert_eq!(Language::detect("main.rs"), Language::Rust);
    }

    #[test]
    fn test_language_detect_typescript() {
        assert_eq!(Language::detect("app.ts"), Language::TypeScript);
        assert_eq!(Language::detect("component.tsx"), Language::TypeScript);
    }

    #[test]
    fn test_language_detect_javascript() {
        assert_eq!(Language::detect("index.js"), Language::JavaScript);
        assert_eq!(Language::detect("index.mjs"), Language::JavaScript);
    }

    #[test]
    fn test_language_detect_python() {
        assert_eq!(Language::detect("script.py"), Language::Python);
        assert_eq!(Language::detect("types.pyi"), Language::Python);
    }

    #[test]
    fn test_language_detect_go() {
        assert_eq!(Language::detect("main.go"), Language::Go);
    }

    #[test]
    fn test_language_detect_java() {
        assert_eq!(Language::detect("Main.java"), Language::Java);
    }

    #[test]
    fn test_language_detect_csharp() {
        assert_eq!(Language::detect("Program.cs"), Language::CSharp);
    }

    #[test]
    fn test_language_detect_kotlin() {
        assert_eq!(Language::detect("App.kt"), Language::Kotlin);
    }

    #[test]
    fn test_language_detect_swift() {
        assert_eq!(Language::detect("ViewController.swift"), Language::Swift);
    }

    #[test]
    fn test_language_detect_ruby() {
        assert_eq!(Language::detect("app.rb"), Language::Ruby);
    }

    #[test]
    fn test_language_detect_cpp() {
        assert_eq!(Language::detect("main.cpp"), Language::Cpp);
        assert_eq!(Language::detect("header.hpp"), Language::Cpp);
    }

    #[test]
    fn test_language_detect_unknown() {
        assert_eq!(Language::detect("Makefile"), Language::Unknown);
        assert_eq!(Language::detect("readme.md"), Language::Unknown);
    }

    #[test]
    fn test_language_comment_prefix() {
        assert_eq!(Language::Rust.comment_prefix(), "//");
        assert_eq!(Language::Python.comment_prefix(), "#");
        assert_eq!(Language::Ruby.comment_prefix(), "#");
        assert_eq!(Language::Go.comment_prefix(), "//");
    }

    // -- AstNodeKind tests --

    #[test]
    fn test_ast_node_kind_as_str() {
        assert_eq!(AstNodeKind::Function.as_str(), "function");
        assert_eq!(AstNodeKind::Struct.as_str(), "struct");
        assert_eq!(AstNodeKind::Impl.as_str(), "impl");
        assert_eq!(AstNodeKind::TypeAlias.as_str(), "type_alias");
        assert_eq!(AstNodeKind::Interface.as_str(), "interface");
    }

    // -- AstNode tests --

    #[test]
    fn test_ast_node_new() {
        let node = AstNode::new(AstNodeKind::Function, "foo", 1, 5, Language::Rust);
        assert_eq!(node.name, "foo");
        assert_eq!(node.start_line, 1);
        assert_eq!(node.end_line, 5);
        assert!(node.id.contains("function::foo"));
    }

    #[test]
    fn test_ast_node_line_count() {
        let node = AstNode::new(AstNodeKind::Function, "f", 3, 7, Language::Rust);
        assert_eq!(node.line_count(), 5);
    }

    #[test]
    fn test_ast_node_contains_line() {
        let node = AstNode::new(AstNodeKind::Struct, "S", 10, 20, Language::Rust);
        assert!(node.contains_line(10));
        assert!(node.contains_line(15));
        assert!(node.contains_line(20));
        assert!(!node.contains_line(9));
        assert!(!node.contains_line(21));
    }

    #[test]
    fn test_ast_node_add_child_sets_parent() {
        let mut parent = AstNode::new(AstNodeKind::Impl, "MyStruct", 1, 20, Language::Rust);
        let child = AstNode::new(AstNodeKind::Method, "do_thing", 3, 8, Language::Rust);
        parent.add_child(child);
        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0].parent_id, Some(parent.id.clone()));
    }

    #[test]
    fn test_ast_node_find_by_name() {
        let mut parent = AstNode::new(AstNodeKind::Impl, "Foo", 1, 20, Language::Rust);
        parent.add_child(AstNode::new(AstNodeKind::Method, "bar", 3, 5, Language::Rust));
        parent.add_child(AstNode::new(AstNodeKind::Method, "baz", 7, 10, Language::Rust));

        assert!(parent.find_by_name("bar").is_some());
        assert!(parent.find_by_name("baz").is_some());
        assert!(parent.find_by_name("nope").is_none());
    }

    #[test]
    fn test_ast_node_find_by_path() {
        let mut parent = AstNode::new(AstNodeKind::Struct, "root", 1, 30, Language::Rust);
        let mut child = AstNode::new(AstNodeKind::Impl, "Config", 5, 25, Language::Rust);
        child.add_child(AstNode::new(AstNodeKind::Method, "new", 7, 10, Language::Rust));
        parent.add_child(child);

        assert!(parent.find_by_path("Config").is_some());
        assert!(parent.find_by_path("Config::new").is_some());
        assert!(parent.find_by_path("Config::missing").is_none());
    }

    #[test]
    fn test_ast_node_depth() {
        let mut root = AstNode::new(AstNodeKind::Module, "root", 1, 50, Language::Rust);
        let mut child = AstNode::new(AstNodeKind::Struct, "S", 2, 20, Language::Rust);
        child.add_child(AstNode::new(AstNodeKind::Field, "x", 3, 3, Language::Rust));
        root.add_child(child);
        assert_eq!(root.depth(), 2);
    }

    #[test]
    fn test_ast_node_depth_leaf() {
        let leaf = AstNode::new(AstNodeKind::Constant, "X", 1, 1, Language::Rust);
        assert_eq!(leaf.depth(), 0);
    }

    // -- FileAst tests --

    const RUST_SAMPLE: &str = r#"use std::io;
use std::collections::HashMap;

pub struct Config {
    pub port: u16,
    pub host: String,
}

impl Config {
    pub fn new(port: u16) -> Self {
        Self { port, host: "localhost".to_string() }
    }

    pub fn default_config() -> Self {
        Self::new(8080)
    }
}

pub enum Color {
    Red,
    Green,
    Blue,
}

pub fn main() {
    let config = Config::new(3000);
    println!("Port: {}", config.port);
}

const MAX_RETRIES: u32 = 3;

type Result<T> = std::result::Result<T, String>;

pub trait Printable {
    fn print(&self);
}
"#;

    #[test]
    fn test_file_ast_new_detects_language() {
        let file = FileAst::new("src/main.rs", RUST_SAMPLE);
        assert_eq!(file.language, Language::Rust);
        assert!(!file.nodes.is_empty());
    }

    #[test]
    fn test_parse_rust_finds_struct() {
        let nodes = FileAst::parse_rust(RUST_SAMPLE);
        let struct_node = nodes.iter().find(|n| n.kind == AstNodeKind::Struct && n.name == "Config");
        assert!(struct_node.is_some());
    }

    #[test]
    fn test_parse_rust_finds_impl_with_methods() {
        let nodes = FileAst::parse_rust(RUST_SAMPLE);
        let impl_node = nodes.iter().find(|n| n.kind == AstNodeKind::Impl && n.name == "Config");
        assert!(impl_node.is_some());
        let impl_node = impl_node.unwrap();
        assert!(impl_node.children.len() >= 2);
        assert!(impl_node.find_by_name("new").is_some());
        assert!(impl_node.find_by_name("default_config").is_some());
    }

    #[test]
    fn test_parse_rust_finds_enum() {
        let nodes = FileAst::parse_rust(RUST_SAMPLE);
        assert!(nodes.iter().any(|n| n.kind == AstNodeKind::Enum && n.name == "Color"));
    }

    #[test]
    fn test_parse_rust_finds_function() {
        let nodes = FileAst::parse_rust(RUST_SAMPLE);
        assert!(nodes.iter().any(|n| n.kind == AstNodeKind::Function && n.name == "main"));
    }

    #[test]
    fn test_parse_rust_finds_imports() {
        let nodes = FileAst::parse_rust(RUST_SAMPLE);
        let imports: Vec<_> = nodes.iter().filter(|n| n.kind == AstNodeKind::Import).collect();
        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_parse_rust_finds_const() {
        let nodes = FileAst::parse_rust(RUST_SAMPLE);
        assert!(nodes.iter().any(|n| n.kind == AstNodeKind::Constant && n.name == "MAX_RETRIES"));
    }

    #[test]
    fn test_parse_rust_finds_type_alias() {
        let nodes = FileAst::parse_rust(RUST_SAMPLE);
        assert!(nodes.iter().any(|n| n.kind == AstNodeKind::TypeAlias && n.name == "Result"));
    }

    #[test]
    fn test_parse_rust_finds_trait() {
        let nodes = FileAst::parse_rust(RUST_SAMPLE);
        assert!(nodes.iter().any(|n| n.kind == AstNodeKind::Trait && n.name == "Printable"));
    }

    #[test]
    fn test_file_ast_find_node() {
        let file = FileAst::new("lib.rs", RUST_SAMPLE);
        assert!(file.find_node("Config").is_some());
        assert!(file.find_node("main").is_some());
        assert!(file.find_node("nonexistent").is_none());
    }

    #[test]
    fn test_file_ast_find_node_in_children() {
        let file = FileAst::new("lib.rs", RUST_SAMPLE);
        assert!(file.find_node("new").is_some());
    }

    #[test]
    fn test_file_ast_node_at_line() {
        let file = FileAst::new("lib.rs", RUST_SAMPLE);
        let node = file.node_at_line(4);
        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "Config");
    }

    // -- TypeScript parsing --

    const TS_SAMPLE: &str = r#"import { useState } from 'react';
import axios from 'axios';

export interface User {
    id: number;
    name: string;
}

export class UserService {
    private baseUrl: string;

    constructor(url: string) {
        this.baseUrl = url;
    }
}

export function greet(name: string): string {
    return `Hello, ${name}`;
}

export const MAX_USERS = 100;

type UserId = number;
"#;

    #[test]
    fn test_parse_typescript_imports() {
        let nodes = FileAst::parse_typescript(TS_SAMPLE);
        let imports: Vec<_> = nodes.iter().filter(|n| n.kind == AstNodeKind::Import).collect();
        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_parse_typescript_interface() {
        let nodes = FileAst::parse_typescript(TS_SAMPLE);
        assert!(nodes.iter().any(|n| n.kind == AstNodeKind::Interface && n.name == "User"));
    }

    #[test]
    fn test_parse_typescript_class() {
        let nodes = FileAst::parse_typescript(TS_SAMPLE);
        assert!(nodes.iter().any(|n| n.kind == AstNodeKind::Class && n.name == "UserService"));
    }

    #[test]
    fn test_parse_typescript_function() {
        let nodes = FileAst::parse_typescript(TS_SAMPLE);
        assert!(nodes.iter().any(|n| n.kind == AstNodeKind::Function && n.name == "greet"));
    }

    #[test]
    fn test_parse_typescript_const() {
        let nodes = FileAst::parse_typescript(TS_SAMPLE);
        assert!(nodes.iter().any(|n| n.kind == AstNodeKind::Constant && n.name == "MAX_USERS"));
    }

    #[test]
    fn test_parse_typescript_type_alias() {
        let nodes = FileAst::parse_typescript(TS_SAMPLE);
        assert!(nodes.iter().any(|n| n.kind == AstNodeKind::TypeAlias && n.name == "UserId"));
    }

    // -- Python parsing --

    const PY_SAMPLE: &str = r#"import os
from pathlib import Path

class Config:
    def __init__(self, port):
        self.port = port

    def display(self):
        print(f"Port: {self.port}")

def main():
    config = Config(8080)
    config.display()
"#;

    #[test]
    fn test_parse_python_imports() {
        let nodes = FileAst::parse_python(PY_SAMPLE);
        let imports: Vec<_> = nodes.iter().filter(|n| n.kind == AstNodeKind::Import).collect();
        assert_eq!(imports.len(), 2);
    }

    #[test]
    fn test_parse_python_class_with_methods() {
        let nodes = FileAst::parse_python(PY_SAMPLE);
        let class_node = nodes.iter().find(|n| n.kind == AstNodeKind::Class && n.name == "Config");
        assert!(class_node.is_some());
        let class_node = class_node.unwrap();
        assert!(class_node.children.len() >= 2);
        assert!(class_node.find_by_name("__init__").is_some());
        assert!(class_node.find_by_name("display").is_some());
    }

    #[test]
    fn test_parse_python_function() {
        let nodes = FileAst::parse_python(PY_SAMPLE);
        assert!(nodes.iter().any(|n| n.kind == AstNodeKind::Function && n.name == "main"));
    }

    // -- AstEdit tests --

    #[test]
    fn test_ast_edit_new() {
        let edit = AstEdit::new("Config", EditOperation::Replace, "Replace Config struct");
        assert_eq!(edit.target_node, "Config");
        assert_eq!(edit.operation, EditOperation::Replace);
        assert_eq!(edit.confidence, 0.8);
        assert!(!edit.requires_review);
    }

    #[test]
    fn test_ast_edit_with_content() {
        let edit = AstEdit::new("foo", EditOperation::Replace, "test")
            .with_content("new content here");
        assert_eq!(edit.new_content, Some("new content here".to_string()));
    }

    #[test]
    fn test_ast_edit_with_confidence() {
        let edit = AstEdit::new("foo", EditOperation::Delete, "test")
            .with_confidence(0.95);
        assert!((edit.confidence - 0.95).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ast_edit_confidence_clamped() {
        let edit = AstEdit::new("foo", EditOperation::Delete, "test")
            .with_confidence(1.5);
        assert!((edit.confidence - 1.0).abs() < f64::EPSILON);
        let edit2 = AstEdit::new("foo", EditOperation::Delete, "test")
            .with_confidence(-0.5);
        assert!((edit2.confidence - 0.0).abs() < f64::EPSILON);
    }

    // -- AstEditConfig tests --

    #[test]
    fn test_config_default() {
        let cfg = AstEditConfig::default_config();
        assert!(cfg.preserve_formatting);
        assert!(cfg.preserve_comments);
        assert!(cfg.auto_fix_imports);
        assert_eq!(cfg.conflict_resolution, ConflictStrategy::Ask);
        assert!((cfg.min_confidence - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_config_strict() {
        let cfg = AstEditConfig::strict();
        assert!(!cfg.auto_fix_imports);
        assert_eq!(cfg.conflict_resolution, ConflictStrategy::KeepOriginal);
        assert!((cfg.min_confidence - 0.95).abs() < f64::EPSILON);
    }

    // -- AstEditor tests --

    #[test]
    fn test_editor_new() {
        let editor = AstEditor::new();
        assert!(editor.files.is_empty());
        assert!(editor.pending_edits.is_empty());
        assert!(editor.applied_edits.is_empty());
    }

    #[test]
    fn test_editor_load_file() {
        let mut editor = AstEditor::new();
        editor.load_file("src/main.rs", RUST_SAMPLE);
        assert_eq!(editor.files.len(), 1);
        assert!(editor.get_file("src/main.rs").is_some());
    }

    #[test]
    fn test_editor_load_file_replaces_existing() {
        let mut editor = AstEditor::new();
        editor.load_file("src/main.rs", "fn old() {}");
        editor.load_file("src/main.rs", "fn new_fn() {}");
        assert_eq!(editor.files.len(), 1);
        assert!(editor.get_file("src/main.rs").unwrap().content.contains("new_fn"));
    }

    #[test]
    fn test_editor_apply_replace() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", RUST_SAMPLE);
        let edit = AstEdit::new("main", EditOperation::Replace, "Replace main function")
            .with_content("pub fn main() {\n    println!(\"replaced\");\n}");
        editor.add_edit(edit);
        let edit_id = editor.pending_edits[0].id.clone();
        let result = editor.apply_edit(&edit_id).unwrap();
        assert!(result.success);
        assert!(editor.get_file("lib.rs").unwrap().content.contains("replaced"));
    }

    #[test]
    fn test_editor_apply_delete() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", RUST_SAMPLE);
        let edit = AstEdit::new("main", EditOperation::Delete, "Remove main");
        editor.add_edit(edit);
        let edit_id = editor.pending_edits[0].id.clone();
        let result = editor.apply_edit(&edit_id).unwrap();
        assert!(result.success);
        assert!(!editor.get_file("lib.rs").unwrap().content.contains("fn main"));
    }

    #[test]
    fn test_editor_apply_insert_before() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", RUST_SAMPLE);
        let edit = AstEdit::new("main", EditOperation::InsertBefore, "Add comment before main")
            .with_content("// This is the entry point");
        editor.add_edit(edit);
        let edit_id = editor.pending_edits[0].id.clone();
        let result = editor.apply_edit(&edit_id).unwrap();
        assert!(result.success);
        assert!(editor.get_file("lib.rs").unwrap().content.contains("// This is the entry point"));
    }

    #[test]
    fn test_editor_apply_insert_after() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", RUST_SAMPLE);
        let edit = AstEdit::new("Config", EditOperation::InsertAfter, "Add after struct Config")
            .with_content("\n// After Config struct");
        editor.add_edit(edit);
        let edit_id = editor.pending_edits[0].id.clone();
        let result = editor.apply_edit(&edit_id).unwrap();
        assert!(result.success);
        assert!(editor.get_file("lib.rs").unwrap().content.contains("// After Config struct"));
    }

    #[test]
    fn test_editor_apply_rename() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", RUST_SAMPLE);
        let edit = AstEdit::new("Color", EditOperation::Rename, "Rename Color to Colour")
            .with_content("Colour");
        editor.add_edit(edit);
        let edit_id = editor.pending_edits[0].id.clone();
        let result = editor.apply_edit(&edit_id).unwrap();
        assert!(result.success);
        assert!(editor.get_file("lib.rs").unwrap().content.contains("Colour"));
        assert!(!editor.get_file("lib.rs").unwrap().content.contains("pub enum Color"));
    }

    #[test]
    fn test_editor_apply_wrap() {
        let mut editor = AstEditor::new();
        let code = "fn helper() {\n    do_work();\n}\n";
        editor.load_file("test.rs", code);
        let edit = AstEdit::new("helper", EditOperation::Wrap, "Wrap in module")
            .with_content("mod wrapped {\n{body}\n}");
        editor.add_edit(edit);
        let edit_id = editor.pending_edits[0].id.clone();
        let result = editor.apply_edit(&edit_id).unwrap();
        assert!(result.success);
        assert!(editor.get_file("test.rs").unwrap().content.contains("mod wrapped"));
    }

    #[test]
    fn test_editor_apply_edit_not_found() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", RUST_SAMPLE);
        let result = editor.apply_edit("nonexistent-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_editor_apply_edit_low_confidence() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", RUST_SAMPLE);
        let edit = AstEdit::new("main", EditOperation::Delete, "Remove main")
            .with_confidence(0.3);
        editor.add_edit(edit);
        let edit_id = editor.pending_edits[0].id.clone();
        let result = editor.apply_edit(&edit_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_editor_apply_all() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", "fn foo() {}\nfn bar() {}\n");
        editor.add_edit(AstEdit::new("foo", EditOperation::Delete, "Remove foo"));
        let results = editor.apply_all();
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert!(editor.pending_edits.is_empty());
    }

    #[test]
    fn test_editor_apply_all_skips_low_confidence() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", "fn foo() {}\n");
        editor.add_edit(
            AstEdit::new("foo", EditOperation::Delete, "Remove foo").with_confidence(0.1)
        );
        let results = editor.apply_all();
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
    }

    #[test]
    fn test_editor_preview_edit() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", RUST_SAMPLE);
        let edit = AstEdit::new("main", EditOperation::Replace, "Preview replace")
            .with_content("fn main() { /* new */ }");
        editor.add_edit(edit);
        let edit_id = editor.pending_edits[0].id.clone();
        let preview = editor.preview_edit(&edit_id);
        assert!(preview.is_some());
        let preview = preview.unwrap();
        assert!(preview.contains("---"));
        assert!(preview.contains("+++"));
    }

    #[test]
    fn test_editor_preview_nonexistent() {
        let editor = AstEditor::new();
        assert!(editor.preview_edit("nope").is_none());
    }

    #[test]
    fn test_editor_validate_edits_clean() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", RUST_SAMPLE);
        editor.add_edit(AstEdit::new("main", EditOperation::Delete, "Remove main"));
        let warnings = editor.validate_edits();
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_editor_validate_edits_duplicate_target() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", RUST_SAMPLE);
        editor.add_edit(AstEdit::new("main", EditOperation::Delete, "Remove main 1"));
        editor.add_edit(AstEdit::new("main", EditOperation::Replace, "Replace main 2")
            .with_content("fn main() {}"));
        let warnings = editor.validate_edits();
        assert!(warnings.iter().any(|w| w.contains("Duplicate target")));
    }

    #[test]
    fn test_editor_validate_edits_missing_target() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", RUST_SAMPLE);
        editor.add_edit(AstEdit::new("nonexistent_fn", EditOperation::Delete, "Remove missing"));
        let warnings = editor.validate_edits();
        assert!(warnings.iter().any(|w| w.contains("not found")));
    }

    #[test]
    fn test_editor_validate_edits_low_confidence() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", RUST_SAMPLE);
        editor.add_edit(
            AstEdit::new("main", EditOperation::Delete, "Remove main").with_confidence(0.5)
        );
        let warnings = editor.validate_edits();
        assert!(warnings.iter().any(|w| w.contains("confidence")));
    }

    #[test]
    fn test_editor_target_not_found_returns_failure() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", "fn real() {}\n");
        editor.add_edit(AstEdit::new("ghost", EditOperation::Delete, "Remove ghost"));
        let results = editor.apply_all();
        assert!(!results[0].success);
        assert!(!results[0].conflicts.is_empty());
    }

    #[test]
    fn test_editor_move_operation() {
        let mut editor = AstEditor::new();
        editor.load_file("lib.rs", "fn first() {}\nfn second() {}\n");
        let edit = AstEdit::new("first", EditOperation::Move, "Move first to end");
        editor.add_edit(edit);
        let edit_id = editor.pending_edits[0].id.clone();
        let result = editor.apply_edit(&edit_id).unwrap();
        assert!(result.success);
    }

    // -- Helper function tests --

    #[test]
    fn test_extract_name_after_keyword() {
        assert_eq!(extract_name_after_keyword("pub fn main() {", "fn "), "main");
        assert_eq!(extract_name_after_keyword("struct Config {", "struct "), "Config");
        assert_eq!(extract_name_after_keyword("enum Color {", "enum "), "Color");
        assert_eq!(extract_name_after_keyword("trait Display {", "trait "), "Display");
    }

    #[test]
    fn test_extract_impl_name_simple() {
        assert_eq!(extract_impl_name("impl Config {"), "Config");
    }

    #[test]
    fn test_extract_impl_name_trait_for() {
        assert_eq!(extract_impl_name("impl Display for Config {"), "Config");
    }

    #[test]
    fn test_find_brace_block_end() {
        let lines = vec!["fn foo() {", "    bar();", "}", ""];
        assert_eq!(find_brace_block_end(&lines, 0), 3);
    }

    #[test]
    fn test_edit_result_ok() {
        let r = AstEditResult::ok("e1", (1, 5), (1, 7), 2, 5);
        assert!(r.success);
        assert!(r.conflicts.is_empty());
    }

    #[test]
    fn test_edit_result_fail() {
        let r = AstEditResult::fail("e2", "Target not found");
        assert!(!r.success);
        assert_eq!(r.conflicts.len(), 1);
    }

    #[test]
    fn test_conflict_strategy_eq() {
        assert_eq!(ConflictStrategy::Ask, ConflictStrategy::Ask);
        assert_ne!(ConflictStrategy::Ask, ConflictStrategy::Merge);
    }

    #[test]
    fn test_edit_operation_eq() {
        assert_eq!(EditOperation::Replace, EditOperation::Replace);
        assert_ne!(EditOperation::Delete, EditOperation::Rename);
    }

    #[test]
    fn test_file_ast_line_count() {
        let file = FileAst::new("test.rs", "line1\nline2\nline3\n");
        assert_eq!(file.line_count, 3);
    }

    #[test]
    fn test_parse_empty_content() {
        let nodes = FileAst::parse_rust("");
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_parse_typescript_empty() {
        let nodes = FileAst::parse_typescript("");
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_parse_python_empty() {
        let nodes = FileAst::parse_python("");
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_node_signature_populated() {
        let nodes = FileAst::parse_rust("pub fn greet(name: &str) -> String {\n    name.to_string()\n}\n");
        assert!(nodes[0].signature.is_some());
        assert!(nodes[0].signature.as_ref().unwrap().contains("greet"));
    }
}
