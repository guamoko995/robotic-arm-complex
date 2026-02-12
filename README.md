# Robotic Arm Complex

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)
[![Rust 1.90+](https://img.shields.io/badge/rust-1.90+-orange.svg)](https://www.rust-lang.org/)

Control system for robotic manipulators, built with Rust for the ESP32.

## Project Status

🚀 **Work in Progress:** For a detailed list of planned features and current progress, please check our [Roadmap](ROADMAP.md).

## Architecture

### `common/` – Shared Library (`no_std`)
Platform-agnostic types and protocol schemas shared between the firmware and the host.
- **Zero-cost Units:** Compile-time verified physical quantities (Radians, Seconds, etc.).
- **Binary Protocol:** Data serialization schemas via [Postcard](https://github.com/jamesmunns/postcard) with LEB128 framing.
- **Data Models:** Definitions for kinematics parameters and network stack configurations.

### `firmware/` – ESP32 Firmware
Dual-core, asynchronous implementation leveraging [Embassy](https://github.com/embassy-rs/embassy).
- **Core 0 (System):** WiFi (AP/STA modes), Flash storage management, and async network stack.
- **Core 1 (Real-time):** Dedicated trajectory interpolation and high-precision PWM control.

### `cli/` – Command-Line Interface
Network client for sending control commands and system configuration.

## Requirements

- **Firmware:** ESP32 (dual-core Xtensa/RISC-V supported by [esp-hal](https://github.com/esp-rs/esp-hal)).
- **Host (CLI):** Linux, macOS, or Windows.
- **Rust:** 1.90 or later (Edition 2024).
- **Build:** cargo-espflash and the Rust Xtensa/RISC-V toolchain.

## License

Licensed under either of Apache License 2.0 or MIT at your option.
