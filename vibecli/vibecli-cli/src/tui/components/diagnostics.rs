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
