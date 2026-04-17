//! Custom REPL macros — define, expand, and manage parameterized command macros.
//! FIT-GAP v11 Phase 48 — closes gap vs Claude Code 1.x.

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A named macro with a body template and parameter list.
#[derive(Debug, Clone)]
pub struct ReplMacro {
    pub name: String,
    pub params: Vec<String>,
    pub body: String,
    pub description: String,
    pub created_ms: u64,
    pub use_count: u64,
}

impl ReplMacro {
    pub fn new(
        name: impl Into<String>,
        params: Vec<String>,
        body: impl Into<String>,
        description: impl Into<String>,
        ts: u64,
    ) -> Self {
        Self {
            name: name.into(),
            params,
            body: body.into(),
            description: description.into(),
            created_ms: ts,
            use_count: 0,
        }
    }

    /// Expand the macro with given argument values.
    /// Returns Err if a required parameter is missing.
    pub fn expand(&self, args: &HashMap<String, String>) -> Result<String, String> {
        let mut result = self.body.clone();
        for param in &self.params {
            let placeholder = format!("${{{}}}", param);
            if let Some(val) = args.get(param) {
                result = result.replace(&placeholder, val);
            } else {
                // Check if there's a default in the placeholder: ${param:default}
                let default_key = format!("${{{}:", param);
                if let Some(start) = result.find(&default_key) {
                    let rest = &result[start + default_key.len()..];
                    if let Some(end) = rest.find('}') {
                        let default_val = &rest[..end];
                        let full_placeholder = format!("${{{}:{}}}", param, default_val);
                        result = result.replace(&full_placeholder, default_val);
                        continue;
                    }
                }
                return Err(format!("missing required parameter: {}", param));
            }
        }
        Ok(result)
    }

    /// Expand using positional arguments.
    pub fn expand_positional(&self, args: &[&str]) -> Result<String, String> {
        if args.len() < self.params.len() {
            return Err(format!(
                "macro '{}' requires {} args, got {}",
                self.name, self.params.len(), args.len()
            ));
        }
        let mut map = HashMap::new();
        for (i, param) in self.params.iter().enumerate() {
            map.insert(param.clone(), args[i].to_string());
        }
        self.expand(&map)
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Manages a collection of REPL macros.
#[derive(Debug, Default)]
pub struct MacroRegistry {
    macros: HashMap<String, ReplMacro>,
}

impl MacroRegistry {
    pub fn new() -> Self { Self::default() }

    /// Register a macro; overwrites if name already exists.
    pub fn define(&mut self, m: ReplMacro) {
        self.macros.insert(m.name.clone(), m);
    }

    /// Remove a macro.
    pub fn undefine(&mut self, name: &str) -> bool {
        self.macros.remove(name).is_some()
    }

    /// Get a macro by name.
    pub fn get(&self, name: &str) -> Option<&ReplMacro> {
        self.macros.get(name)
    }

    /// Expand a macro by name with named args.
    pub fn expand(&mut self, name: &str, args: &HashMap<String, String>) -> Result<String, String> {
        let m = self.macros.get_mut(name).ok_or_else(|| format!("macro '{}' not found", name))?;
        m.use_count += 1;
        m.expand(args)
    }

    /// Expand a macro by name with positional args.
    pub fn expand_positional(&mut self, name: &str, args: &[&str]) -> Result<String, String> {
        let m = self.macros.get_mut(name).ok_or_else(|| format!("macro '{}' not found", name))?;
        m.use_count += 1;
        m.expand_positional(args)
    }

    /// Parse a macro invocation string: `@name arg1 arg2 ...`
    pub fn parse_invocation(input: &str) -> Option<(&str, Vec<&str>)> {
        let input = input.trim();
        if !input.starts_with('@') { return None; }
        let mut parts = input[1..].splitn(2, ' ');
        let name = parts.next()?;
        let args_str = parts.next().unwrap_or("");
        let args: Vec<&str> = if args_str.is_empty() {
            Vec::new()
        } else {
            args_str.split_whitespace().collect()
        };
        Some((name, args))
    }

    /// Process a REPL input line — expands macro if it starts with `@`.
    pub fn process_line(&mut self, line: &str) -> Option<Result<String, String>> {
        if let Some((name, args)) = Self::parse_invocation(line) {
            Some(self.expand_positional(name, &args))
        } else {
            None
        }
    }

    /// List all macros sorted by name.
    pub fn list(&self) -> Vec<&ReplMacro> {
        let mut v: Vec<_> = self.macros.values().collect();
        v.sort_by_key(|m| m.name.as_str());
        v
    }

    /// Most-used macros (descending by use_count).
    pub fn top_used(&self, n: usize) -> Vec<&ReplMacro> {
        let mut v: Vec<_> = self.macros.values().collect();
        v.sort_by(|a, b| b.use_count.cmp(&a.use_count));
        v.truncate(n);
        v
    }

    /// Seed built-in utility macros.
    pub fn load_builtins(&mut self, ts: u64) {
        let builtins = vec![
            ReplMacro::new(
                "test-run",
                vec!["module".to_string()],
                "cargo test --lib -p vibecli -- ${module} --nocapture",
                "Run tests for a specific module",
                ts,
            ),
            ReplMacro::new(
                "check",
                vec![],
                "cargo check --workspace --exclude vibe-collab",
                "Run workspace check",
                ts,
            ),
            ReplMacro::new(
                "review-file",
                vec!["file".to_string(), "depth".to_string()],
                "/explain ${file} --depth ${depth:overview}",
                "Explain a file at a given depth",
                ts,
            ),
            ReplMacro::new(
                "commit-module",
                vec!["module".to_string(), "msg".to_string()],
                "git add vibecli/vibecli-cli/src/${module}.rs && git commit -m \"${msg}\"",
                "Stage and commit a single module file",
                ts,
            ),
        ];
        for m in builtins {
            self.define(m);
        }
    }

    pub fn macro_count(&self) -> usize { self.macros.len() }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn simple_macro() -> ReplMacro {
        ReplMacro::new(
            "greet",
            vec!["name".to_string()],
            "echo Hello, ${name}!",
            "Greet someone",
            0,
        )
    }

    #[test]
    fn test_expand_named_args() {
        let m = simple_macro();
        let mut args = HashMap::new();
        args.insert("name".to_string(), "World".to_string());
        let result = m.expand(&args).unwrap();
        assert_eq!(result, "echo Hello, World!");
    }

    #[test]
    fn test_expand_missing_arg() {
        let m = simple_macro();
        let result = m.expand(&HashMap::new());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("name"));
    }

    #[test]
    fn test_expand_positional() {
        let m = simple_macro();
        let result = m.expand_positional(&["Alice"]).unwrap();
        assert_eq!(result, "echo Hello, Alice!");
    }

    #[test]
    fn test_expand_positional_too_few_args() {
        let m = ReplMacro::new("two-param", vec!["a".to_string(), "b".to_string()], "${a}+${b}", "", 0);
        let result = m.expand_positional(&["only_one"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_value_fallback() {
        let m = ReplMacro::new("review", vec!["depth".to_string()], "/explain --depth ${depth:overview}", "", 0);
        // depth has a default, so passing no args should use "overview"
        let result = m.expand(&HashMap::new()).unwrap();
        assert_eq!(result, "/explain --depth overview");
    }

    #[test]
    fn test_define_and_get() {
        let mut r = MacroRegistry::new();
        r.define(simple_macro());
        assert!(r.get("greet").is_some());
    }

    #[test]
    fn test_undefine() {
        let mut r = MacroRegistry::new();
        r.define(simple_macro());
        assert!(r.undefine("greet"));
        assert!(!r.undefine("greet"));
    }

    #[test]
    fn test_expand_increments_use_count() {
        let mut r = MacroRegistry::new();
        r.define(simple_macro());
        let mut args = HashMap::new();
        args.insert("name".to_string(), "Test".to_string());
        r.expand("greet", &args).unwrap();
        r.expand("greet", &args).unwrap();
        assert_eq!(r.get("greet").unwrap().use_count, 2);
    }

    #[test]
    fn test_parse_invocation() {
        let (name, args) = MacroRegistry::parse_invocation("@greet Alice Bob").unwrap();
        assert_eq!(name, "greet");
        assert_eq!(args, vec!["Alice", "Bob"]);
    }

    #[test]
    fn test_parse_invocation_no_at() {
        assert!(MacroRegistry::parse_invocation("greet Alice").is_none());
    }

    #[test]
    fn test_process_line_macro() {
        let mut r = MacroRegistry::new();
        r.define(simple_macro());
        let result = r.process_line("@greet World").unwrap().unwrap();
        assert!(result.contains("World"));
    }

    #[test]
    fn test_process_line_non_macro() {
        let mut r = MacroRegistry::new();
        assert!(r.process_line("cargo build").is_none());
    }

    #[test]
    fn test_load_builtins() {
        let mut r = MacroRegistry::new();
        r.load_builtins(0);
        assert!(r.macro_count() >= 4);
        assert!(r.get("check").is_some());
    }

    #[test]
    fn test_top_used() {
        let mut r = MacroRegistry::new();
        r.define(simple_macro());
        r.define(ReplMacro::new("other", vec![], "cmd", "", 0));
        let mut args = HashMap::new();
        args.insert("name".to_string(), "X".to_string());
        r.expand("greet", &args).unwrap();
        r.expand("greet", &args).unwrap();
        let top = r.top_used(1);
        assert_eq!(top[0].name, "greet");
    }
}
