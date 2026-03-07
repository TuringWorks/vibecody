#![allow(dead_code)]
//! Diagnostics panel component for the VibeCLI TUI.
//!
//! Populated by `/check` (cargo check / eslint / flake8) and displayed as a
//! compact 4-line pane between the main area and the input bar.

/// Severity level for a single diagnostic entry.
#[derive(Debug, Clone, PartialEq)]
pub enum DiagSeverity {
    Error,
    Warning,
    Info,
}

/// A single diagnostic line shown in the panel.
#[derive(Debug, Clone)]
pub struct TuiDiagnostic {
    pub severity: DiagSeverity,
    pub file: String,
    pub line: u32,
    /// Short message (first line only).
    pub message: String,
}

/// Holds the diagnostics state for the TUI panel.
pub struct DiagnosticsComponent {
    /// Diagnostics from the most recent check.
    pub items: Vec<TuiDiagnostic>,
    /// Vertical scroll offset.
    pub scroll: u16,
    /// Summary line shown when there are no items (or while loading).
    pub status: String,
}

impl DiagnosticsComponent {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            scroll: 0,
            status: "No diagnostics — type /check to run linter".to_string(),
        }
    }

    /// Replace current diagnostics with a new set and reset scroll.
    pub fn set(&mut self, items: Vec<TuiDiagnostic>) {
        self.scroll = 0;
        let errors   = items.iter().filter(|d| d.severity == DiagSeverity::Error).count();
        let warnings = items.iter().filter(|d| d.severity == DiagSeverity::Warning).count();
        self.status = if items.is_empty() {
            "✅ No issues found".to_string()
        } else {
            format!("{} error(s), {} warning(s)", errors, warnings)
        };
        self.items = items;
    }

    /// Clear all diagnostics.
    pub fn clear(&mut self) {
        self.items.clear();
        self.status = "No diagnostics".to_string();
        self.scroll = 0;
    }

    pub fn scroll_down(&mut self) {
        let max = (self.items.len() as u16).saturating_sub(1);
        self.scroll = self.scroll.saturating_add(1).min(max);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }
}

// ── Parser helpers ─────────────────────────────────────────────────────────────

/// Parse `cargo check --message-format=json` output into diagnostics.
/// Falls back to line-by-line text parsing if JSON is unavailable.
pub fn parse_cargo_check(output: &str) -> Vec<TuiDiagnostic> {
    let mut diags = Vec::new();

    for line in output.lines() {
        // Try JSON message format first.
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            if val["reason"].as_str() == Some("compiler-message") {
                let msg = &val["message"];
                let severity = match msg["level"].as_str().unwrap_or("") {
                    "error" => DiagSeverity::Error,
                    "warning" => DiagSeverity::Warning,
                    _ => DiagSeverity::Info,
                };
                let message = msg["message"].as_str().unwrap_or("").to_string();
                // Extract primary span for file + line.
                if let Some(span) = msg["spans"].as_array()
                    .and_then(|s| s.iter().find(|sp| sp["is_primary"].as_bool() == Some(true)))
                {
                    let file = span["file_name"].as_str().unwrap_or("unknown").to_string();
                    let lineno = span["line_start"].as_u64().unwrap_or(0) as u32;
                    diags.push(TuiDiagnostic { severity, file, line: lineno, message });
                } else if !message.is_empty() {
                    diags.push(TuiDiagnostic {
                        severity,
                        file: String::new(),
                        line: 0,
                        message,
                    });
                }
                continue;
            }
        }

        // Fallback: text lines like "error[E0609]: msg" or "warning: msg"
        let (severity, rest) = if line.starts_with("error") {
            let after = line.trim_start_matches("error");
            let after = if after.starts_with('[') {
                after.split_once(']').map(|(_, r)| r).unwrap_or(after)
            } else {
                after
            };
            (DiagSeverity::Error, after)
        } else if line.starts_with("warning") {
            let after = line.trim_start_matches("warning");
            let after = if after.starts_with('[') {
                after.split_once(']').map(|(_, r)| r).unwrap_or(after)
            } else {
                after
            };
            (DiagSeverity::Warning, after)
        } else {
            continue;
        };
        let message = rest.trim_start_matches(':').trim().to_string();
        if !message.is_empty() {
            diags.push(TuiDiagnostic { severity, file: String::new(), line: 0, message });
        }
    }

    diags
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── DiagnosticsComponent::new ───────────────────────────────────────────

    #[test]
    fn new_component_has_empty_items() {
        let dc = DiagnosticsComponent::new();
        assert!(dc.items.is_empty());
    }

    #[test]
    fn new_component_has_zero_scroll() {
        let dc = DiagnosticsComponent::new();
        assert_eq!(dc.scroll, 0);
    }

    #[test]
    fn new_component_has_default_status() {
        let dc = DiagnosticsComponent::new();
        assert!(dc.status.contains("No diagnostics"));
    }

    // ── DiagnosticsComponent::set ───────────────────────────────────────────

    #[test]
    fn set_with_empty_vec_shows_no_issues() {
        let mut dc = DiagnosticsComponent::new();
        dc.scroll = 5; // should be reset
        dc.set(vec![]);
        assert!(dc.status.contains("No issues found"));
        assert_eq!(dc.scroll, 0);
        assert!(dc.items.is_empty());
    }

    #[test]
    fn set_counts_errors_and_warnings() {
        let mut dc = DiagnosticsComponent::new();
        let items = vec![
            TuiDiagnostic { severity: DiagSeverity::Error, file: "a.rs".into(), line: 1, message: "err".into() },
            TuiDiagnostic { severity: DiagSeverity::Error, file: "b.rs".into(), line: 2, message: "err2".into() },
            TuiDiagnostic { severity: DiagSeverity::Warning, file: "c.rs".into(), line: 3, message: "warn".into() },
            TuiDiagnostic { severity: DiagSeverity::Info, file: "d.rs".into(), line: 4, message: "info".into() },
        ];
        dc.set(items);
        assert!(dc.status.contains("2 error(s)"));
        assert!(dc.status.contains("1 warning(s)"));
        assert_eq!(dc.items.len(), 4);
    }

    #[test]
    fn set_resets_scroll_to_zero() {
        let mut dc = DiagnosticsComponent::new();
        dc.scroll = 10;
        dc.set(vec![
            TuiDiagnostic { severity: DiagSeverity::Warning, file: "x.rs".into(), line: 1, message: "w".into() },
        ]);
        assert_eq!(dc.scroll, 0);
    }

    // ── DiagnosticsComponent::clear ─────────────────────────────────────────

    #[test]
    fn clear_removes_all_items() {
        let mut dc = DiagnosticsComponent::new();
        dc.set(vec![
            TuiDiagnostic { severity: DiagSeverity::Error, file: "a.rs".into(), line: 1, message: "e".into() },
        ]);
        dc.clear();
        assert!(dc.items.is_empty());
        assert_eq!(dc.scroll, 0);
        assert_eq!(dc.status, "No diagnostics");
    }

    // ── Scroll ──────────────────────────────────────────────────────────────

    #[test]
    fn scroll_down_increments() {
        let mut dc = DiagnosticsComponent::new();
        dc.items = vec![
            TuiDiagnostic { severity: DiagSeverity::Error, file: "".into(), line: 0, message: "a".into() },
            TuiDiagnostic { severity: DiagSeverity::Error, file: "".into(), line: 0, message: "b".into() },
            TuiDiagnostic { severity: DiagSeverity::Error, file: "".into(), line: 0, message: "c".into() },
        ];
        dc.scroll_down();
        assert_eq!(dc.scroll, 1);
        dc.scroll_down();
        assert_eq!(dc.scroll, 2); // max = 3-1 = 2
        dc.scroll_down();
        assert_eq!(dc.scroll, 2); // clamped
    }

    #[test]
    fn scroll_up_decrements() {
        let mut dc = DiagnosticsComponent::new();
        dc.scroll = 2;
        dc.scroll_up();
        assert_eq!(dc.scroll, 1);
        dc.scroll_up();
        assert_eq!(dc.scroll, 0);
        dc.scroll_up();
        assert_eq!(dc.scroll, 0); // saturating
    }

    #[test]
    fn scroll_down_on_empty_stays_zero() {
        let mut dc = DiagnosticsComponent::new();
        dc.scroll_down();
        assert_eq!(dc.scroll, 0);
    }

    // ── DiagSeverity ────────────────────────────────────────────────────────

    #[test]
    fn diag_severity_eq() {
        assert_eq!(DiagSeverity::Error, DiagSeverity::Error);
        assert_eq!(DiagSeverity::Warning, DiagSeverity::Warning);
        assert_eq!(DiagSeverity::Info, DiagSeverity::Info);
        assert_ne!(DiagSeverity::Error, DiagSeverity::Warning);
    }

    // ── parse_cargo_check - JSON format ─────────────────────────────────────

    #[test]
    fn parse_cargo_check_json_error() {
        let json_line = r#"{"reason":"compiler-message","message":{"level":"error","message":"cannot find value `x`","spans":[{"file_name":"src/main.rs","line_start":10,"is_primary":true}]}}"#;
        let diags = parse_cargo_check(json_line);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, DiagSeverity::Error);
        assert_eq!(diags[0].file, "src/main.rs");
        assert_eq!(diags[0].line, 10);
        assert!(diags[0].message.contains("cannot find value"));
    }

    #[test]
    fn parse_cargo_check_json_warning() {
        let json_line = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused variable","spans":[{"file_name":"lib.rs","line_start":5,"is_primary":true}]}}"#;
        let diags = parse_cargo_check(json_line);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, DiagSeverity::Warning);
        assert_eq!(diags[0].file, "lib.rs");
        assert_eq!(diags[0].line, 5);
    }

    #[test]
    fn parse_cargo_check_json_no_primary_span_uses_message() {
        let json_line = r#"{"reason":"compiler-message","message":{"level":"error","message":"aborting due to error","spans":[]}}"#;
        let diags = parse_cargo_check(json_line);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].file, "");
        assert_eq!(diags[0].line, 0);
        assert!(diags[0].message.contains("aborting"));
    }

    #[test]
    fn parse_cargo_check_json_info_level() {
        let json_line = r#"{"reason":"compiler-message","message":{"level":"note","message":"some note","spans":[{"file_name":"a.rs","line_start":1,"is_primary":true}]}}"#;
        let diags = parse_cargo_check(json_line);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, DiagSeverity::Info);
    }

    #[test]
    fn parse_cargo_check_ignores_non_compiler_message() {
        let json_line = r#"{"reason":"build-script-executed","package_id":"foo"}"#;
        let diags = parse_cargo_check(json_line);
        assert!(diags.is_empty());
    }

    // ── parse_cargo_check - text fallback ───────────────────────────────────

    #[test]
    fn parse_cargo_check_text_error_simple() {
        let output = "error: could not compile `foo`";
        let diags = parse_cargo_check(output);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, DiagSeverity::Error);
        assert!(diags[0].message.contains("could not compile"));
    }

    #[test]
    fn parse_cargo_check_text_error_with_code() {
        let output = "error[E0609]: no field `x` on type `Foo`";
        let diags = parse_cargo_check(output);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, DiagSeverity::Error);
        assert!(diags[0].message.contains("no field"));
    }

    #[test]
    fn parse_cargo_check_text_warning() {
        let output = "warning: unused variable `x`";
        let diags = parse_cargo_check(output);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, DiagSeverity::Warning);
        assert!(diags[0].message.contains("unused variable"));
    }

    #[test]
    fn parse_cargo_check_ignores_unrelated_lines() {
        let output = "   Compiling foo v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s)";
        let diags = parse_cargo_check(output);
        assert!(diags.is_empty());
    }

    #[test]
    fn parse_cargo_check_empty_input() {
        let diags = parse_cargo_check("");
        assert!(diags.is_empty());
    }

    #[test]
    fn parse_cargo_check_mixed_json_and_text() {
        let output = r#"{"reason":"compiler-message","message":{"level":"error","message":"type mismatch","spans":[{"file_name":"main.rs","line_start":3,"is_primary":true}]}}
warning: unused import"#;
        let diags = parse_cargo_check(output);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].severity, DiagSeverity::Error);
        assert_eq!(diags[1].severity, DiagSeverity::Warning);
    }
}
