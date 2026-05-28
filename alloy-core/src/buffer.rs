//! Text buffer backed by a [`ropey::Rope`] with undo/redo support.

use alloy_rpc::types::{Position, TextEdit};
use ropey::Rope;

// ---------------------------------------------------------------------------
// EditOp
// ---------------------------------------------------------------------------

/// A single reversible edit operation stored in the undo/redo history.
#[derive(Debug, Clone)]
pub struct EditOp {
    /// Byte offset at which the edit starts.
    pub byte_start: usize,
    /// Byte offset at which the old text ended (exclusive).
    pub byte_end: usize,
    /// The text that was removed (needed for undo).
    pub old_text: String,
    /// The text that was inserted.
    pub new_text: String,
}

// ---------------------------------------------------------------------------
// Buffer
// ---------------------------------------------------------------------------

/// A text buffer backed by a [`ropey::Rope`] with undo/redo support.
pub struct Buffer {
    rope: Rope,
    history: Vec<EditOp>,
    redo_stack: Vec<EditOp>,
    dirty: bool,
    version: u32,
}

impl Buffer {
    // --- Constructors -------------------------------------------------------

    /// Create a buffer from an in-memory string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        Self {
            rope: Rope::from_str(s),
            history: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
            version: 0,
        }
    }

    /// Create a buffer by reading from any `Read` impl.
    pub fn from_reader(mut r: impl std::io::Read) -> anyhow::Result<Self> {
        let mut s = String::new();
        r.read_to_string(&mut s)?;
        Ok(Self::from_str(&s))
    }

    // --- Core edit ----------------------------------------------------------

    /// Apply an edit operation, record it in history, and bump the version.
    ///
    /// Removes the bytes `[byte_start, byte_end)` (i.e. `op.old_text`) and
    /// inserts `op.new_text` at the same position.
    pub fn apply_edit(&mut self, op: EditOp) {
        let char_start = self
            .rope
            .byte_to_char(op.byte_start.min(self.rope.len_bytes()));
        let char_end = self
            .rope
            .byte_to_char(op.byte_end.min(self.rope.len_bytes()));

        // Remove the old range (may be empty for pure insertions).
        if char_end > char_start {
            self.rope.remove(char_start..char_end);
        }

        // Insert the new text.
        if !op.new_text.is_empty() {
            self.rope.insert(char_start, &op.new_text);
        }

        self.history.push(op);
        self.redo_stack.clear();
        self.version = self.version.wrapping_add(1);
        self.dirty = true;
    }

    /// Convert a [`TextEdit`] (using LSP-style `Position`) into an [`EditOp`]
    /// and apply it.
    pub fn apply_text_edit(&mut self, edit: &TextEdit) -> anyhow::Result<()> {
        let byte_start = self.position_to_offset(edit.range.start)?;
        let byte_end = self.position_to_offset(edit.range.end)?;

        let old_text = self.slice_bytes(byte_start, byte_end);

        self.apply_edit(EditOp {
            byte_start,
            byte_end,
            old_text,
            new_text: edit.new_text.clone(),
        });
        Ok(())
    }

    // --- Undo / redo --------------------------------------------------------

    /// Undo the most recent edit and return the inverted operation, or `None`
    /// when the history is empty.
    pub fn undo(&mut self) -> Option<EditOp> {
        let op = self.history.pop()?;

        // Build the inverse: the new_text we inserted is now old_text to remove.
        let new_text_len = op.new_text.len();
        let inverse = EditOp {
            byte_start: op.byte_start,
            byte_end: op.byte_start + new_text_len,
            old_text: op.new_text.clone(),
            new_text: op.old_text.clone(),
        };

        // Apply the inverse directly (bypassing history recording).
        let char_start = self
            .rope
            .byte_to_char(inverse.byte_start.min(self.rope.len_bytes()));
        let char_end = self
            .rope
            .byte_to_char(inverse.byte_end.min(self.rope.len_bytes()));

        if char_end > char_start {
            self.rope.remove(char_start..char_end);
        }
        if !inverse.new_text.is_empty() {
            self.rope.insert(char_start, &inverse.new_text);
        }

        self.redo_stack.push(op.clone());
        self.version = self.version.wrapping_add(1);
        self.dirty = true;

        Some(op)
    }

    /// Redo the most recently undone edit and return it, or `None` when the
    /// redo stack is empty.
    pub fn redo(&mut self) -> Option<EditOp> {
        let op = self.redo_stack.pop()?;

        let char_start = self
            .rope
            .byte_to_char(op.byte_start.min(self.rope.len_bytes()));
        let char_end = self
            .rope
            .byte_to_char(op.byte_end.min(self.rope.len_bytes()));

        if char_end > char_start {
            self.rope.remove(char_start..char_end);
        }
        if !op.new_text.is_empty() {
            self.rope.insert(char_start, &op.new_text);
        }

        self.history.push(op.clone());
        self.version = self.version.wrapping_add(1);
        self.dirty = true;

        Some(op)
    }

    // --- State queries ------------------------------------------------------

    /// Returns `true` when the buffer has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the buffer as saved (clears the dirty flag).
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Monotonically increasing edit version counter.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Number of lines in the buffer.
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// Length of the buffer in bytes (UTF-8).
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Length of the buffer in Unicode scalar values.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Collect the entire buffer into a `String`.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.rope.to_string()
    }

    /// Returns the text in the byte range `[start, end)` as a `String`.
    ///
    /// Clamps `end` to the buffer length and handles the case where `start >=
    /// end` gracefully.
    pub fn slice_bytes(&self, start: usize, end: usize) -> String {
        let len = self.rope.len_bytes();
        let start = start.min(len);
        let end = end.min(len);
        if start >= end {
            return String::new();
        }
        let char_start = self.rope.byte_to_char(start);
        let char_end = self.rope.byte_to_char(end);
        self.rope.slice(char_start..char_end).to_string()
    }

    /// Returns the text of line `line_idx` (zero-based), including its line
    /// ending, or `None` when the index is out of range.
    pub fn line(&self, line_idx: usize) -> Option<String> {
        if line_idx >= self.rope.len_lines() {
            return None;
        }
        Some(self.rope.line(line_idx).to_string())
    }

    // --- Coordinate conversions ---------------------------------------------

    /// Convert a byte offset to an LSP [`Position`].
    ///
    /// If the offset is past the end of the buffer it is clamped.
    pub fn offset_to_position(&self, byte_offset: usize) -> Position {
        let byte_offset = byte_offset.min(self.rope.len_bytes());
        let char_offset = self.rope.byte_to_char(byte_offset);
        let line = self.rope.char_to_line(char_offset);
        let line_start_char = self.rope.line_to_char(line);
        let character = (char_offset - line_start_char) as u32;
        Position {
            line: line as u32,
            character,
        }
    }

    /// Convert an LSP [`Position`] to a byte offset.
    ///
    /// Returns an error when the position references a line or character that
    /// does not exist.
    pub fn position_to_offset(&self, pos: Position) -> anyhow::Result<usize> {
        let line = pos.line as usize;
        let col = pos.character as usize;

        let line_count = self.rope.len_lines();
        if line >= line_count {
            // Allow placing the cursor at the very end of the last line when
            // line == len_lines() and the rope ends with a newline.
            if line == line_count && col == 0 {
                return Ok(self.rope.len_bytes());
            }
            anyhow::bail!(
                "position {}:{} is out of range (buffer has {} lines)",
                pos.line,
                pos.character,
                line_count
            );
        }

        let line_slice = self.rope.line(line);
        let line_len_chars = {
            // Exclude the trailing newline from the maximum character index so
            // we do not produce a byte offset inside the newline sequence.
            let raw = line_slice.len_chars();
            let s = line_slice.to_string();
            if s.ends_with("\r\n") {
                raw.saturating_sub(2)
            } else if s.ends_with('\n') || s.ends_with('\r') {
                raw.saturating_sub(1)
            } else {
                raw
            }
        };

        if col > line_len_chars {
            // Clamp to end of line rather than erroring — many editors do this.
            let clamped_col = line_len_chars;
            let line_start_char = self.rope.line_to_char(line);
            let char_offset = line_start_char + clamped_col;
            return Ok(self.rope.char_to_byte(char_offset));
        }

        let line_start_char = self.rope.line_to_char(line);
        let char_offset = line_start_char + col;
        Ok(self.rope.char_to_byte(char_offset))
    }

    /// Convert a byte offset to a char offset.
    pub fn byte_to_char(&self, byte: usize) -> usize {
        self.rope.byte_to_char(byte.min(self.rope.len_bytes()))
    }

    /// Convert a char offset to a byte offset.
    pub fn char_to_byte(&self, ch: usize) -> usize {
        self.rope.char_to_byte(ch.min(self.rope.len_chars()))
    }

    // --- Internal rope access (for syntax layer) ----------------------------

    /// Return a reference to the underlying [`Rope`].
    pub fn rope(&self) -> &Rope {
        &self.rope
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_rpc::types::{Position, Range, TextEdit};

    fn buf(s: &str) -> Buffer {
        Buffer::from_str(s)
    }

    // --- Basic insert / delete ----------------------------------------------

    #[test]
    fn test_insert_at_start() {
        let mut b = buf("world");
        b.apply_edit(EditOp {
            byte_start: 0,
            byte_end: 0,
            old_text: String::new(),
            new_text: "hello ".to_string(),
        });
        assert_eq!(b.to_string(), "hello world");
    }

    #[test]
    fn test_delete_range() {
        let mut b = buf("hello world");
        b.apply_edit(EditOp {
            byte_start: 5,
            byte_end: 11,
            old_text: " world".to_string(),
            new_text: String::new(),
        });
        assert_eq!(b.to_string(), "hello");
    }

    #[test]
    fn test_replace_range() {
        let mut b = buf("foo bar baz");
        b.apply_edit(EditOp {
            byte_start: 4,
            byte_end: 7,
            old_text: "bar".to_string(),
            new_text: "qux".to_string(),
        });
        assert_eq!(b.to_string(), "foo qux baz");
    }

    #[test]
    fn test_dirty_and_version() {
        let mut b = buf("abc");
        assert!(!b.is_dirty());
        assert_eq!(b.version(), 0);

        b.apply_edit(EditOp {
            byte_start: 3,
            byte_end: 3,
            old_text: String::new(),
            new_text: "d".to_string(),
        });
        assert!(b.is_dirty());
        assert_eq!(b.version(), 1);

        b.mark_saved();
        assert!(!b.is_dirty());
    }

    // --- Undo / redo --------------------------------------------------------

    #[test]
    fn test_undo_single() {
        let mut b = buf("hello");
        b.apply_edit(EditOp {
            byte_start: 5,
            byte_end: 5,
            old_text: String::new(),
            new_text: " world".to_string(),
        });
        assert_eq!(b.to_string(), "hello world");

        let undone = b.undo();
        assert!(undone.is_some());
        assert_eq!(b.to_string(), "hello");
    }

    #[test]
    fn test_redo_after_undo() {
        let mut b = buf("hello");
        b.apply_edit(EditOp {
            byte_start: 5,
            byte_end: 5,
            old_text: String::new(),
            new_text: " world".to_string(),
        });
        b.undo();
        assert_eq!(b.to_string(), "hello");

        b.redo();
        assert_eq!(b.to_string(), "hello world");
    }

    #[test]
    fn test_undo_clears_redo_on_new_edit() {
        let mut b = buf("abc");
        b.apply_edit(EditOp {
            byte_start: 3,
            byte_end: 3,
            old_text: String::new(),
            new_text: "d".to_string(),
        });
        b.undo();

        // After a new edit the redo stack should be gone.
        b.apply_edit(EditOp {
            byte_start: 3,
            byte_end: 3,
            old_text: String::new(),
            new_text: "X".to_string(),
        });
        let re = b.redo();
        assert!(re.is_none());
    }

    #[test]
    fn test_undo_empty_history() {
        let mut b = buf("hello");
        assert!(b.undo().is_none());
    }

    // --- Position conversion ------------------------------------------------

    #[test]
    fn test_position_to_offset_simple() {
        let b = buf("hello\nworld\n");
        // Start of second line
        let off = b.position_to_offset(Position::new(1, 0)).unwrap();
        assert_eq!(off, 6); // "hello\n" is 6 bytes
    }

    #[test]
    fn test_position_to_offset_col() {
        let b = buf("hello\nworld\n");
        let off = b.position_to_offset(Position::new(1, 3)).unwrap();
        assert_eq!(off, 9); // 6 + 3
    }

    #[test]
    fn test_offset_to_position_roundtrip() {
        let b = buf("line0\nline1\nline2\n");
        for byte in [0usize, 3, 6, 9, 12] {
            let pos = b.offset_to_position(byte);
            let back = b.position_to_offset(pos).unwrap();
            assert_eq!(back, byte, "roundtrip failed for byte={byte}");
        }
    }

    #[test]
    fn test_position_to_offset_end_of_line() {
        let b = buf("hi\nthere\n");
        // Character 2 on line 0 is just before the newline.
        let off = b.position_to_offset(Position::new(0, 2)).unwrap();
        assert_eq!(off, 2);
    }

    // --- Line count ---------------------------------------------------------

    #[test]
    fn test_line_count() {
        let b = buf("a\nb\nc\n");
        // ropey counts the trailing empty "line" after the final newline
        assert!(b.line_count() >= 3);
    }

    #[test]
    fn test_line() {
        let b = buf("alpha\nbeta\ngamma\n");
        assert_eq!(b.line(0).unwrap().trim_end(), "alpha");
        assert_eq!(b.line(1).unwrap().trim_end(), "beta");
        assert!(b.line(100).is_none());
    }

    // --- apply_text_edit ----------------------------------------------------

    #[test]
    fn test_apply_text_edit_replace() {
        let mut b = buf("hello world\n");
        let edit = TextEdit {
            range: Range {
                start: Position::new(0, 6),
                end: Position::new(0, 11),
            },
            new_text: "Rust".to_string(),
        };
        b.apply_text_edit(&edit).unwrap();
        assert_eq!(b.to_string(), "hello Rust\n");
    }
}
