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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a FileTreeComponent with controlled items, bypassing fs::read_dir.
    fn make_tree(items: Vec<PathBuf>) -> FileTreeComponent {
        FileTreeComponent {
            current_dir: PathBuf::from("/tmp"),
            items,
            selected_index: 0,
        }
    }

    // ── next ────────────────────────────────────────────────────────────────

    #[test]
    fn next_wraps_around() {
        let mut ft = make_tree(vec![
            PathBuf::from("a"),
            PathBuf::from("b"),
            PathBuf::from("c"),
        ]);
        ft.selected_index = 2;
        ft.next();
        assert_eq!(ft.selected_index, 0);
    }

    #[test]
    fn next_increments() {
        let mut ft = make_tree(vec![PathBuf::from("a"), PathBuf::from("b")]);
        ft.next();
        assert_eq!(ft.selected_index, 1);
    }

    #[test]
    fn next_on_empty_is_noop() {
        let mut ft = make_tree(vec![]);
        ft.next();
        assert_eq!(ft.selected_index, 0);
    }

    // ── previous ────────────────────────────────────────────────────────────

    #[test]
    fn previous_wraps_to_end() {
        let mut ft = make_tree(vec![
            PathBuf::from("a"),
            PathBuf::from("b"),
            PathBuf::from("c"),
        ]);
        ft.selected_index = 0;
        ft.previous();
        assert_eq!(ft.selected_index, 2);
    }

    #[test]
    fn previous_decrements() {
        let mut ft = make_tree(vec![PathBuf::from("a"), PathBuf::from("b")]);
        ft.selected_index = 1;
        ft.previous();
        assert_eq!(ft.selected_index, 0);
    }

    #[test]
    fn previous_on_empty_is_noop() {
        let mut ft = make_tree(vec![]);
        ft.previous();
        assert_eq!(ft.selected_index, 0);
    }

    // ── enter ───────────────────────────────────────────────────────────────

    #[test]
    fn enter_on_file_returns_path() {
        // Use a real file that exists
        let mut ft = make_tree(vec![]);
        let temp = tempfile::NamedTempFile::new().unwrap();
        ft.items = vec![temp.path().to_path_buf()];
        ft.selected_index = 0;
        let result = ft.enter();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp.path().to_path_buf());
    }

    #[test]
    fn enter_on_directory_returns_none_and_changes_dir() {
        let tmpdir = tempfile::tempdir().unwrap();
        let mut ft = make_tree(vec![tmpdir.path().to_path_buf()]);
        ft.selected_index = 0;
        let result = ft.enter();
        assert!(result.is_none());
        assert_eq!(ft.current_dir, tmpdir.path());
        assert_eq!(ft.selected_index, 0);
    }

    #[test]
    fn enter_out_of_bounds_returns_none() {
        let mut ft = make_tree(vec![]);
        ft.selected_index = 5;
        let result = ft.enter();
        assert!(result.is_none());
    }

    // ── refresh ─────────────────────────────────────────────────────────────

    #[test]
    fn refresh_populates_items_from_dir() {
        let tmpdir = tempfile::tempdir().unwrap();
        std::fs::write(tmpdir.path().join("alpha.txt"), "").unwrap();
        std::fs::write(tmpdir.path().join("beta.txt"), "").unwrap();

        let mut ft = FileTreeComponent {
            current_dir: tmpdir.path().to_path_buf(),
            items: Vec::new(),
            selected_index: 0,
        };
        ft.refresh();
        assert_eq!(ft.items.len(), 2);
        // Items should be sorted
        assert!(ft.items[0] < ft.items[1]);
    }

    // ── go_up ───────────────────────────────────────────────────────────────

    #[test]
    fn go_up_changes_to_parent() {
        let tmpdir = tempfile::tempdir().unwrap();
        let child = tmpdir.path().join("subdir");
        std::fs::create_dir(&child).unwrap();
        let mut ft = FileTreeComponent {
            current_dir: child.clone(),
            items: Vec::new(),
            selected_index: 3,
        };
        ft.go_up();
        assert_eq!(ft.current_dir, tmpdir.path());
        assert_eq!(ft.selected_index, 0);
    }
}
