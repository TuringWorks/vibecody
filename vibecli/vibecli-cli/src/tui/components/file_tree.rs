use std::path::PathBuf;
use std::fs;

pub struct FileTreeComponent {
    pub current_dir: PathBuf,
    pub items: Vec<PathBuf>,
    pub selected_index: usize,
}

impl FileTreeComponent {
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut component = Self {
            current_dir,
            items: Vec::new(),
            selected_index: 0,
        };
        component.refresh();
        component
    }

    pub fn refresh(&mut self) {
        self.items.clear();
        if let Ok(entries) = fs::read_dir(&self.current_dir) {
            for entry in entries.flatten() {
                self.items.push(entry.path());
            }
        }
        self.items.sort();
    }

    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.items.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.items.len() - 1;
            }
        }
    }

    pub fn enter(&mut self) -> Option<PathBuf> {
        if let Some(path) = self.items.get(self.selected_index) {
            if path.is_dir() {
                self.current_dir = path.clone();
                self.selected_index = 0;
                self.refresh();
                None
            } else {
                Some(path.clone())
            }
        } else {
            None
        }
    }

    pub fn go_up(&mut self) {
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.selected_index = 0;
            self.refresh();
        }
    }
}
