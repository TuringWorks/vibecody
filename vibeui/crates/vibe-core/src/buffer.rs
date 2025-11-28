//! Text buffer implementation using rope data structure

use anyhow::Result;
use ropey::Rope;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a cursor position in the text
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Represents a text selection range
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// A cursor in the text buffer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    pub position: Position,
    pub selection: Option<Range>,
}

/// Text buffer backed by a rope data structure
pub struct TextBuffer {
    rope: Rope,
    file_path: Option<PathBuf>,
    modified: bool,
    cursors: Vec<Cursor>,
    undo_stack: Vec<Edit>,
    redo_stack: Vec<Edit>,
}

/// Represents an edit operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Edit {
    Insert {
        position: Position,
        text: String,
    },
    Delete {
        range: Range,
        deleted_text: String,
    },
    Batch(Vec<Edit>),
}

impl TextBuffer {
    /// Create a new empty text buffer
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            file_path: None,
            modified: false,
            cursors: vec![Cursor {
                position: Position::new(0, 0),
                selection: None,
            }],
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Create a text buffer from a string
    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            file_path: None,
            modified: false,
            cursors: vec![Cursor {
                position: Position::new(0, 0),
                selection: None,
            }],
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Load a text buffer from a file
    pub fn from_file(path: PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        Ok(Self {
            rope: Rope::from_str(&content),
            file_path: Some(path),
            modified: false,
            cursors: vec![Cursor {
                position: Position::new(0, 0),
                selection: None,
            }],
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        })
    }

    /// Get the file path associated with this buffer
    pub fn file_path(&self) -> Option<&PathBuf> {
        self.file_path.as_ref()
    }

    /// Check if the buffer has been modified
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Get the total number of lines
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// Get the length of a specific line
    pub fn line_len(&self, line: usize) -> usize {
        if line >= self.line_count() {
            return 0;
        }
        self.rope.line(line).len_chars()
    }

    /// Get the content of a specific line
    pub fn line(&self, line: usize) -> Option<String> {
        if line >= self.line_count() {
            return None;
        }
        Some(self.rope.line(line).to_string())
    }

    /// Get the entire text content
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Convert position to char index
    fn position_to_char(&self, pos: Position) -> usize {
        if pos.line >= self.line_count() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(pos.line);
        let line_len = self.line_len(pos.line);
        line_start + pos.column.min(line_len)
    }

    /// Convert char index to position
    fn char_to_position(&self, char_idx: usize) -> Position {
        let char_idx = char_idx.min(self.rope.len_chars());
        let line = self.rope.char_to_line(char_idx);
        let line_start = self.rope.line_to_char(line);
        let column = char_idx - line_start;
        Position::new(line, column)
    }

    /// Insert text at a position
    pub fn insert(&mut self, position: Position, text: &str) -> Result<()> {
        let char_idx = self.position_to_char(position);
        self.rope.insert(char_idx, text);
        
        // Record edit for undo
        self.undo_stack.push(Edit::Insert {
            position,
            text: text.to_string(),
        });
        self.redo_stack.clear();
        
        self.modified = true;
        Ok(())
    }

    /// Delete text in a range
    pub fn delete(&mut self, range: Range) -> Result<()> {
        let start_char = self.position_to_char(range.start);
        let end_char = self.position_to_char(range.end);
        
        if start_char >= end_char {
            return Ok(());
        }
        
        let deleted_text = self.rope.slice(start_char..end_char).to_string();
        self.rope.remove(start_char..end_char);
        
        // Record edit for undo
        self.undo_stack.push(Edit::Delete {
            range,
            deleted_text,
        });
        self.redo_stack.clear();
        
        self.modified = true;
        Ok(())
    }

    /// Apply a batch of edits
    pub fn apply_edits(&mut self, edits: Vec<Edit>) -> Result<()> {
        // Sort edits in reverse order to avoid index shifting issues
        // We need to convert edits to a comparable format (start char index)
        let mut edits_with_pos: Vec<(usize, Edit)> = edits.into_iter().map(|edit| {
            let pos = match &edit {
                Edit::Insert { position, .. } => *position,
                Edit::Delete { range, .. } => range.start,
                Edit::Batch(_) => Position::new(0, 0), // Should not happen in input
            };
            (self.position_to_char(pos), edit)
        }).collect();

        // Sort by position descending
        edits_with_pos.sort_by(|a, b| b.0.cmp(&a.0));

        let sorted_edits: Vec<Edit> = edits_with_pos.into_iter().map(|(_, edit)| edit).collect();
        let mut applied_edits = Vec::new();

        for edit in sorted_edits {
            match edit {
                Edit::Insert { position, text } => {
                    let char_idx = self.position_to_char(position);
                    self.rope.insert(char_idx, &text);
                    applied_edits.push(Edit::Insert { position, text });
                }
                Edit::Delete { range, deleted_text: _ } => {
                    let start_char = self.position_to_char(range.start);
                    let end_char = self.position_to_char(range.end);
                    if start_char < end_char {
                        let deleted_text = self.rope.slice(start_char..end_char).to_string();
                        self.rope.remove(start_char..end_char);
                        applied_edits.push(Edit::Delete { range, deleted_text });
                    }
                }
                _ => {} // Nested batches not supported in input
            }
        }

        // Record batch edit for undo
        self.undo_stack.push(Edit::Batch(applied_edits));
        self.redo_stack.clear();
        self.modified = true;
        
        Ok(())
    }

    /// Undo the last edit
    pub fn undo(&mut self) -> Result<()> {
        if let Some(edit) = self.undo_stack.pop() {
            self.undo_edit(edit.clone());
            self.redo_stack.push(edit);
            self.modified = true;
        }
        Ok(())
    }

    fn undo_edit(&mut self, edit: Edit) {
        match edit {
            Edit::Insert { position, text } => {
                let start_char = self.position_to_char(position);
                let end_char = start_char + text.len();
                self.rope.remove(start_char..end_char);
            }
            Edit::Delete { range, deleted_text } => {
                let char_idx = self.position_to_char(range.start);
                self.rope.insert(char_idx, &deleted_text);
            }
            Edit::Batch(edits) => {
                // Undo batch edits in reverse order (they were applied in reverse order, 
                // so to undo we iterate normally? No, applied_edits contains them in applied order (reverse pos).
                // To undo, we should undo the first applied (last pos) first?
                // Wait, if we inserted at pos 100 then pos 10, undoing pos 100 doesn't affect pos 10.
                // So order doesn't matter as much if they are non-overlapping.
                // But let's just undo them in reverse of application order to be safe.
                for sub_edit in edits.into_iter().rev() {
                    self.undo_edit(sub_edit);
                }
            }
        }
    }

    /// Redo the last undone edit
    pub fn redo(&mut self) -> Result<()> {
        if let Some(edit) = self.redo_stack.pop() {
            self.redo_edit(edit.clone());
            self.undo_stack.push(edit);
            self.modified = true;
        }
        Ok(())
    }

    fn redo_edit(&mut self, edit: Edit) {
        match edit {
            Edit::Insert { position, text } => {
                let char_idx = self.position_to_char(position);
                self.rope.insert(char_idx, &text);
            }
            Edit::Delete { range, .. } => {
                let start_char = self.position_to_char(range.start);
                let end_char = self.position_to_char(range.end);
                self.rope.remove(start_char..end_char);
            }
            Edit::Batch(edits) => {
                // Redo edits in the order they were originally applied
                for sub_edit in edits {
                    self.redo_edit(sub_edit);
                }
            }
        }
    }

    /// Save the buffer to its associated file
    pub fn save(&mut self) -> Result<()> {
        if let Some(path) = &self.file_path {
            std::fs::write(path, self.text())?;
            self.modified = false;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No file path associated with buffer"))
        }
    }

    /// Save the buffer to a specific file
    pub fn save_as(&mut self, path: PathBuf) -> Result<()> {
        std::fs::write(&path, self.text())?;
        self.file_path = Some(path);
        self.modified = false;
        Ok(())
    }

    /// Get all cursors
    pub fn cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    /// Set cursors (for multi-cursor support)
    pub fn set_cursors(&mut self, cursors: Vec<Cursor>) {
        self.cursors = cursors;
    }

    /// Get a slice of text
    pub fn slice(&self, range: Range) -> String {
        let start_char = self.position_to_char(range.start);
        let end_char = self.position_to_char(range.end);
        self.rope.slice(start_char..end_char).to_string()
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buffer = TextBuffer::new();
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(buffer.text(), "");
        assert!(!buffer.is_modified());
    }

    #[test]
    fn test_from_str() {
        let buffer = TextBuffer::from_str("Hello\nWorld");
        assert_eq!(buffer.line_count(), 2);
        assert_eq!(buffer.line(0), Some("Hello\n".to_string()));
        assert_eq!(buffer.line(1), Some("World".to_string()));
    }

    #[test]
    fn test_insert() {
        let mut buffer = TextBuffer::from_str("Hello");
        buffer.insert(Position::new(0, 5), " World").unwrap();
        assert_eq!(buffer.text(), "Hello World");
        assert!(buffer.is_modified());
    }

    #[test]
    fn test_delete() {
        let mut buffer = TextBuffer::from_str("Hello World");
        buffer.delete(Range {
            start: Position::new(0, 5),
            end: Position::new(0, 11),
        }).unwrap();
        assert_eq!(buffer.text(), "Hello");
    }

    #[test]
    fn test_undo_redo() {
        let mut buffer = TextBuffer::from_str("Hello");
        buffer.insert(Position::new(0, 5), " World").unwrap();
        assert_eq!(buffer.text(), "Hello World");
        
        buffer.undo().unwrap();
        assert_eq!(buffer.text(), "Hello");
        
        buffer.redo().unwrap();
        assert_eq!(buffer.text(), "Hello World");
    }
}
