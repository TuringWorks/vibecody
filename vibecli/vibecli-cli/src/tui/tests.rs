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
}
