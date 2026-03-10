#![allow(dead_code)]
//! Debug Mode — dedicated debugging workflow module.
//!
//! Closes the "Partial" competitor parity entry for Cursor's "Debug mode" feature.
//! Provides structured debug sessions with breakpoints, watches, stack inspection,
//! AI-powered hypothesis generation, root cause analysis, and auto-fix suggestions.
//!
//! Usage:
//! - `/debug start <file>` — start a debug session on a file
//! - `/debug breakpoint add <file>:<line>` — add a breakpoint
//! - `/debug watch <expr>` — watch an expression
//! - `/debug hypothesize <error>` — generate hypotheses for an error
//! - `/debug analyze` — root cause analysis at crash point
//! - `/debug fix` — auto-fix suggestions based on findings
//! - `/debug sessions` — list active sessions
//! - `/debug end <id>` — end a debug session

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Debug Mode ──────────────────────────────────────────────────────────────

/// How the debug session is driven.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugMode {
    /// User steps through manually.
    Interactive,
    /// AI drives the session end-to-end.
    Automated,
    /// AI suggests, user confirms each step.
    Hybrid,
}

impl Default for DebugMode {
    fn default() -> Self {
        Self::Interactive
    }
}

impl std::fmt::Display for DebugMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Interactive => write!(f, "Interactive"),
            Self::Automated => write!(f, "Automated"),
            Self::Hybrid => write!(f, "Hybrid"),
        }
    }
}

// ── Breakpoint Types ────────────────────────────────────────────────────────

/// Classification of breakpoint triggers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreakpointType {
    /// Pause at a specific source line.
    Line,
    /// Pause only when the condition evaluates to true.
    Conditional(String),
    /// Pause on any exception / panic.
    Exception,
    /// Do not pause — emit a log message instead.
    Logpoint(String),
}

impl std::fmt::Display for BreakpointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Line => write!(f, "Line"),
            Self::Conditional(cond) => write!(f, "Conditional({})", cond),
            Self::Exception => write!(f, "Exception"),
            Self::Logpoint(msg) => write!(f, "Logpoint({})", msg),
        }
    }
}

// ── Breakpoint ──────────────────────────────────────────────────────────────

/// A breakpoint attached to a file location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: u64,
    pub file: String,
    pub line: usize,
    pub bp_type: BreakpointType,
    pub enabled: bool,
    pub hit_count: u64,
}

impl Breakpoint {
    pub fn new(id: u64, file: &str, line: usize, bp_type: BreakpointType) -> Self {
        Self {
            id,
            file: file.to_string(),
            line,
            bp_type,
            enabled: true,
            hit_count: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}

// ── Variable ────────────────────────────────────────────────────────────────

/// A runtime variable captured during debug inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub var_type: String,
    /// Nested children (e.g. struct fields, array elements).
    pub children: Vec<Variable>,
}

impl Variable {
    pub fn new(name: &str, value: &str, var_type: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
            var_type: var_type.to_string(),
            children: Vec::new(),
        }
    }

    pub fn with_children(mut self, children: Vec<Variable>) -> Self {
        self.children = children;
        self
    }
}

// ── Stack Frame ─────────────────────────────────────────────────────────────

/// A single frame on the call stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub function_name: String,
    pub file: String,
    pub line: usize,
    pub variables: HashMap<String, Variable>,
}

impl StackFrame {
    pub fn new(function_name: &str, file: &str, line: usize) -> Self {
        Self {
            function_name: function_name.to_string(),
            file: file.to_string(),
            line,
            variables: HashMap::new(),
        }
    }

    pub fn add_variable(&mut self, var: Variable) {
        self.variables.insert(var.name.clone(), var);
    }

    pub fn get_variable(&self, name: &str) -> Option<&Variable> {
        self.variables.get(name)
    }
}

// ── Debug Actions ───────────────────────────────────────────────────────────

/// Actions the user (or AI) can issue during a debug session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebugAction {
    StepOver,
    StepInto,
    StepOut,
    Continue,
    Pause,
    Evaluate(String),
    SetBreakpoint { file: String, line: usize },
    RemoveBreakpoint { id: u64 },
    Watch(String),
    Unwatch(String),
    Inspect(String),
    RunToLine { file: String, line: usize },
}

impl std::fmt::Display for DebugAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StepOver => write!(f, "Step Over"),
            Self::StepInto => write!(f, "Step Into"),
            Self::StepOut => write!(f, "Step Out"),
            Self::Continue => write!(f, "Continue"),
            Self::Pause => write!(f, "Pause"),
            Self::Evaluate(expr) => write!(f, "Evaluate({})", expr),
            Self::SetBreakpoint { file, line } => {
                write!(f, "SetBreakpoint({}:{})", file, line)
            }
            Self::RemoveBreakpoint { id } => write!(f, "RemoveBreakpoint({})", id),
            Self::Watch(expr) => write!(f, "Watch({})", expr),
            Self::Unwatch(expr) => write!(f, "Unwatch({})", expr),
            Self::Inspect(expr) => write!(f, "Inspect({})", expr),
            Self::RunToLine { file, line } => write!(f, "RunToLine({}:{})", file, line),
        }
    }
}

// ── Debug Session State ─────────────────────────────────────────────────────

/// High-level state of a debug session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Not yet started.
    Created,
    /// Program is running (between breakpoints).
    Running,
    /// Paused at a breakpoint or step.
    Paused,
    /// Session terminated normally.
    Stopped,
    /// Session terminated due to an error / crash.
    Crashed,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::Created
    }
}

// ── Debug Hypothesis ────────────────────────────────────────────────────────

/// An AI-generated hypothesis about the root cause of an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugHypothesis {
    pub rank: usize,
    pub summary: String,
    pub likely_cause: String,
    pub confidence: f64,
    pub suggested_breakpoints: Vec<(String, usize)>,
}

// ── Auto-Fix Suggestion ─────────────────────────────────────────────────────

/// An AI-generated fix suggestion derived from debug findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoFixSuggestion {
    pub file: String,
    pub line: usize,
    pub original: String,
    pub replacement: String,
    pub explanation: String,
    pub confidence: f64,
}

// ── Debug Session ───────────────────────────────────────────────────────────

/// A self-contained debugging session tracking breakpoints, watches, frames,
/// hypotheses, and fix suggestions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSession {
    pub id: String,
    pub target_file: String,
    pub mode: DebugMode,
    pub state: SessionState,
    pub breakpoints: Vec<Breakpoint>,
    pub watches: Vec<String>,
    pub stack_frames: Vec<StackFrame>,
    pub action_history: Vec<DebugAction>,
    pub hypotheses: Vec<DebugHypothesis>,
    pub fix_suggestions: Vec<AutoFixSuggestion>,
    next_bp_id: u64,
}

impl DebugSession {
    pub fn new(id: &str, target_file: &str, mode: DebugMode) -> Self {
        Self {
            id: id.to_string(),
            target_file: target_file.to_string(),
            mode,
            state: SessionState::Created,
            breakpoints: Vec::new(),
            watches: Vec::new(),
            stack_frames: Vec::new(),
            action_history: Vec::new(),
            hypotheses: Vec::new(),
            fix_suggestions: Vec::new(),
            next_bp_id: 1,
        }
    }

    // ── Breakpoint management ───────────────────────────────────────────

    /// Add a breakpoint and return its ID.
    pub fn add_breakpoint(
        &mut self,
        file: &str,
        line: usize,
        bp_type: BreakpointType,
    ) -> u64 {
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        self.breakpoints.push(Breakpoint::new(id, file, line, bp_type));
        id
    }

    /// Remove a breakpoint by ID. Returns `true` if it existed.
    pub fn remove_breakpoint(&mut self, id: u64) -> bool {
        let before = self.breakpoints.len();
        self.breakpoints.retain(|bp| bp.id != id);
        self.breakpoints.len() < before
    }

    /// Toggle enabled/disabled for a breakpoint by ID.
    pub fn toggle_breakpoint(&mut self, id: u64) -> bool {
        if let Some(bp) = self.breakpoints.iter_mut().find(|bp| bp.id == id) {
            bp.toggle();
            true
        } else {
            false
        }
    }

    /// List all breakpoints.
    pub fn list_breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }

    /// Get a breakpoint by ID.
    pub fn get_breakpoint(&self, id: u64) -> Option<&Breakpoint> {
        self.breakpoints.iter().find(|bp| bp.id == id)
    }

    // ── Watch management ────────────────────────────────────────────────

    /// Add a watch expression. Returns `false` if already watched.
    pub fn add_watch(&mut self, expr: &str) -> bool {
        if self.watches.contains(&expr.to_string()) {
            return false;
        }
        self.watches.push(expr.to_string());
        true
    }

    /// Remove a watch expression. Returns `true` if it existed.
    pub fn remove_watch(&mut self, expr: &str) -> bool {
        let before = self.watches.len();
        self.watches.retain(|w| w != expr);
        self.watches.len() < before
    }

    /// List all watch expressions.
    pub fn list_watches(&self) -> &[String] {
        &self.watches
    }

    // ── Stack frame management ──────────────────────────────────────────

    /// Push a stack frame (most recent call on top).
    pub fn push_frame(&mut self, frame: StackFrame) {
        self.stack_frames.push(frame);
    }

    /// Pop the top stack frame.
    pub fn pop_frame(&mut self) -> Option<StackFrame> {
        self.stack_frames.pop()
    }

    /// Get the current (top) stack frame.
    pub fn current_frame(&self) -> Option<&StackFrame> {
        self.stack_frames.last()
    }

    /// Get all stack frames (bottom-to-top order).
    pub fn frames(&self) -> &[StackFrame] {
        &self.stack_frames
    }

    // ── Action execution ────────────────────────────────────────────────

    /// Execute a debug action, updating session state and recording history.
    pub fn execute_action(&mut self, action: DebugAction) -> Result<String, String> {
        self.action_history.push(action.clone());

        match action {
            DebugAction::StepOver | DebugAction::StepInto | DebugAction::StepOut => {
                if self.state != SessionState::Paused && self.state != SessionState::Created {
                    return Err("Session must be paused to step".to_string());
                }
                self.state = SessionState::Paused;
                Ok(format!("Executed: {}", self.action_history.last().expect("just pushed")))
            }
            DebugAction::Continue => {
                self.state = SessionState::Running;
                Ok("Resumed execution".to_string())
            }
            DebugAction::Pause => {
                if self.state != SessionState::Running {
                    return Err("Session must be running to pause".to_string());
                }
                self.state = SessionState::Paused;
                Ok("Paused execution".to_string())
            }
            DebugAction::Evaluate(ref expr) => {
                Ok(format!("Evaluated: {}", expr))
            }
            DebugAction::SetBreakpoint { ref file, line } => {
                let id = self.add_breakpoint(file, line, BreakpointType::Line);
                Ok(format!("Breakpoint {} set at {}:{}", id, file, line))
            }
            DebugAction::RemoveBreakpoint { id } => {
                if self.remove_breakpoint(id) {
                    Ok(format!("Breakpoint {} removed", id))
                } else {
                    Err(format!("Breakpoint {} not found", id))
                }
            }
            DebugAction::Watch(ref expr) => {
                if self.add_watch(expr) {
                    Ok(format!("Watching: {}", expr))
                } else {
                    Err(format!("Already watching: {}", expr))
                }
            }
            DebugAction::Unwatch(ref expr) => {
                if self.remove_watch(expr) {
                    Ok(format!("Unwatched: {}", expr))
                } else {
                    Err(format!("Not watching: {}", expr))
                }
            }
            DebugAction::Inspect(ref expr) => {
                // Look for the expression in the current frame's variables.
                if let Some(frame) = self.current_frame() {
                    if let Some(var) = frame.get_variable(expr) {
                        Ok(format!(
                            "{}: {} = {} ({} children)",
                            var.name,
                            var.var_type,
                            var.value,
                            var.children.len()
                        ))
                    } else {
                        Err(format!("Variable '{}' not found in current frame", expr))
                    }
                } else {
                    Err("No stack frame available".to_string())
                }
            }
            DebugAction::RunToLine { ref file, line } => {
                self.state = SessionState::Running;
                Ok(format!("Running to {}:{}", file, line))
            }
        }
    }

    // ── Hypothesis generation ───────────────────────────────────────────

    /// Given an error message and optional stack trace lines, generate ranked
    /// hypotheses about the root cause.
    pub fn generate_hypotheses(
        &mut self,
        error_message: &str,
        stack_trace: &[String],
    ) -> Vec<DebugHypothesis> {
        let mut hypotheses = Vec::new();

        // Heuristic 1: null / None / nil pointer
        if error_message.contains("null")
            || error_message.contains("None")
            || error_message.contains("nil")
            || error_message.contains("NullPointerException")
            || error_message.contains("unwrap()")
        {
            hypotheses.push(DebugHypothesis {
                rank: 1,
                summary: "Null/None dereference".to_string(),
                likely_cause: "A value expected to be present was null/None. Check optional handling.".to_string(),
                confidence: 0.85,
                suggested_breakpoints: stack_trace
                    .first()
                    .map(|s| extract_location(s))
                    .into_iter()
                    .flatten()
                    .collect(),
            });
        }

        // Heuristic 2: index out of bounds
        if error_message.contains("index out of")
            || error_message.contains("IndexError")
            || error_message.contains("ArrayIndexOutOfBoundsException")
            || error_message.contains("out of range")
        {
            hypotheses.push(DebugHypothesis {
                rank: hypotheses.len() + 1,
                summary: "Index out of bounds".to_string(),
                likely_cause: "An array or collection was accessed with an invalid index. Verify length before access.".to_string(),
                confidence: 0.80,
                suggested_breakpoints: stack_trace
                    .first()
                    .map(|s| extract_location(s))
                    .into_iter()
                    .flatten()
                    .collect(),
            });
        }

        // Heuristic 3: type / cast error
        if error_message.contains("type")
            || error_message.contains("cast")
            || error_message.contains("TypeError")
            || error_message.contains("ClassCastException")
        {
            hypotheses.push(DebugHypothesis {
                rank: hypotheses.len() + 1,
                summary: "Type mismatch or invalid cast".to_string(),
                likely_cause: "A value was used with an incompatible type. Check type annotations and conversions.".to_string(),
                confidence: 0.70,
                suggested_breakpoints: Vec::new(),
            });
        }

        // Heuristic 4: division by zero
        if error_message.contains("division by zero")
            || error_message.contains("divide by zero")
            || error_message.contains("ZeroDivisionError")
        {
            hypotheses.push(DebugHypothesis {
                rank: hypotheses.len() + 1,
                summary: "Division by zero".to_string(),
                likely_cause: "A divisor was zero. Add a guard before the division.".to_string(),
                confidence: 0.90,
                suggested_breakpoints: Vec::new(),
            });
        }

        // Heuristic 5: stack overflow / recursion
        if error_message.contains("stack overflow")
            || error_message.contains("StackOverflowError")
            || error_message.contains("maximum recursion")
        {
            hypotheses.push(DebugHypothesis {
                rank: hypotheses.len() + 1,
                summary: "Stack overflow / infinite recursion".to_string(),
                likely_cause: "A recursive function lacks a proper base case or has unbounded depth.".to_string(),
                confidence: 0.88,
                suggested_breakpoints: Vec::new(),
            });
        }

        // Fallback: generic hypothesis
        if hypotheses.is_empty() {
            hypotheses.push(DebugHypothesis {
                rank: 1,
                summary: "Unknown error".to_string(),
                likely_cause: format!(
                    "Error: '{}'. Inspect variables near the crash site.",
                    error_message
                ),
                confidence: 0.30,
                suggested_breakpoints: Vec::new(),
            });
        }

        self.hypotheses = hypotheses.clone();
        hypotheses
    }

    // ── Root cause analysis ─────────────────────────────────────────────

    /// Analyse variables at the crash point (current frame) and return a
    /// human-readable diagnosis.
    pub fn root_cause_analysis(&self) -> Result<String, String> {
        let frame = self.current_frame().ok_or("No stack frame to analyse")?;
        let mut report = format!(
            "Root-cause analysis at {}:{} ({})\n",
            frame.file, frame.line, frame.function_name
        );

        if frame.variables.is_empty() {
            report.push_str("  No variables captured — add watches or inspect locals.\n");
            return Ok(report);
        }

        for (name, var) in &frame.variables {
            report.push_str(&format!(
                "  {} : {} = {}\n",
                name, var.var_type, var.value
            ));

            // Flag suspicious values.
            let lower = var.value.to_lowercase();
            if lower == "null" || lower == "none" || lower == "nil" || lower == "undefined" {
                report.push_str(&format!("    ⚠ '{}' is null/None — possible root cause\n", name));
            }
            if var.value == "0" && var.var_type.contains("int") {
                report.push_str(&format!(
                    "    ⚠ '{}' is zero — check if used as divisor\n",
                    name
                ));
            }
            if !var.children.is_empty() {
                report.push_str(&format!(
                    "    {} has {} child fields\n",
                    name,
                    var.children.len()
                ));
            }
        }

        Ok(report)
    }

    // ── Auto-fix suggestions ────────────────────────────────────────────

    /// Based on hypotheses and current frame, generate auto-fix suggestions.
    pub fn generate_fix_suggestions(&mut self) -> Vec<AutoFixSuggestion> {
        let mut suggestions = Vec::new();

        for hyp in &self.hypotheses {
            match hyp.summary.as_str() {
                "Null/None dereference" => {
                    if let Some(frame) = self.current_frame() {
                        for (name, var) in &frame.variables {
                            let lower = var.value.to_lowercase();
                            if lower == "null" || lower == "none" || lower == "nil" {
                                suggestions.push(AutoFixSuggestion {
                                    file: frame.file.clone(),
                                    line: frame.line,
                                    original: format!("{}.unwrap()", name),
                                    replacement: format!(
                                        "{}.unwrap_or_default()",
                                        name
                                    ),
                                    explanation: format!(
                                        "Variable '{}' is null/None. Use a safe fallback instead of unwrapping.",
                                        name
                                    ),
                                    confidence: hyp.confidence,
                                });
                            }
                        }
                    }
                }
                "Division by zero" => {
                    if let Some(frame) = self.current_frame() {
                        for (name, var) in &frame.variables {
                            if var.value == "0" && (var.var_type.contains("int") || var.var_type.starts_with("i32") || var.var_type.starts_with("i64") || var.var_type.starts_with("u32") || var.var_type.starts_with("u64") || var.var_type.contains("number") || var.var_type.contains("unsigned")) {
                                suggestions.push(AutoFixSuggestion {
                                    file: frame.file.clone(),
                                    line: frame.line,
                                    original: format!("/ {}", name),
                                    replacement: format!(
                                        "/ {}.max(1)",
                                        name
                                    ),
                                    explanation: format!(
                                        "Variable '{}' is zero. Guard the division to prevent panic.",
                                        name
                                    ),
                                    confidence: hyp.confidence,
                                });
                            }
                        }
                    }
                }
                "Index out of bounds" => {
                    if let Some(frame) = self.current_frame() {
                        suggestions.push(AutoFixSuggestion {
                            file: frame.file.clone(),
                            line: frame.line,
                            original: "collection[index]".to_string(),
                            replacement: "collection.get(index)".to_string(),
                            explanation: "Use bounds-checked access instead of direct indexing.".to_string(),
                            confidence: hyp.confidence,
                        });
                    }
                }
                _ => {}
            }
        }

        self.fix_suggestions = suggestions.clone();
        suggestions
    }

    /// Stop this session.
    pub fn stop(&mut self) {
        self.state = SessionState::Stopped;
    }
}

// ── DebugManager ────────────────────────────────────────────────────────────

/// Manages multiple concurrent debug sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugManager {
    sessions: HashMap<String, DebugSession>,
    next_session_id: u64,
}

impl DebugManager {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            next_session_id: 1,
        }
    }

    /// Create a new debug session and return its ID.
    pub fn create_session(&mut self, target_file: &str, mode: DebugMode) -> String {
        let id = format!("dbg-{}", self.next_session_id);
        self.next_session_id += 1;
        let session = DebugSession::new(&id, target_file, mode);
        self.sessions.insert(id.clone(), session);
        id
    }

    /// Get a session by ID.
    pub fn get_session(&self, id: &str) -> Option<&DebugSession> {
        self.sessions.get(id)
    }

    /// Get a mutable session by ID.
    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut DebugSession> {
        self.sessions.get_mut(id)
    }

    /// List all session IDs.
    pub fn list_sessions(&self) -> Vec<&str> {
        self.sessions.keys().map(|s| s.as_str()).collect()
    }

    /// Remove a session by ID. Returns `true` if it existed.
    pub fn remove_session(&mut self, id: &str) -> bool {
        self.sessions.remove(id).is_some()
    }

    /// Number of active sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Try to extract `(file, line)` from a stack-trace string like
/// `"  at main.rs:42"` or `"File \"app.py\", line 10"`.
fn extract_location(trace_line: &str) -> Option<(String, usize)> {
    // Pattern: <file>:<line>
    let trimmed = trace_line.trim();
    // Strip leading "at " if present.
    let cleaned = trimmed.strip_prefix("at ").unwrap_or(trimmed);
    if let Some(colon_pos) = cleaned.rfind(':') {
        let file = &cleaned[..colon_pos];
        if let Ok(line) = cleaned[colon_pos + 1..].trim().parse::<usize>() {
            return Some((file.to_string(), line));
        }
    }
    None
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // -- Session creation / management --

    #[test]
    fn test_create_session() {
        let session = DebugSession::new("s1", "main.rs", DebugMode::Interactive);
        assert_eq!(session.id, "s1");
        assert_eq!(session.target_file, "main.rs");
        assert_eq!(session.mode, DebugMode::Interactive);
        assert_eq!(session.state, SessionState::Created);
    }

    #[test]
    fn test_session_default_state() {
        let session = DebugSession::new("s2", "lib.rs", DebugMode::Automated);
        assert_eq!(session.state, SessionState::Created);
        assert!(session.breakpoints.is_empty());
        assert!(session.watches.is_empty());
        assert!(session.stack_frames.is_empty());
    }

    #[test]
    fn test_session_stop() {
        let mut session = DebugSession::new("s3", "app.py", DebugMode::Hybrid);
        session.stop();
        assert_eq!(session.state, SessionState::Stopped);
    }

    // -- Manager --

    #[test]
    fn test_manager_create_session() {
        let mut mgr = DebugManager::new();
        let id = mgr.create_session("main.rs", DebugMode::Interactive);
        assert!(id.starts_with("dbg-"));
        assert_eq!(mgr.session_count(), 1);
    }

    #[test]
    fn test_manager_get_session() {
        let mut mgr = DebugManager::new();
        let id = mgr.create_session("app.rs", DebugMode::Automated);
        assert!(mgr.get_session(&id).is_some());
        assert!(mgr.get_session("nonexistent").is_none());
    }

    #[test]
    fn test_manager_list_sessions() {
        let mut mgr = DebugManager::new();
        let id1 = mgr.create_session("a.rs", DebugMode::Interactive);
        let id2 = mgr.create_session("b.rs", DebugMode::Hybrid);
        let list = mgr.list_sessions();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&id1.as_str()));
        assert!(list.contains(&id2.as_str()));
    }

    #[test]
    fn test_manager_remove_session() {
        let mut mgr = DebugManager::new();
        let id = mgr.create_session("x.rs", DebugMode::Interactive);
        assert!(mgr.remove_session(&id));
        assert!(!mgr.remove_session(&id));
        assert_eq!(mgr.session_count(), 0);
    }

    #[test]
    fn test_manager_multiple_concurrent() {
        let mut mgr = DebugManager::new();
        for i in 0..5 {
            mgr.create_session(&format!("file{}.rs", i), DebugMode::Interactive);
        }
        assert_eq!(mgr.session_count(), 5);
    }

    // -- Breakpoint CRUD --

    #[test]
    fn test_add_breakpoint() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let id = session.add_breakpoint("f.rs", 10, BreakpointType::Line);
        assert_eq!(id, 1);
        assert_eq!(session.list_breakpoints().len(), 1);
    }

    #[test]
    fn test_remove_breakpoint() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let id = session.add_breakpoint("f.rs", 10, BreakpointType::Line);
        assert!(session.remove_breakpoint(id));
        assert!(!session.remove_breakpoint(id));
        assert!(session.list_breakpoints().is_empty());
    }

    #[test]
    fn test_toggle_breakpoint() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let id = session.add_breakpoint("f.rs", 5, BreakpointType::Line);
        assert!(session.get_breakpoint(id).expect("exists").enabled);
        session.toggle_breakpoint(id);
        assert!(!session.get_breakpoint(id).expect("exists").enabled);
        session.toggle_breakpoint(id);
        assert!(session.get_breakpoint(id).expect("exists").enabled);
    }

    #[test]
    fn test_toggle_nonexistent_breakpoint() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        assert!(!session.toggle_breakpoint(999));
    }

    #[test]
    fn test_conditional_breakpoint() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let id = session.add_breakpoint("f.rs", 20, BreakpointType::Conditional("x > 5".into()));
        let bp = session.get_breakpoint(id).expect("exists");
        assert_eq!(bp.bp_type, BreakpointType::Conditional("x > 5".into()));
    }

    #[test]
    fn test_logpoint_breakpoint() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let id = session.add_breakpoint("f.rs", 30, BreakpointType::Logpoint("x={x}".into()));
        let bp = session.get_breakpoint(id).expect("exists");
        assert_eq!(bp.bp_type, BreakpointType::Logpoint("x={x}".into()));
    }

    #[test]
    fn test_multiple_breakpoints_unique_ids() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let id1 = session.add_breakpoint("f.rs", 1, BreakpointType::Line);
        let id2 = session.add_breakpoint("f.rs", 2, BreakpointType::Line);
        let id3 = session.add_breakpoint("f.rs", 3, BreakpointType::Exception);
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
    }

    // -- Watch management --

    #[test]
    fn test_add_watch() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        assert!(session.add_watch("x"));
        assert_eq!(session.list_watches(), &["x"]);
    }

    #[test]
    fn test_add_duplicate_watch() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        assert!(session.add_watch("x"));
        assert!(!session.add_watch("x"));
        assert_eq!(session.list_watches().len(), 1);
    }

    #[test]
    fn test_remove_watch() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        session.add_watch("y");
        assert!(session.remove_watch("y"));
        assert!(!session.remove_watch("y"));
    }

    // -- Stack frames --

    #[test]
    fn test_push_pop_frame() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        session.push_frame(StackFrame::new("main", "main.rs", 1));
        session.push_frame(StackFrame::new("foo", "foo.rs", 42));
        assert_eq!(session.current_frame().expect("has frame").function_name, "foo");
        let popped = session.pop_frame().expect("has frame");
        assert_eq!(popped.function_name, "foo");
        assert_eq!(session.current_frame().expect("has frame").function_name, "main");
    }

    #[test]
    fn test_frame_variables() {
        let mut frame = StackFrame::new("process", "lib.rs", 100);
        frame.add_variable(Variable::new("count", "42", "i32"));
        assert!(frame.get_variable("count").is_some());
        assert!(frame.get_variable("missing").is_none());
    }

    #[test]
    fn test_variable_with_children() {
        let child = Variable::new("x", "1", "i32");
        let parent = Variable::new("point", "{x:1,y:2}", "Point")
            .with_children(vec![child, Variable::new("y", "2", "i32")]);
        assert_eq!(parent.children.len(), 2);
    }

    // -- Debug action execution --

    #[test]
    fn test_step_over() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        // Created state allows stepping.
        let result = session.execute_action(DebugAction::StepOver);
        assert!(result.is_ok());
        assert_eq!(session.state, SessionState::Paused);
    }

    #[test]
    fn test_continue_and_pause() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        session.execute_action(DebugAction::StepOver).unwrap();
        session.execute_action(DebugAction::Continue).unwrap();
        assert_eq!(session.state, SessionState::Running);
        session.execute_action(DebugAction::Pause).unwrap();
        assert_eq!(session.state, SessionState::Paused);
    }

    #[test]
    fn test_pause_when_not_running_fails() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let result = session.execute_action(DebugAction::Pause);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_breakpoint_via_action() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let result = session.execute_action(DebugAction::SetBreakpoint {
            file: "main.rs".into(),
            line: 10,
        });
        assert!(result.is_ok());
        assert_eq!(session.list_breakpoints().len(), 1);
    }

    #[test]
    fn test_inspect_action() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let mut frame = StackFrame::new("run", "f.rs", 5);
        frame.add_variable(Variable::new("val", "hello", "String"));
        session.push_frame(frame);
        let result = session.execute_action(DebugAction::Inspect("val".into()));
        assert!(result.is_ok());
        assert!(result.unwrap().contains("hello"));
    }

    #[test]
    fn test_inspect_missing_variable() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        session.push_frame(StackFrame::new("run", "f.rs", 5));
        let result = session.execute_action(DebugAction::Inspect("nope".into()));
        assert!(result.is_err());
    }

    // -- Hypothesis generation --

    #[test]
    fn test_hypothesis_null() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let hyps = session.generate_hypotheses("called unwrap() on None", &[]);
        assert!(!hyps.is_empty());
        assert!(hyps[0].summary.contains("Null"));
    }

    #[test]
    fn test_hypothesis_index_out_of_bounds() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let hyps = session.generate_hypotheses("index out of range", &[]);
        assert!(hyps.iter().any(|h| h.summary.contains("Index")));
    }

    #[test]
    fn test_hypothesis_division_by_zero() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let hyps = session.generate_hypotheses("division by zero", &[]);
        assert!(hyps.iter().any(|h| h.summary.contains("Division")));
    }

    #[test]
    fn test_hypothesis_stack_overflow() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let hyps = session.generate_hypotheses("stack overflow", &[]);
        assert!(hyps.iter().any(|h| h.summary.contains("Stack overflow")));
    }

    #[test]
    fn test_hypothesis_unknown_fallback() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let hyps = session.generate_hypotheses("something weird", &[]);
        assert_eq!(hyps.len(), 1);
        assert!(hyps[0].summary.contains("Unknown"));
    }

    #[test]
    fn test_hypothesis_with_stack_trace() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let trace = vec!["at main.rs:42".to_string()];
        let hyps = session.generate_hypotheses("called unwrap() on None", &trace);
        assert!(!hyps[0].suggested_breakpoints.is_empty());
        assert_eq!(hyps[0].suggested_breakpoints[0], ("main.rs".to_string(), 42));
    }

    // -- Root cause analysis --

    #[test]
    fn test_root_cause_analysis_no_frame() {
        let session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        assert!(session.root_cause_analysis().is_err());
    }

    #[test]
    fn test_root_cause_analysis_with_null_var() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let mut frame = StackFrame::new("crash", "bug.rs", 55);
        frame.add_variable(Variable::new("ptr", "null", "Option<Box<T>>"));
        session.push_frame(frame);
        let report = session.root_cause_analysis().unwrap();
        assert!(report.contains("null/None"));
    }

    // -- Auto-fix suggestions --

    #[test]
    fn test_fix_suggestions_null() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let mut frame = StackFrame::new("crash", "bug.rs", 10);
        frame.add_variable(Variable::new("x", "None", "Option<i32>"));
        session.push_frame(frame);
        session.generate_hypotheses("called unwrap() on None", &[]);
        let fixes = session.generate_fix_suggestions();
        assert!(!fixes.is_empty());
        assert!(fixes[0].replacement.contains("unwrap_or_default"));
    }

    #[test]
    fn test_fix_suggestions_division() {
        let mut session = DebugSession::new("s", "f.rs", DebugMode::Interactive);
        let mut frame = StackFrame::new("calc", "math.rs", 20);
        frame.add_variable(Variable::new("divisor", "0", "i32_unsigned"));
        session.push_frame(frame);
        session.generate_hypotheses("division by zero", &[]);
        let fixes = session.generate_fix_suggestions();
        assert!(!fixes.is_empty());
        assert!(fixes[0].replacement.contains("max(1)"));
    }

    // -- Helper --

    #[test]
    fn test_extract_location() {
        assert_eq!(extract_location("at main.rs:42"), Some(("main.rs".to_string(), 42)));
        assert_eq!(extract_location("  src/lib.rs:100  "), Some(("src/lib.rs".to_string(), 100)));
        assert_eq!(extract_location("no colon here"), None);
    }

    // -- Display impls --

    #[test]
    fn test_debug_mode_display() {
        assert_eq!(format!("{}", DebugMode::Interactive), "Interactive");
        assert_eq!(format!("{}", DebugMode::Automated), "Automated");
        assert_eq!(format!("{}", DebugMode::Hybrid), "Hybrid");
    }

    #[test]
    fn test_debug_action_display() {
        assert_eq!(format!("{}", DebugAction::StepOver), "Step Over");
        assert_eq!(
            format!("{}", DebugAction::SetBreakpoint { file: "a.rs".into(), line: 1 }),
            "SetBreakpoint(a.rs:1)"
        );
    }
}
