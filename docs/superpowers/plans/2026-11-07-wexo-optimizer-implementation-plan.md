# Wexo Optimizer — Implementation Plan

**Date:** 2026-11-07
**Based on:** `docs/superpowers/specs/2026-11-07-wexo-optimizer-design.md`
**Status:** Active
**Manager:** npm (Node 24.x)
**Toolchain:** Rust stable (MSVC), Tauri v2, React 18 + TypeScript + Vite

---

## 0. Decisions locked for v1

| Topic | Decision | Notes |
|-------|----------|-------|
| Package manager | **npm** | Spec used `bun` in CI examples; npm is installed and sufficient. Can migrate later. |
| Rust edition | **2021** | Maximum crate compatibility; revisit at 1.0. |
| TS type sharing | **ts-rs** | Derive `TS` on public DTOs in `wexo-core`; export to `src/types/generated/`. |
| Routing | **react-router-dom v7** | Needed by the 6-screen sidebar layout. |
| Tailwind | **v4 + `@tailwindcss/vite`** | CSS-variable design tokens for dark/light/high-contrast. |
| Component primitives | **Radix UI** | Dialog, DropdownMenu, Tabs, Tooltip, Switch, Progress. |
| Charts | **uPlot** | Wrapped in a typed React component with resize observer. |
| Local persistence | **SQLite (`rusqlite`, bundled)** | Transaction log per spec §5.3. Bundled feature avoids system dependency. |
| Config | `serde` + JSON in `%LOCALAPPDATA%\WexoOptimizer\settings.json` | Atomic write via temp file + rename. |
| Logging | `tracing` + `tracing-appender` daily rolling | `%LOCALAPPDATA%\WexoOptimizer\logs\`. |
| Errors | `thiserror` in crates, `anyhow` at command boundary | Commands return `Result<T, CommandError>` serializable. |
| Admin elevation | Detect + prompt; never auto-elevate silently | Monitor works unelevated; write actions require admin. |
| Windows-only code | `#[cfg(windows)]` behind `wexo-sysinfo` trait | Keeps `cargo test` runnable for pure logic on any host. |

---

## 1. Target repository layout (final)

```
WexoOptimizer/
├── Cargo.toml                     # Rust workspace
├── package.json
├── vite.config.ts
├── tsconfig.json
├── biome.json
├── index.html
├── .gitignore
├── docs/
│   └── superpowers/{specs,plans}/
├── src/                           # React frontend
│   ├── main.tsx
│   ├── App.tsx
│   ├── styles/globals.css
│   ├── components/
│   │   ├── layout/{AppShell,Sidebar,TopBar}.tsx
│   │   └── ui/{Button,Card,Badge,Switch,Progress,Tabs,Tooltip}.tsx
│   ├── pages/{Dashboard,Diagnostics,Optimizations,Monitor,History,Settings}.tsx
│   ├── stores/{appStore,monitorStore}.ts
│   ├── hooks/{useTauriEvent,useSystemProfile}.ts
│   ├── lib/{queryClient,ipc}.ts
│   ├── i18n/{index.ts,locales/{es,en}.json}
│   └── types/{index.ts,generated/}
├── src-tauri/                     # Tauri shell crate (binary)
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   ├── icons/
│   └── src/{main.rs,lib.rs,commands.rs,events.rs,state.rs,error.rs}
├── crates/
│   ├── wexo-core/                 # Pure domain types + logic (no Windows deps)
│   ├── wexo-sysinfo/              # Hardware/OS collectors (WMI, PDH, sysinfo)
│   ├── wexo-optimizer/            # Optimization registry, apply/verify/rollback
│   ├── wexo-safety/               # Snapshots, transaction log, preflight
│   ├── wexo-rules/                # Rules engine + JSON loading
│   └── wexo-monitor/              # Sampling engine + ring buffer
├── rules/rules.json               # Built-in recommendation rules
├── tests/
│   ├── e2e/                       # Playwright (M5)
│   └── fixtures/
└── .github/workflows/ci.yml
```

---

## 2. Milestone M0 — Foundation (current)

**Goal:** a compiling, testable, running shell: Tauri window opens, sidebar navigation works in ES/EN, theme switching works, a real Tauri command returns live system info, logging and config persist, CI is green.

### Phase M0.1 — Repository + toolchain

| ID | Task | Files | Acceptance |
|----|------|-------|------------|
| M0.1.1 | Install Rust stable MSVC + verify `cargo --version` | — | `rustc --version` works in new shell |
| M0.1.2 | Init git repo, `.gitignore` (node_modules, target, dist, gen) | `.gitignore` | `git status` clean after first commit |
| M0.1.3 | Create Rust workspace manifest with 6 member crates | `Cargo.toml`, `crates/*/Cargo.toml` | `cargo check --workspace` passes |
| M0.1.4 | Scaffold frontend manifest + Vite/TS config | `package.json`, `vite.config.ts`, `tsconfig*.json`, `index.html` | `npm run build` passes |

### Phase M0.2 — Domain model (`wexo-core`)

| ID | Task | Files | Acceptance |
|----|------|-------|------------|
| M0.2.1 | Define `SystemProfile`, `CpuInfo`, `GpuInfo`, `MemoryInfo`, `DiskInfo`, `NetworkInfo`, `OsInfo` with `ts-rs` derives | `crates/wexo-core/src/profile.rs` | Unit test serializes round-trip |
| M0.2.2 | Define `HealthScore`, `BottleneckReport`, `Severity`, `Confidence` | `crates/wexo-core/src/diagnostics.rs` | Score clamped 0..=100 test |
| M0.2.3 | Define `Optimization`, `Category`, `RiskLevel`, `Applicability`, `SnapshotSpec`, `VerificationResult` | `crates/wexo-core/src/optimization.rs` | Construction + risk ordering tests |
| M0.2.4 | Define `Recommendation`, `BenchmarkDelta`, `TransactionRecord` | `crates/wexo-core/src/{rules,benchmark,transaction}.rs` | TS bindings export test |
| M0.2.5 | `cargo test` export TS bindings into `src/types/generated/` | `crates/wexo-core/tests/export_bindings.rs` | Files appear; git tracked |

### Phase M0.3 — Tauri shell + IPC

| ID | Task | Files | Acceptance |
|----|------|-------|------------|
| M0.3.1 | Tauri v2 app crate wired to workspace crates | `src-tauri/Cargo.toml`, `build.rs`, `tauri.conf.json`, `capabilities/default.json` | `npm run tauri dev` opens window |
| M0.3.2 | App state: config + logger + version; init on startup | `src-tauri/src/{lib.rs,state.rs}` | Log file created in LOCALAPPDATA |
| M0.3.3 | Atomic JSON settings load/save with defaults | `src-tauri/src/config.rs` | Test: corrupt file → defaults, backup kept |
| M0.3.4 | Commands: `ping`, `get_app_info`, `get_system_profile`, `get_settings`, `save_settings` | `src-tauri/src/commands.rs`, `error.rs` | Invoked from UI; errors serialize |
| M0.3.5 | Minimal capabilities (no shell/fs beyond app dirs) | `capabilities/default.json` | No `shell:allow-execute` present |

### Phase M0.4 — UI shell

| ID | Task | Files | Acceptance |
|----|------|-------|------------|
| M0.4.1 | Design tokens: dark/light/HC CSS variables + Tailwind theme | `src/styles/globals.css` | Contrast AA in devtools check |
| M0.4.2 | App shell: sidebar (6 routes), top bar (admin badge, theme, language) | `src/components/layout/*`, `src/App.tsx` | Navigation works without reload |
| M0.4.3 | UI primitives: Button, Card, Badge, Switch, Progress, Tabs, Tooltip | `src/components/ui/*` | Render tests pass |
| M0.4.4 | i18n ES/EN with persisted language | `src/i18n/*`, `TopBar` | Switching updates all visible strings |
| M0.4.5 | Zustand app store (theme, language, settings) + React Query client | `src/stores/appStore.ts`, `src/lib/queryClient.ts` | Settings survive restart via backend |
| M0.4.6 | Six page skeletons with real shell content (Dashboard shows live profile/health placeholder) | `src/pages/*` | No console errors |

### Phase M0.5 — Quality gates

| ID | Task | Files | Acceptance |
|----|------|-------|------------|
| M0.5.1 | Vitest + Testing Library setup, first component tests | `src/**/*.test.tsx`, `vitest.config` | `npm test` green |
| M0.5.2 | Biome lint/format config | `biome.json` | `npm run lint` green |
| M0.5.3 | Rust fmt/clippy config, `-D warnings` | `rustfmt.toml`, `.clippy.toml` | `cargo clippy --workspace -- -D warnings` green |
| M0.5.4 | GitHub Actions CI (frontend + Rust, windows-latest) | `.github/workflows/ci.yml` | Workflow file validates |

**M0 exit criteria**
1. `npm run build` and `cargo check --workspace` pass from a clean clone.
2. `npm run tauri dev` launches, shows live CPU/RAM/OS from `get_system_profile`.
3. Theme + language persist across restarts.
4. No capability broader than needed; no network calls.

---

## 3. Milestone M1 — Diagnostics

| ID | Task | Files | Acceptance |
|----|------|-------|------------|
| M1.1 | `wexo-sysinfo` collector traits + mock implementation | `crates/wexo-sysinfo/src/lib.rs` | Unit tests run on mock |
| M1.2 | CPU collector (`sysinfo` + `Win32_Processor`) | `collectors/cpu.rs` | Model, cores, base freq populated |
| M1.3 | Memory collector incl. channel detection via `BankLabel` | `collectors/memory.rs` | Single/dual-channel flag correct on fixture |
| M1.4 | Disk collector: type (NVMe/SATA/HDD), SMART health | `collectors/disk.rs` | Graceful `Unknown` when SMART blocked |
| M1.5 | GPU collector: vendor, VRAM, driver, temp fallback | `collectors/gpu.rs` | NVIDIA/AMD/Intel no-panic |
| M1.6 | Network adapter collector | `collectors/network.rs` | Link speed + type |
| M1.7 | PDH counters abstraction (`perf` crate) | `counters.rs` | Counter read test skips on CI if unavailable |
| M1.8 | Diagnostic engine: thresholds → bottlenecks with evidence | `crates/wexo-core/src/diagnostics/engine.rs` | Golden tests for 4 scenarios |
| M1.9 | Health score aggregator (per subsystem + global) | `diagnostics/score.rs` | Deterministic, clamped, weighted |
| M1.10 | `diagnose` command + Diagnostics page wired | `src-tauri/src/commands.rs`, `src/pages/Diagnostics.tsx` | Shows profile + bottlenecks + scores |

---

## 4. Milestone M2 — Optimizer core

| ID | Task | Files | Acceptance |
|----|------|-------|------------|
| M2.1 | Preflight: admin, free disk, pending reboot, restore point | `crates/wexo-safety/src/preflight.rs` | Denies apply when unelevated |
| M2.2 | SQLite transaction log with migrations | `wexo-safety/src/transaction_log.rs` | Apply + rollback entries recover after restart |
| M2.3 | Snapshot engine: registry export, service state, file backup + SHA256 | `wexo-safety/src/snapshot.rs` | Hash mismatch aborts rollback |
| M2.4 | Executor pipeline: snapshot → apply → verify → log (atomic) | `crates/wexo-optimizer/src/executor.rs` | Failure rolls back automatically |
| M2.5 | Optimization registry with metadata + applicability filters | `wexo-optimizer/src/registry.rs` | Duplicate ID rejected |
| M2.6 | 15 optimizations (power, services, startup, NTFS, network, visuals, privacy) | `wexo-optimizer/src/optimizations/*.rs` | Each has apply/verify/rollback + dry-run |
| M2.7 | Dry-run mode | `executor.rs` | No system mutation |
| M2.8 | `list_optimizations`, `apply_optimization`, `rollback_transaction` commands | `commands.rs` | Errors carry rollback state |
| M2.9 | Optimizations + History pages | `src/pages/*` | Filter/search/expand; undo works |

**Gate:** 100 sequential apply/rollback cycles on a VM with zero residue (spec §8 stress).

---

## 5. Milestone M3 — Monitor

| ID | Task | Files | Acceptance |
|----|------|-------|------------|
| M3.1 | Sampling engine with adaptive frequency + ring buffer (1 h) | `crates/wexo-monitor/src/*` | < 1% CPU measured via Criterion bench |
| M3.2 | Event stream `monitor://metrics` (batched 1 s) | `src-tauri/src/events.rs` | Dropped frames don't leak |
| M3.3 | Frontend event hook + Zustand ring buffer | `src/hooks/useMetrics.ts`, `src/stores/monitorStore.ts` | Reconnect on window focus |
| M3.4 | uPlot chart wrapper + 4 charts | `src/components/charts/*` | 60 fps, resize-safe |
| M3.5 | Process table (top CPU/RAM/disk) | `src/components/monitor/ProcessTable.tsx` | Sort + virtualized |
| M3.6 | Threshold alerts → native toast + log | `wexo-monitor/src/alerts.rs` | No duplicate toast spam |
| M3.7 | Pause/resume + export CSV/JSON | `Monitor` page | Export validates against schema |

---

## 6. Milestone M4 — Rules & one-click UX

| ID | Task | Files | Acceptance |
|----|------|-------|------------|
| M4.1 | Rule schema + safe expression evaluator (no eval) | `crates/wexo-rules/src/{schema,eval}.rs` | Rejects unknown identifiers |
| M4.2 | Loader: embedded rules + user override directory | `wexo-rules/src/loader.rs` | Bad user rule skipped with warning |
| M4.3 | Rule → Recommendation ranking (priority × confidence) | `wexo-rules/src/rank.rs` | Stable order test |
| M4.4 | Seed `rules/rules.json` (≥ 20 rules) | `rules/rules.json` | Schema validation test |
| M4.5 | `analyze` command returning recommendations | `commands.rs` | < 5 s on target hardware |
| M4.6 | One-click flow UI with review + live progress log | `src/pages/optimizations/OneClickFlow.tsx` | Cancel mid-flight safe |
| M4.7 | Dashboard: score, alerts, quick actions, before/after | `src/pages/Dashboard.tsx` | Matches spec §6.2 |

---

## 7. Milestone M5 — Ship

| ID | Task | Acceptance |
|----|------|------------|
| M5.1 | Benchmark suite (disk sequential/random, CPU, RAM, boot time) | Reproducible ±5% |
| M5.2 | Auto-update (stable/beta) + AppInstaller XML | Update round-trip on VM |
| M5.3 | MSIX + NSIS packaging in CI | Both artifacts install |
| M5.4 | Playwright E2E: diagnose → optimize → rollback | Green on Win10/11 VMs |
| M5.5 | SBOM (CycloneDX), signing hooks, reproducible build docs | Artifacts signed when secrets present |
| M5.6 | Docs: README, privacy policy, user guide ES/EN | Published |

---

## 8. Cross-cutting conventions

- **Command naming:** `snake_case` in Rust; frontend wrapper in `src/lib/ipc.ts` typed via generated bindings.
- **Event naming:** `domain://event` (e.g. `monitor://metrics`, `task://progress`).
- **Timestamps:** UTC ISO-8601 in storage, local formatting only in UI.
- **IDs:** optimization IDs are `kebab-case` and stable forever (rollback log references them).
- **Units:** bytes, millidegrees Celsius stored as integer in Rust; formatting in UI.
- **No hidden network:** any HTTP use must be explicit, user-visible, and opt-in.
- **Destructive actions:** always behind snapshot + confirmation dialog + typed reason in log.

---

## 9. Risk checkpoints

| When | Check |
|------|-------|
| End of M0 | `cargo clippy -D warnings`, no Tauri capability beyond `core:default` + dialog/notification |
| Before M2.6 | Dry-run + VM snapshot harness ready |
| Before M3 | Bench baseline recorded (CPU/RAM idle) |
| Before M5 | Win10 21H2 + Win11 24H2 VMs available |
