#![allow(dead_code)]
//! Layered configuration system — two complementary subsystems.
//!
//! ## Subsystem A — Wave 4 resolver (system → project → user → env → cli)
//! Claw-code parity: typed `ConfigValue` entries with override / inherit /
//! clear semantics, tracked origins, and sorted priority resolution.
//!
//! ## Subsystem B — Three-level JSON deep-merge (user → project → local)
//! `LayeredConfig` merges `~/.vibecli/config.toml` (user), workspace
//! `.vibecli/settings.json` (project), and `.vibecli/settings.local.json`
//! (local / gitignored).  Deep-merge semantics: overlay keys win, objects
//! merge recursively, arrays are replaced (not appended).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

// ─── Config Layer ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LayerPriority {
    /// Lowest priority: built-in defaults.
    System,
    /// Per-repo CLAUDE.md / .claude/settings.json.
    Project,
    /// User's ~/.claude/settings.json.
    User,
    /// Environment variable overrides.
    Environment,
    /// CLI flags: highest priority.
    Cli,
}

impl std::fmt::Display for LayerPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::System      => write!(f, "system"),
            Self::Project     => write!(f, "project"),
            Self::User        => write!(f, "user"),
            Self::Environment => write!(f, "env"),
            Self::Cli         => write!(f, "cli"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigLayerEntry {
    pub priority: LayerPriority,
    pub source: String,
    pub values: HashMap<String, ConfigValue>,
}

impl ConfigLayerEntry {
    pub fn new(priority: LayerPriority, source: impl Into<String>) -> Self {
        Self { priority, source: source.into(), values: HashMap::new() }
    }

    pub fn set(&mut self, key: impl Into<String>, value: ConfigValue) {
        self.values.insert(key.into(), value);
    }
    pub fn set_str(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.set(key, ConfigValue::Str(value.into()));
    }
    pub fn set_bool(&mut self, key: impl Into<String>, value: bool) {
        self.set(key, ConfigValue::Bool(value));
    }
    pub fn set_int(&mut self, key: impl Into<String>, value: i64) {
        self.set(key, ConfigValue::Int(value));
    }
    pub fn clear(&mut self, key: impl Into<String>) {
        self.set(key, ConfigValue::Cleared);
    }
}

// ─── Config Values ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConfigValue {
    Str(String),
    Bool(bool),
    Int(i64),
    Float(f64),
    List(Vec<String>),
    /// Explicitly clears a key set at a lower priority.
    Cleared,
}

impl ConfigValue {
    pub fn as_str(&self) -> Option<&str> { if let Self::Str(s) = self { Some(s) } else { None } }
    pub fn as_bool(&self) -> Option<bool> { if let Self::Bool(b) = self { Some(*b) } else { None } }
    pub fn as_int(&self) -> Option<i64>   { if let Self::Int(i) = self { Some(*i) } else { None } }
}

impl std::fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Str(s)    => write!(f, "{s}"),
            Self::Bool(b)   => write!(f, "{b}"),
            Self::Int(i)    => write!(f, "{i}"),
            Self::Float(x)  => write!(f, "{x}"),
            Self::List(v)   => write!(f, "[{}]", v.join(", ")),
            Self::Cleared   => write!(f, "(cleared)"),
        }
    }
}

// ─── Resolved Config ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedConfig {
    pub values: HashMap<String, ConfigValue>,
    /// Which layer set each key.
    pub origins: HashMap<String, LayerPriority>,
}

impl ResolvedConfig {
    pub fn get(&self, key: &str) -> Option<&ConfigValue> { self.values.get(key) }
    pub fn get_str(&self, key: &str) -> Option<&str> { self.get(key)?.as_str() }
    pub fn get_bool(&self, key: &str) -> Option<bool> { self.get(key)?.as_bool() }
    pub fn get_int(&self, key: &str) -> Option<i64> { self.get(key)?.as_int() }
    pub fn origin_of(&self, key: &str) -> Option<&LayerPriority> { self.origins.get(key) }
}

// ─── Config Resolver ─────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ConfigResolver {
    pub layers: Vec<ConfigLayerEntry>,
}

impl ConfigResolver {
    pub fn new() -> Self { Self { layers: Vec::new() } }

    pub fn add_layer(&mut self, layer: ConfigLayerEntry) {
        self.layers.push(layer);
        // Keep sorted by priority
        self.layers.sort_by(|a, b| a.priority.cmp(&b.priority));
    }

    /// Resolve all keys: higher-priority layers win; Cleared removes the key.
    pub fn resolve(&self) -> ResolvedConfig {
        let mut values: HashMap<String, ConfigValue> = HashMap::new();
        let mut origins: HashMap<String, LayerPriority> = HashMap::new();

        // Process layers from lowest to highest priority
        for layer in &self.layers {
            for (key, value) in &layer.values {
                if value == &ConfigValue::Cleared {
                    values.remove(key);
                    origins.remove(key);
                } else {
                    values.insert(key.clone(), value.clone());
                    origins.insert(key.clone(), layer.priority.clone());
                }
            }
        }

        ResolvedConfig { values, origins }
    }

    pub fn layer_count(&self) -> usize { self.layers.len() }
}

impl Default for ConfigResolver {
    fn default() -> Self { Self::new() }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn layer(p: LayerPriority, kv: &[(&str, &str)]) -> ConfigLayerEntry {
        let mut l = ConfigLayerEntry::new(p, "test");
        for (k, v) in kv { l.set_str(*k, *v); }
        l
    }

    #[test]
    fn test_higher_priority_wins() {
        let mut res = ConfigResolver::new();
        res.add_layer(layer(LayerPriority::System,  &[("model", "haiku")]));
        res.add_layer(layer(LayerPriority::Project, &[("model", "sonnet")]));
        let cfg = res.resolve();
        assert_eq!(cfg.get_str("model"), Some("sonnet"));
    }

    #[test]
    fn test_cli_overrides_all() {
        let mut res = ConfigResolver::new();
        res.add_layer(layer(LayerPriority::System, &[("model", "haiku")]));
        res.add_layer(layer(LayerPriority::User,   &[("model", "opus")]));
        res.add_layer(layer(LayerPriority::Cli,    &[("model", "custom")]));
        let cfg = res.resolve();
        assert_eq!(cfg.get_str("model"), Some("custom"));
    }

    #[test]
    fn test_cleared_removes_key() {
        let mut res = ConfigResolver::new();
        res.add_layer(layer(LayerPriority::System, &[("timeout", "30")]));
        let mut cli = ConfigLayerEntry::new(LayerPriority::Cli, "cli");
        cli.clear("timeout");
        res.add_layer(cli);
        let cfg = res.resolve();
        assert!(cfg.get("timeout").is_none());
    }

    #[test]
    fn test_missing_key_returns_none() {
        let res = ConfigResolver::new();
        let cfg = res.resolve();
        assert!(cfg.get("anything").is_none());
    }

    #[test]
    fn test_origin_tracked() {
        let mut res = ConfigResolver::new();
        res.add_layer(layer(LayerPriority::Project, &[("k", "v")]));
        let cfg = res.resolve();
        assert_eq!(cfg.origin_of("k"), Some(&LayerPriority::Project));
    }

    #[test]
    fn test_bool_value() {
        let mut l = ConfigLayerEntry::new(LayerPriority::System, "s");
        l.set_bool("verbose", true);
        let mut res = ConfigResolver::new();
        res.add_layer(l);
        let cfg = res.resolve();
        assert_eq!(cfg.get_bool("verbose"), Some(true));
    }

    #[test]
    fn test_int_value() {
        let mut l = ConfigLayerEntry::new(LayerPriority::System, "s");
        l.set_int("max_tokens", 4096);
        let mut res = ConfigResolver::new();
        res.add_layer(l);
        let cfg = res.resolve();
        assert_eq!(cfg.get_int("max_tokens"), Some(4096));
    }

    #[test]
    fn test_multiple_keys_merged() {
        let mut res = ConfigResolver::new();
        res.add_layer(layer(LayerPriority::System,  &[("a", "1"), ("b", "2")]));
        res.add_layer(layer(LayerPriority::Project, &[("c", "3")]));
        let cfg = res.resolve();
        assert_eq!(cfg.get_str("a"), Some("1"));
        assert_eq!(cfg.get_str("b"), Some("2"));
        assert_eq!(cfg.get_str("c"), Some("3"));
    }

    #[test]
    fn test_layers_sorted_by_priority() {
        let mut res = ConfigResolver::new();
        res.add_layer(layer(LayerPriority::Cli,    &[]));
        res.add_layer(layer(LayerPriority::System, &[]));
        res.add_layer(layer(LayerPriority::User,   &[]));
        assert_eq!(res.layers[0].priority, LayerPriority::System);
        assert_eq!(res.layers[2].priority, LayerPriority::Cli);
    }

    #[test]
    fn test_layer_count() {
        let mut res = ConfigResolver::new();
        res.add_layer(layer(LayerPriority::System, &[]));
        assert_eq!(res.layer_count(), 1);
    }

    #[test]
    fn test_config_value_display() {
        assert_eq!(ConfigValue::Str("hello".into()).to_string(), "hello");
        assert_eq!(ConfigValue::Bool(true).to_string(), "true");
        assert_eq!(ConfigValue::Int(42).to_string(), "42");
        assert_eq!(ConfigValue::Cleared.to_string(), "(cleared)");
    }

    #[test]
    fn test_priority_ordering() {
        assert!(LayerPriority::System < LayerPriority::Cli);
        assert!(LayerPriority::Project < LayerPriority::Environment);
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(LayerPriority::Environment.to_string(), "env");
        assert_eq!(LayerPriority::Cli.to_string(), "cli");
    }

    #[test]
    fn test_env_overrides_project() {
        let mut res = ConfigResolver::new();
        res.add_layer(layer(LayerPriority::Project,     &[("key", "project-val")]));
        res.add_layer(layer(LayerPriority::Environment, &[("key", "env-val")]));
        let cfg = res.resolve();
        assert_eq!(cfg.get_str("key"), Some("env-val"));
        assert_eq!(cfg.origin_of("key"), Some(&LayerPriority::Environment));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Subsystem B — Three-level JSON deep-merge  (user → project → local)
// ═══════════════════════════════════════════════════════════════════════════════

// ── ConfigLayer (JSON-merge variant) ─────────────────────────────────────────

/// Identifies which of the three JSON-merge layers a diagnostic came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigLayer {
    User,
    Project,
    Local,
}

impl std::fmt::Display for ConfigLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User    => write!(f, "user"),
            Self::Project => write!(f, "project"),
            Self::Local   => write!(f, "local"),
        }
    }
}

// ── ConfigError ───────────────────────────────────────────────────────────────

/// A validation or parse error tied to a specific JSON-merge config layer.
#[derive(Debug, Clone)]
pub struct ConfigError {
    pub layer: ConfigLayer,
    pub line: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(line) = self.line {
            write!(f, "[{}:{}] {}", self.layer, line, self.message)
        } else {
            write!(f, "[{}] {}", self.layer, self.message)
        }
    }
}

// ── LayeredConfig ─────────────────────────────────────────────────────────────

/// Three-level JSON deep-merge configuration.
///
/// Priority (ascending): `user` → `project` → `local`.
/// Objects are merged recursively; all other types (including arrays) are
/// replaced wholesale by the overlay value.
#[derive(Debug, Clone, Default)]
pub struct LayeredConfig {
    pub user:    Value,
    pub project: Value,
    pub local:   Value,
}

impl LayeredConfig {
    /// Deep-merge `overlay` onto `base`.
    ///
    /// * `Object` + `Object` → keys merged recursively; overlay wins on
    ///   conflict.
    /// * Any other combination → `overlay` replaces `base` entirely.
    pub fn deep_merge(base: &Value, overlay: &Value) -> Value {
        match (base, overlay) {
            (Value::Object(base_map), Value::Object(overlay_map)) => {
                let mut merged = base_map.clone();
                for (key, overlay_val) in overlay_map {
                    let merged_val = match merged.get(key) {
                        Some(base_val) => Self::deep_merge(base_val, overlay_val),
                        None           => overlay_val.clone(),
                    };
                    merged.insert(key.clone(), merged_val);
                }
                Value::Object(merged)
            }
            // Non-object: overlay always wins.
            (_, overlay_val) => overlay_val.clone(),
        }
    }

    /// Merge all three layers in priority order: user → project → local.
    pub fn merge(&self) -> Value {
        let after_project = Self::deep_merge(&self.user, &self.project);
        Self::deep_merge(&after_project, &self.local)
    }

    /// Validate that `value` is a JSON object (or null).
    /// Returns a list of `ConfigError`s referencing `layer`.
    pub fn validate_schema(value: &Value, layer: &ConfigLayer) -> Vec<ConfigError> {
        match value {
            Value::Object(_) | Value::Null => vec![],
            other => vec![ConfigError {
                layer: layer.clone(),
                line: None,
                message: format!(
                    "expected object, found {}",
                    json_type_name(other)
                ),
            }],
        }
    }

    /// Load configuration from `workspace`.
    ///
    /// * User layer  → `~/.vibecli/config.toml` (converted to JSON)
    /// * Project layer → `<workspace>/.vibecli/settings.json`
    /// * Local layer   → `<workspace>/.vibecli/settings.local.json`
    ///
    /// Missing or unparseable files silently produce an empty object.
    pub fn load(workspace: &Path) -> Result<Self, String> {
        let user_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".vibecli")
            .join("config.toml");

        let project_path = workspace.join(".vibecli").join("settings.json");
        let local_path   = workspace.join(".vibecli").join("settings.local.json");

        Ok(Self {
            user:    load_toml_as_json(&user_path),
            project: read_json_file(&project_path),
            local:   read_json_file(&local_path),
        })
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null      => "null",
        Value::Bool(_)   => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_)  => "array",
        Value::Object(_) => "object",
    }
}

/// Read a JSON file; return an empty object on any error.
fn read_json_file(path: &Path) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()))
}

/// Read a TOML file and convert it to a JSON `Value`; return an empty object
/// on any error.
fn load_toml_as_json(path: &Path) -> Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str::<toml::Value>(&s).ok())
        .and_then(|v| serde_json::to_value(v).ok())
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()))
}

// ── Unit tests for subsystem B ────────────────────────────────────────────────

#[cfg(test)]
mod layered_config_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn deep_merge_overlay_wins() {
        let base    = json!({"model": "gpt-4"});
        let overlay = json!({"model": "claude"});
        let merged  = LayeredConfig::deep_merge(&base, &overlay);
        assert_eq!(merged["model"], "claude");
    }

    #[test]
    fn deep_merge_nested_objects() {
        let base    = json!({"a": {"x": 1, "y": 2}});
        let overlay = json!({"a": {"y": 3}});
        let merged  = LayeredConfig::deep_merge(&base, &overlay);
        assert_eq!(merged["a"]["x"], 1);
        assert_eq!(merged["a"]["y"], 3);
    }

    #[test]
    fn deep_merge_base_keys_preserved() {
        let base    = json!({"keep": "me", "override": "old"});
        let overlay = json!({"override": "new"});
        let merged  = LayeredConfig::deep_merge(&base, &overlay);
        assert_eq!(merged["keep"], "me");
        assert_eq!(merged["override"], "new");
    }

    #[test]
    fn deep_merge_array_replaced_not_appended() {
        let base    = json!({"items": [1, 2, 3]});
        let overlay = json!({"items": [4, 5]});
        let merged  = LayeredConfig::deep_merge(&base, &overlay);
        assert_eq!(merged["items"], json!([4, 5]));
    }

    #[test]
    fn layer_priority_local_over_project_over_user() {
        let config = LayeredConfig {
            user:    json!({"model": "gpt-4"}),
            project: json!({"model": "claude"}),
            local:   json!({"model": "ollama"}),
        };
        assert_eq!(config.merge()["model"], "ollama");
    }

    #[test]
    fn missing_layer_files_produce_empty_object() {
        let empty = read_json_file(Path::new("/nonexistent/path/settings.json"));
        assert!(empty.is_object());
        assert_eq!(empty.as_object().map_or(0, |m| m.len()), 0);
    }

    #[test]
    fn layer_name_display() {
        assert_eq!(ConfigLayer::User.to_string(),    "user");
        assert_eq!(ConfigLayer::Project.to_string(), "project");
        assert_eq!(ConfigLayer::Local.to_string(),   "local");
    }

    #[test]
    fn validate_schema_reports_layer_name_on_error() {
        let bad    = Value::String("not-an-object".into());
        let errors = LayeredConfig::validate_schema(&bad, &ConfigLayer::Project);
        assert!(!errors.is_empty());
        assert!(errors[0].to_string().contains("project"));
    }
}
