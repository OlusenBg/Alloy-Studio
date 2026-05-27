//! FTC-specific repair engine: maps known build errors to actionable suggestions.

use alloy_rpc::types::BuildError;
use regex::Regex;

/// A human-readable suggestion with an optional automatic patch.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RepairSuggestion {
    /// Unique slug, e.g. `"fix-sdk-version"`.
    pub id: String,
    /// Short human-readable title.
    pub title: String,
    /// Detailed explanation shown to the user.
    pub description: String,
    /// Estimated confidence that this suggestion applies (0.0 – 1.0).
    pub confidence: f32,
    /// The patch to apply (or show to the user).
    pub patch: RepairPatch,
    /// If `true`, the patch can be applied without user confirmation.
    pub auto_applicable: bool,
}

/// A concrete action that fixes (or describes how to fix) a build error.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum RepairPatch {
    /// Edit a file in-place: replace the first occurrence of `search` with `replace`.
    EditFile {
        path: String,
        search: String,
        replace: String,
    },
    /// Run gradle with the given extra arguments.
    GradleCommand { args: Vec<String> },
    /// Show plain-text instructions to the user (manual fix required).
    Instructions(String),
    /// Apply multiple patches in sequence.
    Composite(Vec<RepairPatch>),
}

// ── Internal rule table ───────────────────────────────────────────────────────

struct CompiledRule {
    id: &'static str,
    pattern: Regex,
    title: &'static str,
    description: &'static str,
    confidence: f32,
    auto_applicable: bool,
    make_patch: fn(&str) -> RepairPatch,
}

fn make_rules() -> Vec<CompiledRule> {
    // Each entry: (id, pattern, title, description, confidence, auto_applicable, make_patch)
    let specs: &[(
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        f32,
        bool,
        fn(&str) -> RepairPatch,
    )] = &[
        // Rule 1 — could-not-resolve-sdk
        (
            "could-not-resolve-sdk",
            r"Could not resolve com\.qualcomm\.robotcore:ftc-sdk:([0-9.]+)",
            "FTC SDK version not found",
            "The specified FTC SDK version could not be downloaded. \
             Try pinning to a known-good version.",
            0.9,
            true,
            |_msg| RepairPatch::GradleCommand {
                args: vec!["--refresh-dependencies".into()],
            },
        ),
        // Rule 2 — java-source-compat
        (
            "java-source-compat",
            r"Unsupported class file major version (\d+)",
            "Java compatibility mismatch",
            "Your Java version is too new for the configured source/target compatibility. Add the following to your module build.gradle:\n\nandroid {\n    compileOptions {\n        sourceCompatibility JavaVersion.VERSION_11\n        targetCompatibility JavaVersion.VERSION_11\n    }\n}",
            0.85,
            false,
            |_msg| RepairPatch::Instructions(
                concat!(
                    "In your module build.gradle add:\n\n",
                    "android {\n    compileOptions {\n",
                    "        sourceCompatibility JavaVersion.VERSION_11\n",
                    "        targetCompatibility JavaVersion.VERSION_11\n",
                    "    }\n}"
                ).into(),
            ),
        ),
        // Rule 3 — java-home-not-set
        (
            "java-home-not-set",
            r"(?i)JAVA_HOME not found|No JDK found",
            "JAVA_HOME not configured",
            "Set the JAVA_HOME environment variable to your JDK installation directory.",
            0.95,
            false,
            |_msg| RepairPatch::Instructions(
                "Set JAVA_HOME to your JDK directory:\n\
                 • Linux/macOS: export JAVA_HOME=/path/to/jdk (add to ~/.bashrc or ~/.zshrc)\n\
                 • Windows: set JAVA_HOME=C:\\Path\\To\\JDK in System Environment Variables\n\
                 • Android Studio: use File → Project Structure → SDK Location → JDK location"
                    .into(),
            ),
        ),
        // Rule 4 — gradle-wrapper-missing
        (
            "gradle-wrapper-missing",
            r"(?i)gradlew.*not found|wrapper.*missing",
            "Gradle wrapper not found",
            "The gradlew wrapper script is missing from the project root.",
            0.9,
            false,
            |_msg| RepairPatch::Instructions(
                "Run the following command in the project root to regenerate the wrapper:\n\
                 \n    gradle wrapper --gradle-version 8.0\n\
                 \nIf you do not have Gradle installed, download it from https://gradle.org/install/"
                    .into(),
            ),
        ),
        // Rule 5 — sdk-version-deprecated
        (
            "sdk-version-deprecated",
            r"ftc-sdk:([0-9]+\.[0-9]+)",
            "FTC SDK may be outdated",
            "Consider updating to the latest FTC SDK version (currently 9.2).",
            0.6,
            false,
            |_msg| RepairPatch::Instructions(
                "Update your build.gradle dependency to the latest SDK:\n\
                 \n    implementation 'com.qualcomm.robotcore:ftc-sdk:9.2'\n\
                 \nThen run Gradle Sync."
                    .into(),
            ),
        ),
        // Rule 6 — duplicate-class
        (
            "duplicate-class",
            r"Duplicate class ([\w.]+)",
            "Duplicate class in classpath",
            "A class appears twice in your dependency tree. \
             Check for conflicting library versions.",
            0.75,
            false,
            |_msg| RepairPatch::Instructions(
                concat!(
                    "Add to the root build.gradle `allprojects` block:\n\n",
                    "    configurations.all {\n",
                    "        resolutionStrategy.preferProjectModules()\n",
                    "    }\n\n",
                    "Alternatively, exclude the conflicting module from one of the dependencies."
                ).into(),
            ),
        ),
        // Rule 7 — compile-error-generic
        (
            "compile-error-generic",
            r"error: cannot find symbol",
            "Missing symbol — check imports",
            "A class or method cannot be found. \
             Ensure you have the correct import and the FTC SDK is on the classpath.",
            0.5,
            false,
            |_msg| RepairPatch::Instructions(
                "1. Check that you have the correct import statement at the top of the file.\n\
                 2. Verify that the FTC SDK dependency is present in your module build.gradle.\n\
                 3. Run 'Build → Clean Project' then rebuild."
                    .into(),
            ),
        ),
        // Rule 8 — annotation-processor
        (
            "annotation-processor",
            r"(?i)annotation.*processor.*error|APT error",
            "Annotation processor error",
            "An annotation processor failed during compilation.",
            0.6,
            true,
            |_msg| RepairPatch::GradleCommand {
                args: vec!["--rerun-tasks".into(), "build".into()],
            },
        ),
    ];

    specs
        .iter()
        .map(|(id, pat, title, desc, conf, auto, make_patch)| CompiledRule {
            id,
            pattern: Regex::new(pat).expect("static regex should be valid"),
            title,
            description: desc,
            confidence: *conf,
            auto_applicable: *auto,
            make_patch: *make_patch,
        })
        .collect()
}

// ── RepairEngine ──────────────────────────────────────────────────────────────

/// Diagnoses a list of build errors and returns actionable repair suggestions.
pub struct RepairEngine {
    rules: Vec<CompiledRule>,
}

impl Default for RepairEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RepairEngine {
    /// Create a new engine pre-loaded with all built-in rules.
    pub fn new() -> Self {
        Self {
            rules: make_rules(),
        }
    }

    /// Diagnose a slice of build errors and return deduplicated suggestions.
    pub fn diagnose(&self, errors: &[BuildError]) -> Vec<RepairSuggestion> {
        let mut suggestions: Vec<RepairSuggestion> = Vec::new();
        let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        for error in errors {
            for rule in &self.rules {
                if let Some(suggestion) = self.match_rule(rule, error) {
                    if seen_ids.insert(suggestion.id.clone()) {
                        suggestions.push(suggestion);
                    }
                }
            }
        }

        // Sort by confidence descending
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        suggestions
    }

    fn match_rule(&self, rule: &CompiledRule, error: &BuildError) -> Option<RepairSuggestion> {
        if !rule.pattern.is_match(&error.message) {
            return None;
        }
        Some(RepairSuggestion {
            id: rule.id.to_string(),
            title: rule.title.to_string(),
            description: rule.description.to_string(),
            confidence: rule.confidence,
            patch: (rule.make_patch)(&error.message),
            auto_applicable: rule.auto_applicable,
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_rpc::types::{BuildError, BuildErrorKind};

    fn make_error(message: &str, kind: BuildErrorKind) -> BuildError {
        BuildError {
            file: None,
            line: None,
            column: None,
            message: message.into(),
            kind,
        }
    }

    #[test]
    fn diagnoses_sdk_resolve_error() {
        let engine = RepairEngine::new();
        let errors = vec![make_error(
            "Could not resolve com.qualcomm.robotcore:ftc-sdk:9.1.0",
            BuildErrorKind::Sdk,
        )];
        let suggestions = engine.diagnose(&errors);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].id, "could-not-resolve-sdk");
        assert!(suggestions[0].auto_applicable);
    }

    #[test]
    fn diagnoses_java_compat_error() {
        let engine = RepairEngine::new();
        let errors = vec![make_error(
            "Unsupported class file major version 61",
            BuildErrorKind::Compile,
        )];
        let suggestions = engine.diagnose(&errors);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].id, "java-source-compat");
    }

    #[test]
    fn deduplicates_suggestions() {
        let engine = RepairEngine::new();
        let errors = vec![
            make_error("error: cannot find symbol", BuildErrorKind::Compile),
            make_error("error: cannot find symbol", BuildErrorKind::Compile),
        ];
        let suggestions = engine.diagnose(&errors);
        // Should only appear once
        let count = suggestions.iter().filter(|s| s.id == "compile-error-generic").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn diagnoses_duplicate_class() {
        let engine = RepairEngine::new();
        let errors = vec![make_error(
            "Duplicate class com.example.Foo found in ...",
            BuildErrorKind::Gradle,
        )];
        let suggestions = engine.diagnose(&errors);
        assert!(suggestions.iter().any(|s| s.id == "duplicate-class"));
    }

    #[test]
    fn no_suggestions_for_unrecognised_error() {
        let engine = RepairEngine::new();
        let errors = vec![make_error(
            "some completely unknown error xyz 99999",
            BuildErrorKind::Gradle,
        )];
        let suggestions = engine.diagnose(&errors);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn annotation_processor_patch_is_gradle_command() {
        let engine = RepairEngine::new();
        let errors = vec![make_error(
            "Annotation processor error in processing",
            BuildErrorKind::Compile,
        )];
        let suggestions = engine.diagnose(&errors);
        let s = suggestions.iter().find(|s| s.id == "annotation-processor").unwrap();
        assert!(matches!(s.patch, RepairPatch::GradleCommand { .. }));
        assert!(s.auto_applicable);
    }
}
