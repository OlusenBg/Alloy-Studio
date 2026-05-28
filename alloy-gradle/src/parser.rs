use alloy_rpc::types::{BuildError, BuildErrorKind, BuildEvent};
use once_cell::sync::Lazy;
use regex::Regex;

// ── Regex patterns ────────────────────────────────────────────────────────────

static COMPILE_ERROR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^(.+\.(?:java|kt)):\s*(\d+):\s*error:\s*(.+)$").unwrap());

static DEP_RESOLVE_ERROR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"Could not resolve ([^\s:]+:[^\s:]+:[^\s]+)").unwrap());

static SDK_VERSION_ERROR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"ftc-sdk:([0-9]+\.[0-9]+(?:\.[0-9]+)?)").unwrap());

static GRADLE_FAILURE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^FAILURE:|^BUILD FAILED").unwrap());

static GRADLE_SUCCESS: Lazy<Regex> = Lazy::new(|| Regex::new(r"^BUILD SUCCESSFUL").unwrap());

static TASK_LINE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^> Task :([^\s]+)").unwrap());

// ── Parser ────────────────────────────────────────────────────────────────────

/// Stateful line-by-line parser for Gradle build output.
pub struct BuildOutputParser {
    errors: Vec<BuildError>,
    current_task: Option<String>,
    in_failure_section: bool,
}

impl Default for BuildOutputParser {
    fn default() -> Self {
        Self::new()
    }
}

impl BuildOutputParser {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            current_task: None,
            in_failure_section: false,
        }
    }

    /// Feed one line of output. Returns `Some(BuildEvent::ErrorDetected)` when an
    /// error pattern is recognised, `None` otherwise.
    pub fn feed_line(&mut self, line: &str) -> Option<BuildEvent> {
        // Track current task
        if let Some(caps) = TASK_LINE.captures(line) {
            self.current_task = Some(caps[1].to_string());
        }

        // Track failure section
        if GRADLE_FAILURE.is_match(line) {
            self.in_failure_section = true;
        }
        if GRADLE_SUCCESS.is_match(line) {
            self.in_failure_section = false;
        }

        // ── Compile error: File.java:42: error: message ──
        if let Some(caps) = COMPILE_ERROR.captures(line) {
            let file = caps[1].to_string();
            let line_num: u32 = caps[2].parse().unwrap_or(0);
            let message = caps[3].to_string();
            let error = BuildError {
                file: Some(file),
                line: Some(line_num),
                column: None,
                message,
                kind: BuildErrorKind::Compile,
            };
            self.errors.push(error.clone());
            return Some(BuildEvent::ErrorDetected(error));
        }

        // ── SDK version in a dependency-resolve error ──
        // (check this before generic dep resolve so we can set kind = Sdk)
        if let Some(caps) = DEP_RESOLVE_ERROR.captures(line) {
            let dep = caps[1].to_string();
            let kind = if SDK_VERSION_ERROR.is_match(&dep) {
                BuildErrorKind::Sdk
            } else {
                BuildErrorKind::Dependency
            };
            let message = format!("Could not resolve {dep}");
            let error = BuildError {
                file: None,
                line: None,
                column: None,
                message,
                kind,
            };
            self.errors.push(error.clone());
            return Some(BuildEvent::ErrorDetected(error));
        }

        // ── Generic Gradle failure marker (no separate pattern matched) ──
        if self.in_failure_section && line.starts_with("* What went wrong:") {
            // Next few lines will contain the actual message; mark state only.
        }

        None
    }

    /// Consume the parser and return the collected errors.
    pub fn finish(self) -> Vec<BuildError> {
        self.errors
    }

    /// Borrow the currently collected errors.
    pub fn errors(&self) -> &[BuildError] {
        &self.errors
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_rpc::types::BuildErrorKind;

    #[test]
    fn parses_java_compile_error() {
        let mut parser = BuildOutputParser::new();
        let line = "TeamCode/src/main/java/org/firstinspires/ftc/teamcode/Auto.java: 42: error: cannot find symbol";
        let event = parser.feed_line(line);
        assert!(event.is_some(), "should detect compile error");
        match event.unwrap() {
            BuildEvent::ErrorDetected(e) => {
                assert_eq!(e.kind, BuildErrorKind::Compile);
                assert_eq!(e.line, Some(42));
                assert!(e.file.is_some());
            }
            other => panic!("unexpected event: {other:?}"),
        }
        assert_eq!(parser.errors().len(), 1);
    }

    #[test]
    fn parses_kotlin_compile_error() {
        let mut parser = BuildOutputParser::new();
        let line = "src/main/java/org/firstinspires/ftc/teamcode/Drive.kt: 10: error: unresolved reference: HardwareMap";
        let event = parser.feed_line(line);
        assert!(event.is_some());
        if let Some(BuildEvent::ErrorDetected(e)) = event {
            assert_eq!(e.kind, BuildErrorKind::Compile);
        }
    }

    #[test]
    fn parses_dependency_resolve_error() {
        let mut parser = BuildOutputParser::new();
        let line = "Could not resolve org.firstinspires:ftc-sdk:9.2.0";
        let event = parser.feed_line(line);
        assert!(event.is_some());
        if let Some(BuildEvent::ErrorDetected(e)) = event {
            assert_eq!(e.kind, BuildErrorKind::Sdk);
        }
    }

    #[test]
    fn parses_generic_dep_error() {
        let mut parser = BuildOutputParser::new();
        let line = "Could not resolve com.example:some-lib:1.0.0";
        let event = parser.feed_line(line);
        assert!(event.is_some());
        if let Some(BuildEvent::ErrorDetected(e)) = event {
            assert_eq!(e.kind, BuildErrorKind::Dependency);
        }
    }

    #[test]
    fn tracks_task_lines() {
        let mut parser = BuildOutputParser::new();
        parser.feed_line("> Task :TeamCode:compileDebugJavaWithJavac");
        assert_eq!(
            parser.current_task.as_deref(),
            Some("TeamCode:compileDebugJavaWithJavac")
        );
    }

    #[test]
    fn finish_returns_all_errors() {
        let mut parser = BuildOutputParser::new();
        parser.feed_line("src/Foo.java: 1: error: first error");
        parser.feed_line("src/Bar.java: 2: error: second error");
        let errors = parser.finish();
        assert_eq!(errors.len(), 2);
    }
}
