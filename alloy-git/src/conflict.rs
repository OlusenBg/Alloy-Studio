//! Conflict file parsing and student-friendly resolution model.
//!
//! Reads a file that contains standard Git conflict markers:
//! ```text
//! <<<<<<< HEAD
//! our code
//! =======
//! their code
//! >>>>>>> branch-name
//! ```
//! and exposes each conflict hunk with both sides, plus resolution helpers.

use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

// ── ConflictSide ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictSide {
    /// Human-readable label extracted from the marker (e.g. `"HEAD"` or a branch name).
    pub label: String,
    /// The text content for this side (may contain multiple lines).
    pub content: String,
}

// ── ConflictHunk ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictHunk {
    /// Zero-based index of this hunk within the file.
    pub index: usize,
    /// 1-based line number of the `<<<<<<<` marker.
    pub start_line: u32,
    /// 1-based line number of the `>>>>>>>` marker.
    pub end_line: u32,
    pub ours: ConflictSide,
    pub theirs: ConflictSide,
}

// ── ConflictFile ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictFile {
    /// Absolute (or repo-relative) path to the file.
    pub path: String,
    pub hunks: Vec<ConflictHunk>,
}

impl ConflictFile {
    /// Parse conflict markers in the file at `path`.
    pub fn parse(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("reading conflict file {}", path.display()))?;

        let hunks = parse_conflict_markers(&text)?;

        Ok(ConflictFile {
            path: path.to_string_lossy().into_owned(),
            hunks,
        })
    }

    /// Return the file content with every conflict resolved by taking our side.
    pub fn resolve_take_ours(&self) -> String {
        let text = std::fs::read_to_string(&self.path).unwrap_or_default();
        let choices: Vec<ResolutionChoice> =
            self.hunks.iter().map(|_| ResolutionChoice::Ours).collect();
        apply_resolutions(&text, &self.hunks, &choices)
    }

    /// Return the file content with every conflict resolved by taking their side.
    pub fn resolve_take_theirs(&self) -> String {
        let text = std::fs::read_to_string(&self.path).unwrap_or_default();
        let choices: Vec<ResolutionChoice> = self
            .hunks
            .iter()
            .map(|_| ResolutionChoice::Theirs)
            .collect();
        apply_resolutions(&text, &self.hunks, &choices)
    }

    /// Return the file content with per-hunk resolution choices applied.
    ///
    /// `choices` must have the same length as `self.hunks`.  Extra choices are
    /// ignored; missing choices default to `Ours`.
    pub fn resolve_mixed(&self, choices: &[ResolutionChoice]) -> String {
        let text = std::fs::read_to_string(&self.path).unwrap_or_default();
        apply_resolutions(&text, &self.hunks, choices)
    }

    /// Overwrite the file with the resolved content.
    pub fn write_resolved(&self, content: &str) -> anyhow::Result<()> {
        std::fs::write(&self.path, content)
            .with_context(|| format!("writing resolved file {}", self.path))?;
        Ok(())
    }
}

// ── ResolutionChoice ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum ResolutionChoice {
    Ours,
    Theirs,
    Custom(String),
}

// ── internal helpers ──────────────────────────────────────────────────────────

/// Parse conflict markers from `text`, returning a `Vec<ConflictHunk>`.
fn parse_conflict_markers(text: &str) -> anyhow::Result<Vec<ConflictHunk>> {
    #[derive(PartialEq)]
    enum State {
        Outside,
        InOurs,
        InTheirs,
    }

    let mut hunks = Vec::new();
    let mut state = State::Outside;

    let mut ours_label = String::new();
    let mut ours_lines: Vec<&str> = Vec::new();
    let mut theirs_lines: Vec<&str> = Vec::new();
    let mut start_line: u32 = 0;
    let mut hunk_index = 0;

    for (idx, line) in text.lines().enumerate() {
        let lineno = (idx + 1) as u32;

        if state == State::Outside {
            if let Some(rest) = line.strip_prefix("<<<<<<<") {
                ours_label = rest.trim().to_string();
                if ours_label.is_empty() {
                    ours_label = "HEAD".to_string();
                }
                start_line = lineno;
                ours_lines.clear();
                theirs_lines.clear();
                state = State::InOurs;
            }
            // Lines outside conflict blocks are left as-is.
        } else if state == State::InOurs {
            if line.starts_with("=======") {
                state = State::InTheirs;
            } else if line.starts_with("<<<<<<<") {
                // Nested conflict — treat as plain text in ours for now.
                ours_lines.push(line);
            } else {
                ours_lines.push(line);
            }
        } else if state == State::InTheirs {
            if let Some(rest) = line.strip_prefix(">>>>>>>") {
                let theirs_label = {
                    let l = rest.trim().to_string();
                    if l.is_empty() {
                        "theirs".to_string()
                    } else {
                        l
                    }
                };

                hunks.push(ConflictHunk {
                    index: hunk_index,
                    start_line,
                    end_line: lineno,
                    ours: ConflictSide {
                        label: ours_label.clone(),
                        content: ours_lines.join("\n"),
                    },
                    theirs: ConflictSide {
                        label: theirs_label.clone(),
                        content: theirs_lines.join("\n"),
                    },
                });

                hunk_index += 1;
                state = State::Outside;
            } else {
                theirs_lines.push(line);
            }
        }
    }

    Ok(hunks)
}

/// Apply `choices` to `text`, replacing each conflict block with the chosen content.
fn apply_resolutions(text: &str, hunks: &[ConflictHunk], choices: &[ResolutionChoice]) -> String {
    // Build a line-index map: line_no (1-based) → line content.
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len() as u32;

    // We iterate through lines and skip/replace conflict blocks.
    let mut out = String::with_capacity(text.len());
    let mut current: u32 = 1; // 1-based

    for (hunk_idx, hunk) in hunks.iter().enumerate() {
        // Emit lines before this hunk.
        while current < hunk.start_line {
            let idx = (current - 1) as usize;
            if idx < lines.len() {
                out.push_str(lines[idx]);
                out.push('\n');
            }
            current += 1;
        }

        // Determine resolved content.
        let choice = choices.get(hunk_idx).unwrap_or(&ResolutionChoice::Ours);
        let resolved = match choice {
            ResolutionChoice::Ours => hunk.ours.content.clone(),
            ResolutionChoice::Theirs => hunk.theirs.content.clone(),
            ResolutionChoice::Custom(s) => s.clone(),
        };

        if !resolved.is_empty() {
            out.push_str(&resolved);
            // Ensure trailing newline.
            if !resolved.ends_with('\n') {
                out.push('\n');
            }
        }

        // Skip past the conflict block (inclusive of the >>>>>>> line).
        current = hunk.end_line + 1;
    }

    // Emit any trailing lines after the last hunk.
    while current <= total {
        let idx = (current - 1) as usize;
        if idx < lines.len() {
            out.push_str(lines[idx]);
            out.push('\n');
        }
        current += 1;
    }

    out
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    const CONFLICT_TEXT: &str = "\
line 1
<<<<<<< HEAD
our code A
our code B
=======
their code X
their code Y
>>>>>>> feature-branch
line 9
";

    fn write_conflict_file(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn parse_finds_one_hunk() {
        let f = write_conflict_file(CONFLICT_TEXT);
        let cf = ConflictFile::parse(f.path()).unwrap();
        assert_eq!(cf.hunks.len(), 1);
    }

    #[test]
    fn parse_extracts_labels() {
        let f = write_conflict_file(CONFLICT_TEXT);
        let cf = ConflictFile::parse(f.path()).unwrap();
        let hunk = &cf.hunks[0];
        assert_eq!(hunk.ours.label, "HEAD");
        assert_eq!(hunk.theirs.label, "feature-branch");
    }

    #[test]
    fn parse_extracts_content() {
        let f = write_conflict_file(CONFLICT_TEXT);
        let cf = ConflictFile::parse(f.path()).unwrap();
        let hunk = &cf.hunks[0];
        assert!(hunk.ours.content.contains("our code A"));
        assert!(hunk.ours.content.contains("our code B"));
        assert!(hunk.theirs.content.contains("their code X"));
        assert!(hunk.theirs.content.contains("their code Y"));
    }

    #[test]
    fn parse_line_numbers() {
        let f = write_conflict_file(CONFLICT_TEXT);
        let cf = ConflictFile::parse(f.path()).unwrap();
        let hunk = &cf.hunks[0];
        // `<<<<<<<` is on line 2, `>>>>>>>` is on line 8.
        assert_eq!(hunk.start_line, 2);
        assert_eq!(hunk.end_line, 8);
    }

    #[test]
    fn resolve_take_ours_keeps_ours() {
        let f = write_conflict_file(CONFLICT_TEXT);
        let cf = ConflictFile::parse(f.path()).unwrap();
        let resolved = cf.resolve_take_ours();
        assert!(resolved.contains("our code A"));
        assert!(!resolved.contains("their code X"));
        assert!(resolved.contains("line 1"));
        assert!(resolved.contains("line 9"));
    }

    #[test]
    fn resolve_take_theirs_keeps_theirs() {
        let f = write_conflict_file(CONFLICT_TEXT);
        let cf = ConflictFile::parse(f.path()).unwrap();
        let resolved = cf.resolve_take_theirs();
        assert!(!resolved.contains("our code A"));
        assert!(resolved.contains("their code X"));
    }

    #[test]
    fn resolve_mixed_custom() {
        let f = write_conflict_file(CONFLICT_TEXT);
        let cf = ConflictFile::parse(f.path()).unwrap();
        let choices = vec![ResolutionChoice::Custom("custom resolution\n".to_string())];
        let resolved = cf.resolve_mixed(&choices);
        assert!(resolved.contains("custom resolution"));
        assert!(!resolved.contains("our code A"));
        assert!(!resolved.contains("their code X"));
    }

    #[test]
    fn parse_two_hunks() {
        let text = "\
line 1
<<<<<<< HEAD
ours 1
=======
theirs 1
>>>>>>> branch
line 6
<<<<<<< HEAD
ours 2
=======
theirs 2
>>>>>>> branch
line 12
";
        let f = write_conflict_file(text);
        let cf = ConflictFile::parse(f.path()).unwrap();
        assert_eq!(cf.hunks.len(), 2);
        assert_eq!(cf.hunks[0].index, 0);
        assert_eq!(cf.hunks[1].index, 1);
    }

    #[test]
    fn parse_no_hunks_clean_file() {
        let text = "just a clean file\nno conflicts here\n";
        let f = write_conflict_file(text);
        let cf = ConflictFile::parse(f.path()).unwrap();
        assert!(cf.hunks.is_empty());
    }
}
