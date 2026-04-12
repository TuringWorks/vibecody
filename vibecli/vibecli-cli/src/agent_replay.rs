#![allow(dead_code)]
//! Agent replay debugger — records agent execution traces and replays them
//! step-by-step for debugging. Extends the existing `repro_agent` module.
//!
//! Features:
//! - Record agent turns (tool calls, responses, state transitions)
//! - Replay at full speed or step-by-step
//! - Time-travel: jump to any step index
//! - Diff actual vs expected outputs
//! - Export trace as JSON

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single agent action captured during execution.
#[derive(Debug, Clone)]
pub struct AgentStep {
    pub index: usize,
    pub kind: StepKind,
    pub timestamp_ms: u64,
    pub input: Option<String>,
    pub output: Option<String>,
    pub tool_name: Option<String>,
    pub tool_args: Option<String>,
    pub state_before: Option<String>,
    pub state_after: Option<String>,
    pub duration_ms: u64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepKind {
    /// Agent received a user message.
    UserMessage,
    /// Agent produced a text response.
    AssistantMessage,
    /// Agent invoked a tool.
    ToolCall,
    /// Tool returned a result.
    ToolResult,
    /// Agent state transition (FSM event).
    StateTransition,
    /// Agent planning step (thought).
    Thought,
    /// Error occurred.
    Error,
}

impl std::fmt::Display for StepKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepKind::UserMessage => write!(f, "user_msg"),
            StepKind::AssistantMessage => write!(f, "asst_msg"),
            StepKind::ToolCall => write!(f, "tool_call"),
            StepKind::ToolResult => write!(f, "tool_result"),
            StepKind::StateTransition => write!(f, "state_transition"),
            StepKind::Thought => write!(f, "thought"),
            StepKind::Error => write!(f, "error"),
        }
    }
}

/// A complete execution trace recorded from one agent run.
#[derive(Debug, Clone)]
pub struct ExecutionTrace {
    pub session_id: String,
    pub agent_name: String,
    pub started_at_ms: u64,
    pub ended_at_ms: Option<u64>,
    pub steps: Vec<AgentStep>,
    pub final_state: Option<String>,
    pub exit_reason: Option<String>,
}

impl ExecutionTrace {
    pub fn new(session_id: impl Into<String>, agent_name: impl Into<String>, started_at_ms: u64) -> Self {
        Self {
            session_id: session_id.into(),
            agent_name: agent_name.into(),
            started_at_ms,
            ended_at_ms: None,
            steps: Vec::new(),
            final_state: None,
            exit_reason: None,
        }
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.ended_at_ms.map(|end| end.saturating_sub(self.started_at_ms))
    }

    pub fn tool_calls(&self) -> Vec<&AgentStep> {
        self.steps.iter().filter(|s| s.kind == StepKind::ToolCall).collect()
    }

    pub fn errors(&self) -> Vec<&AgentStep> {
        self.steps.iter().filter(|s| s.kind == StepKind::Error).collect()
    }
}

// ---------------------------------------------------------------------------
// Recorder
// ---------------------------------------------------------------------------

/// Records agent steps into a trace.
pub struct TraceRecorder {
    trace: ExecutionTrace,
    clock_ms: u64,
}

impl TraceRecorder {
    pub fn new(session_id: impl Into<String>, agent_name: impl Into<String>, clock_ms: u64) -> Self {
        Self {
            trace: ExecutionTrace::new(session_id, agent_name, clock_ms),
            clock_ms,
        }
    }

    /// Advance the mock clock (for deterministic tests).
    pub fn advance(&mut self, ms: u64) {
        self.clock_ms += ms;
    }

    pub fn record_step(&mut self, kind: StepKind, input: Option<String>, output: Option<String>, duration_ms: u64) -> usize {
        let index = self.trace.steps.len();
        self.trace.steps.push(AgentStep {
            index,
            kind,
            timestamp_ms: self.clock_ms,
            input,
            output,
            tool_name: None,
            tool_args: None,
            state_before: None,
            state_after: None,
            duration_ms,
            metadata: HashMap::new(),
        });
        self.advance(duration_ms);
        index
    }

    pub fn record_tool_call(&mut self, tool: impl Into<String>, args: impl Into<String>, result: impl Into<String>, duration_ms: u64) -> usize {
        let index = self.trace.steps.len();
        let tool_s = tool.into();
        self.trace.steps.push(AgentStep {
            index,
            kind: StepKind::ToolCall,
            timestamp_ms: self.clock_ms,
            input: Some(args.into()),
            output: Some(result.into()),
            tool_name: Some(tool_s),
            tool_args: None,
            state_before: None,
            state_after: None,
            duration_ms,
            metadata: HashMap::new(),
        });
        self.advance(duration_ms);
        index
    }

    pub fn finish(mut self, exit_reason: impl Into<String>, final_state: impl Into<String>) -> ExecutionTrace {
        self.trace.ended_at_ms = Some(self.clock_ms);
        self.trace.exit_reason = Some(exit_reason.into());
        self.trace.final_state = Some(final_state.into());
        self.trace
    }
}

// ---------------------------------------------------------------------------
// Replayer
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayMode {
    /// Execute all steps immediately.
    FullSpeed,
    /// Pause after each step.
    StepByStep,
}

/// Step-by-step replay of a recorded execution trace.
pub struct TraceReplayer {
    pub trace: ExecutionTrace,
    pub current_index: usize,
    pub mode: ReplayMode,
}

impl TraceReplayer {
    pub fn new(trace: ExecutionTrace) -> Self {
        Self {
            trace,
            current_index: 0,
            mode: ReplayMode::StepByStep,
        }
    }

    /// Jump to a specific step index.
    pub fn seek(&mut self, index: usize) -> Result<(), String> {
        if index >= self.trace.steps.len() {
            return Err(format!("Step {} out of range (max {})", index, self.trace.steps.len() - 1));
        }
        self.current_index = index;
        Ok(())
    }

    /// Returns the current step without advancing.
    pub fn peek(&self) -> Option<&AgentStep> {
        self.trace.steps.get(self.current_index)
    }

    /// Advance to next step. Returns the step just completed.
    pub fn step(&mut self) -> Option<&AgentStep> {
        let step = self.trace.steps.get(self.current_index)?;
        self.current_index += 1;
        Some(step)
    }

    /// Collect all remaining steps.
    pub fn run_all(&mut self) -> Vec<&AgentStep> {
        let start = self.current_index;
        self.current_index = self.trace.steps.len();
        self.trace.steps[start..].iter().collect()
    }

    pub fn is_done(&self) -> bool {
        self.current_index >= self.trace.steps.len()
    }

    pub fn remaining(&self) -> usize {
        self.trace.steps.len().saturating_sub(self.current_index)
    }

    /// Render a summary of the entire trace.
    pub fn summary(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Trace: {} ({})\n", self.trace.session_id, self.trace.agent_name));
        out.push_str(&format!("Steps: {} | Duration: {}ms\n",
            self.trace.steps.len(),
            self.trace.duration_ms().unwrap_or(0)
        ));
        out.push_str(&format!("Exit: {}\n\n", self.trace.exit_reason.as_deref().unwrap_or("?")));
        for step in &self.trace.steps {
            let marker = if step.index < self.current_index { "✓" } else if step.index == self.current_index { "▶" } else { " " };
            out.push_str(&format!("{} [{:02}] {} {}ms",
                marker, step.index, step.kind,
                step.duration_ms
            ));
            if let Some(tool) = &step.tool_name {
                out.push_str(&format!(" ({})", tool));
            }
            out.push('\n');
        }
        out
    }

    /// Compare step output against an expected string.
    pub fn assert_step_output(&self, index: usize, expected: &str) -> Result<(), String> {
        let step = self.trace.steps.get(index)
            .ok_or_else(|| format!("Step {} not found", index))?;
        let actual = step.output.as_deref().unwrap_or("");
        if actual == expected {
            Ok(())
        } else {
            Err(format!("Step {} output mismatch:\nExpected: {:?}\nActual:   {:?}", index, expected, actual))
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trace() -> ExecutionTrace {
        let mut rec = TraceRecorder::new("sess-1", "coder-agent", 1000);
        rec.record_step(StepKind::UserMessage, Some("Fix the bug".into()), None, 5);
        rec.record_step(StepKind::Thought, None, Some("I need to read the file".into()), 10);
        rec.record_tool_call("read_file", "main.rs", "fn main() {}", 20);
        rec.record_step(StepKind::AssistantMessage, None, Some("Fixed!".into()), 15);
        rec.finish("complete", "idle")
    }

    #[test]
    fn test_trace_has_steps() {
        let trace = make_trace();
        assert_eq!(trace.steps.len(), 4);
    }

    #[test]
    fn test_trace_duration() {
        let trace = make_trace();
        // 5 + 10 + 20 + 15 = 50ms
        assert_eq!(trace.duration_ms(), Some(50));
    }

    #[test]
    fn test_tool_calls() {
        let trace = make_trace();
        let calls = trace.tool_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name.as_deref(), Some("read_file"));
    }

    #[test]
    fn test_errors_none() {
        let trace = make_trace();
        assert!(trace.errors().is_empty());
    }

    #[test]
    fn test_replayer_step_by_step() {
        let trace = make_trace();
        let mut replayer = TraceReplayer::new(trace);
        let s0 = replayer.step().unwrap();
        assert_eq!(s0.kind, StepKind::UserMessage);
        assert_eq!(replayer.current_index, 1);
        assert!(!replayer.is_done());
    }

    #[test]
    fn test_replayer_seek() {
        let trace = make_trace();
        let mut replayer = TraceReplayer::new(trace);
        replayer.seek(2).unwrap();
        let step = replayer.peek().unwrap();
        assert_eq!(step.kind, StepKind::ToolCall);
    }

    #[test]
    fn test_replayer_seek_out_of_range() {
        let trace = make_trace();
        let mut replayer = TraceReplayer::new(trace);
        assert!(replayer.seek(100).is_err());
    }

    #[test]
    fn test_replayer_run_all() {
        let trace = make_trace();
        let mut replayer = TraceReplayer::new(trace);
        let steps = replayer.run_all();
        assert_eq!(steps.len(), 4);
        assert!(replayer.is_done());
    }

    #[test]
    fn test_replayer_remaining() {
        let trace = make_trace();
        let mut replayer = TraceReplayer::new(trace);
        assert_eq!(replayer.remaining(), 4);
        replayer.step();
        assert_eq!(replayer.remaining(), 3);
    }

    #[test]
    fn test_assert_step_output_ok() {
        let trace = make_trace();
        let replayer = TraceReplayer::new(trace);
        // step 2 is tool call with output "fn main() {}"
        assert!(replayer.assert_step_output(2, "fn main() {}").is_ok());
    }

    #[test]
    fn test_assert_step_output_mismatch() {
        let trace = make_trace();
        let replayer = TraceReplayer::new(trace);
        let result = replayer.assert_step_output(2, "wrong output");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mismatch"));
    }

    #[test]
    fn test_summary_output() {
        let trace = make_trace();
        let replayer = TraceReplayer::new(trace);
        let s = replayer.summary();
        assert!(s.contains("sess-1"));
        assert!(s.contains("tool_call"));
    }

    #[test]
    fn test_record_error_step() {
        let mut rec = TraceRecorder::new("s", "a", 0);
        rec.record_step(StepKind::Error, None, Some("timeout".into()), 0);
        let trace = rec.finish("error", "aborted");
        assert_eq!(trace.errors().len(), 1);
    }

    #[test]
    fn test_peek_does_not_advance() {
        let trace = make_trace();
        let replayer = TraceReplayer::new(trace);
        replayer.peek();
        assert_eq!(replayer.current_index, 0);
    }
}
