#![allow(dead_code)]
//! IDE bridge — editor state synchronisation and context building for agent use.

use serde::{Deserialize, Serialize};

// ─── EditorSelection ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorSelection {
    pub start_line: u32,
    pub end_line: u32,
    pub start_col: u32,
    pub end_col: u32,
}

// ─── OpenFile ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenFile {
    pub path: String,
    pub content_hash: String,
    pub is_dirty: bool,
    pub language: String,
}

// ─── BuildResult ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub succeeded: bool,
}

// ─── TestResult ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestResult {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_ms: u64,
}

impl TestResult {
    /// Returns pass rate as a percentage (0–100).  Returns 0 if total == 0.
    pub fn pass_rate(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            self.passed as f32 / self.total as f32 * 100.0
        }
    }

    /// True when there are no failed tests.
    pub fn is_passing(&self) -> bool {
        self.failed == 0
    }
}

// ─── IdeBridgeState ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IdeBridgeState {
    pub open_files: Vec<OpenFile>,
    pub active_file: Option<String>,
    pub active_selection: Option<EditorSelection>,
    pub last_build: Option<BuildResult>,
    pub last_test: Option<TestResult>,
    /// Capped at 100 lines.
    pub terminal_last_lines: Vec<String>,
}

impl IdeBridgeState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a terminal line, keeping the buffer capped at 100 entries.
    pub fn push_terminal_line(&mut self, line: String) {
        if self.terminal_last_lines.len() >= 100 {
            self.terminal_last_lines.remove(0);
        }
        self.terminal_last_lines.push(line);
    }

    /// Produces a compact context block string for the agent.
    pub fn format_context_block(&self) -> String {
        let file = self
            .active_file
            .as_deref()
            .unwrap_or("(none)");

        let selection = if let Some(sel) = &self.active_selection {
            format!("lines {}-{}", sel.start_line, sel.end_line)
        } else {
            "none".to_string()
        };

        let build = if let Some(b) = &self.last_build {
            if b.succeeded { "pass" } else { "fail" }.to_string()
        } else {
            "unknown".to_string()
        };

        let tests = if let Some(t) = &self.last_test {
            format!("{:.0}%", t.pass_rate())
        } else {
            "unknown".to_string()
        };

        format!(
            "Active: {}\nSelection: {}\nBuild: {}\nTests: {}\nOpen files: {}",
            file,
            selection,
            build,
            tests,
            self.open_files.len()
        )
    }

    pub fn has_active_editor(&self) -> bool {
        self.active_file.is_some()
    }

    pub fn open_file_count(&self) -> usize {
        self.open_files.len()
    }
}

// ─── ConnectionStatus ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected { socket_path: String },
    Error(String),
}

// ─── BridgeClient ────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct BridgeClient {
    status: ConnectionStatus,
    cached_state: Option<IdeBridgeState>,
}

impl BridgeClient {
    pub fn new() -> Self {
        Self {
            status: ConnectionStatus::Disconnected,
            cached_state: None,
        }
    }

    pub fn status(&self) -> &ConnectionStatus {
        &self.status
    }

    pub fn set_status(&mut self, status: ConnectionStatus) {
        self.status = status;
    }

    pub fn cached_state(&self) -> Option<&IdeBridgeState> {
        self.cached_state.as_ref()
    }

    pub fn update_state(&mut self, state: IdeBridgeState) {
        self.cached_state = Some(state);
    }

    /// Returns a context string suitable for passing to the agent.
    pub fn context_for_agent(&self) -> String {
        match &self.status {
            ConnectionStatus::Connected { .. } => {
                if let Some(state) = &self.cached_state {
                    state.format_context_block()
                } else {
                    "No IDE connected".to_string()
                }
            }
            _ => "No IDE connected".to_string(),
        }
    }
}

impl Default for BridgeClient {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── TestResult ────────────────────────────────────────────────────────

    #[test]
    fn test_pass_rate_all_pass() {
        let t = TestResult { total: 10, passed: 10, failed: 0, skipped: 0, duration_ms: 100 };
        assert!((t.pass_rate() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_pass_rate_half() {
        let t = TestResult { total: 10, passed: 5, failed: 5, skipped: 0, duration_ms: 100 };
        assert!((t.pass_rate() - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_pass_rate_zero_total() {
        let t = TestResult { total: 0, passed: 0, failed: 0, skipped: 0, duration_ms: 0 };
        assert!((t.pass_rate() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_pass_rate_none_pass() {
        let t = TestResult { total: 5, passed: 0, failed: 5, skipped: 0, duration_ms: 10 };
        assert!((t.pass_rate() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_pass_rate_with_skipped() {
        let t = TestResult { total: 10, passed: 8, failed: 0, skipped: 2, duration_ms: 50 };
        assert!((t.pass_rate() - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_is_passing_no_failures() {
        let t = TestResult { total: 5, passed: 5, failed: 0, skipped: 0, duration_ms: 10 };
        assert!(t.is_passing());
    }

    #[test]
    fn test_is_passing_with_failures() {
        let t = TestResult { total: 5, passed: 4, failed: 1, skipped: 0, duration_ms: 10 };
        assert!(!t.is_passing());
    }

    #[test]
    fn test_is_passing_all_failed() {
        let t = TestResult { total: 3, passed: 0, failed: 3, skipped: 0, duration_ms: 5 };
        assert!(!t.is_passing());
    }

    #[test]
    fn test_is_passing_zero_total() {
        let t = TestResult { total: 0, passed: 0, failed: 0, skipped: 0, duration_ms: 0 };
        assert!(t.is_passing());
    }

    // ── IdeBridgeState ─────────────────────────────────────────────────────

    #[test]
    fn test_bridge_state_new_defaults() {
        let s = IdeBridgeState::new();
        assert!(!s.has_active_editor());
        assert_eq!(s.open_file_count(), 0);
    }

    #[test]
    fn test_has_active_editor_true() {
        let mut s = IdeBridgeState::new();
        s.active_file = Some("main.rs".into());
        assert!(s.has_active_editor());
    }

    #[test]
    fn test_open_file_count() {
        let mut s = IdeBridgeState::new();
        s.open_files.push(OpenFile {
            path: "a.rs".into(),
            content_hash: "abc".into(),
            is_dirty: false,
            language: "rust".into(),
        });
        assert_eq!(s.open_file_count(), 1);
    }

    #[test]
    fn test_format_context_block_no_active_file() {
        let s = IdeBridgeState::new();
        let ctx = s.format_context_block();
        assert!(ctx.contains("(none)"));
    }

    #[test]
    fn test_format_context_block_with_active_file() {
        let mut s = IdeBridgeState::new();
        s.active_file = Some("src/main.rs".into());
        let ctx = s.format_context_block();
        assert!(ctx.contains("src/main.rs"));
    }

    #[test]
    fn test_format_context_block_with_selection() {
        let mut s = IdeBridgeState::new();
        s.active_selection = Some(EditorSelection {
            start_line: 10,
            end_line: 20,
            start_col: 0,
            end_col: 5,
        });
        let ctx = s.format_context_block();
        assert!(ctx.contains("lines 10-20"));
    }

    #[test]
    fn test_format_context_block_no_selection() {
        let s = IdeBridgeState::new();
        let ctx = s.format_context_block();
        assert!(ctx.contains("Selection: none"));
    }

    #[test]
    fn test_format_context_block_build_pass() {
        let mut s = IdeBridgeState::new();
        s.last_build = Some(BuildResult {
            exit_code: 0,
            stdout: "ok".into(),
            stderr: "".into(),
            duration_ms: 500,
            succeeded: true,
        });
        let ctx = s.format_context_block();
        assert!(ctx.contains("Build: pass"));
    }

    #[test]
    fn test_format_context_block_build_fail() {
        let mut s = IdeBridgeState::new();
        s.last_build = Some(BuildResult {
            exit_code: 1,
            stdout: "".into(),
            stderr: "error".into(),
            duration_ms: 200,
            succeeded: false,
        });
        let ctx = s.format_context_block();
        assert!(ctx.contains("Build: fail"));
    }

    #[test]
    fn test_format_context_block_tests_pass_rate() {
        let mut s = IdeBridgeState::new();
        s.last_test = Some(TestResult {
            total: 10,
            passed: 8,
            failed: 2,
            skipped: 0,
            duration_ms: 300,
        });
        let ctx = s.format_context_block();
        assert!(ctx.contains("80%"));
    }

    #[test]
    fn test_format_context_block_open_files_count() {
        let mut s = IdeBridgeState::new();
        s.open_files.push(OpenFile {
            path: "foo.rs".into(),
            content_hash: "x".into(),
            is_dirty: false,
            language: "rust".into(),
        });
        s.open_files.push(OpenFile {
            path: "bar.rs".into(),
            content_hash: "y".into(),
            is_dirty: true,
            language: "rust".into(),
        });
        let ctx = s.format_context_block();
        assert!(ctx.contains("Open files: 2"));
    }

    #[test]
    fn test_terminal_line_cap() {
        let mut s = IdeBridgeState::new();
        for i in 0..105 {
            s.push_terminal_line(format!("line {}", i));
        }
        assert_eq!(s.terminal_last_lines.len(), 100);
    }

    #[test]
    fn test_terminal_line_order() {
        let mut s = IdeBridgeState::new();
        s.push_terminal_line("first".into());
        s.push_terminal_line("second".into());
        assert_eq!(s.terminal_last_lines.last().unwrap(), "second");
    }

    // ── BridgeClient ──────────────────────────────────────────────────────

    #[test]
    fn test_bridge_client_new_disconnected() {
        let c = BridgeClient::new();
        assert_eq!(*c.status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_bridge_client_set_status_connecting() {
        let mut c = BridgeClient::new();
        c.set_status(ConnectionStatus::Connecting);
        assert_eq!(*c.status(), ConnectionStatus::Connecting);
    }

    #[test]
    fn test_bridge_client_set_status_connected() {
        let mut c = BridgeClient::new();
        c.set_status(ConnectionStatus::Connected { socket_path: "/tmp/ide.sock".into() });
        assert!(matches!(c.status(), ConnectionStatus::Connected { .. }));
    }

    #[test]
    fn test_bridge_client_set_status_error() {
        let mut c = BridgeClient::new();
        c.set_status(ConnectionStatus::Error("timeout".into()));
        assert!(matches!(c.status(), ConnectionStatus::Error(_)));
    }

    #[test]
    fn test_bridge_client_no_cached_state_initially() {
        let c = BridgeClient::new();
        assert!(c.cached_state().is_none());
    }

    #[test]
    fn test_bridge_client_update_state() {
        let mut c = BridgeClient::new();
        let state = IdeBridgeState::new();
        c.update_state(state);
        assert!(c.cached_state().is_some());
    }

    #[test]
    fn test_context_for_agent_not_connected() {
        let c = BridgeClient::new();
        assert_eq!(c.context_for_agent(), "No IDE connected");
    }

    #[test]
    fn test_context_for_agent_connecting() {
        let mut c = BridgeClient::new();
        c.set_status(ConnectionStatus::Connecting);
        assert_eq!(c.context_for_agent(), "No IDE connected");
    }

    #[test]
    fn test_context_for_agent_connected_no_state() {
        let mut c = BridgeClient::new();
        c.set_status(ConnectionStatus::Connected { socket_path: "/tmp/ide.sock".into() });
        assert_eq!(c.context_for_agent(), "No IDE connected");
    }

    #[test]
    fn test_context_for_agent_connected_with_state() {
        let mut c = BridgeClient::new();
        c.set_status(ConnectionStatus::Connected { socket_path: "/tmp/ide.sock".into() });
        let mut state = IdeBridgeState::new();
        state.active_file = Some("main.rs".into());
        c.update_state(state);
        let ctx = c.context_for_agent();
        assert!(ctx.contains("main.rs"));
    }

    #[test]
    fn test_context_for_agent_error_status() {
        let mut c = BridgeClient::new();
        c.set_status(ConnectionStatus::Error("conn refused".into()));
        assert_eq!(c.context_for_agent(), "No IDE connected");
    }

    #[test]
    fn test_bridge_state_format_contains_active_label() {
        let s = IdeBridgeState::new();
        let ctx = s.format_context_block();
        assert!(ctx.contains("Active:"));
    }

    #[test]
    fn test_bridge_state_format_contains_build_label() {
        let s = IdeBridgeState::new();
        let ctx = s.format_context_block();
        assert!(ctx.contains("Build:"));
    }

    #[test]
    fn test_bridge_state_format_contains_tests_label() {
        let s = IdeBridgeState::new();
        let ctx = s.format_context_block();
        assert!(ctx.contains("Tests:"));
    }
}
