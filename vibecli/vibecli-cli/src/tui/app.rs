use tokio::sync::oneshot;
use vibe_ai::tools::{ToolCall, ToolResult};
use crate::tui::theme::{Theme, get_theme};

#[derive(Debug, Clone)]
pub enum TuiMessage {
    User(String),
    Assistant(String),
    /// Live streaming response — shown with animated cursor.
    AssistantStreaming(String),
    System(String),
    #[allow(dead_code)]
    CommandOutput {
        command: String,
        output: String,
    },
    #[allow(dead_code)]
    FileList {
        path: String,
        files: Vec<String>,
    },
    #[allow(dead_code)]
    Diff {
        file: String,
        diff: String,
    },
    Error(String),
}

pub enum CurrentScreen {
    Chat,
    FileTree,
    DiffView,
    Agent,
    VimEditor,
}

use crate::tui::components::agent_view::AgentViewComponent;
use crate::tui::components::diagnostics::DiagnosticsComponent;
use crate::tui::components::file_tree::FileTreeComponent;
use crate::tui::components::diff_view::DiffViewComponent;
use crate::tui::components::vim_editor::VimEditorComponent;

/// Holds a pending tool-call approval: the call to show the user and the
/// channel to send the approved result (or None for rejection) back to the agent.
pub struct PendingApproval {
    pub call: ToolCall,
    pub result_tx: oneshot::Sender<Option<ToolResult>>,
}

pub struct App {
    pub current_screen: CurrentScreen,
    pub should_quit: bool,
    pub messages: Vec<TuiMessage>,
    pub input: String,
    pub file_tree: FileTreeComponent,
    pub diff_view: DiffViewComponent,
    pub agent_view: AgentViewComponent,
    pub vim_editor: VimEditorComponent,
    pub exit_pending: bool,
    pub scroll_offset: u16,
    /// Pending approval for the current tool call (Suggest mode).
    pub pending_approval: Option<PendingApproval>,
    /// Active color theme.
    pub theme: Theme,
    /// Diagnostics pane (populated by /check command).
    pub diagnostics_panel: DiagnosticsComponent,
}

impl App {
    pub fn new() -> Self {
        let config = crate::config::Config::load().unwrap_or_default();
        let theme_name = config.ui.theme.as_deref().unwrap_or("dark");
        Self {
            current_screen: CurrentScreen::Chat,
            should_quit: false,
            messages: Vec::new(),
            input: String::new(),
            file_tree: FileTreeComponent::new(),
            diff_view: DiffViewComponent::new(),
            agent_view: AgentViewComponent::new(),
            vim_editor: VimEditorComponent::new(),
            exit_pending: false,
            scroll_offset: 0,
            pending_approval: None,
            theme: get_theme(theme_name),
            diagnostics_panel: DiagnosticsComponent::new(),
        }
    }

    #[allow(dead_code)]
    pub fn on_tick(&mut self) {}

    pub fn on_key(&mut self, c: char) {
        self.input.push(c);
    }

    pub fn on_backspace(&mut self) {
        self.input.pop();
    }

    pub fn on_enter(&mut self) -> Option<String> {
        if !self.input.is_empty() {
            let content: String = self.input.drain(..).collect();
            self.messages.push(TuiMessage::User(content.clone()));
            return Some(content);
        }
        None
    }
}
