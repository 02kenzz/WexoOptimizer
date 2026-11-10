<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/icon.png">
    <img alt="Wexo Optimizer" src="assets/icon.png" width="128">
  </picture>
</p>

<h1 align="center">Wexo Optimizer</h1>

<p align="center">
  <strong>System control center — diagnose, optimize, monitor.</strong><br>
  Runs on Windows 10/11 and Linux (GNOME, systemd).
  Every change is atomic, reversible, and verifiable.
</p>

<p align="center">
  <a href="https://github.com/02kenzz/WexoOptimizer/releases/latest"><img alt="Latest Release" src="https://img.shields.io/github/v/release/02kenzz/WexoOptimizer?style=flat&color=5C6BC0&label=Release"></a>
  <a href="https://github.com/02kenzz/WexoOptimizer/actions/workflows/ci.yml"><img alt="CI Status" src="https://img.shields.io/github/actions/workflow/status/02kenzz/WexoOptimizer/ci.yml?branch=master&style=flat&label=CI"></a>
  <a href="https://github.com/02kenzz/WexoOptimizer/blob/master/LICENSE"><img alt="License" src="https://img.shields.io/github/license/02kenzz/WexoOptimizer?style=flat&color=5C6BC0"></a>
  <a href="https://discord.gg/"><img alt="Discord" src="https://img.shields.io/discord/123456789?style=flat&color=5865F2&label=Discord&logo=discord&logoColor=white"></a>
</p>

---

## 🎯 What it does

| Feature | Description |
|---------|-------------|
| **Diagnose** | Live hardware telemetry — CPU, GPU, memory, disks, network, thermals |
| **Optimize** | One-click toggles for services, power plans, telemetry, visual effects and kernel tuning (Windows and Linux) |
| **Monitor** | Circular buffers, bottleneck detection, historical trends |
| **Overlay** | Always-on-top, click-through FPS / CPU / GPU / RAM panel for games — enabled from Settings (Windows). Frame rate is counted from ETW present events, so nothing is injected into the game and the app needs administrator rights for it |
| **Safety** | Snapshots before every change, SQLite transaction log, one-click rollback |

> **Status:** shell, monitoring, optimization engine and rollback work on Windows and Linux. Windows ships seven registry/service/power tweaks; Linux ships GNOME animations, Bluetooth and `vm.swappiness`.

---

## ✨ Quick start

### Install (Windows 10/11)

```powershell
# Download latest .exe from Releases
irm "https://github.com/02kenzz/WexoOptimizer/releases/latest/download/Wexo.Optimizer_0.1.0_x64-setup.exe" -OutFile "$env:TEMP\wexo-setup.exe"
Start-Process "$env:TEMP\wexo-setup.exe" -Wait
```

> **SmartScreen warning?** The installer is unsigned (open source, no cert budget). Click *More info → Run anyway*. [Why?](https://github.com/02kenzz/WexoOptimizer#code-signing)

> **Administrator rights:** the app elevates itself every time it starts (UAC on Windows, `pkexec` on Linux). If you decline the prompt it still opens, but the tweaks marked *Requires administrator* stay disabled. Launch it with `--no-elevate` to skip the prompt deliberately.

> **CPU temperature:** some desktop boards do not expose ACPI thermal zones. On those, the Monitor reads the CPU package sensor from [LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor). When it is installed the app starts it minimized and configures its local web server; it needs the same administrator rights as the app. Without it the temperature tile shows `—` instead of a made-up value.

### Build from source

```bash
# Prerequisites: Node 20+, Rust stable (MSVC), VS Build Tools (C++ desktop)
git clone https://github.com/02kenzz/WexoOptimizer
cd WexoOptimizer
npm install
npm run tauri dev        # Dev app with Rust backend
# or
npm run dev              # Frontend only (mocked data)
```

### Build from source (Linux)

```bash
# Prerequisites: Node 20+, Rust stable, WebKitGTK and the usual Tauri deps
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
npm install
npm run tauri dev
```

Linux tweaks drive GNOME (`gsettings`), systemd and sysctl. When a change needs
root, the app relaunches through `pkexec`; snapshots keep systemd unit states and
every file they touch — including files the tweak created, which rollback removes.

---

## 🏗 Architecture

```
crates/wexo-core       Pure domain types (no Windows deps) — testable logic
crates/wexo-sysinfo    Hardware/OS collectors (sysinfo; WMI/PDH on Windows, /sys on Linux)
crates/wexo-optimizer  Optimization registry + atomic apply/verify/rollback engine
crates/wexo-safety     Preflight checks, registry/file/systemd snapshots, SQLite txn log
crates/wexo-rules      Rules engine → recommendations (planned M4)
crates/wexo-monitor    Metrics sampling, circular buffers, bottleneck detection (planned M3)
src-tauri              Tauri shell, commands, state, logging
src/                   React 18 + TypeScript + Vite + Tailwind 4
```

**Stack:** Tauri v2 · Rust workspace · React 18 · TypeScript · Tailwind CSS 4 · Zustand · TanStack Query · `ts-rs` for typed IPC · Vitest · `cargo test`

---

## 🔐 Code signing

Open source project — no EV certificate budget.

| Option | Status |
|--------|--------|
| **Unsigned (current)** | Works, shows SmartScreen warning |
| **SignPath.io** | Free for FOSS — planned |
| **Self-signed** | Internal testing only |

[Details →](docs/CODE_SIGNING.md)

---

## 📦 Releases

| Channel | Link |
|---------|------|
| **Stable** | [Latest release](https://github.com/02kenzz/WexoOptimizer/releases/latest) |
| **All versions** | [Releases page](https://github.com/02kenzz/WexoOptimizer/releases) |
| **Changelog** | [CHANGELOG.md](CHANGELOG.md) |

---

## 🔗 Links

- **Code signing** → [docs/CODE_SIGNING.md](docs/CODE_SIGNING.md)
- **Issues** → [GitHub Issues](https://github.com/02kenzz/WexoOptimizer/issues)
- **Discussions** → [GitHub Discussions](https://github.com/02kenzz/WexoOptimizer/discussions)

---

## 📄 License

MIT © 2026 [02kenzz](https://github.com/02kenzz)