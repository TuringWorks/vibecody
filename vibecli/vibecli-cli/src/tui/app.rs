
#[derive(Debug, Clone)]
pub enum TuiMessage {
    User(String),
    Assistant(String),
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
}

use crate::tui::components::file_tree::FileTreeComponent;
use crate::tui::components::diff_view::DiffViewComponent;

pub struct App {
    pub current_screen: CurrentScreen,
    pub should_quit: bool,
    pub messages: Vec<TuiMessage>,
    pub input: String,
    pub file_tree: FileTreeComponent,
    pub diff_view: DiffViewComponent,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_screen: CurrentScreen::Chat,
            should_quit: false,
            messages: Vec::new(),
            input: String::new(),
            file_tree: FileTreeComponent::new(),
            diff_view: DiffViewComponent::new(),
        }
    }

    #[allow(dead_code)]
    pub fn on_tick(&mut self) {
        // Handle tick events
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
