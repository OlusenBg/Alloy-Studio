//! FTC SDK locator and JDTLS configuration builder.
//!
//! Responsible for:
//! * Finding the Android SDK and FTC-SDK JAR files on the developer's machine.
//! * Generating the JDTLS launch arguments and initialization options JSON.
//! * Writing `.settings/org.eclipse.jdt.core.prefs` into the project.

use std::path::{Path, PathBuf};

use anyhow::Context;
use tracing::debug;

// ── FtcSdkLocator ────────────────────────────────────────────────────────────

/// Utility struct for locating the Android SDK and FTC-SDK artefacts.
pub struct FtcSdkLocator;

impl FtcSdkLocator {
    /// Try to find the Android SDK root.
    ///
    /// Search order:
    /// 1. `sdk.dir` in `<project_root>/local.properties`
    /// 2. `ANDROID_HOME` environment variable
    /// 3. `ANDROID_SDK_ROOT` environment variable
    /// 4. Common platform-specific locations
    pub fn find_android_sdk(project_root: &Path) -> Option<PathBuf> {
        // 1. local.properties
        if let Some(p) = Self::read_local_properties(project_root) {
            if p.exists() {
                return Some(p);
            }
        }

        // 2. ANDROID_HOME
        if let Ok(val) = std::env::var("ANDROID_HOME") {
            let p = PathBuf::from(val);
            if p.exists() {
                return Some(p);
            }
        }

        // 3. ANDROID_SDK_ROOT
        if let Ok(val) = std::env::var("ANDROID_SDK_ROOT") {
            let p = PathBuf::from(val);
            if p.exists() {
                return Some(p);
            }
        }

        // 4. Common locations
        let candidates: Vec<PathBuf> = {
            let mut v = Vec::new();

            // Linux
            if let Some(home) = dirs_next::home_dir() {
                v.push(home.join("Android").join("Sdk"));
                // macOS
                v.push(home.join("Library").join("Android").join("sdk"));
                // Windows
                v.push(
                    home.join("AppData")
                        .join("Local")
                        .join("Android")
                        .join("Sdk"),
                );
            }
            v
        };

        candidates.into_iter().find(|c| c.exists())
    }

    /// Find FTC SDK JARs in `<project_root>/libs/` and any sub-directories.
    ///
    /// Matches: `FtcRobotController*.jar`, `RobotCore*.jar`, `Hardware*.jar`,
    /// `Inspection*.jar`, `ftc-sdk*.jar`, and any JAR in the `libs` tree.
    pub fn find_ftc_jars(project_root: &Path) -> Vec<PathBuf> {
        let libs_dir = project_root.join("libs");
        if !libs_dir.exists() {
            return Vec::new();
        }

        let mut jars = Vec::new();

        let walker = walkdir::WalkDir::new(&libs_dir)
            .follow_links(true)
            .max_depth(4);

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jar") {
                jars.push(path.to_path_buf());
            }
        }

        jars
    }

    /// Parse `sdk.dir` from `<project_root>/local.properties`.
    pub fn read_local_properties(project_root: &Path) -> Option<PathBuf> {
        let file = project_root.join("local.properties");
        let text = std::fs::read_to_string(&file).ok()?;

        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix("sdk.dir=") {
                let path_str = rest.trim();
                // Java properties files use `\:` on Windows — normalise.
                let normalised = path_str.replace("\\:", ":");
                return Some(PathBuf::from(normalised));
            }
        }

        None
    }
}

// ── FtcJdtlsConfig ───────────────────────────────────────────────────────────

/// Full configuration needed to launch JDTLS for an FTC project.
#[derive(Debug, Clone)]
pub struct FtcJdtlsConfig {
    /// Path to the JDTLS installation (contains `plugins/` and `config_*`).
    pub jdtls_home: PathBuf,
    /// Per-workspace data directory, isolated by project path hash.
    pub workspace_data_dir: PathBuf,
    /// Java source directories (e.g. `TeamCode/src/main/java`).
    pub source_paths: Vec<PathBuf>,
    /// Full classpath entries (FTC JARs + Android SDK).
    pub classpath: Vec<PathBuf>,
    /// JVM heap in megabytes.
    pub jvm_heap_mb: u32,
}

impl FtcJdtlsConfig {
    /// Build a config by scanning the project root and JDTLS installation.
    pub fn build(project_root: &Path, jdtls_home: &Path, heap_mb: u32) -> anyhow::Result<Self> {
        // Compute a stable workspace data dir from the project path.
        let workspace_data_dir = Self::workspace_data_dir(project_root)?;

        // Source paths: walk for typical FTC Gradle project layouts.
        let source_paths = Self::discover_source_paths(project_root);

        // Classpath: FTC JARs from libs/ + Android platforms.
        let mut classpath = FtcSdkLocator::find_ftc_jars(project_root);

        // Add android.jar from the SDK if found.
        if let Some(sdk) = FtcSdkLocator::find_android_sdk(project_root) {
            let platforms_dir = sdk.join("platforms");
            if let Ok(entries) = std::fs::read_dir(&platforms_dir) {
                // Pick the latest platform by directory name (alphabetical sort gives latest).
                let mut platform_dirs: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .collect();
                platform_dirs.sort();

                if let Some(latest) = platform_dirs.last() {
                    let android_jar = latest.join("android.jar");
                    if android_jar.exists() {
                        classpath.push(android_jar);
                    }
                }
            }
        }

        debug!(
            "FtcJdtlsConfig: {} source dirs, {} classpath entries",
            source_paths.len(),
            classpath.len()
        );

        Ok(Self {
            jdtls_home: jdtls_home.to_path_buf(),
            workspace_data_dir,
            source_paths,
            classpath,
            jvm_heap_mb: heap_mb,
        })
    }

    /// Write Eclipse JDT core preferences into the project's `.settings/` directory.
    pub fn write_project_settings(&self, project_root: &Path) -> anyhow::Result<()> {
        let settings_dir = project_root.join(".settings");
        std::fs::create_dir_all(&settings_dir).context("creating .settings directory")?;

        // org.eclipse.jdt.core.prefs — compiler compliance level for FTC (Java 8).
        let prefs = "\
eclipse.preferences.version=1\n\
org.eclipse.jdt.core.compiler.codegen.targetPlatform=1.8\n\
org.eclipse.jdt.core.compiler.compliance=1.8\n\
org.eclipse.jdt.core.compiler.source=1.8\n\
org.eclipse.jdt.core.compiler.problem.forbiddenReference=warning\n\
";

        let prefs_path = settings_dir.join("org.eclipse.jdt.core.prefs");
        std::fs::write(&prefs_path, prefs)
            .with_context(|| format!("writing {}", prefs_path.display()))?;

        // .classpath — Eclipse-format classpath file.
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<classpath>\n");
        for src in &self.source_paths {
            xml.push_str(&format!(
                "  <classpathentry kind=\"src\" path=\"{}\"/>\n",
                src.display()
            ));
        }
        for entry in &self.classpath {
            xml.push_str(&format!(
                "  <classpathentry kind=\"lib\" path=\"{}\"/>\n",
                entry.display()
            ));
        }
        xml.push_str(
            "  <classpathentry kind=\"con\" path=\"org.eclipse.jdt.launching.JRE_CONTAINER\"/>\n",
        );
        xml.push_str("  <classpathentry kind=\"output\" path=\"bin\"/>\n</classpath>\n");

        let cp_path = project_root.join(".classpath");
        std::fs::write(&cp_path, xml).with_context(|| format!("writing {}", cp_path.display()))?;

        Ok(())
    }

    /// Build the command-line arguments to pass to the JDTLS launcher.
    pub fn server_start_args(&self) -> Vec<String> {
        // Find the JDTLS equinox launcher JAR.
        let launcher_jar = self.find_launcher_jar();

        // Find the OS-specific JDTLS config directory.
        let config_dir = self.find_config_dir();

        let mut args = vec![
            format!("-Xms{}m", self.jvm_heap_mb / 2),
            format!("-Xmx{}m", self.jvm_heap_mb),
            "--add-modules=ALL-SYSTEM".to_string(),
            "--add-opens=java.base/java.util=ALL-UNNAMED".to_string(),
            "--add-opens=java.base/java.lang=ALL-UNNAMED".to_string(),
        ];

        if let Some(jar) = launcher_jar {
            args.push("-jar".to_string());
            args.push(jar.to_string_lossy().into_owned());
        }

        args.push("-configuration".to_string());
        if let Some(cfg) = config_dir {
            args.push(cfg.to_string_lossy().into_owned());
        } else {
            args.push(
                self.jdtls_home
                    .join("config_linux")
                    .to_string_lossy()
                    .into_owned(),
            );
        }

        args.push("-data".to_string());
        args.push(self.workspace_data_dir.to_string_lossy().into_owned());

        args
    }

    /// Return the JDTLS initialization options JSON for `initialize` request.
    pub fn init_options(&self) -> serde_json::Value {
        let classpath_strs: Vec<String> = self
            .classpath
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        let source_strs: Vec<String> = self
            .source_paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();

        serde_json::json!({
            "bundles": [],
            "workspaceFolders": source_strs,
            "settings": {
                "java": {
                    "home": std::env::var("JAVA_HOME").unwrap_or_default(),
                    "configuration": {
                        "updateBuildConfiguration": "automatic",
                        "runtimes": []
                    },
                    "completion": {
                        "enabled": true,
                        "importOrder": ["java", "javax", "com", "org"]
                    },
                    "sources": {
                        "organizeImports": {
                            "starThreshold": 99,
                            "staticStarThreshold": 99
                        }
                    },
                    "classpath": classpath_strs,
                    "project": {
                        "referencedLibraries": classpath_strs
                    },
                    "format": {
                        "enabled": true
                    },
                    "saveActions": {
                        "organizeImports": false
                    },
                    "eclipse": {
                        "downloadSources": false
                    },
                    "maven": {
                        "downloadSources": false
                    },
                    "implementationsCodeLens": {
                        "enabled": false
                    },
                    "referencesCodeLens": {
                        "enabled": false
                    },
                    "signatureHelp": {
                        "enabled": true
                    }
                }
            }
        })
    }

    // ── private helpers ───────────────────────────────────────────────────────

    fn workspace_data_dir(project_root: &Path) -> anyhow::Result<PathBuf> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        project_root.hash(&mut hasher);
        let hash = hasher.finish();

        let base = dirs_next::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(".local/share"))
            .join("alloy-studio")
            .join("jdtls-workspace")
            .join(format!("{hash:016x}"));

        std::fs::create_dir_all(&base)
            .with_context(|| format!("creating JDTLS workspace dir {}", base.display()))?;

        Ok(base)
    }

    fn discover_source_paths(project_root: &Path) -> Vec<PathBuf> {
        // Common FTC Gradle project layouts.
        let candidates = [
            "TeamCode/src/main/java",
            "FtcRobotController/src/main/java",
            "src/main/java",
            "src",
        ];

        let mut paths = Vec::new();
        for rel in &candidates {
            let abs = project_root.join(rel);
            if abs.exists() {
                paths.push(abs);
            }
        }

        // If nothing found, fall back to project root.
        if paths.is_empty() {
            paths.push(project_root.to_path_buf());
        }

        paths
    }

    fn find_launcher_jar(&self) -> Option<PathBuf> {
        let plugins_dir = self.jdtls_home.join("plugins");
        let entries = std::fs::read_dir(&plugins_dir).ok()?;

        entries.filter_map(|e| e.ok()).map(|e| e.path()).find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("org.eclipse.equinox.launcher_") && n.ends_with(".jar"))
                .unwrap_or(false)
        })
    }

    fn find_config_dir(&self) -> Option<PathBuf> {
        let os = if cfg!(target_os = "macos") {
            "mac"
        } else if cfg!(target_os = "windows") {
            "win"
        } else {
            "linux"
        };

        // JDTLS ships both `config_linux` and `config_linux_arm` variants.
        let candidates = [
            format!("config_{os}"),
            format!("config_{os}_arm"),
            format!("config_{os}64"),
        ];

        for name in &candidates {
            let p = self.jdtls_home.join(name);
            if p.exists() {
                return Some(p);
            }
        }

        None
    }
}

// ── dirs_next shim ────────────────────────────────────────────────────────────
// The workspace uses `dirs = "5"` not `dirs-next`. Provide a minimal shim so
// the code above compiles.
mod dirs_next {
    pub fn home_dir() -> Option<std::path::PathBuf> {
        dirs::home_dir()
    }
    pub fn data_local_dir() -> Option<std::path::PathBuf> {
        dirs::data_local_dir()
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn read_local_properties_parses_sdk_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let props = tmp.path().join("local.properties");
        let mut f = std::fs::File::create(&props).unwrap();
        writeln!(f, "# comment").unwrap();
        writeln!(f, "sdk.dir=/opt/android/sdk").unwrap();

        let result = FtcSdkLocator::read_local_properties(tmp.path());
        assert_eq!(result, Some(PathBuf::from("/opt/android/sdk")));
    }

    #[test]
    fn find_ftc_jars_finds_jars_in_libs() {
        let tmp = tempfile::tempdir().unwrap();
        let libs = tmp.path().join("libs");
        std::fs::create_dir_all(&libs).unwrap();
        std::fs::write(libs.join("RobotCore.jar"), b"fake").unwrap();
        std::fs::write(libs.join("Hardware.jar"), b"fake").unwrap();
        std::fs::write(libs.join("not-a-jar.txt"), b"fake").unwrap();

        let jars = FtcSdkLocator::find_ftc_jars(tmp.path());
        assert_eq!(jars.len(), 2);
        assert!(jars.iter().all(|p| p.extension().unwrap() == "jar"));
    }

    #[test]
    fn init_options_contains_classpath() {
        let tmp = tempfile::tempdir().unwrap();
        let jdtls_home = tmp.path().join("jdtls");
        std::fs::create_dir_all(&jdtls_home).unwrap();

        let cfg = FtcJdtlsConfig {
            jdtls_home,
            workspace_data_dir: tmp.path().join("ws"),
            source_paths: vec![tmp.path().join("src")],
            classpath: vec![tmp.path().join("libs/Robot.jar")],
            jvm_heap_mb: 512,
        };

        let opts = cfg.init_options();
        let cp = &opts["settings"]["java"]["classpath"];
        assert!(cp.is_array());
        assert_eq!(cp.as_array().unwrap().len(), 1);
    }
}
