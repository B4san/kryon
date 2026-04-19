//! Incremental search state management.
//!
//! Tracks search query, match positions, and active match navigation
//! for the Ctrl+F search overlay.

/// State for the incremental search feature.
#[derive(Debug, Clone)]
pub struct SearchState {
    /// The current search query typed by the user.
    pub query: String,
    /// Character offsets of all matches in the buffer.
    pub matches: Vec<usize>,
    /// Index of the currently active (highlighted) match.
    pub active_match: usize,
    /// Whether the search panel is visible.
    pub visible: bool,
}

impl SearchState {
    /// Create a new hidden search state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            active_match: 0,
            visible: false,
        }
    }

    /// Open the search panel.
    pub fn open(&mut self) {
        self.visible = true;
    }

    /// Close the search panel and clear state.
    pub fn close(&mut self) {
        self.visible = false;
        self.query.clear();
        self.matches.clear();
        self.active_match = 0;
    }

    /// Update the search query and recalculate matches.
    pub fn set_query(&mut self, query: String, match_offsets: Vec<usize>) {
        self.query = query;
        self.matches = match_offsets;
        self.active_match = 0;
    }

    /// Jump to the next match. Wraps around.
    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.active_match = (self.active_match + 1) % self.matches.len();
        }
    }

    /// Jump to the previous match. Wraps around.
    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.active_match = if self.active_match == 0 {
                self.matches.len() - 1
            } else {
                self.active_match - 1
            };
        }
    }

    /// Get the character offset of the current active match.
    #[must_use]
    pub fn active_offset(&self) -> Option<usize> {
        self.matches.get(self.active_match).copied()
    }

    /// Total number of matches.
    #[must_use]
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Display text for the match counter (e.g., "3 of 10").
    #[must_use]
    pub fn match_display(&self) -> String {
        if self.matches.is_empty() {
            if self.query.is_empty() {
                String::new()
            } else {
                "No results".to_string()
            }
        } else {
            format!("{} of {}", self.active_match + 1, self.matches.len())
        }
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for the Go-to-Line prompt.
#[derive(Debug, Clone)]
pub struct GoToLineState {
    /// The line number input typed by the user.
    pub input: String,
    /// Whether the prompt is visible.
    pub visible: bool,
}

impl GoToLineState {
    /// Create a new hidden go-to-line state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input: String::new(),
            visible: false,
        }
    }

    /// Open the prompt.
    pub fn open(&mut self) {
        self.visible = true;
        self.input.clear();
    }

    /// Close the prompt.
    pub fn close(&mut self) {
        self.visible = false;
        self.input.clear();
    }
}

impl Default for GoToLineState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_navigation() {
        let mut search = SearchState::new();
        search.set_query("test".to_string(), vec![0, 10, 20]);
        assert_eq!(search.active_offset(), Some(0));

        search.next_match();
        assert_eq!(search.active_offset(), Some(10));

        search.next_match();
        assert_eq!(search.active_offset(), Some(20));

        // Wrap around
        search.next_match();
        assert_eq!(search.active_offset(), Some(0));

        // Prev from 0 wraps to end
        search.prev_match();
        assert_eq!(search.active_offset(), Some(20));
    }

    #[test]
    fn test_match_display() {
        let mut search = SearchState::new();
        assert_eq!(search.match_display(), "");

        search.query = "test".to_string();
        assert_eq!(search.match_display(), "No results");

        search.matches = vec![0, 10, 20];
        assert_eq!(search.match_display(), "1 of 3");
    }
}
