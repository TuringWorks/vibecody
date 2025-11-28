use vibecli_core::diff::{DiffEngine, DiffHunk};

pub struct DiffViewComponent {
    pub hunks: Vec<DiffHunk>,
    pub raw_lines: Vec<String>,
    pub scroll: u16,
}

impl DiffViewComponent {
    pub fn new() -> Self {
        Self {
            hunks: Vec::new(),
            raw_lines: Vec::new(),
            scroll: 0,
        }
    }

    pub fn set_diff(&mut self, original: &str, modified: &str) {
        self.hunks = DiffEngine::generate_diff(original, modified);
        self.raw_lines.clear();
        self.scroll = 0;
    }

    pub fn set_raw_diff(&mut self, diff: &str) {
        self.raw_lines = diff.lines().map(|s| s.to_string()).collect();
        self.hunks.clear();
        self.scroll = 0;
    }

    pub fn clear(&mut self) {
        self.hunks.clear();
        self.raw_lines.clear();
        self.scroll = 0;
    }

    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll += 1;
    }
}
