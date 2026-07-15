//! `${…}` expression resolution over workflow and task state.
//!
//! Supported reference roots:
//! - `workflow.input.<path>`
//! - `workflow.variables.<path>`
//! - `workflow.output.<path>`
//! - `<taskRef>.output.<path>`
//! - `<taskRef>.input.<path>`
//!
//! Paths use dotted segments with optional `[index]` array access, e.g.
//! `orders.output.items[0].id`. A string that is *exactly* `${expr}` resolves to the
//! typed value; a string with embedded `${expr}` fragments resolves by interpolation.

use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Read-only view of the state an expression may reference.
pub struct EvalContext<'a> {
    /// The workflow input document.
    pub workflow_input: &'a Value,
    /// Workflow-scoped variables.
    pub workflow_variables: &'a Map<String, Value>,
    /// The workflow output accumulated so far.
    pub workflow_output: &'a Value,
    /// Task reference name → task output.
    pub task_outputs: &'a BTreeMap<String, Value>,
    /// Task reference name → task input.
    pub task_inputs: &'a BTreeMap<String, Value>,
}

impl<'a> EvalContext<'a> {
    /// Resolve every `${…}` expression contained in `value`, recursing through arrays and objects.
    pub fn resolve(&self, value: &Value) -> Value {
        match value {
            Value::String(s) => self.resolve_string(s),
            Value::Array(items) => Value::Array(items.iter().map(|v| self.resolve(v)).collect()),
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), self.resolve(v)))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    /// Resolve a parameter map (each value resolved independently).
    pub fn resolve_map(&self, params: &Map<String, Value>) -> Map<String, Value> {
        params
            .iter()
            .map(|(k, v)| (k.clone(), self.resolve(v)))
            .collect()
    }

    fn resolve_string(&self, s: &str) -> Value {
        // Whole-string expression → typed value.
        if let Some(expr) = whole_expression(s) {
            return self.lookup(expr).unwrap_or(Value::Null);
        }
        // Otherwise interpolate any embedded ${…} fragments as strings.
        if !s.contains("${") {
            return Value::String(s.to_string());
        }
        Value::String(self.interpolate(s))
    }

    fn interpolate(&self, s: &str) -> String {
        let mut out = String::new();
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                if let Some(end) = s[i + 2..].find('}') {
                    let expr = &s[i + 2..i + 2 + end];
                    let resolved = self.lookup(expr).unwrap_or(Value::Null);
                    out.push_str(&stringify(&resolved));
                    i = i + 2 + end + 1;
                    continue;
                }
            }
            out.push(bytes[i] as char);
            i += 1;
        }
        out
    }

    /// Resolve a single reference path (without the surrounding `${}`), e.g. `workflow.input.x`.
    pub fn lookup(&self, expr: &str) -> Option<Value> {
        let expr = expr.trim();
        let (root, rest) = split_first(expr);
        match root {
            "workflow" => {
                let (kind, path) = split_first(rest);
                match kind {
                    "input" => traverse(self.workflow_input, path),
                    "output" => traverse(self.workflow_output, path),
                    "variables" => {
                        traverse(&Value::Object(self.workflow_variables.clone()), path)
                    }
                    _ => None,
                }
            }
            task_ref if !task_ref.is_empty() => {
                let (kind, path) = split_first(rest);
                match kind {
                    "output" => self.task_outputs.get(task_ref).and_then(|v| traverse(v, path)),
                    "input" => self.task_inputs.get(task_ref).and_then(|v| traverse(v, path)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

/// If `s` is exactly `${…}`, return the inner expression.
fn whole_expression(s: &str) -> Option<&str> {
    let t = s.trim();
    let inner = t.strip_prefix("${")?.strip_suffix('}')?;
    // Reject if the inner itself contains a nested `${` (that's interpolation, not a whole expr).
    if inner.contains("${") {
        return None;
    }
    Some(inner)
}

/// Split `a.b.c` into (`a`, `b.c`); the remainder is empty when there is no dot.
fn split_first(path: &str) -> (&str, &str) {
    match path.find('.') {
        Some(i) => (&path[..i], &path[i + 1..]),
        None => (path, ""),
    }
}

/// Walk a dotted/indexed path into a JSON value.
fn traverse(value: &Value, path: &str) -> Option<Value> {
    if path.is_empty() {
        return Some(value.clone());
    }
    let mut current = value.clone();
    for raw_segment in path.split('.') {
        let (key, indices) = parse_segment(raw_segment);
        if !key.is_empty() {
            current = current.get(key)?.clone();
        }
        for idx in indices {
            current = current.get(idx)?.clone();
        }
    }
    Some(current)
}

/// Parse `items[0][1]` into (`items`, [0, 1]).
fn parse_segment(segment: &str) -> (&str, Vec<usize>) {
    let key_end = segment.find('[').unwrap_or(segment.len());
    let key = &segment[..key_end];
    let indices = segment[key_end..]
        .split(['[', ']'])
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<usize>().ok())
        .collect();
    (key, indices)
}

/// Render a JSON value as a string for interpolation.
fn stringify(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}
