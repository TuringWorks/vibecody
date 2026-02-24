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
