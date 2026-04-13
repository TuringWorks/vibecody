//! mcp_tool_search — Lazy/deferred MCP tool schema loading.
//!
//! Only fetch full JSON schemas when the model selects a specific tool,
//! reducing upfront context overhead (analogous to Claude Code's 85% reduction).

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Lightweight descriptor for a tool — no parameter schema yet.
#[derive(Debug, Clone)]
pub struct ToolStub {
    pub name: String,
    pub description: String,
    pub category: Option<String>,
}

/// Full schema for a tool including JSON Schema parameters and usage examples.
#[derive(Debug, Clone)]
pub struct ToolSchema {
    pub stub: ToolStub,
    pub parameters: serde_json::Value,
    pub examples: Vec<String>,
}

/// Loading state of a lazy tool.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadState {
    Stub,
    Loaded,
    Failed(String),
}

/// A tool entry that starts as a stub and may be promoted to fully loaded.
#[derive(Debug)]
pub struct LazyTool {
    pub stub: ToolStub,
    pub state: LoadState,
    pub schema: Option<ToolSchema>,
}

/// Registry of lazy tools with hit/miss tracking.
#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, LazyTool>,
    pub hits: u64,
    pub misses: u64,
}

// ---------------------------------------------------------------------------
// ToolStub
// ---------------------------------------------------------------------------

impl ToolStub {
    /// Create a new stub with the given name and description.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            category: None,
        }
    }

    /// Builder-style method to set the category.
    pub fn with_category(mut self, cat: impl Into<String>) -> Self {
        self.category = Some(cat.into());
        self
    }

    /// Returns a compact single-line summary: `"- tool_name: description"`.
    pub fn stub_context_line(&self) -> String {
        format!("- {}: {}", self.name, self.description)
    }
}

// ---------------------------------------------------------------------------
// ToolSchema
// ---------------------------------------------------------------------------

impl ToolSchema {
    /// Create a new schema for the given stub and parameter JSON Schema object.
    pub fn new(stub: ToolStub, parameters: serde_json::Value) -> Self {
        Self {
            stub,
            parameters,
            examples: vec![],
        }
    }

    /// Returns the number of properties defined in the parameters JSON Schema object.
    pub fn param_count(&self) -> usize {
        self.parameters
            .get("properties")
            .and_then(|p| p.as_object())
            .map(|o| o.len())
            .unwrap_or(0)
    }
}

// ---------------------------------------------------------------------------
// ToolRegistry
// ---------------------------------------------------------------------------

impl ToolRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new stub tool. If a tool with the same name already exists it
    /// is replaced.
    pub fn register(&mut self, stub: ToolStub) {
        let name = stub.name.clone();
        self.tools.insert(
            name,
            LazyTool {
                stub,
                state: LoadState::Stub,
                schema: None,
            },
        );
    }

    /// Returns the total number of registered stubs (regardless of load state).
    pub fn stub_count(&self) -> usize {
        self.tools.len()
    }

    /// Returns the number of tools that have been fully loaded.
    pub fn loaded_count(&self) -> usize {
        self.tools
            .values()
            .filter(|t| t.state == LoadState::Loaded)
            .count()
    }

    /// Returns a reference to the stub for the named tool, if registered.
    pub fn get_stub(&self, name: &str) -> Option<&ToolStub> {
        self.tools.get(name).map(|t| &t.stub)
    }

    /// Promote a tool from stub to loaded by supplying its full schema.
    /// Returns `false` if the tool name is not registered.
    pub fn load_schema(&mut self, name: &str, schema: ToolSchema) -> bool {
        if let Some(tool) = self.tools.get_mut(name) {
            tool.state = LoadState::Loaded;
            tool.schema = Some(schema);
            true
        } else {
            false
        }
    }

    /// Returns the full schema for a loaded tool, or `None` if not loaded.
    pub fn get_schema(&self, name: &str) -> Option<&ToolSchema> {
        self.tools
            .get(name)
            .and_then(|t| t.schema.as_ref())
    }

    /// Returns a compact multi-line context string with one line per registered
    /// tool: `"- tool_name: description"`.
    pub fn stubs_context(&self) -> String {
        let mut lines: Vec<String> = self
            .tools
            .values()
            .map(|t| t.stub.stub_context_line())
            .collect();
        lines.sort(); // deterministic output
        lines.join("\n")
    }

    /// Returns the full JSON schemas for the named tools only (for injection
    /// into model context after selection).
    pub fn schemas_context(&self, names: &[&str]) -> String {
        let mut parts = Vec::new();
        for &name in names {
            if let Some(schema) = self.get_schema(name) {
                parts.push(format!(
                    "## {}\n{}\n{}",
                    name,
                    schema.stub.description,
                    serde_json::to_string_pretty(&schema.parameters)
                        .unwrap_or_else(|_| "{}".to_string())
                ));
            }
        }
        parts.join("\n\n")
    }

    /// Returns the hit rate as a fraction in [0.0, 1.0]. Returns 0.0 when no
    /// calls have been recorded to avoid NaN.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Record a cache hit.
    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    /// Record a cache miss.
    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    /// Rough token estimate: stubs_context chars/4 + schema JSON chars/4 for
    /// each named tool.
    pub fn context_token_estimate(&self, names: &[&str]) -> usize {
        let stubs_chars = self.stubs_context().len();
        let schema_chars: usize = names
            .iter()
            .filter_map(|&n| self.get_schema(n))
            .map(|s| serde_json::to_string(&s.parameters).unwrap_or_default().len())
            .sum();
        (stubs_chars + schema_chars) / 4
    }

    /// Percentage of context saved by only loading schemas for `selected_names`
    /// instead of all tools.  Clamped to [0.0, 100.0].
    pub fn savings_pct(&self, selected_names: &[&str]) -> f32 {
        let total = self.stub_count();
        if total == 0 {
            return 100.0;
        }
        let selected = selected_names.len().min(total);
        let pct = (1.0 - selected as f32 / total as f32) * 100.0;
        pct.clamp(0.0, 100.0)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_registry_with_tools(names: &[&str]) -> ToolRegistry {
        let mut reg = ToolRegistry::new();
        for &n in names {
            reg.register(ToolStub::new(n, format!("Description of {}", n)));
        }
        reg
    }

    fn make_schema(name: &str) -> ToolSchema {
        let stub = ToolStub::new(name, format!("Description of {}", name));
        ToolSchema::new(
            stub,
            json!({ "properties": { "input": { "type": "string" } }, "required": ["input"] }),
        )
    }

    #[test]
    fn test_register_increases_stub_count() {
        let mut reg = ToolRegistry::new();
        assert_eq!(reg.stub_count(), 0);
        reg.register(ToolStub::new("tool_a", "Tool A"));
        assert_eq!(reg.stub_count(), 1);
        reg.register(ToolStub::new("tool_b", "Tool B"));
        assert_eq!(reg.stub_count(), 2);
    }

    #[test]
    fn test_get_stub_returns_registered() {
        let reg = make_registry_with_tools(&["read_file"]);
        let stub = reg.get_stub("read_file");
        assert!(stub.is_some());
        assert_eq!(stub.unwrap().name, "read_file");
    }

    #[test]
    fn test_get_stub_missing_returns_none() {
        let reg = make_registry_with_tools(&["read_file"]);
        assert!(reg.get_stub("write_file").is_none());
    }

    #[test]
    fn test_load_schema_marks_loaded() {
        let mut reg = make_registry_with_tools(&["read_file"]);
        let schema = make_schema("read_file");
        let ok = reg.load_schema("read_file", schema);
        assert!(ok);
        assert_eq!(reg.tools["read_file"].state, LoadState::Loaded);
    }

    #[test]
    fn test_loaded_count_after_schema_load() {
        let mut reg = make_registry_with_tools(&["a", "b", "c"]);
        assert_eq!(reg.loaded_count(), 0);
        reg.load_schema("a", make_schema("a"));
        assert_eq!(reg.loaded_count(), 1);
        reg.load_schema("b", make_schema("b"));
        assert_eq!(reg.loaded_count(), 2);
    }

    #[test]
    fn test_stubs_context_contains_all_names() {
        let reg = make_registry_with_tools(&["tool_x", "tool_y", "tool_z"]);
        let ctx = reg.stubs_context();
        assert!(ctx.contains("tool_x"), "Missing tool_x in: {}", ctx);
        assert!(ctx.contains("tool_y"), "Missing tool_y in: {}", ctx);
        assert!(ctx.contains("tool_z"), "Missing tool_z in: {}", ctx);
    }

    #[test]
    fn test_schemas_context_contains_only_selected() {
        let mut reg = make_registry_with_tools(&["alpha", "beta", "gamma"]);
        reg.load_schema("alpha", make_schema("alpha"));
        reg.load_schema("beta", make_schema("beta"));
        reg.load_schema("gamma", make_schema("gamma"));

        let ctx = reg.schemas_context(&["alpha", "beta"]);
        assert!(ctx.contains("alpha"), "Missing alpha");
        assert!(ctx.contains("beta"), "Missing beta");
        assert!(!ctx.contains("gamma"), "gamma should not be included");
    }

    #[test]
    fn test_hit_rate_zero_when_no_calls() {
        let reg = ToolRegistry::new();
        assert_eq!(reg.hit_rate(), 0.0);
    }

    #[test]
    fn test_savings_pct_all_selected_is_zero() {
        let reg = make_registry_with_tools(&["a", "b", "c"]);
        let pct = reg.savings_pct(&["a", "b", "c"]);
        assert!(pct < 1.0, "Expected ~0%, got {}", pct);
    }

    #[test]
    fn test_savings_pct_none_selected_is_100() {
        let reg = make_registry_with_tools(&["a", "b", "c"]);
        let pct = reg.savings_pct(&[]);
        assert!((pct - 100.0).abs() < 0.01, "Expected 100%, got {}", pct);
    }
}
