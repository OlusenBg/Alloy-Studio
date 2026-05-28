use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;
use regex::Regex;

static SDK_VERSION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"ftc-sdk:([0-9]+\.[0-9]+(?:\.[0-9]+)?)").unwrap());

static FTC_MARKER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:com\.qualcomm\.robotcore|FtcRobotController|TeamCode)").unwrap());

/// A detected FTC project rooted at a directory containing gradlew.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FtcProject {
    pub root: PathBuf,
    /// TeamCode/src/main/java or similar
    pub team_code_dir: PathBuf,
    /// root build.gradle
    pub build_gradle: PathBuf,
    pub local_properties: Option<PathBuf>,
    /// e.g. "9.2"
    pub sdk_version: Option<String>,
    pub gradlew: PathBuf,
    pub has_kotlin: bool,
}

impl FtcProject {
    /// Walk up from `start` (up to 6 levels) looking for an FTC project root.
    /// An FTC project root is a directory that contains either:
    ///   - a `build.gradle` that mentions com.qualcomm.robotcore or FtcRobotController
    ///   - a `settings.gradle` that includes TeamCode
    ///   - a `gradlew` file alongside FTC-specific sub-directories
    pub fn detect(start: &Path) -> anyhow::Result<Self> {
        let mut candidate = start.to_path_buf();
        for _ in 0..7 {
            if Self::looks_like_ftc_root(&candidate) {
                return Self::from_root(candidate);
            }
            match candidate.parent() {
                Some(p) => candidate = p.to_path_buf(),
                None => break,
            }
        }
        Err(crate::error::GradleError::NotFtcProject {
            path: start.display().to_string(),
        }
        .into())
    }

    fn looks_like_ftc_root(dir: &Path) -> bool {
        // Must have gradlew
        if !dir.join("gradlew").exists() {
            return false;
        }
        // Check build.gradle
        let build_gradle = dir.join("build.gradle");
        if build_gradle.exists() {
            if let Ok(content) = std::fs::read_to_string(&build_gradle) {
                if FTC_MARKER_RE.is_match(&content) {
                    return true;
                }
            }
        }
        // Check settings.gradle
        let settings_gradle = dir.join("settings.gradle");
        if settings_gradle.exists() {
            if let Ok(content) = std::fs::read_to_string(&settings_gradle) {
                if content.contains("TeamCode") || content.contains("FtcRobotController") {
                    return true;
                }
            }
        }
        // Check for typical FTC directory structure
        if dir.join("TeamCode").exists() || dir.join("FtcRobotController").exists() {
            return true;
        }
        false
    }

    fn from_root(root: PathBuf) -> anyhow::Result<Self> {
        let build_gradle = root.join("build.gradle");
        let gradlew = root.join("gradlew");

        if !gradlew.exists() {
            return Err(crate::error::GradleError::GradlewNotFound {
                path: root.display().to_string(),
            }
            .into());
        }

        let sdk_version = Self::parse_sdk_version(&build_gradle);
        let team_code_dir = Self::find_team_code_dir(&root);
        let has_kotlin = Self::has_kotlin(&team_code_dir);

        let local_properties = {
            let p = root.join("local.properties");
            if p.exists() {
                Some(p)
            } else {
                None
            }
        };

        Ok(Self {
            build_gradle,
            gradlew,
            sdk_version,
            team_code_dir,
            has_kotlin,
            local_properties,
            root,
        })
    }

    /// Parse build.gradle for the ftc-sdk version string.
    fn parse_sdk_version(build_gradle: &Path) -> Option<String> {
        let content = std::fs::read_to_string(build_gradle).ok()?;
        SDK_VERSION_RE
            .captures(&content)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }

    /// Find the TeamCode source directory. Tries several canonical paths.
    fn find_team_code_dir(root: &Path) -> PathBuf {
        let candidates = [
            root.join("TeamCode").join("src").join("main").join("java"),
            root.join("FtcRobotController")
                .join("TeamCode")
                .join("src")
                .join("main")
                .join("java"),
            root.join("src").join("main").join("java"),
        ];
        for c in &candidates {
            if c.exists() {
                return c.clone();
            }
        }
        // Return first candidate as best-guess even if it doesn't exist
        candidates[0].clone()
    }

    /// Returns true if any .kt files exist under team_code_dir.
    fn has_kotlin(team_code_dir: &Path) -> bool {
        if !team_code_dir.exists() {
            return false;
        }
        walkdir::WalkDir::new(team_code_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .any(|e| e.path().extension().map(|x| x == "kt").unwrap_or(false))
    }

    pub fn gradlew_path(&self) -> &Path {
        &self.gradlew
    }

    /// Returns true if gradlew exists and is executable (on Unix).
    pub fn is_valid(&self) -> bool {
        if !self.gradlew.exists() {
            return false;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = std::fs::metadata(&self.gradlew) {
                return meta.permissions().mode() & 0o111 != 0;
            }
            false
        }
        #[cfg(not(unix))]
        true
    }
}
