//! Hardware map parser and declaration generator for FTC OpModes.

use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

// ── Regex patterns ────────────────────────────────────────────────────────────

static HARDWARE_MAP_GET: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"hardwareMap\.get\(\s*(\w+(?:\.\w+)*?)\.class\s*,\s*"([^"]+)"\s*\)"#,
    )
    .unwrap()
});

static HARDWARE_MAP_TRY_GET: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"hardwareMap\.tryGet\(\s*(\w+(?:\.\w+)*?)\.class\s*,\s*"([^"]+)"\s*\)"#,
    )
    .unwrap()
});

static HARDWARE_MAP_SHORTHAND: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"hardwareMap\.(dcMotor|servo|colorSensor|touchSensor|distanceSensor|imu)\s*\.\s*get\s*\(\s*"([^"]+)"\s*\)"#,
    )
    .unwrap()
});

// Map shorthand names → proper Java type names
fn shorthand_to_type(shorthand: &str) -> &'static str {
    match shorthand {
        "dcMotor"         => "DcMotor",
        "servo"           => "Servo",
        "colorSensor"     => "ColorSensor",
        "touchSensor"     => "TouchSensor",
        "distanceSensor"  => "DistanceSensor",
        "imu"             => "IMU",
        _other            => "Unknown",
    }
}

// ── Types ─────────────────────────────────────────────────────────────────────

/// A single hardware device referenced in source code.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceDeclaration {
    /// Variable name used in Java/Kotlin code (may be synthesised from config name).
    pub variable_name: String,
    /// Java type, e.g. `"DcMotor"`, `"Servo"`.
    pub device_type: String,
    /// Name as it appears in the robot controller hardware config, e.g. `"left_drive"`.
    pub config_name: String,
    /// 1-based line number where the `hardwareMap.get(...)` call appears.
    pub line: u32,
}

/// A port assignment from the visual mapper UI.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PortAssignment {
    /// 0 = Control Hub, 1 = Expansion Hub.
    pub hub: u8,
    pub port: u8,
    pub device_type: String,
    pub config_name: String,
    pub variable_name: String,
}

// ── HardwareConfig ────────────────────────────────────────────────────────────

/// Parsed hardware declarations extracted from a source file or UI assignments.
pub struct HardwareConfig {
    pub declarations: Vec<DeviceDeclaration>,
}

impl HardwareConfig {
    /// Parse a Java source file and extract all `hardwareMap.get(...)` / `hardwareMap.tryGet(...)`
    /// / shorthand calls.
    pub fn parse_java_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut declarations = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Helper: count line number of a byte offset
        let line_of = |offset: usize| -> u32 {
            content[..offset.min(content.len())]
                .chars()
                .filter(|&c| c == '\n')
                .count() as u32
                + 1
        };

        // hardwareMap.get(Type.class, "name")
        for caps in HARDWARE_MAP_GET.captures_iter(&content) {
            let device_type = caps[1].to_string();
            let config_name = caps[2].to_string();
            let key = format!("{}:{}", device_type, config_name);
            if seen.insert(key) {
                let offset = caps.get(0).unwrap().start();
                declarations.push(DeviceDeclaration {
                    variable_name: Self::config_name_to_var_name(&config_name),
                    device_type,
                    config_name,
                    line: line_of(offset),
                });
            }
        }

        // hardwareMap.tryGet(Type.class, "name")
        for caps in HARDWARE_MAP_TRY_GET.captures_iter(&content) {
            let device_type = caps[1].to_string();
            let config_name = caps[2].to_string();
            let key = format!("{}:{}", device_type, config_name);
            if seen.insert(key) {
                let offset = caps.get(0).unwrap().start();
                declarations.push(DeviceDeclaration {
                    variable_name: Self::config_name_to_var_name(&config_name),
                    device_type,
                    config_name,
                    line: line_of(offset),
                });
            }
        }

        // hardwareMap.dcMotor.get("name")  etc.
        for caps in HARDWARE_MAP_SHORTHAND.captures_iter(&content) {
            let shorthand = &caps[1];
            let device_type = shorthand_to_type(shorthand).to_string();
            let config_name = caps[2].to_string();
            let key = format!("{}:{}", device_type, config_name);
            if seen.insert(key) {
                let offset = caps.get(0).unwrap().start();
                declarations.push(DeviceDeclaration {
                    variable_name: Self::config_name_to_var_name(&config_name),
                    device_type,
                    config_name,
                    line: line_of(offset),
                });
            }
        }

        // Sort by line number for deterministic output
        declarations.sort_by_key(|d| d.line);

        Ok(Self { declarations })
    }

    /// Build a `HardwareConfig` from port assignments sent by the UI.
    pub fn from_port_assignments(assignments: &[PortAssignment]) -> Self {
        let declarations = assignments
            .iter()
            .map(|a| DeviceDeclaration {
                variable_name: a.variable_name.clone(),
                device_type: a.device_type.clone(),
                config_name: a.config_name.clone(),
                line: 0,
            })
            .collect();
        Self { declarations }
    }

    /// Format declarations as a Java code block.
    ///
    /// Example output:
    /// ```text
    /// DcMotor leftDrive = hardwareMap.get(DcMotor.class, "left_drive");
    /// Servo   armServo  = hardwareMap.get(Servo.class, "arm_servo");
    /// ```
    pub fn format_java_block(&self) -> String {
        self.declarations
            .iter()
            .map(|d| {
                format!(
                    "{} {} = hardwareMap.get({}.class, \"{}\");",
                    d.device_type, d.variable_name, d.device_type, d.config_name
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Format declarations as a Kotlin code block.
    ///
    /// Example output:
    /// ```text
    /// val leftDrive = hardwareMap.get(DcMotor::class.java, "left_drive")
    /// val armServo  = hardwareMap.get(Servo::class.java, "arm_servo")
    /// ```
    pub fn format_kotlin_block(&self) -> String {
        self.declarations
            .iter()
            .map(|d| {
                format!(
                    "val {} = hardwareMap.get({}::class.java, \"{}\")",
                    d.variable_name, d.device_type, d.config_name
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Convert a snake_case config name to camelCase variable name.
    ///
    /// `"left_drive"` → `"leftDrive"`, `"arm_servo_2"` → `"armServo2"`
    fn config_name_to_var_name(config_name: &str) -> String {
        let mut result = String::new();
        let mut capitalise_next = false;

        for ch in config_name.chars() {
            if ch == '_' || ch == '-' {
                capitalise_next = true;
            } else if capitalise_next {
                result.extend(ch.to_uppercase());
                capitalise_next = false;
            } else {
                result.push(ch);
            }
        }

        result
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
    fn camel_case_conversion() {
        assert_eq!(HardwareConfig::config_name_to_var_name("left_drive"), "leftDrive");
        assert_eq!(HardwareConfig::config_name_to_var_name("arm_servo"), "armServo");
        assert_eq!(HardwareConfig::config_name_to_var_name("motor1"), "motor1");
        assert_eq!(HardwareConfig::config_name_to_var_name("front_left_wheel"), "frontLeftWheel");
    }

    #[test]
    fn parse_hardware_map_get() {
        let content = r#"
DcMotor leftDrive = hardwareMap.get(DcMotor.class, "left_drive");
Servo armServo = hardwareMap.get(Servo.class, "arm_servo");
"#;
        let file = write_java(content);
        let config = HardwareConfig::parse_java_file(file.path()).unwrap();
        assert_eq!(config.declarations.len(), 2);
        assert_eq!(config.declarations[0].config_name, "left_drive");
        assert_eq!(config.declarations[0].device_type, "DcMotor");
        assert_eq!(config.declarations[1].config_name, "arm_servo");
        assert_eq!(config.declarations[1].device_type, "Servo");
    }

    #[test]
    fn parse_hardware_map_try_get() {
        let content = r#"
DcMotor m = hardwareMap.tryGet(DcMotor.class, "motor_a");
"#;
        let file = write_java(content);
        let config = HardwareConfig::parse_java_file(file.path()).unwrap();
        assert_eq!(config.declarations.len(), 1);
        assert_eq!(config.declarations[0].config_name, "motor_a");
    }

    #[test]
    fn parse_shorthand_hardware_map() {
        let content = r#"
DcMotor m = hardwareMap.dcMotor.get("intake_motor");
Servo s = hardwareMap.servo.get("gripper");
"#;
        let file = write_java(content);
        let config = HardwareConfig::parse_java_file(file.path()).unwrap();
        assert_eq!(config.declarations.len(), 2);
        assert_eq!(config.declarations[0].device_type, "DcMotor");
        assert_eq!(config.declarations[0].config_name, "intake_motor");
        assert_eq!(config.declarations[1].device_type, "Servo");
        assert_eq!(config.declarations[1].config_name, "gripper");
    }

    #[test]
    fn deduplicates_same_device() {
        let content = r#"
DcMotor m1 = hardwareMap.get(DcMotor.class, "motor_a");
DcMotor m2 = hardwareMap.get(DcMotor.class, "motor_a");
"#;
        let file = write_java(content);
        let config = HardwareConfig::parse_java_file(file.path()).unwrap();
        assert_eq!(config.declarations.len(), 1);
    }

    #[test]
    fn format_java_block() {
        let config = HardwareConfig::from_port_assignments(&[
            PortAssignment {
                hub: 0,
                port: 0,
                device_type: "DcMotor".into(),
                config_name: "left_drive".into(),
                variable_name: "leftDrive".into(),
            },
        ]);
        let block = config.format_java_block();
        assert!(block.contains("DcMotor leftDrive = hardwareMap.get(DcMotor.class, \"left_drive\");"));
    }

    #[test]
    fn format_kotlin_block() {
        let config = HardwareConfig::from_port_assignments(&[
            PortAssignment {
                hub: 0,
                port: 0,
                device_type: "Servo".into(),
                config_name: "arm_servo".into(),
                variable_name: "armServo".into(),
            },
        ]);
        let block = config.format_kotlin_block();
        assert!(block.contains("val armServo = hardwareMap.get(Servo::class.java, \"arm_servo\")"));
    }

    #[test]
    fn from_port_assignments() {
        let assignments = vec![
            PortAssignment {
                hub: 0,
                port: 0,
                device_type: "DcMotor".into(),
                config_name: "motor1".into(),
                variable_name: "motor1".into(),
            },
            PortAssignment {
                hub: 1,
                port: 2,
                device_type: "Servo".into(),
                config_name: "servo_a".into(),
                variable_name: "servoA".into(),
            },
        ];
        let config = HardwareConfig::from_port_assignments(&assignments);
        assert_eq!(config.declarations.len(), 2);
        assert_eq!(config.declarations[0].device_type, "DcMotor");
        assert_eq!(config.declarations[1].device_type, "Servo");
    }
}
