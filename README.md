<div align="center">

<img src="extra/images/logo.png" alt="Alloy Studio" width="180"/>

# Alloy Studio

**The IDE built for builders**

*A GPU-accelerated, AI-native code editor purpose-built for FIRST Tech Challenge robotics teams.*

[![Build Status](https://img.shields.io/github/actions/workflow/status/olusenbg/alloy-studio/ci.yml?branch=main&label=build)](https://github.com/olusenbg/alloy-studio/actions)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](#installation)
[![Rust](https://img.shields.io/badge/rust-1.87%2B-orange.svg)](https://www.rust-lang.org/)
[![FTC](https://img.shields.io/badge/FTC-SDK%20Compatible-red.svg)](https://github.com/FIRST-Tech-Challenge/FtcRobotController)

</div>

---

## The Problem

Android Studio is the default IDE for FTC teams — and it's crushing them before they write a single line of code.

| Pain Point | Reality |
|---|---|
| **4 GB RAM minimum** | Most school laptops have 4–8 GB total |
| **10 GB disk footprint** | Chromebooks and budget laptops run out of space |
| **Cryptic Gradle errors** | `Could not resolve com.qualcomm.robotcore:ftc-sdk:9.2` tells rookies nothing |
| **No Git tooling for students** | Teams pass code on USB drives instead of using version control |
| **Zero robotics context** | No motor configs, no telemetry, no FTC SDK awareness |

Alloy Studio is a GPU-accelerated code editor written in pure Rust, built from the ground up with a suite of robotics-specific tools that solve every one of these problems.

---

## Features

### AI Gradle Repair Engine
When a Gradle build fails, Alloy intercepts the error, reads your `build.gradle` and SDK version, and proposes or auto-applies a one-click fix. No more Googling cryptic dependency errors.

### Visual Hardware Mapper
A 2D diagram of the REV Control Hub lets students click on a port, type a name, and Alloy generates the corresponding `HardwareMap` declaration in Java automatically.

### Student Git Wrapper
OAuth-based Git with AI-generated commit messages and a visual conflict resolver. When two students edit the same variable, the UI shows: *"Alex set arm to 12 in — Sarah set 15 in — pick one."*

### Live Telemetry Panel
Real-time charts of encoder ticks, battery voltage, and gyro heading streamed over Wi-Fi Direct from the Robot Controller, directly inside the editor.

### JDTLS + FTC SDK Integration
Eclipse JDT Language Server pre-configured with FTC SDK class paths. Autocompletion, diagnostics, and inline docs work out of the box — no manual SDK setup required.

---

## Architecture

```
alloy-studio/
├── alloy-app/          # Core editor shell
├── alloy-core/         # Buffer, syntax, rope, Gradle repair engine, hardware config, telemetry server
├── alloy-ui/           # Custom Floem panels (hardware mapper, telemetry, gradle repair, git timeline)
├── alloy-git/          # Student Git wrapper + AI commit generation
├── alloy-lsp/          # JDTLS configuration + FTC SDK bindings
├── alloy-proxy/        # Remote filesystem proxy
├── alloy-rpc/          # IPC protocol
│
└── docs/
    └── architecture.md
```

See [`docs/architecture.md`](docs/architecture.md) for the full technical breakdown.

---

## Installation

> **Note:** Alloy Studio is in early development. Pre-built binaries are not yet available.

### Build from Source

**Prerequisites:**
- Rust 1.87+ (`rustup update stable`)
- A C compiler (`gcc` / `clang` / MSVC)
- On Linux: `libxkbcommon-dev`, `libwayland-dev` (or X11 equivalents)

```bash
git clone https://github.com/olusenbg/alloy-studio
cd alloy-studio
cargo build --release --bin alloy
./target/release/alloy
```

For a faster iterative build during development:

```bash
cargo build --profile fastdev --bin alloy
```

Full build-from-source guide: [`docs/building-from-source.md`](docs/building-from-source.md)

---

## Contributing

Contributions from FTC mentors, students, and Rust developers are all welcome.

Before opening a pull request:

1. Read [`CONTRIBUTING.md`](CONTRIBUTING.md) for setup instructions and project conventions.
2. Run `cargo fmt --all` and `cargo clippy` and resolve any issues.
3. Open an issue first for substantial new features so we can discuss scope.

**Good first issues** are tagged [`good first issue`](https://github.com/olusenbg/alloy-studio/issues?q=label%3A%22good+first+issue%22) on GitHub.

---

## License

Alloy Studio is released under the **Apache License, Version 2.0**.  
See [`LICENSE`](LICENSE) for the full license text.
