//! A document represents an open file with its text buffer and selections.

use crate::buffer::Buffer;
use crate::selection::{Cursor, Selection, SelectionSet};
use alloy_rpc::types::TextEdit;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------

/// An open file in the editor, combining a [`Buffer`] with metadata and
/// multi-cursor selection state.
pub struct Document {
    /// The `file://` URI that uniquely identifies this document.
    pub uri: String,
    /// The file-system path for reading/writing.
    pub path: PathBuf,
    /// Language identifier string (e.g. `"java"`, `"kotlin"`, `"xml"`).
    pub language_id: String,
    /// The text content managed by a rope-backed [`Buffer`].
    pub buffer: Buffer,
    /// LSP document version, incremented by the client on each change.
    pub version: i32,
    selections: SelectionSet,
}

impl Document {
    // --- Constructors -------------------------------------------------------

    /// Open a file from disk, inferring the language from its extension.
    pub fn open(path: PathBuf) -> anyhow::Result<Self> {
        let file = std::fs::File::open(&path)?;
        let buffer = Buffer::from_reader(file)?;
        let language_id = Self::language_id_for_path(&path).to_owned();
        let uri = Self::path_to_uri(&path);
        Ok(Self {
            uri,
            path,
            language_id,
            buffer,
            version: 0,
            selections: SelectionSet::new(Selection::cursor(Cursor::new(0, 0))),
        })
    }

    /// Construct a document from an in-memory string (e.g. from an LSP
    /// `textDocument/didOpen` notification).
    pub fn from_text(path: PathBuf, text: String, version: i32) -> Self {
        let language_id = Self::language_id_for_path(&path).to_owned();
        let uri = Self::path_to_uri(&path);
        Self {
            uri,
            path,
            language_id,
            buffer: Buffer::from_str(&text),
            version,
            selections: SelectionSet::new(Selection::cursor(Cursor::new(0, 0))),
        }
    }

    // --- Content updates ----------------------------------------------------

    /// Replace the entire buffer content (e.g. on `textDocument/didChange`
    /// with a single full-document range).
    pub fn apply_full_text(&mut self, text: &str, version: i32) {
        self.buffer = Buffer::from_str(text);
        self.version = version;
    }

    /// Apply a list of incremental [`TextEdit`]s in the order provided.
    ///
    /// LSP specifies that edits must be applied in reverse order when the
    /// ranges do not overlap, to keep earlier offsets stable.  We honour
    /// that by sorting descending before application.
    pub fn apply_edits(&mut self, edits: &[TextEdit], version: i32) -> anyhow::Result<()> {
        // Sort edits in reverse document order so earlier byte offsets are not
        // invalidated when we apply later edits first.
        let mut sorted: Vec<&TextEdit> = edits.iter().collect();
        sorted.sort_by(|a, b| {
            let la = a.range.start.line;
            let lb = b.range.start.line;
            let ca = a.range.start.character;
            let cb = b.range.start.character;
            lb.cmp(&la).then(cb.cmp(&ca))
        });

        for edit in sorted {
            self.buffer.apply_text_edit(edit)?;
        }
        self.version = version;
        Ok(())
    }

    // --- Persistence --------------------------------------------------------

    /// Write the buffer content to [`Self::path`] and mark the buffer clean.
    pub fn save(&mut self) -> anyhow::Result<()> {
        let content = self.buffer.to_string();
        std::fs::write(&self.path, content.as_bytes())?;
        self.buffer.mark_saved();
        Ok(())
    }

    // --- Language detection -------------------------------------------------

    /// Map a file extension to an LSP language identifier.
    pub fn language_id_for_path(path: &Path) -> &'static str {
        match path.extension().and_then(|e| e.to_str()) {
            Some("java") => "java",
            Some("kt") | Some("kts") => "kotlin",
            Some("gradle") => "groovy",
            Some("xml") => "xml",
            Some("json") => "json",
            Some("md") => "markdown",
            Some("toml") => "toml",
            Some("yaml") | Some("yml") => "yaml",
            Some("properties") => "properties",
            _ => "plaintext",
        }
    }

    // --- URI helpers --------------------------------------------------------

    /// Convert a filesystem path to a `file://` URI.
    pub fn path_to_uri(path: &Path) -> String {
        format!("file://{}", path.display())
    }

    /// Parse a `file://` URI back to a [`PathBuf`].  Returns `None` if the
    /// URI does not start with `file://`.
    pub fn uri_to_path(uri: &str) -> Option<PathBuf> {
        uri.strip_prefix("file://").map(PathBuf::from)
    }

    // --- Selection access ---------------------------------------------------

    pub fn selections(&self) -> &SelectionSet {
        &self.selections
    }

    pub fn selections_mut(&mut self) -> &mut SelectionSet {
        &mut self.selections
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_rpc::types::{Position, Range, TextEdit};
    use std::path::PathBuf;

    fn fake_path(name: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/{name}"))
    }

    #[test]
    fn test_from_text_language_java() {
        let doc = Document::from_text(fake_path("Foo.java"), "class Foo {}".into(), 1);
        assert_eq!(doc.language_id, "java");
        assert_eq!(doc.version, 1);
    }

    #[test]
    fn test_apply_full_text() {
        let mut doc = Document::from_text(fake_path("Main.java"), "old".into(), 0);
        doc.apply_full_text("new content", 1);
        assert_eq!(doc.buffer.to_string(), "new content");
        assert_eq!(doc.version, 1);
    }

    #[test]
    fn test_apply_edits() {
        let mut doc = Document::from_text(fake_path("Test.java"), "hello world\n".into(), 0);
        let edit = TextEdit {
            range: Range {
                start: Position::new(0, 6),
                end: Position::new(0, 11),
            },
            new_text: "Rust".into(),
        };
        doc.apply_edits(&[edit], 1).unwrap();
        assert_eq!(doc.buffer.to_string(), "hello Rust\n");
        assert_eq!(doc.version, 1);
    }

    #[test]
    fn test_uri_round_trip() {
        let path = PathBuf::from("/home/user/project/src/Main.java");
        let uri = Document::path_to_uri(&path);
        assert!(uri.starts_with("file://"));
        let back = Document::uri_to_path(&uri).unwrap();
        assert_eq!(back, path);
    }

    #[test]
    fn test_language_id_for_path() {
        assert_eq!(
            Document::language_id_for_path(Path::new("Foo.java")),
            "java"
        );
        assert_eq!(
            Document::language_id_for_path(Path::new("build.gradle")),
            "groovy"
        );
        assert_eq!(
            Document::language_id_for_path(Path::new("AndroidManifest.xml")),
            "xml"
        );
        assert_eq!(
            Document::language_id_for_path(Path::new("config.txt")),
            "plaintext"
        );
    }
}
