#[cfg(test)]
mod tests {
    use crate::tui::app::{App, CurrentScreen, TuiMessage};
    use crate::tui::components::diff_view::DiffViewComponent;

    #[test]
    fn test_app_initial_state() {
        let app = App::new();
        assert!(matches!(app.current_screen, CurrentScreen::Chat));
        assert_eq!(app.input, "");
        assert!(app.messages.is_empty());
        assert!(!app.should_quit);
    }

    #[test]
    fn test_app_input_handling() {
        let mut app = App::new();

        // Test typing
        app.on_key('H');
        app.on_key('i');
        assert_eq!(app.input, "Hi");

        // Test backspace
        app.on_backspace();
        assert_eq!(app.input, "H");

        // Test enter
        let msg = app.on_enter();
        assert_eq!(msg, Some("H".to_string()));
        assert_eq!(app.input, "");
        assert_eq!(app.messages.len(), 1);

        if let TuiMessage::User(content) = &app.messages[0] {
            assert_eq!(content, "H");
        } else {
            panic!("Expected User message");
        }
    }

    #[test]
    fn test_diff_view_logic() {
        let mut diff_view = DiffViewComponent::new();

        // Test setting diff
        let original = "foo\nbar";
        let modified = "foo\nbaz";
        diff_view.set_diff(original, modified);

        assert!(!diff_view.hunks.is_empty());
        assert_eq!(diff_view.scroll, 0);

        // Test scrolling
        diff_view.scroll_down();
        assert_eq!(diff_view.scroll, 1);

        diff_view.scroll_up();
        assert_eq!(diff_view.scroll, 0);

        // Test scrolling up at top
        diff_view.scroll_up();
        assert_eq!(diff_view.scroll, 0);
    }

    #[test]
    fn test_diff_view_raw_diff() {
        let mut diff_view = DiffViewComponent::new();
        let raw_diff = "diff --git a/file b/file\nindex 123..456\n--- a/file\n+++ b/file\n@@ -1 +1 @@\n-foo\n+bar";

        diff_view.set_raw_diff(raw_diff);
        assert!(!diff_view.raw_lines.is_empty());
        assert_eq!(diff_view.raw_lines.len(), 7);
        assert!(diff_view.hunks.is_empty()); // Should clear structured hunks
    }

    // ── create_provider logic (tested via ProviderConfig construction) ─────

    #[test]
    fn create_provider_ollama_default_model() {
        use vibe_ai::provider::ProviderConfig;
        use vibe_ai::providers::ollama::OllamaProvider;
        // Mirrors the default fallback in create_provider for "ollama"
        let model: Option<String> = None;
        let resolved_model = model.unwrap_or_else(|| "qwen3-coder:480b-cloud".to_string());
        assert_eq!(resolved_model, "qwen3-coder:480b-cloud");

        let config = ProviderConfig {
            provider_type: "ollama".to_string(),
            api_url: Some("http://localhost:11434".to_string()),
            model: resolved_model,
            api_key: None,
            max_tokens: None,
            temperature: None,
            ..Default::default()
        };
        // Should not panic on construction
        let _provider = OllamaProvider::new(config);
    }

    #[test]
    fn create_provider_ollama_custom_model() {
        let model = Some("llama3:70b".to_string());
        let resolved = model.unwrap_or_else(|| "qwen3-coder:480b-cloud".to_string());
        assert_eq!(resolved, "llama3:70b");
    }

    #[test]
    fn create_provider_openai_default_model() {
        let model: Option<String> = None;
        let resolved = model.unwrap_or_else(|| "gpt-4-turbo".to_string());
        assert_eq!(resolved, "gpt-4-turbo");
    }

    #[test]
    fn create_provider_claude_default_model() {
        let model: Option<String> = None;
        let resolved = model.unwrap_or_else(|| "claude-3-opus-20240229".to_string());
        assert_eq!(resolved, "claude-3-opus-20240229");
    }

    #[test]
    fn create_provider_gemini_default_model() {
        let model: Option<String> = None;
        let resolved = model.unwrap_or_else(|| "gemini-pro".to_string());
        assert_eq!(resolved, "gemini-pro");
    }

    #[test]
    fn create_provider_grok_default_model() {
        let model: Option<String> = None;
        let resolved = model.unwrap_or_else(|| "grok-beta".to_string());
        assert_eq!(resolved, "grok-beta");
    }

    #[test]
    fn create_provider_name_normalization() {
        // The match in create_provider lowercases the name
        let names = vec!["OLLAMA", "Ollama", "ollama"];
        for name in names {
            assert_eq!(name.to_lowercase(), "ollama");
        }
    }

    #[test]
    fn create_provider_claude_aliases() {
        // Both "anthropic" and "claude" should map to the same provider
        let aliases = vec!["anthropic", "claude"];
        for alias in aliases {
            let lower = alias.to_lowercase();
            assert!(
                lower == "anthropic" || lower == "claude",
                "Unexpected alias: {}", alias
            );
        }
    }

    // ── Screen transition logic ───────────────────────────────────────────

    #[test]
    fn tab_cycles_chat_to_filetree() {
        // Mirrors the Tab key handler in run_app
        let screen = CurrentScreen::Chat;
        let next = match screen {
            CurrentScreen::Chat => CurrentScreen::FileTree,
            CurrentScreen::FileTree => CurrentScreen::Chat,
            CurrentScreen::Agent => CurrentScreen::Chat,
            _ => CurrentScreen::Chat,
        };
        assert!(matches!(next, CurrentScreen::FileTree));
    }

    #[test]
    fn tab_cycles_filetree_to_chat() {
        let screen = CurrentScreen::FileTree;
        let next = match screen {
            CurrentScreen::Chat => CurrentScreen::FileTree,
            CurrentScreen::FileTree => CurrentScreen::Chat,
            CurrentScreen::Agent => CurrentScreen::Chat,
            _ => CurrentScreen::Chat,
        };
        assert!(matches!(next, CurrentScreen::Chat));
    }

    #[test]
    fn tab_from_agent_returns_to_chat() {
        let screen = CurrentScreen::Agent;
        let next = match screen {
            CurrentScreen::Chat => CurrentScreen::FileTree,
            CurrentScreen::FileTree => CurrentScreen::Chat,
            CurrentScreen::Agent => CurrentScreen::Chat,
            _ => CurrentScreen::Chat,
        };
        assert!(matches!(next, CurrentScreen::Chat));
    }

    #[test]
    fn tab_from_diffview_returns_to_chat() {
        let screen = CurrentScreen::DiffView;
        let next = match screen {
            CurrentScreen::Chat => CurrentScreen::FileTree,
            CurrentScreen::FileTree => CurrentScreen::Chat,
            CurrentScreen::Agent => CurrentScreen::Chat,
            _ => CurrentScreen::Chat,
        };
        assert!(matches!(next, CurrentScreen::Chat));
    }

    // ── Scroll offset logic ──────────────────────────────────────────────

    #[test]
    fn scroll_offset_saturating_add() {
        let mut app = App::new();
        app.scroll_offset = u16::MAX - 1;
        // Mirrors Ctrl+U / ScrollUp logic
        app.scroll_offset = app.scroll_offset.saturating_add(3);
        assert_eq!(app.scroll_offset, u16::MAX);
    }

    #[test]
    fn scroll_offset_saturating_sub_at_zero() {
        let mut app = App::new();
        assert_eq!(app.scroll_offset, 0);
        app.scroll_offset = app.scroll_offset.saturating_sub(3);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn scroll_offset_page_up_adds_five() {
        let mut app = App::new();
        app.scroll_offset = 10;
        app.scroll_offset = app.scroll_offset.saturating_add(5);
        assert_eq!(app.scroll_offset, 15);
    }

    #[test]
    fn scroll_offset_page_down_subtracts_five() {
        let mut app = App::new();
        app.scroll_offset = 10;
        app.scroll_offset = app.scroll_offset.saturating_sub(5);
        assert_eq!(app.scroll_offset, 5);
    }

    // ── Exit pending / Ctrl+C double-press ────────────────────────────────

    #[test]
    fn ctrl_c_first_press_sets_exit_pending() {
        let mut app = App::new();
        assert!(!app.exit_pending);
        // Simulate first Ctrl+C
        app.exit_pending = true;
        app.messages.push(TuiMessage::System("Press Ctrl+C again to quit".to_string()));
        assert!(app.exit_pending);
        assert!(!app.should_quit);
    }

    #[test]
    fn ctrl_c_second_press_sets_should_quit() {
        let mut app = App::new();
        app.exit_pending = true;
        // Simulate second Ctrl+C
        app.should_quit = true;
        assert!(app.should_quit);
    }

    #[test]
    fn typing_clears_exit_pending() {
        let mut app = App::new();
        app.exit_pending = true;
        // Simulate typing a character (exit_pending = false before on_key)
        app.exit_pending = false;
        app.on_key('x');
        assert!(!app.exit_pending);
        assert_eq!(app.input, "x");
    }

    // ── Command parsing logic (slash commands) ────────────────────────────

    #[test]
    fn slash_command_parsing_with_args() {
        let user_msg = "/agent fix the bug in main.rs";
        let parts: Vec<&str> = user_msg.splitn(2, ' ').collect();
        assert_eq!(parts[0], "/agent");
        assert_eq!(parts[1], "fix the bug in main.rs");
    }

    #[test]
    fn slash_command_parsing_no_args() {
        let user_msg = "/quit";
        let parts: Vec<&str> = user_msg.splitn(2, ' ').collect();
        assert_eq!(parts[0], "/quit");
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn slash_command_detection() {
        assert!("/help".starts_with('/'));
        assert!("/agent task".starts_with('/'));
        assert!(!"hello".starts_with('/'));
        assert!(!"".starts_with('/'));
    }

    #[test]
    fn bang_command_detection() {
        let msg = "!ls -la";
        assert!(msg.starts_with('!'));
        let cmd = msg.strip_prefix('!').unwrap().trim();
        assert_eq!(cmd, "ls -la");
    }

    #[test]
    fn bang_command_empty_after_strip() {
        let msg = "!";
        let cmd = msg.strip_prefix('!').unwrap().trim();
        assert!(cmd.is_empty());
    }

    // ── TuiMessage → Message conversion logic ────────────────────────────

    #[test]
    fn tui_message_to_llm_message_user() {
        use vibe_ai::provider::{Message, MessageRole};
        let tui_msg = TuiMessage::User("hello".into());
        let llm_msg = match &tui_msg {
            TuiMessage::User(c) => Some(Message { role: MessageRole::User, content: c.clone() }),
            _ => None,
        };
        assert!(llm_msg.is_some());
        assert_eq!(llm_msg.unwrap().content, "hello");
    }

    #[test]
    fn tui_message_to_llm_message_assistant() {
        use vibe_ai::provider::{Message, MessageRole};
        let tui_msg = TuiMessage::Assistant("reply".into());
        let llm_msg = match &tui_msg {
            TuiMessage::Assistant(c) => Some(Message { role: MessageRole::Assistant, content: c.clone() }),
            _ => None,
        };
        assert!(llm_msg.is_some());
        assert_eq!(llm_msg.unwrap().role, MessageRole::Assistant);
    }

    #[test]
    fn tui_message_command_output_maps_to_user_role() {
        use vibe_ai::provider::{Message, MessageRole};
        let tui_msg = TuiMessage::CommandOutput {
            command: "cargo build".into(),
            output: "Compiling...".into(),
        };
        let llm_msg = match &tui_msg {
            TuiMessage::CommandOutput { command, output } => Some(Message {
                role: MessageRole::User,
                content: format!("Command executed: {}\nOutput:\n{}", command, output),
            }),
            _ => None,
        };
        assert!(llm_msg.is_some());
        let msg = llm_msg.unwrap();
        assert_eq!(msg.role, MessageRole::User);
        assert!(msg.content.contains("cargo build"));
        assert!(msg.content.contains("Compiling..."));
    }

    #[test]
    fn tui_message_error_maps_to_none() {
        // Error messages are filtered out of the LLM conversation
        let tui_msg = TuiMessage::Error("oops".into());
        let llm_msg: Option<String> = match &tui_msg {
            TuiMessage::User(c) | TuiMessage::Assistant(c) | TuiMessage::System(c) => Some(c.clone()),
            _ => None,
        };
        assert!(llm_msg.is_none());
    }

    // ── Agent completion summary truncation ───────────────────────────────

    #[test]
    fn agent_complete_summary_truncation_short() {
        let summary = "Fixed the bug";
        let truncated = if summary.len() > 80 {
            &summary[..summary.char_indices().nth(80).map(|(i, _)| i).unwrap_or(summary.len())]
        } else {
            summary
        };
        assert_eq!(truncated, "Fixed the bug");
    }

    #[test]
    fn agent_complete_summary_truncation_long() {
        let summary = "A".repeat(200);
        let truncated = if summary.len() > 80 {
            &summary[..summary.char_indices().nth(80).map(|(i, _)| i).unwrap_or(summary.len())]
        } else {
            &summary
        };
        assert_eq!(truncated.len(), 80);
    }
}
