//! Layered configuration resolution (system → project → local → env).
//!
//! Claw-code parity Wave 4: merges config from multiple sources with
//! well-defined precedence, supporting override, inherit, and clear semantics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
pub struct ConfigLayer {
    pub priority: LayerPriority,
    pub source: String,
    pub values: HashMap<String, ConfigValue>,
}

impl ConfigLayer {
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

pub struct ConfigResolver {
    pub layers: Vec<ConfigLayer>,
}

impl ConfigResolver {
    pub fn new() -> Self { Self { layers: Vec::new() } }

    pub fn add_layer(&mut self, layer: ConfigLayer) {
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

    fn layer(p: LayerPriority, kv: &[(&str, &str)]) -> ConfigLayer {
        let mut l = ConfigLayer::new(p, "test");
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
        let mut cli = ConfigLayer::new(LayerPriority::Cli, "cli");
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
        let mut l = ConfigLayer::new(LayerPriority::System, "s");
        l.set_bool("verbose", true);
        let mut res = ConfigResolver::new();
        res.add_layer(l);
        let cfg = res.resolve();
        assert_eq!(cfg.get_bool("verbose"), Some(true));
    }

    #[test]
    fn test_int_value() {
        let mut l = ConfigLayer::new(LayerPriority::System, "s");
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
