# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.2] - 2026-11-11

### Changed
- Replaced Polish with Russian in the language selector and translations

## [0.4.1] - 2026-11-11

### Changed
- Replaced Hindi with German and Bengali with Polish in the language selector and translations

## [0.4.0] - 2026-11-11

### Added
- Full UI translations for the seven most spoken languages: Spanish, English, Simplified Chinese, Hindi, French, Modern Standard Arabic and Bengali
- Right-to-left layout support for Arabic, applied automatically to the whole app and the overlay
- Language selector in Settings showing each language in its own script

### Changed
- The floating overlay now follows the saved language

## [0.3.3] - 2026-11-11

### Changed
- LibreHardwareMonitor now runs on a private hidden desktop: no window, taskbar button or tray icon is ever visible while CPU temperature keeps working

## [0.3.2] - 2026-11-11

### Added
- Shader cache cleanup: removes stale DirectX, NVIDIA, AMD and Intel shader caches and reports the space freed
- Cloud clipboard and cross-device sync can be disabled from the Optimizations page
- DNS cache flush plus Winsock reset, with a reboot note

### Changed
- The overlay starts hidden while remaining enabled, so `Alt+R` reveals it and it never pops up at launch
- Saving any other setting no longer opens the overlay

## [0.3.1] - 2026-11-11

### Added
- Overlay configuration dialog: metrics, horizontal/vertical layout, screen corner and opacity
- `Alt+R` global shortcut to show or hide the overlay while it is enabled
- 1% low frame rate and a frame-time trend on the overlay

### Changed
- The overlay opens in the horizontal layout by default
- LibreHardwareMonitor now starts without a tray icon and with its window hidden

### Fixed
- Stale `Badge` test asserting a class the component no longer renders

## [0.3.0] - 2026-11-10

### Added
- Tools page with real cleanup preview, SFC/DISM/defrag/TRIM/CHKDSK jobs and SQLite maintenance
- Overlay page plus FPS and GPU sampling (ETW present events, PDH counters)
- CPU temperature fallback through LibreHardwareMonitor's local web server, started minimized when it is installed

### Changed
- Monitor reports GPU temperature from NVML alongside per-core load

### Fixed
- CPU temperature on desktop boards without ACPI thermal zones

## [Unreleased]

### Added
- Optimization engine core (`wexo-optimizer` crate)
- Safety layer: snapshots + transaction log (`wexo-safety` crate)
- 8 built-in optimizations (services, power, telemetry, visual effects, startup, NTFS, TCP, HAGS)
- Rules engine framework (`wexo-rules` crate)
- GPU info collection via WMI
- SMART disk health monitoring

### Changed
- Tauri v2 migration
- Workspace restructure into modular crates

### Fixed
- `wexo-safety` compilation (winreg 0.11 API)

## [0.1.1] - 2026-11-08

### Added
- GitHub Actions release workflow (build → artifact → GitHub Release)
- NSIS installer generation
- Professional README with badges, quick start, architecture diagram

### Fixed
- Installer path in release workflow (`target/` vs `src-tauri/target/`)

## [0.1.0] - 2026-11-08

### Added
- Initial project structure
- Tauri v2 + React + TypeScript frontend
- Rust workspace with 6 crates
- Live hardware diagnostics (CPU, memory, disks, network)
- CI workflow (fmt, clippy, test, typecheck, lint)

---

[Unreleased]: https://github.com/02kenzz/WexoOptimizer/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/02kenzz/WexoOptimizer/releases/tag/v0.1.1
[0.1.0]: https://github.com/02kenzz/WexoOptimizer/releases/tag/v0.1.0