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
    /// Latest JobManager metrics snapshot — populated by the event loop
    /// (or a periodic tick) when a daemon is reachable. When `Some`, the
    /// UI renders a compact single-line strip above the input area.
    pub job_metrics: Option<crate::job_manager::JobManagerMetrics>,
    /// Monotonic timestamp of the most recent metrics tick. Used to fade
    /// the strip out when the daemon stops responding — see
    /// `MetricsFreshness::classify`.
    pub last_metrics_tick: Option<std::time::Instant>,
}

/// How fresh the last metrics snapshot is. Drives the strip's render mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricsFreshness {
    /// Under `STALE_AFTER_MS`: render in full color.
    Fresh,
    /// Between `STALE_AFTER_MS` and `HIDE_AFTER_MS`: render dimmed so the
    /// operator knows the numbers aren't live but can still read the last
    /// known state.
    Stale,
    /// Over `HIDE_AFTER_MS`, or no tick ever received: don't render.
    Hidden,
}

/// Fade the strip to dim after this many ms without a tick.
pub const STALE_AFTER_MS: u128 = 30_000;
/// Hide the strip entirely after this many ms without a tick. The last
/// known snapshot is kept in memory so a returning daemon shows it
/// immediately rather than waiting for the next tick.
pub const HIDE_AFTER_MS: u128 = 90_000;

impl MetricsFreshness {
    /// Pure classifier — takes the elapsed time since the last tick in ms.
    /// `None` means no tick has ever arrived.
    pub fn classify(elapsed_ms: Option<u128>) -> Self {
        match elapsed_ms {
            None => MetricsFreshness::Hidden,
            Some(ms) if ms < STALE_AFTER_MS => MetricsFreshness::Fresh,
            Some(ms) if ms < HIDE_AFTER_MS => MetricsFreshness::Stale,
            Some(_) => MetricsFreshness::Hidden,
        }
    }
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
            job_metrics: None,
            last_metrics_tick: None,
        }
    }

    /// Classify the current metrics freshness based on the monotonic
    /// timestamp of the last tick.
    pub fn metrics_freshness(&self) -> MetricsFreshness {
        let elapsed = self
            .last_metrics_tick
            .map(|t| t.elapsed().as_millis());
        MetricsFreshness::classify(elapsed)
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    // ── App::new defaults ───────────────────────────────────────────────────

    #[test]
    fn app_new_starts_on_chat_screen() {
        let app = App::new();
        assert!(matches!(app.current_screen, CurrentScreen::Chat));
    }

    #[test]
    fn app_new_should_quit_is_false() {
        let app = App::new();
        assert!(!app.should_quit);
    }

    #[test]
    fn app_new_exit_pending_is_false() {
        let app = App::new();
        assert!(!app.exit_pending);
    }

    #[test]
    fn app_new_scroll_offset_is_zero() {
        let app = App::new();
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn app_new_no_pending_approval() {
        let app = App::new();
        assert!(app.pending_approval.is_none());
    }

    #[test]
    fn app_new_has_a_theme() {
        let app = App::new();
        // Should have a valid theme name
        assert!(!app.theme.name.is_empty());
    }

    // ── on_key / on_backspace ───────────────────────────────────────────────

    #[test]
    fn on_key_appends_characters() {
        let mut app = App::new();
        app.on_key('a');
        app.on_key('b');
        app.on_key('c');
        assert_eq!(app.input, "abc");
    }

    #[test]
    fn on_backspace_removes_last_char() {
        let mut app = App::new();
        app.on_key('x');
        app.on_key('y');
        app.on_backspace();
        assert_eq!(app.input, "x");
    }

    #[test]
    fn on_backspace_on_empty_is_noop() {
        let mut app = App::new();
        app.on_backspace();
        assert_eq!(app.input, "");
    }

    // ── on_enter ────────────────────────────────────────────────────────────

    #[test]
    fn on_enter_with_content_returns_some() {
        let mut app = App::new();
        app.on_key('H');
        app.on_key('i');
        let result = app.on_enter();
        assert_eq!(result, Some("Hi".to_string()));
    }

    #[test]
    fn on_enter_clears_input() {
        let mut app = App::new();
        app.on_key('x');
        app.on_enter();
        assert!(app.input.is_empty());
    }

    #[test]
    fn on_enter_pushes_user_message() {
        let mut app = App::new();
        app.on_key('t');
        app.on_key('e');
        app.on_key('s');
        app.on_key('t');
        app.on_enter();
        assert_eq!(app.messages.len(), 1);
        match &app.messages[0] {
            TuiMessage::User(content) => assert_eq!(content, "test"),
            _ => panic!("Expected TuiMessage::User"),
        }
    }

    #[test]
    fn on_enter_with_empty_input_returns_none() {
        let mut app = App::new();
        let result = app.on_enter();
        assert!(result.is_none());
        assert!(app.messages.is_empty());
    }

    #[test]
    fn on_enter_twice_pushes_two_messages() {
        let mut app = App::new();
        app.on_key('a');
        app.on_enter();
        app.on_key('b');
        app.on_enter();
        assert_eq!(app.messages.len(), 2);
    }

    // ── TuiMessage variants ─────────────────────────────────────────────────

    #[test]
    fn tui_message_user_stores_content() {
        let msg = TuiMessage::User("hello".into());
        match msg {
            TuiMessage::User(s) => assert_eq!(s, "hello"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn tui_message_command_output_stores_both_fields() {
        let msg = TuiMessage::CommandOutput {
            command: "ls".into(),
            output: "file.txt".into(),
        };
        match msg {
            TuiMessage::CommandOutput { command, output } => {
                assert_eq!(command, "ls");
                assert_eq!(output, "file.txt");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn tui_message_error_stores_message() {
        let msg = TuiMessage::Error("oops".into());
        match msg {
            TuiMessage::Error(s) => assert_eq!(s, "oops"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn tui_message_clone() {
        let msg = TuiMessage::Assistant("reply".into());
        let cloned = msg.clone();
        match cloned {
            TuiMessage::Assistant(s) => assert_eq!(s, "reply"),
            _ => panic!("wrong variant"),
        }
    }

    // ── MetricsFreshness::classify ──────────────────────────────────────────

    #[test]
    fn metrics_freshness_hidden_without_tick() {
        assert_eq!(MetricsFreshness::classify(None), MetricsFreshness::Hidden);
    }

    #[test]
    fn metrics_freshness_fresh_just_after_tick() {
        assert_eq!(MetricsFreshness::classify(Some(0)), MetricsFreshness::Fresh);
        assert_eq!(
            MetricsFreshness::classify(Some(STALE_AFTER_MS - 1)),
            MetricsFreshness::Fresh
        );
    }

    #[test]
    fn metrics_freshness_stale_at_stale_threshold() {
        assert_eq!(
            MetricsFreshness::classify(Some(STALE_AFTER_MS)),
            MetricsFreshness::Stale
        );
        assert_eq!(
            MetricsFreshness::classify(Some(HIDE_AFTER_MS - 1)),
            MetricsFreshness::Stale
        );
    }

    #[test]
    fn metrics_freshness_hidden_at_hide_threshold() {
        assert_eq!(
            MetricsFreshness::classify(Some(HIDE_AFTER_MS)),
            MetricsFreshness::Hidden
        );
        assert_eq!(
            MetricsFreshness::classify(Some(HIDE_AFTER_MS * 10)),
            MetricsFreshness::Hidden
        );
    }

    #[test]
    fn metrics_freshness_fresh_after_recording_tick() {
        let mut app = App::new();
        app.last_metrics_tick = Some(std::time::Instant::now());
        assert_eq!(app.metrics_freshness(), MetricsFreshness::Fresh);
    }
}
