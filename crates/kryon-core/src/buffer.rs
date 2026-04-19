//! Text buffer backed by the ropey rope data structure.
//!
//! Provides O(log n) insert and delete operations, efficient line-based
//! access, and thread-safe snapshots for background processing.

use ropey::Rope;
use std::ops::Range;

/// Errors from buffer operations.
#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    /// The byte offset is out of bounds.
    #[error("byte offset {offset} is out of bounds (buffer length: {length})")]
    OffsetOutOfBounds {
        /// The attempted offset.
        offset: usize,
        /// The buffer's byte length.
        length: usize,
    },

    /// The line index is out of bounds.
    #[error("line index {index} is out of bounds (line count: {count})")]
    LineOutOfBounds {
        /// The attempted line index.
        index: usize,
        /// The total line count.
        count: usize,
    },

    /// An invalid byte range was provided.
    #[error("invalid range {start}..{end} (buffer length: {length})")]
    InvalidRange {
        /// Range start.
        start: usize,
        /// Range end.
        end: usize,
        /// Buffer byte length.
        length: usize,
    },
}

/// A text buffer backed by a rope data structure.
///
/// Wraps ropey's `Rope` with editor-specific operations and validation.
/// All byte offsets refer to positions within the UTF-8 encoded text.
#[derive(Debug, Clone)]
pub struct TextBuffer {
    /// The underlying rope.
    rope: Rope,
    /// Whether the buffer has been modified since last save.
    modified: bool,
}

impl TextBuffer {
    /// Create a new empty buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            modified: false,
        }
    }

    /// Create a buffer from a string.
    #[must_use]
    pub fn from_text(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            modified: false,
        }
    }

    /// Get the total number of bytes in the buffer.
    #[must_use]
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Get the total number of characters in the buffer.
    #[must_use]
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Check if the buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rope.len_bytes() == 0
    }

    /// Get the number of lines in the buffer.
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// Whether the buffer has been modified since last save.
    #[must_use]
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Mark the buffer as saved (not modified).
    pub fn mark_saved(&mut self) {
        self.modified = false;
    }

    /// Get the text content of a specific line by zero-based index.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::LineOutOfBounds` if the index exceeds line count.
    pub fn line(&self, index: usize) -> Result<String, BufferError> {
        if index >= self.rope.len_lines() {
            return Err(BufferError::LineOutOfBounds {
                index,
                count: self.rope.len_lines(),
            });
        }
        Ok(self.rope.line(index).to_string())
    }

    /// Insert text at the given character offset.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::OffsetOutOfBounds` if the offset exceeds the
    /// buffer's character count.
    pub fn insert(&mut self, char_offset: usize, text: &str) -> Result<(), BufferError> {
        if char_offset > self.rope.len_chars() {
            return Err(BufferError::OffsetOutOfBounds {
                offset: char_offset,
                length: self.rope.len_chars(),
            });
        }
        self.rope.insert(char_offset, text);
        self.modified = true;
        Ok(())
    }

    /// Delete a range of characters from the buffer.
    ///
    /// Returns the deleted text for undo support.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::InvalidRange` if the range is out of bounds.
    pub fn delete(&mut self, range: Range<usize>) -> Result<String, BufferError> {
        if range.end > self.rope.len_chars() || range.start > range.end {
            return Err(BufferError::InvalidRange {
                start: range.start,
                end: range.end,
                length: self.rope.len_chars(),
            });
        }
        let deleted = self.rope.slice(range.clone()).to_string();
        self.rope.remove(range);
        self.modified = true;
        Ok(deleted)
    }

    /// Replace a range of characters with new text.
    ///
    /// Returns the replaced text for undo support.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::InvalidRange` if the range is out of bounds.
    pub fn replace(&mut self, range: Range<usize>, text: &str) -> Result<String, BufferError> {
        let deleted = self.delete(range.clone())?;
        self.insert(range.start, text)?;
        Ok(deleted)
    }

    /// Get the entire buffer content as a string.
    #[must_use]
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Get a reference to the underlying rope.
    #[must_use]
    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    /// Convert a character offset to a (line, column) pair.
    ///
    /// Both line and column are zero-based.
    ///
    /// # Errors
    ///
    /// Returns `BufferError::OffsetOutOfBounds` if the offset exceeds the
    /// buffer's character count.
    pub fn offset_to_line_col(&self, char_offset: usize) -> Result<(usize, usize), BufferError> {
        if char_offset > self.rope.len_chars() {
            return Err(BufferError::OffsetOutOfBounds {
                offset: char_offset,
                length: self.rope.len_chars(),
            });
        }
        let line = self.rope.char_to_line(char_offset);
        let line_start = self.rope.line_to_char(line);
        let col = char_offset - line_start;
        Ok((line, col))
    }

    /// Convert a (line, column) pair to a character offset.
    ///
    /// Both line and column are zero-based.
    ///
    /// # Errors
    ///
    /// Returns an error if the line or column is out of bounds.
    pub fn line_col_to_offset(&self, line: usize, col: usize) -> Result<usize, BufferError> {
        if line >= self.rope.len_lines() {
            return Err(BufferError::LineOutOfBounds {
                index: line,
                count: self.rope.len_lines(),
            });
        }
        let line_start = self.rope.line_to_char(line);
        let line_len = self.rope.line(line).len_chars();
        let clamped_col = col.min(line_len.saturating_sub(1));
        Ok(line_start + clamped_col)
    }

    // ── Word navigation ──────────────────────────────────────

    /// Find the offset of the previous word boundary (for Ctrl+Left).
    ///
    /// Scans left: skip whitespace/punctuation, then skip word characters.
    #[must_use]
    pub fn word_boundary_left(&self, offset: usize) -> usize {
        if offset == 0 {
            return 0;
        }
        let text = self.rope.to_string();
        let chars: Vec<char> = text.chars().collect();
        let mut pos = offset.min(chars.len());

        // Step back one position to start scanning from previous char
        pos = pos.saturating_sub(1);

        // Skip whitespace/newlines going left
        while pos > 0 && (chars[pos].is_whitespace() || is_punctuation(chars[pos])) {
            pos -= 1;
        }

        // Skip word characters going left
        while pos > 0 && is_word_char(chars[pos - 1]) {
            pos -= 1;
        }

        pos
    }

    /// Find the offset of the next word boundary (for Ctrl+Right).
    ///
    /// Scans right: skip word characters, then skip whitespace/punctuation.
    #[must_use]
    pub fn word_boundary_right(&self, offset: usize) -> usize {
        let text = self.rope.to_string();
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let mut pos = offset.min(len);

        // Skip word characters going right
        while pos < len && is_word_char(chars[pos]) {
            pos += 1;
        }

        // Skip whitespace/punctuation going right
        while pos < len && (chars[pos].is_whitespace() || is_punctuation(chars[pos])) {
            pos += 1;
        }

        pos
    }

    // ── Search ───────────────────────────────────────────────

    /// Find all character offsets where `query` occurs in the buffer.
    ///
    /// Returns a sorted vector of starting character offsets.
    /// The search is case-sensitive.
    #[must_use]
    pub fn find_all(&self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return Vec::new();
        }
        let text = self.rope.to_string();
        let mut matches = Vec::new();
        let mut start = 0;
        while let Some(byte_pos) = text[start..].find(query) {
            let abs_byte = start + byte_pos;
            // Convert byte offset to char offset
            let char_offset = text[..abs_byte].chars().count();
            matches.push(char_offset);
            start = abs_byte + query.len();
        }
        matches
    }

    /// Find all character offsets where `query` occurs (case-insensitive).
    #[must_use]
    pub fn find_all_ci(&self, query: &str) -> Vec<usize> {
        if query.is_empty() {
            return Vec::new();
        }
        let text = self.rope.to_string().to_lowercase();
        let query_lower = query.to_lowercase();
        let mut matches = Vec::new();
        let mut start = 0;
        while let Some(byte_pos) = text[start..].find(&query_lower) {
            let abs_byte = start + byte_pos;
            let char_offset = text[..abs_byte].chars().count();
            matches.push(char_offset);
            start = abs_byte + query_lower.len();
        }
        matches
    }
}

/// Whether a character is part of a "word" (alphanumeric or underscore).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether a character is punctuation (not whitespace and not a word char).
fn is_punctuation(c: char) -> bool {
    !c.is_whitespace() && !is_word_char(c)
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
    fn test_new_buffer_is_empty() {
        let buf = TextBuffer::new();
        assert!(buf.is_empty());
        assert_eq!(buf.len_chars(), 0);
        assert!(!buf.is_modified());
    }

    #[test]
    fn test_from_str() {
        let buf = TextBuffer::from_text("hello\nworld");
        assert_eq!(buf.len_chars(), 11);
        assert_eq!(buf.line_count(), 2);
        assert!(!buf.is_modified());
    }

    #[test]
    fn test_insert_at_beginning() {
        let mut buf = TextBuffer::from_text("world");
        buf.insert(0, "hello ").unwrap();
        assert_eq!(buf.text(), "hello world");
        assert!(buf.is_modified());
    }

    #[test]
    fn test_insert_at_end() {
        let mut buf = TextBuffer::from_text("hello");
        buf.insert(5, " world").unwrap();
        assert_eq!(buf.text(), "hello world");
    }

    #[test]
    fn test_insert_out_of_bounds() {
        let mut buf = TextBuffer::from_text("hello");
        let result = buf.insert(100, "x");
        assert!(matches!(result, Err(BufferError::OffsetOutOfBounds { .. })));
    }

    #[test]
    fn test_delete_range() {
        let mut buf = TextBuffer::from_text("hello world");
        let deleted = buf.delete(5..11).unwrap();
        assert_eq!(deleted, " world");
        assert_eq!(buf.text(), "hello");
    }

    #[test]
    fn test_delete_invalid_range() {
        let mut buf = TextBuffer::from_text("hello");
        let result = buf.delete(0..100);
        assert!(matches!(result, Err(BufferError::InvalidRange { .. })));
    }

    #[test]
    fn test_replace() {
        let mut buf = TextBuffer::from_text("hello world");
        let replaced = buf.replace(6..11, "rust").unwrap();
        assert_eq!(replaced, "world");
        assert_eq!(buf.text(), "hello rust");
    }

    #[test]
    fn test_line_access() {
        let buf = TextBuffer::from_text("line1\nline2\nline3");
        assert_eq!(buf.line(0).unwrap(), "line1\n");
        assert_eq!(buf.line(2).unwrap(), "line3");
        assert!(buf.line(5).is_err());
    }

    #[test]
    fn test_offset_to_line_col() {
        let buf = TextBuffer::from_text("hello\nworld");
        assert_eq!(buf.offset_to_line_col(0).unwrap(), (0, 0));
        assert_eq!(buf.offset_to_line_col(5).unwrap(), (0, 5));
        assert_eq!(buf.offset_to_line_col(6).unwrap(), (1, 0));
        assert_eq!(buf.offset_to_line_col(11).unwrap(), (1, 5));
    }

    #[test]
    fn test_line_col_to_offset() {
        let buf = TextBuffer::from_text("hello\nworld");
        assert_eq!(buf.line_col_to_offset(0, 0).unwrap(), 0);
        assert_eq!(buf.line_col_to_offset(1, 0).unwrap(), 6);
        assert_eq!(buf.line_col_to_offset(1, 3).unwrap(), 9);
    }

    #[test]
    fn test_mark_saved() {
        let mut buf = TextBuffer::from_text("hello");
        buf.insert(5, " world").unwrap();
        assert!(buf.is_modified());
        buf.mark_saved();
        assert!(!buf.is_modified());
    }

    #[test]
    fn test_word_boundary_left() {
        let buf = TextBuffer::from_text("hello world foo_bar");
        // From end of "hello" → start of "hello"
        assert_eq!(buf.word_boundary_left(5), 0);
        // From middle of "world" → start of "world"
        assert_eq!(buf.word_boundary_left(8), 6);
        // From start → stays at 0
        assert_eq!(buf.word_boundary_left(0), 0);
        // From "foo_bar" end → start of "foo_bar"
        assert_eq!(buf.word_boundary_left(19), 12);
    }

    #[test]
    fn test_word_boundary_right() {
        let buf = TextBuffer::from_text("hello world foo_bar");
        // From start of "hello" → start of "world"
        assert_eq!(buf.word_boundary_right(0), 6);
        // From start of "world" → start of "foo_bar"
        assert_eq!(buf.word_boundary_right(6), 12);
        // From start of "foo_bar" → end
        assert_eq!(buf.word_boundary_right(12), 19);
    }

    #[test]
    fn test_find_all() {
        let buf = TextBuffer::from_text("hello world hello");
        let matches = buf.find_all("hello");
        assert_eq!(matches, vec![0, 12]);

        let matches = buf.find_all("xyz");
        assert!(matches.is_empty());

        let matches = buf.find_all("");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_find_all_ci() {
        let buf = TextBuffer::from_text("Hello WORLD hello");
        let matches = buf.find_all_ci("hello");
        assert_eq!(matches, vec![0, 12]);
    }
}
