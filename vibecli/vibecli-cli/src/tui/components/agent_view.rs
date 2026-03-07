//! TUI component that renders the agent step list and approval prompt.

use vibe_ai::agent::AgentStep;
use vibe_ai::tools::ToolCall;

#[derive(Debug, Default)]
pub enum AgentStatus {
    #[default]
    Running,
    WaitingApproval,
    Complete(String),
    Error(String),
}

pub struct AgentViewComponent {
    pub steps: Vec<AgentStep>,
    /// Text accumulated from the current streaming turn.
    pub streaming_text: String,
    /// Tool call awaiting user approval (Suggest mode).
    pub pending_call: Option<ToolCall>,
    pub status: AgentStatus,
    pub scroll: u16,
}

impl Default for AgentViewComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentViewComponent {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            streaming_text: String::new(),
            pending_call: None,
            status: AgentStatus::Running,
            scroll: 0,
        }
    }

    pub fn add_step(&mut self, step: AgentStep) {
        self.steps.push(step);
        self.streaming_text.clear();
    }

    pub fn append_stream(&mut self, text: &str) {
        self.streaming_text.push_str(text);
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_add(3);
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_sub(3);
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vibe_ai::tools::ToolResult;

    // ── AgentViewComponent::new ─────────────────────────────────────────────

    #[test]
    fn new_has_empty_steps() {
        let av = AgentViewComponent::new();
        assert!(av.steps.is_empty());
    }

    #[test]
    fn new_has_empty_streaming_text() {
        let av = AgentViewComponent::new();
        assert!(av.streaming_text.is_empty());
    }

    #[test]
    fn new_has_no_pending_call() {
        let av = AgentViewComponent::new();
        assert!(av.pending_call.is_none());
    }

    #[test]
    fn new_has_running_status() {
        let av = AgentViewComponent::new();
        assert!(matches!(av.status, AgentStatus::Running));
    }

    #[test]
    fn new_has_zero_scroll() {
        let av = AgentViewComponent::new();
        assert_eq!(av.scroll, 0);
    }

    // ── Default trait ───────────────────────────────────────────────────────

    #[test]
    fn default_matches_new() {
        let d = AgentViewComponent::default();
        let n = AgentViewComponent::new();
        assert_eq!(d.steps.len(), n.steps.len());
        assert_eq!(d.streaming_text, n.streaming_text);
        assert_eq!(d.scroll, n.scroll);
    }

    // ── append_stream ───────────────────────────────────────────────────────

    #[test]
    fn append_stream_accumulates_text() {
        let mut av = AgentViewComponent::new();
        av.append_stream("Hello ");
        av.append_stream("world");
        assert_eq!(av.streaming_text, "Hello world");
    }

    #[test]
    fn append_stream_empty_string_is_noop() {
        let mut av = AgentViewComponent::new();
        av.append_stream("");
        assert!(av.streaming_text.is_empty());
    }

    // ── add_step ────────────────────────────────────────────────────────────

    #[test]
    fn add_step_pushes_and_clears_streaming() {
        let mut av = AgentViewComponent::new();
        av.append_stream("partial text");
        let step = AgentStep {
            step_num: 1,
            tool_call: ToolCall::Bash { command: "ls".into() },
            tool_result: ToolResult { tool_name: "bash".into(), output: "file.txt".into(), success: true, truncated: false },
            approved: true,
        };
        av.add_step(step);
        assert_eq!(av.steps.len(), 1);
        assert!(av.streaming_text.is_empty());
    }

    #[test]
    fn add_step_preserves_step_data() {
        let mut av = AgentViewComponent::new();
        let step = AgentStep {
            step_num: 42,
            tool_call: ToolCall::ReadFile { path: "/tmp/x".into() },
            tool_result: ToolResult { tool_name: "read_file".into(), output: "contents".into(), success: true, truncated: false },
            approved: false,
        };
        av.add_step(step);
        assert_eq!(av.steps[0].step_num, 42);
        assert!(!av.steps[0].approved);
    }

    // ── scroll_up / scroll_down ─────────────────────────────────────────────

    #[test]
    fn scroll_up_adds_three() {
        let mut av = AgentViewComponent::new();
        av.scroll_up();
        assert_eq!(av.scroll, 3);
        av.scroll_up();
        assert_eq!(av.scroll, 6);
    }

    #[test]
    fn scroll_down_subtracts_three() {
        let mut av = AgentViewComponent::new();
        av.scroll = 10;
        av.scroll_down();
        assert_eq!(av.scroll, 7);
        av.scroll_down();
        assert_eq!(av.scroll, 4);
    }

    #[test]
    fn scroll_down_saturates_at_zero() {
        let mut av = AgentViewComponent::new();
        av.scroll = 1;
        av.scroll_down();
        assert_eq!(av.scroll, 0);
        av.scroll_down();
        assert_eq!(av.scroll, 0);
    }

    // ── reset ───────────────────────────────────────────────────────────────

    #[test]
    fn reset_clears_everything() {
        let mut av = AgentViewComponent::new();
        av.append_stream("some text");
        av.scroll = 20;
        let step = AgentStep {
            step_num: 1,
            tool_call: ToolCall::Bash { command: "echo hi".into() },
            tool_result: ToolResult { tool_name: "bash".into(), output: "hi".into(), success: true, truncated: false },
            approved: true,
        };
        av.add_step(step);
        av.status = AgentStatus::Complete("done".into());

        av.reset();
        assert!(av.steps.is_empty());
        assert!(av.streaming_text.is_empty());
        assert!(av.pending_call.is_none());
        assert_eq!(av.scroll, 0);
        assert!(matches!(av.status, AgentStatus::Running));
    }

    // ── AgentStatus ─────────────────────────────────────────────────────────

    #[test]
    fn agent_status_default_is_running() {
        let status = AgentStatus::default();
        assert!(matches!(status, AgentStatus::Running));
    }
}
