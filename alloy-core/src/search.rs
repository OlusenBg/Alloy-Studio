//! Text search over buffers and the entire workspace file tree.

use crate::buffer::{Buffer, EditOp};
use alloy_rpc::types::{Position, Range};
use regex::Regex;

// ---------------------------------------------------------------------------
// SearchMatch
// ---------------------------------------------------------------------------

/// A single search result, referencing a location inside a document.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchMatch {
    /// The document URI in which the match was found.
    pub uri: String,
    /// Zero-based line number.
    pub line: u32,
    /// Zero-based start column (character offset on the line).
    pub col: u32,
    /// Zero-based end column (exclusive).
    pub end_col: u32,
    /// The full text of the matching line, for context display.
    pub line_text: String,
    /// The LSP-style range of the match.
    pub range: Range,
}

// ---------------------------------------------------------------------------
// BufferSearcher
// ---------------------------------------------------------------------------

/// Searches and replaces text within a single [`Buffer`].
pub struct BufferSearcher;

impl BufferSearcher {
    /// Search for `pattern` in `buf`.
    ///
    /// When `is_regex` is `false` the pattern is treated as a plain literal
    /// string (special regex characters are escaped).
    pub fn find(
        buf: &Buffer,
        uri: &str,
        pattern: &str,
        is_regex: bool,
    ) -> anyhow::Result<Vec<SearchMatch>> {
        if pattern.is_empty() {
            return Ok(Vec::new());
        }

        let re = build_regex(pattern, is_regex)?;
        let text = buf.to_string();
        let mut matches = Vec::new();

        for (line_idx, line_text) in text.lines().enumerate() {
            for mat in re.find_iter(line_text) {
                let col = mat.start() as u32;
                let end_col = mat.end() as u32;

                let start = Position {
                    line: line_idx as u32,
                    character: col,
                };
                let end = Position {
                    line: line_idx as u32,
                    character: end_col,
                };

                matches.push(SearchMatch {
                    uri: uri.to_owned(),
                    line: line_idx as u32,
                    col,
                    end_col,
                    line_text: line_text.to_owned(),
                    range: Range { start, end },
                });
            }
        }

        Ok(matches)
    }

    /// Replace all occurrences of `pattern` with `replacement` in `buf`.
    ///
    /// Applies each replacement as an individual [`EditOp`] in **reverse**
    /// order (highest offset first) so earlier offsets stay valid.
    ///
    /// Returns the number of replacements made.
    pub fn replace(
        buf: &mut Buffer,
        pattern: &str,
        replacement: &str,
        is_regex: bool,
    ) -> anyhow::Result<usize> {
        if pattern.is_empty() {
            return Ok(0);
        }

        let re = build_regex(pattern, is_regex)?;

        // Collect all (byte_start, byte_end, replacement_text) tuples.
        let text = buf.to_string();
        let mut ops: Vec<(usize, usize, String)> = Vec::new();

        for mat in re.find_iter(&text) {
            let replacement_text = if is_regex {
                // For regex mode expand back-references.
                let mut dest = String::new();
                if let Some(caps) = re.captures(&text[mat.start()..mat.end()]) {
                    caps.expand(replacement, &mut dest);
                }
                dest
            } else {
                replacement.to_owned()
            };
            ops.push((mat.start(), mat.end(), replacement_text));
        }

        let count = ops.len();

        // Apply in reverse order.
        for (byte_start, byte_end, new_text) in ops.into_iter().rev() {
            let old_text = buf.slice_bytes(byte_start, byte_end);
            buf.apply_edit(EditOp {
                byte_start,
                byte_end,
                old_text,
                new_text,
            });
        }

        Ok(count)
    }
}

// ---------------------------------------------------------------------------
// WorkspaceSearcher
// ---------------------------------------------------------------------------

/// Searches for a pattern across all readable files in a directory tree.
pub struct WorkspaceSearcher;

impl WorkspaceSearcher {
    /// Walk `root` recursively, searching every text file for `pattern`.
    ///
    /// - `file_glob` optionally restricts results to filenames matching a
    ///   simple glob (e.g. `"*.java"`).  When `None`, all files are searched.
    /// - Binary files (files containing a NUL byte in their first 8 KiB) are
    ///   silently skipped.
    /// - Directories named `.git`, `build`, or `.gradle` are skipped.
    ///
    /// The heavy work is offloaded to [`tokio::task::spawn_blocking`].
    pub async fn search(
        root: &std::path::Path,
        pattern: &str,
        is_regex: bool,
        file_glob: Option<&str>,
    ) -> anyhow::Result<Vec<SearchMatch>> {
        let root = root.to_path_buf();
        let pattern = pattern.to_owned();
        let file_glob = file_glob.map(|s| s.to_owned());

        tokio::task::spawn_blocking(move || {
            search_blocking(&root, &pattern, is_regex, file_glob.as_deref())
        })
        .await
        .map_err(|e| anyhow::anyhow!("workspace search task panicked: {e}"))?
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a [`Regex`], optionally escaping the pattern for literal matching.
fn build_regex(pattern: &str, is_regex: bool) -> anyhow::Result<Regex> {
    let raw = if is_regex {
        pattern.to_owned()
    } else {
        regex::escape(pattern)
    };
    Regex::new(&raw).map_err(|e| anyhow::anyhow!("invalid regex pattern '{pattern}': {e}"))
}

/// Directories to skip during workspace search.
fn is_ignored_dir(name: &str) -> bool {
    matches!(
        name,
        ".git" | "build" | ".gradle" | ".idea" | "node_modules" | "__pycache__"
    )
}

/// The blocking work for [`WorkspaceSearcher::search`].
fn search_blocking(
    root: &std::path::Path,
    pattern: &str,
    is_regex: bool,
    file_glob: Option<&str>,
) -> anyhow::Result<Vec<SearchMatch>> {
    if pattern.is_empty() {
        return Ok(Vec::new());
    }

    let re = build_regex(pattern, is_regex)?;
    let glob_re: Option<Regex> = file_glob
        .map(|g| {
            // Convert a simple glob (* → .*, ? → .) to a regex.
            let escaped = regex::escape(g).replace(r"\*", ".*").replace(r"\?", ".");
            Regex::new(&format!("^{escaped}$"))
                .map_err(|e| anyhow::anyhow!("invalid glob '{g}': {e}"))
        })
        .transpose()?;

    let mut results: Vec<SearchMatch> = Vec::new();

    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !is_ignored_dir(name);
                }
            }
            true
        });

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                tracing::warn!("workspace search walk error: {err}");
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Apply file-name glob filter.
        if let Some(ref g) = glob_re {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !g.is_match(name) {
                continue;
            }
        }

        // Read the file, skipping anything that can't be read.
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(_) => continue,
        };

        // Skip binary files: check first 8 KiB for a NUL byte.
        let probe = &bytes[..bytes.len().min(8192)];
        if probe.contains(&0u8) {
            continue;
        }

        let text = match std::str::from_utf8(&bytes) {
            Ok(t) => t,
            Err(_) => continue, // skip non-UTF-8 files
        };

        let uri = format!("file://{}", path.display());

        for (line_idx, line_text) in text.lines().enumerate() {
            for mat in re.find_iter(line_text) {
                let col = mat.start() as u32;
                let end_col = mat.end() as u32;
                results.push(SearchMatch {
                    uri: uri.clone(),
                    line: line_idx as u32,
                    col,
                    end_col,
                    line_text: line_text.to_owned(),
                    range: Range {
                        start: Position {
                            line: line_idx as u32,
                            character: col,
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: end_col,
                        },
                    },
                });
            }
        }
    }

    Ok(results)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Buffer;

    // --- BufferSearcher -------------------------------------------------------

    #[test]
    fn test_find_literal() {
        let buf = Buffer::from_str("hello world\nhello rust\n");
        let hits = BufferSearcher::find(&buf, "file:///test.txt", "hello", false).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].line, 0);
        assert_eq!(hits[0].col, 0);
        assert_eq!(hits[1].line, 1);
    }

    #[test]
    fn test_find_regex() {
        let buf = Buffer::from_str("foo123\nbar456\nbaz\n");
        let hits = BufferSearcher::find(&buf, "file:///f.txt", r"\d+", true).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].line, 0);
        assert_eq!(hits[1].line, 1);
    }

    #[test]
    fn test_find_empty_pattern() {
        let buf = Buffer::from_str("hello");
        let hits = BufferSearcher::find(&buf, "file:///f.txt", "", false).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn test_find_no_match() {
        let buf = Buffer::from_str("hello world\n");
        let hits = BufferSearcher::find(&buf, "file:///f.txt", "rust", false).unwrap();
        assert!(hits.is_empty());
    }

    #[test]
    fn test_replace_literal() {
        let mut buf = Buffer::from_str("foo bar foo\n");
        let count = BufferSearcher::replace(&mut buf, "foo", "baz", false).unwrap();
        assert_eq!(count, 2);
        assert_eq!(buf.to_string(), "baz bar baz\n");
    }

    #[test]
    fn test_replace_regex() {
        let mut buf = Buffer::from_str("hello 123 world 456\n");
        let count = BufferSearcher::replace(&mut buf, r"\d+", "NUM", true).unwrap();
        assert_eq!(count, 2);
        assert_eq!(buf.to_string(), "hello NUM world NUM\n");
    }

    #[test]
    fn test_replace_no_match() {
        let mut buf = Buffer::from_str("hello world\n");
        let count = BufferSearcher::replace(&mut buf, "xyz", "ABC", false).unwrap();
        assert_eq!(count, 0);
        assert_eq!(buf.to_string(), "hello world\n");
    }

    #[test]
    fn test_find_literal_special_chars() {
        let buf = Buffer::from_str("a.b.c\n");
        // Without regex the dot should be a literal dot, not a wildcard.
        let hits = BufferSearcher::find(&buf, "file:///f.txt", "a.b", false).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].col, 0);
    }

    // --- WorkspaceSearcher ---------------------------------------------------

    #[tokio::test]
    async fn test_workspace_search_literal() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.java"), b"hello world\nhello rust\n").unwrap();
        std::fs::write(tmp.path().join("b.java"), b"goodbye world\n").unwrap();

        let results = WorkspaceSearcher::search(tmp.path(), "hello", false, None)
            .await
            .unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_workspace_search_glob_filter() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Foo.java"), b"hello\n").unwrap();
        std::fs::write(tmp.path().join("Foo.txt"), b"hello\n").unwrap();

        let results = WorkspaceSearcher::search(tmp.path(), "hello", false, Some("*.java"))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].uri.ends_with("Foo.java"));
    }

    #[tokio::test]
    async fn test_workspace_search_skips_binary() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Write a file with a NUL byte — should be skipped.
        std::fs::write(tmp.path().join("binary.bin"), b"hel\x00lo").unwrap();
        std::fs::write(tmp.path().join("text.txt"), b"hello\n").unwrap();

        let results = WorkspaceSearcher::search(tmp.path(), "hello", false, None)
            .await
            .unwrap();
        // Only the text file should be matched.
        assert_eq!(results.len(), 1);
        assert!(results[0].uri.ends_with("text.txt"));
    }
}
