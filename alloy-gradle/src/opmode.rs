//! OpMode scanner — walks TeamCode directories and extracts `@TeleOp`/`@Autonomous` metadata.

use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ── Patterns ──────────────────────────────────────────────────────────────────

static TELEOP_ANNOTATION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"@TeleOp\s*\(\s*name\s*=\s*"([^"]+)"(?:\s*,\s*group\s*=\s*"([^"]+)")?\s*\)"#,
    )
    .unwrap()
});

static AUTONOMOUS_ANNOTATION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"@Autonomous\s*\(\s*name\s*=\s*"([^"]+)"(?:\s*,\s*group\s*=\s*"([^"]+)")?\s*\)"#,
    )
    .unwrap()
});

static CLASS_DECLARATION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\bclass\s+(\w+)\s+extends\s+\w*(?:Linear)?OpMode\b").unwrap()
});

static DISABLED_ANNOTATION: Lazy<Regex> = Lazy::new(|| Regex::new(r"@Disabled").unwrap());

// ── Types ─────────────────────────────────────────────────────────────────────

/// Whether an OpMode is driver-controlled or autonomous.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum OpModeKind {
    TeleOp,
    Autonomous,
}

/// Information about a single discovered OpMode.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpModeInfo {
    /// Java/Kotlin class name (e.g. `"MyTeleOp"`).
    pub class_name: String,
    /// Name shown in the robot controller driver station.
    pub display_name: String,
    pub kind: OpModeKind,
    /// Optional group tag from the annotation.
    pub group: Option<String>,
    /// `true` if `@Disabled` is present.
    pub is_disabled: bool,
    /// Absolute path to the source file.
    pub file: PathBuf,
    /// 1-based line number of the annotation.
    pub line: u32,
    /// Java package from `package foo.bar;` declaration.
    pub package: Option<String>,
}

// ── Scanner ───────────────────────────────────────────────────────────────────

pub struct OpModeScanner;

impl OpModeScanner {
    /// Walk `team_code_dir` recursively and return all discovered OpModes.
    pub fn scan(team_code_dir: &Path) -> anyhow::Result<Vec<OpModeInfo>> {
        let mut results = Vec::new();

        for entry in WalkDir::new(team_code_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "java" && ext != "kt" {
                continue;
            }
            match Self::scan_file(path) {
                Ok(mut found) => results.append(&mut found),
                Err(e) => {
                    tracing::warn!("Failed to scan {:?}: {}", path, e);
                }
            }
        }

        Ok(results)
    }

    /// Scan a single Java or Kotlin source file for OpMode annotations.
    fn scan_file(path: &Path) -> anyhow::Result<Vec<OpModeInfo>> {
        let content = std::fs::read_to_string(path)?;
        let mut results = Vec::new();

        // Extract package declaration
        let package = Self::extract_package(&content);

        // Collect annotations with their byte offsets and line numbers
        let annotations = Self::collect_annotations(&content);

        // Find the class declaration (we take the first one as the "owner")
        // If none, no results.
        for (kind, display_name, group, ann_byte_offset) in annotations {
            let line = Self::byte_offset_to_line(&content, ann_byte_offset);

            // Look for @Disabled within a 3-line window before the class declaration
            // or anywhere before the class declaration after this annotation.
            let after_annotation = &content[ann_byte_offset..];
            let is_disabled = {
                // Check the region between this annotation and the class keyword
                let class_pos = CLASS_DECLARATION
                    .find(after_annotation)
                    .map(|m| m.start())
                    .unwrap_or(after_annotation.len());
                let between = &after_annotation[..class_pos.min(after_annotation.len())];
                DISABLED_ANNOTATION.is_match(between)
                    // also check the few lines before the annotation
                    || {
                        let before = &content[..ann_byte_offset];
                        // only check up to 5 lines back
                        let recent_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
                        let recent_start2 = if recent_start > 0 {
                            content[..recent_start - 1]
                                .rfind('\n')
                                .map(|i| i + 1)
                                .unwrap_or(0)
                        } else {
                            0
                        };
                        DISABLED_ANNOTATION.is_match(&content[recent_start2..ann_byte_offset])
                    }
            };

            // Find the class name from the class declaration following this annotation
            let class_name = CLASS_DECLARATION
                .captures(after_annotation)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| {
                    // Fall back to the file stem
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown")
                        .to_string()
                });

            results.push(OpModeInfo {
                class_name,
                display_name,
                kind,
                group,
                is_disabled,
                file: path.to_path_buf(),
                line,
                package: package.clone(),
            });
        }

        Ok(results)
    }

    /// Collect all `@TeleOp` and `@Autonomous` annotations as
    /// `(kind, display_name, group, byte_offset)`.
    fn collect_annotations(
        content: &str,
    ) -> Vec<(OpModeKind, String, Option<String>, usize)> {
        let mut out = Vec::new();

        for caps in TELEOP_ANNOTATION.captures_iter(content) {
            let m = caps.get(0).unwrap();
            let display_name = caps[1].to_string();
            let group = caps.get(2).map(|g| g.as_str().to_string());
            out.push((OpModeKind::TeleOp, display_name, group, m.start()));
        }

        for caps in AUTONOMOUS_ANNOTATION.captures_iter(content) {
            let m = caps.get(0).unwrap();
            let display_name = caps[1].to_string();
            let group = caps.get(2).map(|g| g.as_str().to_string());
            out.push((OpModeKind::Autonomous, display_name, group, m.start()));
        }

        // Sort by byte offset so we process them in file order
        out.sort_by_key(|&(_, _, _, offset)| offset);
        out
    }

    /// Convert a byte offset into a 1-based line number.
    fn byte_offset_to_line(content: &str, offset: usize) -> u32 {
        let safe_offset = offset.min(content.len());
        content[..safe_offset].chars().filter(|&c| c == '\n').count() as u32 + 1
    }

    /// Extract the `package foo.bar.baz;` declaration from Java/Kotlin source.
    fn extract_package(content: &str) -> Option<String> {
        static PACKAGE_RE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"^\s*package\s+([\w.]+)").unwrap());
        PACKAGE_RE
            .captures(content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_java(content: &str) -> NamedTempFile {
        let mut f = tempfile::Builder::new()
            .suffix(".java")
            .tempfile()
            .unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn scans_teleop_annotation() {
        let content = r#"
package org.firstinspires.ftc.teamcode;

import com.qualcomm.robotcore.eventloop.opmode.TeleOp;

@TeleOp(name = "My Drive", group = "Drive")
public class MyDrive extends LinearOpMode {
    public void runOpMode() {}
}
"#;
        let file = write_java(content);
        let opmodes = OpModeScanner::scan_file(file.path()).unwrap();
        assert_eq!(opmodes.len(), 1);
        let op = &opmodes[0];
        assert_eq!(op.display_name, "My Drive");
        assert_eq!(op.kind, OpModeKind::TeleOp);
        assert_eq!(op.group.as_deref(), Some("Drive"));
        assert!(!op.is_disabled);
        assert_eq!(op.package.as_deref(), Some("org.firstinspires.ftc.teamcode"));
    }

    #[test]
    fn scans_autonomous_annotation() {
        let content = r#"
package org.firstinspires.ftc.teamcode;

@Autonomous(name = "Red Auto", group = "Auto")
public class RedAuto extends OpMode {
    public void init() {}
    public void loop() {}
}
"#;
        let file = write_java(content);
        let opmodes = OpModeScanner::scan_file(file.path()).unwrap();
        assert_eq!(opmodes.len(), 1);
        let op = &opmodes[0];
        assert_eq!(op.display_name, "Red Auto");
        assert_eq!(op.kind, OpModeKind::Autonomous);
    }

    #[test]
    fn scans_disabled_opmode() {
        let content = r#"
@Disabled
@TeleOp(name = "Debug TeleOp")
public class DebugTeleOp extends LinearOpMode {
    public void runOpMode() {}
}
"#;
        let file = write_java(content);
        let opmodes = OpModeScanner::scan_file(file.path()).unwrap();
        assert_eq!(opmodes.len(), 1);
        assert!(opmodes[0].is_disabled);
    }

    #[test]
    fn scans_multiple_opmodes_in_one_file() {
        // While unusual, two inner-class opmodes can exist in one file.
        let content = r#"
@TeleOp(name = "TeleOp A")
class TeleOpA extends LinearOpMode { public void runOpMode() {} }

@Autonomous(name = "Auto B")
class AutoB extends LinearOpMode { public void runOpMode() {} }
"#;
        let file = write_java(content);
        let opmodes = OpModeScanner::scan_file(file.path()).unwrap();
        assert_eq!(opmodes.len(), 2);
        assert_eq!(opmodes[0].kind, OpModeKind::TeleOp);
        assert_eq!(opmodes[1].kind, OpModeKind::Autonomous);
    }

    #[test]
    fn line_numbers_are_correct() {
        let content = "package foo;\n\n@TeleOp(name = \"X\")\npublic class X extends OpMode {}\n";
        let file = write_java(content);
        let opmodes = OpModeScanner::scan_file(file.path()).unwrap();
        assert_eq!(opmodes.len(), 1);
        // Annotation is on line 3
        assert_eq!(opmodes[0].line, 3);
    }

    #[test]
    fn no_opmodes_in_plain_class() {
        let content = r#"
public class Helper {
    public static void doSomething() {}
}
"#;
        let file = write_java(content);
        let opmodes = OpModeScanner::scan_file(file.path()).unwrap();
        assert!(opmodes.is_empty());
    }
}
