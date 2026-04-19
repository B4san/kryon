//! Command pattern for undoable editor operations.

use crate::buffer::{BufferError, TextBuffer};
use crate::cursor::CursorSet;
use std::ops::Range;

/// An edit that can be undone and redone.
#[derive(Debug, Clone)]
pub enum EditCommand {
    /// Insert text at a character offset.
    Insert {
        /// Character offset where text was inserted.
        offset: usize,
        /// The inserted text.
        text: String,
    },
    /// Delete text from a range, storing what was deleted.
    Delete {
        /// The range that was deleted (character offsets).
        range: Range<usize>,
        /// The text that was deleted (for undo).
        deleted_text: String,
    },
    /// Replace text in a range.
    Replace {
        /// The range that was replaced.
        range: Range<usize>,
        /// The new text that replaced the old.
        new_text: String,
        /// The old text that was replaced (for undo).
        old_text: String,
    },
}

impl EditCommand {
    /// Apply this command to the buffer.
    ///
    /// # Errors
    ///
    /// Returns `BufferError` if the operation fails.
    pub fn execute(&self, buffer: &mut TextBuffer, cursors: &mut CursorSet) -> Result<(), BufferError> {
        match self {
            EditCommand::Insert { offset, text } => {
                buffer.insert(*offset, text)?;
                let delta = text.len().cast_signed();
                cursors.adjust_offsets(*offset, delta);
            }
            EditCommand::Delete { range, .. } => {
                buffer.delete(range.clone())?;
                let delta = -(range.len().cast_signed());
                cursors.adjust_offsets(range.start, delta);
            }
            EditCommand::Replace { range, new_text, .. } => {
                buffer.delete(range.clone())?;
                buffer.insert(range.start, new_text)?;
                let delta = new_text.len().cast_signed() - range.len().cast_signed();
                cursors.adjust_offsets(range.start, delta);
            }
        }
        Ok(())
    }

    /// Undo this command, restoring the buffer to its previous state.
    ///
    /// # Errors
    ///
    /// Returns `BufferError` if the undo fails.
    pub fn undo(&self, buffer: &mut TextBuffer, cursors: &mut CursorSet) -> Result<(), BufferError> {
        match self {
            EditCommand::Insert { offset, text } => {
                let end = *offset + text.chars().count();
                buffer.delete(*offset..end)?;
                let delta = -(text.len().cast_signed());
                cursors.adjust_offsets(*offset, delta);
            }
            EditCommand::Delete { range, deleted_text } => {
                buffer.insert(range.start, deleted_text)?;
                let delta = deleted_text.len().cast_signed();
                cursors.adjust_offsets(range.start, delta);
            }
            EditCommand::Replace { range, new_text, old_text } => {
                let new_end = range.start + new_text.chars().count();
                buffer.delete(range.start..new_end)?;
                buffer.insert(range.start, old_text)?;
                let delta = old_text.len().cast_signed() - new_text.len().cast_signed();
                cursors.adjust_offsets(range.start, delta);
            }
        }
        Ok(())
    }
}

/// Undo/redo history stack.
#[derive(Debug)]
pub struct CommandHistory {
    /// Past commands (for undo).
    undo_stack: Vec<EditCommand>,
    /// Future commands (for redo, cleared on new edits).
    redo_stack: Vec<EditCommand>,
    /// Maximum history size.
    max_size: usize,
}

impl CommandHistory {
    /// Create a new command history with default max size.
    #[must_use]
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_size: 10_000,
        }
    }

    /// Push a command onto the undo stack and clear the redo stack.
    pub fn push(&mut self, command: EditCommand) {
        self.redo_stack.clear();
        self.undo_stack.push(command);
        if self.undo_stack.len() > self.max_size {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last command. Returns the command for execution.
    pub fn undo(&mut self) -> Option<EditCommand> {
        let cmd = self.undo_stack.pop()?;
        self.redo_stack.push(cmd.clone());
        Some(cmd)
    }

    /// Redo the last undone command. Returns the command for execution.
    pub fn redo(&mut self) -> Option<EditCommand> {
        let cmd = self.redo_stack.pop()?;
        self.undo_stack.push(cmd.clone());
        Some(cmd)
    }

    /// Whether there are commands to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool { !self.undo_stack.is_empty() }

    /// Whether there are commands to redo.
    #[must_use]
    pub fn can_redo(&self) -> bool { !self.redo_stack.is_empty() }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl Default for CommandHistory {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_undo() {
        let mut buf = TextBuffer::from_text("hello");
        let mut cursors = CursorSet::at(5);
        let cmd = EditCommand::Insert { offset: 5, text: " world".to_string() };
        cmd.execute(&mut buf, &mut cursors).unwrap();
        assert_eq!(buf.text(), "hello world");
        cmd.undo(&mut buf, &mut cursors).unwrap();
        assert_eq!(buf.text(), "hello");
    }

    #[test]
    fn test_delete_and_undo() {
        let mut buf = TextBuffer::from_text("hello world");
        let mut cursors = CursorSet::at(0);
        let cmd = EditCommand::Delete { range: 5..11, deleted_text: " world".to_string() };
        cmd.execute(&mut buf, &mut cursors).unwrap();
        assert_eq!(buf.text(), "hello");
        cmd.undo(&mut buf, &mut cursors).unwrap();
        assert_eq!(buf.text(), "hello world");
    }

    #[test]
    fn test_history_undo_redo() {
        let mut history = CommandHistory::new();
        let cmd = EditCommand::Insert { offset: 0, text: "a".to_string() };
        history.push(cmd);
        assert!(history.can_undo());
        assert!(!history.can_redo());

        let undone = history.undo();
        assert!(undone.is_some());
        assert!(!history.can_undo());
        assert!(history.can_redo());

        let redone = history.redo();
        assert!(redone.is_some());
        assert!(history.can_undo());
    }

    #[test]
    fn test_history_new_edit_clears_redo() {
        let mut history = CommandHistory::new();
        history.push(EditCommand::Insert { offset: 0, text: "a".to_string() });
        history.push(EditCommand::Insert { offset: 1, text: "b".to_string() });
        history.undo();
        assert!(history.can_redo());

        // New edit should clear redo
        history.push(EditCommand::Insert { offset: 1, text: "c".to_string() });
        assert!(!history.can_redo());
    }
}
