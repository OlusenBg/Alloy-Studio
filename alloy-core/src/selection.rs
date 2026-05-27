//! Cursor and selection primitives for the editor.

use alloy_rpc::types::{Position, Range};

// ---------------------------------------------------------------------------
// Cursor
// ---------------------------------------------------------------------------

/// A point in the text expressed as (line, character) — both zero-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cursor {
    pub line: u32,
    pub character: u32,
}

impl Cursor {
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }

    /// Convert to an RPC [`Position`].
    pub fn to_position(self) -> Position {
        Position {
            line: self.line,
            character: self.character,
        }
    }

    /// Build from an RPC [`Position`].
    pub fn from_position(p: Position) -> Self {
        Self {
            line: p.line,
            character: p.character,
        }
    }

    /// Returns `true` when `self` comes strictly before `other` in the
    /// document (i.e. earlier line, or same line with earlier column).
    pub fn is_before(&self, other: &Cursor) -> bool {
        self.line < other.line || (self.line == other.line && self.character < other.character)
    }
}

impl PartialOrd for Cursor {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Cursor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.line
            .cmp(&other.line)
            .then(self.character.cmp(&other.character))
    }
}

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------

/// A selection range in the editor, consisting of an `anchor` (where the
/// selection started) and an `active` cursor (where it currently ends).
///
/// Either end may be in front of the other — use [`Selection::normalized`] to
/// obtain a (start, end) pair in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// The fixed end of the selection (where the mouse/keyboard pressed down).
    pub anchor: Cursor,
    /// The moving end of the selection (where the cursor is now).
    pub active: Cursor,
}

impl Selection {
    /// Create a collapsed (empty) selection at `pos`.
    pub fn cursor(pos: Cursor) -> Self {
        Self {
            anchor: pos,
            active: pos,
        }
    }

    /// Create a selection with explicit anchor and active positions.
    pub fn new(anchor: Cursor, active: Cursor) -> Self {
        Self { anchor, active }
    }

    /// Returns `true` when the selection is empty (anchor == active).
    pub fn is_empty(&self) -> bool {
        self.anchor == self.active
    }

    /// Returns `(start, end)` in document order (start ≤ end).
    pub fn normalized(&self) -> (Cursor, Cursor) {
        if self.anchor.is_before(&self.active) || self.anchor == self.active {
            (self.anchor, self.active)
        } else {
            (self.active, self.anchor)
        }
    }

    /// Express the selection as an RPC [`Range`] (always start ≤ end).
    pub fn as_range(&self) -> Range {
        let (start, end) = self.normalized();
        Range {
            start: start.to_position(),
            end: end.to_position(),
        }
    }

    /// Returns `true` when `pos` falls inside (or on the boundary of) this
    /// selection.
    pub fn contains(&self, pos: Cursor) -> bool {
        let (start, end) = self.normalized();
        pos >= start && pos <= end
    }
}

// ---------------------------------------------------------------------------
// SelectionSet
// ---------------------------------------------------------------------------

/// A set of selections with a designated "primary" selection.
///
/// Multi-cursor editing uses multiple selections simultaneously.  The primary
/// selection drives UI affordances like the visible cursor blink position.
pub struct SelectionSet {
    selections: Vec<Selection>,
    primary: usize,
}

impl SelectionSet {
    /// Create a set containing exactly one selection, which becomes the primary.
    pub fn new(sel: Selection) -> Self {
        Self {
            selections: vec![sel],
            primary: 0,
        }
    }

    /// Return a reference to the primary selection.
    pub fn primary(&self) -> &Selection {
        &self.selections[self.primary]
    }

    /// Return a mutable reference to the primary selection.
    pub fn primary_mut(&mut self) -> &mut Selection {
        &mut self.selections[self.primary]
    }

    /// Return all selections in the set.
    pub fn all(&self) -> &[Selection] {
        &self.selections
    }

    /// Add a new selection.  It is *not* made primary.
    pub fn add(&mut self, sel: Selection) {
        self.selections.push(sel);
    }

    /// Discard all non-primary selections, keeping only the primary.
    pub fn reduce_to_primary(&mut self) {
        let primary_sel = self.selections[self.primary].clone();
        self.selections.clear();
        self.selections.push(primary_sel);
        self.primary = 0;
    }

    /// Replace the primary selection with `sel` (and keep it as primary).
    pub fn set_primary(&mut self, sel: Selection) {
        self.selections[self.primary] = sel;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_ordering() {
        let a = Cursor::new(0, 5);
        let b = Cursor::new(1, 0);
        assert!(a.is_before(&b));
        assert!(!b.is_before(&a));
    }

    #[test]
    fn test_selection_empty() {
        let s = Selection::cursor(Cursor::new(3, 7));
        assert!(s.is_empty());
    }

    #[test]
    fn test_selection_normalized() {
        let s = Selection::new(Cursor::new(2, 5), Cursor::new(0, 3));
        let (start, end) = s.normalized();
        assert!(start.is_before(&end) || start == end);
    }

    #[test]
    fn test_selection_contains() {
        let s = Selection::new(Cursor::new(0, 0), Cursor::new(0, 10));
        assert!(s.contains(Cursor::new(0, 5)));
        assert!(!s.contains(Cursor::new(0, 11)));
    }

    #[test]
    fn test_selection_set_reduce() {
        let mut ss = SelectionSet::new(Selection::cursor(Cursor::new(0, 0)));
        ss.add(Selection::cursor(Cursor::new(1, 0)));
        ss.add(Selection::cursor(Cursor::new(2, 0)));
        assert_eq!(ss.all().len(), 3);
        ss.reduce_to_primary();
        assert_eq!(ss.all().len(), 1);
    }
}
