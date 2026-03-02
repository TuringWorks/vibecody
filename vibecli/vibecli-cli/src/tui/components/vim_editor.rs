//! Vim-like modal text editor component for the VibeCLI TUI.
//!
//! Supports:
//! - **Normal mode**: hjkl/arrows, w/b, 0/$, gg/G, i/a/o/I/A/O, x, dd, yy, p/P, u, /search, n/N, :commands
//! - **Insert mode**: typed text insertion, Esc returns to Normal
//! - **Visual / Visual-line mode**: v/V, y/d operations on selection
//! - **Command mode**: :w :q :wq :noh :set number/nonumber
//! - **Search mode**: / forward search, n/N for next/prev match

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::path::PathBuf;

// ── Modes ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
    VisualLine,
    Command,
    Search,
}

impl VimMode {
    pub fn label(&self) -> &'static str {
        match self {
            VimMode::Normal     => "NORMAL",
            VimMode::Insert     => "INSERT",
            VimMode::Visual     => "VISUAL",
            VimMode::VisualLine => "V-LINE",
            VimMode::Command    => "COMMAND",
            VimMode::Search     => "SEARCH",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            VimMode::Normal     => Color::Blue,
            VimMode::Insert     => Color::Green,
            VimMode::Visual     => Color::Magenta,
            VimMode::VisualLine => Color::Magenta,
            VimMode::Command    => Color::Yellow,
            VimMode::Search     => Color::Cyan,
        }
    }
}

// ── Editor ────────────────────────────────────────────────────────────────────

/// Holds all state for the embedded vim-like editor.
pub struct VimEditorComponent {
    pub mode: VimMode,
    /// File currently being edited.
    pub file_path: Option<PathBuf>,
    /// Buffer: one String per line (never empty — always ≥1 line).
    pub lines: Vec<String>,
    /// (row, col) cursor position, 0-indexed.
    pub cursor: (usize, usize),
    /// First visible line (vertical scroll offset).
    pub scroll_row: usize,
    /// Is the buffer modified since last save?
    pub modified: bool,
    /// Text typed in `:` command mode.
    pub command_buf: String,
    /// Text typed in `/` search mode.
    pub search_buf: String,
    /// Committed search query (highlighted).
    pub last_search: String,
    /// One-key prefix accumulator for two-char commands like `gg`, `dd`, `yy`.
    pub pending_key: Option<char>,
    /// Yank register: lines copied by `yy`/`d`.
    pub yank_reg: Vec<String>,
    /// Single undo level: snapshot of (lines, cursor) before last mutation.
    pub undo_snap: Option<(Vec<String>, (usize, usize))>,
    /// Visual mode anchor (row, col).
    pub visual_start: Option<(usize, usize)>,
    /// Show line numbers.
    pub show_line_numbers: bool,
    /// Status message to display for one render cycle.
    pub status_msg: Option<String>,
    /// Number qualifier accumulator (e.g. `3` before `j`).
    count_buf: String,
}

impl VimEditorComponent {
    pub fn new() -> Self {
        Self {
            mode: VimMode::Normal,
            file_path: None,
            lines: vec![String::new()],
            cursor: (0, 0),
            scroll_row: 0,
            modified: false,
            command_buf: String::new(),
            search_buf: String::new(),
            last_search: String::new(),
            pending_key: None,
            yank_reg: Vec::new(),
            undo_snap: None,
            visual_start: None,
            show_line_numbers: true,
            status_msg: None,
            count_buf: String::new(),
        }
    }

    /// Open a file into the buffer.
    pub fn open(&mut self, path: PathBuf) {
        self.file_path = Some(path.clone());
        self.lines = match std::fs::read_to_string(&path) {
            Ok(content) => {
                let mut ls: Vec<String> = content.lines().map(str::to_owned).collect();
                if ls.is_empty() { ls.push(String::new()); }
                ls
            }
            Err(_) => vec![String::new()],
        };
        self.cursor = (0, 0);
        self.scroll_row = 0;
        self.modified = false;
        self.mode = VimMode::Normal;
        self.status_msg = Some(format!("Opened \"{}\"  {} lines", path.display(), self.lines.len()));
    }

    /// Set buffer content without a file path (e.g. scratch buffer).
    #[allow(dead_code)]
    pub fn set_content(&mut self, content: &str) {
        let mut ls: Vec<String> = content.lines().map(str::to_owned).collect();
        if ls.is_empty() { ls.push(String::new()); }
        self.lines = ls;
        self.cursor = (0, 0);
        self.scroll_row = 0;
        self.modified = false;
    }

    /// Save buffer to its file path. Returns error string on failure.
    pub fn save(&mut self) -> Result<(), String> {
        match &self.file_path {
            None => Err("No file name".to_string()),
            Some(path) => {
                let content = self.lines.join("\n");
                std::fs::write(path, content).map_err(|e| e.to_string())?;
                self.modified = false;
                self.status_msg = Some(format!(
                    "\"{}\" {}L written",
                    path.display(),
                    self.lines.len()
                ));
                Ok(())
            }
        }
    }

    /// True if the user wants to quit cleanly (no unsaved changes or :q! requested).
    #[allow(dead_code)]
    pub fn wants_quit(&self) -> bool {
        !self.modified
    }

    // ── Cursor helpers ────────────────────────────────────────────────────────

    fn clamp_cursor(&mut self) {
        let max_row = self.lines.len().saturating_sub(1);
        if self.cursor.0 > max_row { self.cursor.0 = max_row; }
        let line_len = self.lines[self.cursor.0].len();
        let max_col = if self.mode == VimMode::Insert {
            line_len
        } else {
            line_len.saturating_sub(1)
        };
        if self.cursor.1 > max_col { self.cursor.1 = max_col; }
    }

    fn scroll_to_cursor(&mut self, height: usize) {
        if self.cursor.0 < self.scroll_row {
            self.scroll_row = self.cursor.0;
        } else if self.cursor.0 >= self.scroll_row + height {
            self.scroll_row = self.cursor.0 + 1 - height;
        }
    }

    fn take_count(&mut self) -> usize {
        let n: usize = self.count_buf.parse().unwrap_or(1).max(1);
        self.count_buf.clear();
        n
    }

    // ── Normal-mode motions ───────────────────────────────────────────────────

    fn move_left(&mut self) {
        if self.cursor.1 > 0 {
            let line = &self.lines[self.cursor.0];
            // Find the previous char boundary
            self.cursor.1 = line[..self.cursor.1]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }
    fn move_right(&mut self) {
        let line = &self.lines[self.cursor.0];
        let max = if self.mode == VimMode::Insert { line.len() } else { line.len().saturating_sub(1) };
        if self.cursor.1 < max {
            // Find the next char boundary
            self.cursor.1 = line[self.cursor.1..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor.1 + i)
                .unwrap_or(line.len());
            if self.mode != VimMode::Insert && self.cursor.1 > max {
                self.cursor.1 = max;
            }
        }
    }
    fn move_up(&mut self, n: usize) {
        self.cursor.0 = self.cursor.0.saturating_sub(n);
        self.clamp_cursor();
    }
    fn move_down(&mut self, n: usize) {
        let max = self.lines.len().saturating_sub(1);
        self.cursor.0 = (self.cursor.0 + n).min(max);
        self.clamp_cursor();
    }
    fn move_word_forward(&mut self) {
        let row = self.cursor.0;
        let col = self.cursor.1;
        let line = &self.lines[row];
        let chars: Vec<char> = line.chars().collect();
        let mut i = col + 1;
        while i < chars.len() && chars[i].is_alphanumeric() { i += 1; }
        while i < chars.len() && !chars[i].is_alphanumeric() { i += 1; }
        self.cursor.1 = i.min(chars.len().saturating_sub(1));
    }
    fn move_word_back(&mut self) {
        let col = self.cursor.1;
        let line = &self.lines[self.cursor.0];
        let chars: Vec<char> = line.chars().collect();
        if col == 0 { return; }
        let mut i = col.saturating_sub(1);
        while i > 0 && !chars[i].is_alphanumeric() { i -= 1; }
        while i > 0 && chars[i - 1].is_alphanumeric() { i -= 1; }
        self.cursor.1 = i;
    }

    // ── Mutation helpers (all save undo snap) ─────────────────────────────────

    fn save_undo(&mut self) {
        self.undo_snap = Some((self.lines.clone(), self.cursor));
    }

    fn delete_line(&mut self, row: usize) -> String {
        self.save_undo();
        if self.lines.len() == 1 {
            let removed = self.lines[0].clone();
            self.lines[0].clear();
            self.modified = true;
            return removed;
        }
        let removed = self.lines.remove(row);
        self.modified = true;
        // clamp row
        if self.cursor.0 >= self.lines.len() {
            self.cursor.0 = self.lines.len().saturating_sub(1);
        }
        removed
    }

    fn yank_line(&mut self, row: usize) {
        let line = self.lines[row].clone();
        self.yank_reg = vec![line];
        self.status_msg = Some("1 line yanked".to_string());
    }

    fn paste_after(&mut self) {
        if self.yank_reg.is_empty() { return; }
        self.save_undo();
        let insert_at = self.cursor.0 + 1;
        for (i, line) in self.yank_reg.iter().enumerate() {
            self.lines.insert(insert_at + i, line.clone());
        }
        self.cursor.0 = insert_at;
        self.cursor.1 = 0;
        self.modified = true;
    }

    fn paste_before(&mut self) {
        if self.yank_reg.is_empty() { return; }
        self.save_undo();
        let insert_at = self.cursor.0;
        for (i, line) in self.yank_reg.iter().enumerate() {
            self.lines.insert(insert_at + i, line.clone());
        }
        self.cursor.0 = insert_at;
        self.cursor.1 = 0;
        self.modified = true;
    }

    // ── Search ────────────────────────────────────────────────────────────────

    /// Find all matches of `pattern` (case-insensitive).
    fn find_matches(&self, pattern: &str) -> Vec<(usize, usize)> {
        if pattern.is_empty() { return Vec::new(); }
        let pat = pattern.to_lowercase();
        let mut matches = Vec::new();
        for (row, line) in self.lines.iter().enumerate() {
            let lower = line.to_lowercase();
            let mut start = 0;
            while let Some(idx) = lower[start..].find(&pat) {
                matches.push((row, start + idx));
                start += idx + pat.len();
            }
        }
        matches
    }

    pub fn search_next(&mut self) {
        if self.last_search.is_empty() { return; }
        let matches = self.find_matches(&self.last_search);
        if matches.is_empty() {
            self.status_msg = Some(format!("Pattern not found: {}", self.last_search));
            return;
        }
        let cur = self.cursor;
        let next = matches.iter()
            .find(|&&(r, c)| r > cur.0 || (r == cur.0 && c > cur.1))
            .or_else(|| matches.first())
            .copied();
        if let Some((r, c)) = next {
            self.cursor = (r, c);
        }
    }

    pub fn search_prev(&mut self) {
        if self.last_search.is_empty() { return; }
        let matches = self.find_matches(&self.last_search);
        if matches.is_empty() {
            self.status_msg = Some(format!("Pattern not found: {}", self.last_search));
            return;
        }
        let cur = self.cursor;
        let prev = matches.iter().rev()
            .find(|&&(r, c)| r < cur.0 || (r == cur.0 && c < cur.1))
            .or_else(|| matches.last())
            .copied();
        if let Some((r, c)) = prev {
            self.cursor = (r, c);
        }
    }

    // ── Key handler entry point ───────────────────────────────────────────────

    /// Process a key event. Returns `true` if the editor wants to close.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent, viewport_height: u16) -> bool {
        let vp = viewport_height as usize;

        match self.mode {
            VimMode::Insert  => self.handle_insert(key),
            VimMode::Command => self.handle_command_input(key),
            VimMode::Search  => self.handle_search_input(key),
            VimMode::Visual | VimMode::VisualLine => {
                let quit = self.handle_visual(key, vp);
                if quit { return true; }
            }
            VimMode::Normal => {
                let quit = self.handle_normal(key, vp);
                if quit { return true; }
            }
        }

        self.clamp_cursor();
        self.scroll_to_cursor(vp.saturating_sub(2)); // subtract status bar rows
        false
    }

    // ── Normal mode ───────────────────────────────────────────────────────────

    fn handle_normal(&mut self, key: crossterm::event::KeyEvent, vp: usize) -> bool {
        use crossterm::event::KeyCode;

        // Handle pending two-char sequences
        if let Some(first) = self.pending_key.take() {
            return self.handle_normal_chord(first, key);
        }

        match key.code {
            // Count prefix
            KeyCode::Char(c @ '0'..='9') if !(c == '0' && self.count_buf.is_empty()) => {
                self.count_buf.push(c);
                return false;
            }

            // Motions
            KeyCode::Char('h') | KeyCode::Left  => { let n = self.take_count(); for _ in 0..n { self.move_left(); } }
            KeyCode::Char('l') | KeyCode::Right => { let n = self.take_count(); for _ in 0..n { self.move_right(); } }
            KeyCode::Char('j') | KeyCode::Down  => { let n = self.take_count(); self.move_down(n); }
            KeyCode::Char('k') | KeyCode::Up    => { let n = self.take_count(); self.move_up(n); }
            KeyCode::Char('w') => { let n = self.take_count(); for _ in 0..n { self.move_word_forward(); } }
            KeyCode::Char('0') => { self.cursor.1 = 0; self.count_buf.clear(); }
            KeyCode::Char('$') => {
                let len = self.lines[self.cursor.0].len().saturating_sub(1);
                self.cursor.1 = len;
                self.count_buf.clear();
            }
            KeyCode::Char('G') => {
                let n = if self.count_buf.is_empty() {
                    self.lines.len().saturating_sub(1)
                } else {
                    let c = self.count_buf.parse::<usize>().unwrap_or(1).saturating_sub(1);
                    self.count_buf.clear();
                    c.min(self.lines.len().saturating_sub(1))
                };
                self.cursor.0 = n;
                self.clamp_cursor();
            }
            KeyCode::Char('g') => {
                self.count_buf.clear();
                self.pending_key = Some('g');
            }

            // Page scroll (must come before unguarded Char('b')/Char('f'))
            KeyCode::Char('f') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                let n = self.take_count();
                self.move_down(vp.saturating_sub(2) * n);
            }
            KeyCode::Char('b') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                let n = self.take_count();
                self.move_up(vp.saturating_sub(2) * n);
            }

            // Word back
            KeyCode::Char('b') => { let n = self.take_count(); for _ in 0..n { self.move_word_back(); } }

            // Insert mode entry
            KeyCode::Char('i') => { self.count_buf.clear(); self.mode = VimMode::Insert; }
            KeyCode::Char('a') => {
                self.count_buf.clear();
                let len = self.lines[self.cursor.0].len();
                if self.cursor.1 < len { self.cursor.1 += 1; }
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('I') => {
                self.count_buf.clear();
                self.cursor.1 = 0;
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('A') => {
                self.count_buf.clear();
                self.cursor.1 = self.lines[self.cursor.0].len();
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('o') => {
                self.count_buf.clear();
                self.save_undo();
                let row = self.cursor.0 + 1;
                self.lines.insert(row, String::new());
                self.cursor.0 = row;
                self.cursor.1 = 0;
                self.modified = true;
                self.mode = VimMode::Insert;
            }
            KeyCode::Char('O') => {
                self.count_buf.clear();
                self.save_undo();
                let row = self.cursor.0;
                self.lines.insert(row, String::new());
                self.cursor.1 = 0;
                self.modified = true;
                self.mode = VimMode::Insert;
            }

            // Delete char
            KeyCode::Char('x') => {
                self.save_undo();
                let col = self.cursor.1;
                let line = &mut self.lines[self.cursor.0];
                if col < line.len() && line.is_char_boundary(col) {
                    line.remove(col);
                    self.modified = true;
                }
                self.clamp_cursor();
                self.count_buf.clear();
            }

            // Yank / delete / paste chords (dd, yy)
            KeyCode::Char('d') => { self.count_buf.clear(); self.pending_key = Some('d'); }
            KeyCode::Char('y') => { self.count_buf.clear(); self.pending_key = Some('y'); }
            KeyCode::Char('p') => { self.count_buf.clear(); self.paste_after(); }
            KeyCode::Char('P') => { self.count_buf.clear(); self.paste_before(); }

            // Undo
            KeyCode::Char('u') => {
                self.count_buf.clear();
                if let Some((lines, cursor)) = self.undo_snap.take() {
                    self.lines = lines;
                    self.cursor = cursor;
                    self.modified = true;
                    self.status_msg = Some("1 change undone".to_string());
                } else {
                    self.status_msg = Some("Already at oldest change".to_string());
                }
            }

            // Visual mode
            KeyCode::Char('v') => { self.count_buf.clear(); self.mode = VimMode::Visual; self.visual_start = Some(self.cursor); }
            KeyCode::Char('V') => { self.count_buf.clear(); self.mode = VimMode::VisualLine; self.visual_start = Some(self.cursor); }

            // Search
            KeyCode::Char('/') => {
                self.count_buf.clear();
                self.search_buf.clear();
                self.mode = VimMode::Search;
            }
            KeyCode::Char('n') => { self.count_buf.clear(); self.search_next(); }
            KeyCode::Char('N') => { self.count_buf.clear(); self.search_prev(); }

            // Command mode
            KeyCode::Char(':') => { self.count_buf.clear(); self.command_buf.clear(); self.mode = VimMode::Command; }

            // Esc
            KeyCode::Esc => { self.count_buf.clear(); self.status_msg = None; }

            _ => { self.count_buf.clear(); }
        }
        false
    }

    fn handle_normal_chord(&mut self, first: char, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::KeyCode;
        match (first, key.code) {
            ('g', KeyCode::Char('g')) => {
                self.cursor = (0, 0);
            }
            ('d', KeyCode::Char('d')) => {
                let row = self.cursor.0;
                let removed = self.delete_line(row);
                self.yank_reg = vec![removed];
                self.status_msg = Some("1 line deleted".to_string());
            }
            ('y', KeyCode::Char('y')) => {
                let row = self.cursor.0;
                self.yank_line(row);
            }
            _ => {
                self.status_msg = Some(format!("Unknown sequence: {}{:?}", first, key.code));
            }
        }
        false
    }

    // ── Insert mode ───────────────────────────────────────────────────────────

    fn handle_insert(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode};
        match key.code {
            KeyCode::Esc => {
                self.mode = VimMode::Normal;
                // Adjust col: in normal mode col can't exceed len-1
                if !self.lines[self.cursor.0].is_empty() && self.cursor.1 > 0 {
                    self.cursor.1 -= 1;
                }
            }
            KeyCode::Enter => {
                self.save_undo();
                let row = self.cursor.0;
                let col = self.cursor.1;
                let rest = self.lines[row].split_off(col);
                self.cursor.0 += 1;
                self.cursor.1 = 0;
                self.lines.insert(self.cursor.0, rest);
                self.modified = true;
            }
            KeyCode::Backspace => {
                if self.cursor.1 > 0 {
                    // Find the char boundary before cursor
                    let line = &self.lines[self.cursor.0];
                    let prev_boundary = line[..self.cursor.1]
                        .char_indices()
                        .next_back()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    let removed = self.lines[self.cursor.0].remove(prev_boundary);
                    self.cursor.1 -= removed.len_utf8();
                    self.modified = true;
                } else if self.cursor.0 > 0 {
                    // Merge with previous line
                    self.save_undo();
                    let row = self.cursor.0;
                    let rest = self.lines.remove(row);
                    self.cursor.0 -= 1;
                    self.cursor.1 = self.lines[self.cursor.0].len();
                    self.lines[self.cursor.0].push_str(&rest);
                    self.modified = true;
                }
            }
            KeyCode::Delete => {
                let row = self.cursor.0;
                let col = self.cursor.1;
                let line_len = self.lines[row].len();
                if col < line_len && self.lines[row].is_char_boundary(col) {
                    self.lines[row].remove(col);
                    self.modified = true;
                } else if row + 1 < self.lines.len() {
                    let next = self.lines.remove(row + 1);
                    self.lines[row].push_str(&next);
                    self.modified = true;
                }
            }
            KeyCode::Tab => {
                let row = self.cursor.0;
                let col = self.cursor.1;
                if self.lines[row].is_char_boundary(col) {
                    self.lines[row].insert_str(col, "    ");
                    self.cursor.1 += 4;
                    self.modified = true;
                }
            }
            KeyCode::Char(c) if !key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                let row = self.cursor.0;
                let col = self.cursor.1;
                self.lines[row].insert(col, c);
                self.cursor.1 += c.len_utf8();
                self.modified = true;
            }
            _ => {}
        }
    }

    // ── Command mode ──────────────────────────────────────────────────────────

    fn handle_command_input(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => { self.mode = VimMode::Normal; self.command_buf.clear(); }
            KeyCode::Backspace => { self.command_buf.pop(); }
            KeyCode::Enter => {
                let cmd = self.command_buf.trim().to_string();
                self.command_buf.clear();
                self.mode = VimMode::Normal;
                self.execute_command(&cmd);
            }
            KeyCode::Char(c) => { self.command_buf.push(c); }
            _ => {}
        }
    }

    fn execute_command(&mut self, cmd: &str) {
        match cmd {
            "w" | "write" => {
                if let Err(e) = self.save() {
                    self.status_msg = Some(format!("E: {}", e));
                }
            }
            "q" | "quit" => {
                if self.modified {
                    self.status_msg = Some("Unsaved changes! Use :q! to force quit or :wq to save".to_string());
                } else {
                    // Signal quit via a flag — caller checks wants_quit()
                    self.modified = false; // no-op, quit handled by caller
                    self.status_msg = Some("Use :q! to confirm".to_string());
                }
            }
            "q!" => {
                self.modified = false; // caller sees wants_quit() = true
            }
            "wq" | "x" => {
                let _ = self.save();
                self.modified = false;
            }
            "noh" | "nohlsearch" => {
                self.last_search.clear();
                self.status_msg = Some("Search highlight cleared".to_string());
            }
            "set number" | "set nu" => {
                self.show_line_numbers = true;
                self.status_msg = Some("Line numbers on".to_string());
            }
            "set nonumber" | "set nonu" => {
                self.show_line_numbers = false;
                self.status_msg = Some("Line numbers off".to_string());
            }
            other => {
                self.status_msg = Some(format!("Unknown command: :{}", other));
            }
        }
    }

    // ── Search mode ───────────────────────────────────────────────────────────

    fn handle_search_input(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => { self.mode = VimMode::Normal; self.search_buf.clear(); }
            KeyCode::Backspace => { self.search_buf.pop(); }
            KeyCode::Enter => {
                self.last_search = self.search_buf.clone();
                self.search_buf.clear();
                self.mode = VimMode::Normal;
                if self.last_search.is_empty() { return; }
                let matches = self.find_matches(&self.last_search);
                let cur = self.cursor;
                if let Some(&(r, c)) = matches.iter()
                    .find(|&&(row, col)| row > cur.0 || (row == cur.0 && col > cur.1))
                    .or_else(|| matches.first())
                {
                    self.cursor = (r, c);
                } else {
                    self.status_msg = Some(format!("Pattern not found: {}", self.last_search));
                }
            }
            KeyCode::Char(c) => { self.search_buf.push(c); }
            _ => {}
        }
    }

    // ── Visual mode ───────────────────────────────────────────────────────────

    fn handle_visual(&mut self, key: crossterm::event::KeyEvent, vp: usize) -> bool {
        use crossterm::event::KeyCode;
        let is_visual_line = self.mode == VimMode::VisualLine;
        match key.code {
            KeyCode::Esc | KeyCode::Char('v') | KeyCode::Char('V') => {
                self.mode = VimMode::Normal;
                self.visual_start = None;
            }
            KeyCode::Char('h') | KeyCode::Left  => self.move_left(),
            KeyCode::Char('l') | KeyCode::Right => self.move_right(),
            KeyCode::Char('j') | KeyCode::Down  => self.move_down(1),
            KeyCode::Char('k') | KeyCode::Up    => self.move_up(1),
            KeyCode::Char('y') => {
                // Yank selection
                if let Some(start) = self.visual_start {
                    let (r1, r2) = if start.0 <= self.cursor.0 {
                        (start.0, self.cursor.0)
                    } else {
                        (self.cursor.0, start.0)
                    };
                    self.yank_reg = self.lines[r1..=r2].to_vec();
                    self.status_msg = Some(format!("{} lines yanked", r2 - r1 + 1));
                }
                self.mode = VimMode::Normal;
                self.visual_start = None;
            }
            KeyCode::Char('d') | KeyCode::Char('x') => {
                // Delete selection
                if let Some(start) = self.visual_start.take() {
                    let (r1, r2) = if start.0 <= self.cursor.0 {
                        (start.0, self.cursor.0)
                    } else {
                        (self.cursor.0, start.0)
                    };
                    self.save_undo();
                    self.yank_reg = self.lines[r1..=r2].to_vec();
                    let count = r2 - r1 + 1;
                    self.lines.drain(r1..=r2);
                    if self.lines.is_empty() { self.lines.push(String::new()); }
                    self.cursor.0 = r1.min(self.lines.len().saturating_sub(1));
                    self.cursor.1 = 0;
                    self.modified = true;
                    self.status_msg = Some(format!("{} lines deleted", count));
                }
                self.mode = VimMode::Normal;
            }
            _ => {}
        }
        let _ = (is_visual_line, vp);
        false
    }

    // ── Rendering ─────────────────────────────────────────────────────────────

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let inner_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(3), // space for border + status bar
        };

        // Gutter width
        let gutter_w: u16 = if self.show_line_numbers {
            let digits = self.lines.len().to_string().len() as u16;
            digits + 2 // space + number + space
        } else {
            0
        };
        let text_w = inner_area.width.saturating_sub(gutter_w) as usize;

        // Build text lines to render
        let visible_height = inner_area.height as usize;
        let search_pat = if self.last_search.is_empty() { None } else { Some(self.last_search.to_lowercase()) };

        let mut text_lines: Vec<Line> = Vec::with_capacity(visible_height);
        for screen_row in 0..visible_height {
            let buf_row = self.scroll_row + screen_row;
            if buf_row >= self.lines.len() {
                // Tilde for empty rows (like vim)
                text_lines.push(Line::from(vec![
                    Span::styled("~", Style::default().fg(Color::DarkGray)),
                ]));
                continue;
            }

            let line_str = &self.lines[buf_row];
            let is_cursor_row = buf_row == self.cursor.0;

            let mut spans: Vec<Span> = Vec::new();

            // Line number gutter
            if self.show_line_numbers {
                let num = format!("{:>width$} ", buf_row + 1, width = (gutter_w as usize).saturating_sub(1));
                let gutter_style = if is_cursor_row {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                spans.push(Span::styled(num, gutter_style));
            }

            // Visual selection highlight range
            let (vis_lo, vis_hi) = self.visual_range_for_row(buf_row);

            // Render line content with cursor, search, visual highlights
            let chars: Vec<char> = line_str.chars().collect();
            let mut col = 0usize;
            while col <= chars.len() && col < text_w + self.cursor.1.saturating_sub(text_w).max(0) {
                if col == chars.len() {
                    // End-of-line cursor in insert mode
                    if is_cursor_row && self.cursor.1 == col && (self.mode == VimMode::Insert || self.mode == VimMode::Command) {
                        spans.push(Span::styled(" ", Style::default().bg(Color::White).fg(Color::Black)));
                    }
                    break;
                }
                let ch = chars[col];
                let is_cursor = is_cursor_row && col == self.cursor.1;
                let in_visual = vis_lo.map_or(false, |lo| col >= lo) && vis_hi.map_or(false, |hi| col <= hi);
                let in_search = search_pat.as_ref().map_or(false, |pat| {
                    // Use char indices for correct multi-byte UTF-8 handling
                    let lower_chars: Vec<char> = line_str.to_lowercase().chars().collect();
                    let pat_chars: Vec<char> = pat.chars().collect();
                    if pat_chars.is_empty() { return false; }
                    let mut m = false;
                    let mut s = 0usize;
                    while s + pat_chars.len() <= lower_chars.len() {
                        if lower_chars[s..s + pat_chars.len()] == pat_chars[..] {
                            if col >= s && col < s + pat_chars.len() { m = true; break; }
                            s += 1;
                        } else {
                            s += 1;
                        }
                    }
                    m
                });

                let style = if is_cursor {
                    Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD)
                } else if in_visual {
                    Style::default().bg(Color::Blue).fg(Color::White)
                } else if in_search {
                    Style::default().bg(Color::Yellow).fg(Color::Black)
                } else {
                    Style::default().fg(Color::White)
                };

                spans.push(Span::styled(ch.to_string(), style));
                col += 1;
            }

            text_lines.push(Line::from(spans));
        }

        // Border and file title
        let title = {
            let fname = self.file_path.as_ref()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("[No Name]");
            let modified = if self.modified { " [+]" } else { "" };
            format!(" {}{} ", fname, modified)
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        f.render_widget(block, area);

        let paragraph = Paragraph::new(text_lines);
        f.render_widget(paragraph, inner_area);

        // ── Status bar ────────────────────────────────────────────────────────
        let status_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(2),
            width: area.width,
            height: 1,
        };
        let cmd_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: 1,
        };

        // Mode indicator + position
        let mode_label = self.mode.label();
        let mode_color = self.mode.color();
        let pos = format!("{}:{}", self.cursor.0 + 1, self.cursor.1 + 1);
        let pct = if self.lines.len() <= 1 { "All".to_string() } else {
            format!("{}%", 100 * self.cursor.0 / self.lines.len().saturating_sub(1))
        };
        let status_text = format!(" {:6}  {}  {}  {}  ", mode_label, pos, pct,
            self.file_path.as_ref().and_then(|p| p.file_name()).and_then(|n| n.to_str()).unwrap_or(""));
        let status_line = Line::from(vec![
            Span::styled(format!(" {} ", mode_label), Style::default().fg(Color::Black).bg(mode_color).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("  {}  {}  ", pos, pct),
                Style::default().fg(Color::DarkGray).bg(Color::Reset),
            ),
        ]);
        let _ = status_text;
        f.render_widget(Paragraph::new(status_line), status_area);

        // Command / search line
        let cmd_line = match self.mode {
            VimMode::Command => Line::from(vec![
                Span::raw(":"),
                Span::raw(self.command_buf.clone()),
                Span::styled("█", Style::default().fg(Color::White)),
            ]),
            VimMode::Search => Line::from(vec![
                Span::raw("/"),
                Span::raw(self.search_buf.clone()),
                Span::styled("█", Style::default().fg(Color::Cyan)),
            ]),
            _ => {
                if let Some(ref msg) = self.status_msg {
                    Line::from(Span::styled(msg.clone(), Style::default().fg(Color::Yellow)))
                } else {
                    Line::from("")
                }
            }
        };
        f.render_widget(Paragraph::new(cmd_line), cmd_area);
    }

    // ── Visual range helper ───────────────────────────────────────────────────

    fn visual_range_for_row(&self, row: usize) -> (Option<usize>, Option<usize>) {
        match (&self.mode, self.visual_start) {
            (VimMode::VisualLine, Some(start)) => {
                let (r1, r2) = if start.0 <= self.cursor.0 { (start.0, self.cursor.0) } else { (self.cursor.0, start.0) };
                if row >= r1 && row <= r2 { (Some(0), Some(usize::MAX)) } else { (None, None) }
            }
            (VimMode::Visual, Some(start)) if start.0 == self.cursor.0 && row == self.cursor.0 => {
                let (c1, c2) = if start.1 <= self.cursor.1 { (start.1, self.cursor.1) } else { (self.cursor.1, start.1) };
                (Some(c1), Some(c2))
            }
            (VimMode::Visual, Some(start)) => {
                let (r1, r2) = if start.0 <= self.cursor.0 { (start.0, self.cursor.0) } else { (self.cursor.0, start.0) };
                if row < r1 || row > r2 { return (None, None); }
                if row == r1 && r1 != r2 {
                    let c = if start.0 <= self.cursor.0 { start.1 } else { self.cursor.1 };
                    (Some(c), Some(usize::MAX))
                } else if row == r2 && r1 != r2 {
                    let c = if start.0 <= self.cursor.0 { self.cursor.1 } else { start.1 };
                    (Some(0), Some(c))
                } else {
                    (Some(0), Some(usize::MAX))
                }
            }
            _ => (None, None),
        }
    }

    /// Current line count.
    #[allow(dead_code)]
    pub fn line_count(&self) -> usize { self.lines.len() }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[allow(dead_code)]
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn char_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn make_editor(content: &str) -> VimEditorComponent {
        let mut ed = VimEditorComponent::new();
        ed.set_content(content);
        ed
    }

    #[test]
    fn test_insert_char() {
        let mut ed = make_editor("hello");
        ed.mode = VimMode::Insert;
        ed.cursor = (0, 5);
        ed.handle_key(char_key('!'), 24);
        assert_eq!(ed.lines[0], "hello!");
    }

    #[test]
    fn test_delete_char_x() {
        let mut ed = make_editor("hello");
        ed.cursor = (0, 0);
        ed.handle_key(char_key('x'), 24);
        assert_eq!(ed.lines[0], "ello");
    }

    #[test]
    fn test_dd_deletes_line() {
        let mut ed = make_editor("line1\nline2\nline3");
        ed.cursor = (1, 0);
        ed.handle_key(char_key('d'), 24);
        ed.handle_key(char_key('d'), 24);
        assert_eq!(ed.lines.len(), 2);
        assert_eq!(ed.lines[1], "line3");
    }

    #[test]
    fn test_yy_paste() {
        let mut ed = make_editor("line1\nline2");
        ed.cursor = (0, 0);
        ed.handle_key(char_key('y'), 24);
        ed.handle_key(char_key('y'), 24);
        ed.handle_key(char_key('p'), 24);
        assert_eq!(ed.lines.len(), 3);
        assert_eq!(ed.lines[1], "line1");
    }

    #[test]
    fn test_undo() {
        let mut ed = make_editor("hello");
        ed.cursor = (0, 0);
        ed.handle_key(char_key('d'), 24);
        ed.handle_key(char_key('d'), 24);
        assert_eq!(ed.lines[0], "");
        ed.handle_key(char_key('u'), 24);
        assert_eq!(ed.lines[0], "hello");
    }

    #[test]
    fn test_search_find() {
        let mut ed = make_editor("hello world\nfoo bar\nhello again");
        ed.last_search = "hello".to_string();
        ed.cursor = (0, 0);
        ed.search_next();
        assert_eq!(ed.cursor.0, 2); // jumps to second "hello"
    }

    #[test]
    fn test_gg_goes_to_top() {
        let mut ed = make_editor("a\nb\nc");
        ed.cursor = (2, 0);
        ed.handle_key(char_key('g'), 24);
        ed.handle_key(char_key('g'), 24);
        assert_eq!(ed.cursor.0, 0);
    }

    #[test]
    fn test_o_insert_line_below() {
        let mut ed = make_editor("first\nlast");
        ed.cursor = (0, 0);
        ed.handle_key(char_key('o'), 24);
        assert_eq!(ed.mode, VimMode::Insert);
        assert_eq!(ed.lines.len(), 3);
        assert_eq!(ed.cursor.0, 1);
    }

    #[test]
    fn test_save_to_tempfile() {
        let f = tempfile::NamedTempFile::new().unwrap();
        let path = f.path().to_owned();
        let mut ed = make_editor("test content");
        ed.file_path = Some(path.clone());
        ed.save().unwrap();
        let read_back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read_back, "test content");
    }
}
