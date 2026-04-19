//! Cursor and selection model for the editor.

use std::cmp::Ordering;

/// A cursor with anchor (selection start) and head (current position).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    /// Selection anchor. Stays fixed during selection.
    pub anchor: usize,
    /// Current position. Moves with cursor movement.
    pub head: usize,
}

impl Cursor {
    /// New cursor at position with no selection.
    #[must_use]
    pub fn new(position: usize) -> Self {
        Self { anchor: position, head: position }
    }

    /// Cursor with a selection range.
    #[must_use]
    pub fn with_selection(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    /// Whether this cursor has an active selection.
    #[must_use]
    pub fn has_selection(&self) -> bool { self.anchor != self.head }

    /// Selection range as (start, end) where start <= end.
    #[must_use]
    pub fn selection_range(&self) -> (usize, usize) {
        if self.anchor <= self.head { (self.anchor, self.head) }
        else { (self.head, self.anchor) }
    }

    /// Minimum of anchor and head.
    #[must_use]
    pub fn min_offset(&self) -> usize { self.anchor.min(self.head) }

    /// Maximum of anchor and head.
    #[must_use]
    pub fn max_offset(&self) -> usize { self.anchor.max(self.head) }

    /// Move cursor, collapsing any selection.
    pub fn move_to(&mut self, position: usize) {
        self.anchor = position;
        self.head = position;
    }

    /// Extend selection to new head (anchor stays fixed).
    pub fn select_to(&mut self, position: usize) { self.head = position; }

    /// Check if two cursors overlap.
    #[must_use]
    pub fn overlaps(&self, other: &Cursor) -> bool {
        let (s1, e1) = self.selection_range();
        let (s2, e2) = other.selection_range();
        s1 < e2 && s2 < e1
    }

    /// Merge with another overlapping cursor.
    #[must_use]
    pub fn merge(&self, other: &Cursor) -> Cursor {
        let min = self.min_offset().min(other.min_offset());
        let max = self.max_offset().max(other.max_offset());
        if self.head >= self.anchor {
            Cursor::with_selection(min, max)
        } else {
            Cursor::with_selection(max, min)
        }
    }
}

impl Ord for Cursor {
    fn cmp(&self, other: &Self) -> Ordering {
        self.min_offset().cmp(&other.min_offset())
            .then(self.max_offset().cmp(&other.max_offset()))
    }
}

impl PartialOrd for Cursor {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

/// Sorted, deduplicated collection of cursors.
#[derive(Debug, Clone)]
pub struct CursorSet {
    cursors: Vec<Cursor>,
}

impl CursorSet {
    /// Single cursor at position 0.
    #[must_use]
    pub fn new() -> Self { Self { cursors: vec![Cursor::new(0)] } }

    /// Single cursor at given position.
    #[must_use]
    pub fn at(position: usize) -> Self { Self { cursors: vec![Cursor::new(position)] } }

    /// Primary cursor (index 0).
    #[must_use]
    pub fn primary(&self) -> &Cursor { &self.cursors[0] }

    /// Mutable primary cursor.
    pub fn primary_mut(&mut self) -> &mut Cursor { &mut self.cursors[0] }

    /// All cursors.
    #[must_use]
    pub fn all(&self) -> &[Cursor] { &self.cursors }

    /// Cursor count.
    #[must_use]
    pub fn len(&self) -> usize { self.cursors.len() }

    /// Whether the cursor set is empty (should never be true in practice).
    #[must_use]
    pub fn is_empty(&self) -> bool { self.cursors.is_empty() }

    /// Single cursor check.
    #[must_use]
    pub fn is_single(&self) -> bool { self.cursors.len() == 1 }

    /// Add a cursor and normalize.
    pub fn add(&mut self, cursor: Cursor) {
        self.cursors.push(cursor);
        self.normalize();
    }

    /// Set to single cursor at position.
    pub fn set_single(&mut self, position: usize) {
        self.cursors.clear();
        self.cursors.push(Cursor::new(position));
    }

    /// Adjust all cursor offsets after an edit.
    pub fn adjust_offsets(&mut self, at_or_after: usize, delta: isize) {
        for cursor in &mut self.cursors {
            cursor.anchor = adjust_single(cursor.anchor, at_or_after, delta);
            cursor.head = adjust_single(cursor.head, at_or_after, delta);
        }
    }

    /// Sort and merge overlapping cursors.
    ///
    /// # Panics
    ///
    /// This method will not panic. The internal `expect` is guarded by
    /// the initialization of `merged` with at least one element.
    pub fn normalize(&mut self) {
        if self.cursors.len() <= 1 { return; }
        self.cursors.sort();
        let mut merged: Vec<Cursor> = Vec::with_capacity(self.cursors.len());
        merged.push(self.cursors[0]);
        for cursor in &self.cursors[1..] {
            let last = merged.last_mut().expect("merged is never empty");
            if last.max_offset() >= cursor.min_offset() {
                *last = last.merge(cursor);
            } else {
                merged.push(*cursor);
            }
        }
        self.cursors = merged;
    }
}

impl Default for CursorSet {
    fn default() -> Self { Self::new() }
}

fn adjust_single(offset: usize, edit_pos: usize, delta: isize) -> usize {
    if offset < edit_pos { return offset; }
    if delta >= 0 { return offset + delta.cast_unsigned(); }
    let abs_delta = (-delta).cast_unsigned();
    if offset >= edit_pos + abs_delta { offset - abs_delta } else { edit_pos }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_basics() {
        let c = Cursor::new(5);
        assert!(!c.has_selection());
        assert_eq!(c.selection_range(), (5, 5));
    }

    #[test]
    fn test_cursor_selection() {
        let c = Cursor::with_selection(3, 7);
        assert!(c.has_selection());
        assert_eq!(c.selection_range(), (3, 7));
        let c2 = Cursor::with_selection(7, 3);
        assert_eq!(c2.selection_range(), (3, 7));
    }

    #[test]
    fn test_cursor_move_and_select() {
        let mut c = Cursor::with_selection(3, 7);
        c.move_to(10);
        assert!(!c.has_selection());
        c.select_to(15);
        assert!(c.has_selection());
    }

    #[test]
    fn test_cursor_set_normalize() {
        let mut set = CursorSet::at(0);
        set.add(Cursor::with_selection(0, 5));
        set.add(Cursor::with_selection(3, 8));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_adjust_offsets() {
        let mut set = CursorSet::new();
        set.set_single(10);
        set.adjust_offsets(5, 3);
        assert_eq!(set.primary().head, 13);
        set.adjust_offsets(5, -3);
        assert_eq!(set.primary().head, 10);
    }
}
