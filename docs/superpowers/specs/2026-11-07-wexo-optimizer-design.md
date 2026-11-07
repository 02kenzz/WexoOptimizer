# Wexo Optimizer — Design Specification

**Date:** 2026-11-07  
**Status:** Approved for planning  
**Target:** Tauri v2 + Rust (backend) + React/TypeScript (frontend)

---

## 1. Intent & Why

**Wexo Optimizer** es un centro de control inteligente para Windows que diagnostica, optimiza y monitoriza el sistema de forma segura y medible. No es una colección de interruptores: analiza el hardware y uso real de cada equipo, recomienda optimizaciones específicas, las aplica de forma atómica y reversible, y muestra el impacto antes/después con datos reales.

**Problema que resuelve:** Las herramientas actuales (HONE, optimizadores genéricos) aplican cambios "a ciegas", no explican qué hacen, no permiten medir resultados, y a menudo rompen cosas sin forma de volver atrás.

**Diferenciador:** Diagnóstico basado en datos → Optimizaciones atómicas con rollback → Verificación de impacto real → Monitorización continua.

---

## 2. Goals, Non-Goals, Anti-Goals

### Goals (must have)
- **Diagnóstico inteligente:** Perfilado de hardware (CPU, GPU, RAM, disco, red), detección de cuellos de botella, scoring de rendimiento
- **Optimizaciones reales y seguras:** Cada cambio es atómico, reversible, con snapshot previo y explicación en lenguaje natural
- **Benchmark before/after:** Medición objetiva del impacto (latencia, throughput, temps, FPS, boot time, etc.)
- **Monitor en tiempo real:** CPU/GPU/RAM/Disco/Red/Temps/Procesos con overhead < 1% CPU en idle
- **Motor de recomendaciones:** Rules engine que adapta optimizaciones a hardware/uso detectado
- **Distribución profesional:** Instalador MSIX, auto-update, firma de código, sin telemetría obligatoria
- **Recursos mínimos en background:** Servicio Rust < 50 MB RAM, < 0.5% CPU idle

### Non-Goals (explicitly out of scope)
- Overclocking/undervolting automático (riesgo de inestabilidad/hardware)
- Modificación de BIOS/UEFI
- Drivers personalizados o kernel-mode
- "Game boosters" que matan procesos indiscriminadamente
- Limpieza de registro agresiva (riesgo alto, beneficio bajo)

### Anti-Goals (failure if present)
- Cualquier optimización sin rollback automático probado
- Cambios opacos sin explicación al usuario
- Telemetría obligatoria o datos que salgan del PC sin consentimiento explícito
- UI que se cuelgue o consuma > 100 MB RAM en idle
- Falsos positivos en diagnóstico (recomendar lo que perjudica)

---

## 3. Constraints

| Constraint | Detail |
|------------|--------|
| **OS** | Windows 10 21H2+ / Windows 11 (x64, ARM64 opcional) |
| **Permisos** | Admin requerido para optimizaciones; monitor funciona sin admin |
| **Runtime** | Tauri v2 (WebView2), Rust 1.78+, Node 20+ |
| **Distribución** | MSIX (Store/sideload) + NSIS portable; firma EV/Standard |
| **Idiomas** | ES/EN al lanzamiento; i18n ready |
| **Accesibilidad** | WCAG 2.1 AA, high contrast, screen readers |
| **Offline-first** | Sin conexión requerida; updates opcionales |

---

## 4. Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Tauri App (WebView2)                   │
│  ┌──────────────────┐    ┌──────────────────────────────┐  │
│  │  Frontend        │◄──►│  Backend (Rust)              │  │
│  │  React + TS      │    │  Tauri Commands / Events     │  │
│  │  - UI/State      │    │  - System APIs (WMI, PDH)    │  │
│  │  - Charts/Views  │    │  - Optimization Engine       │  │
│  │  - Settings      │    │  - Snapshot/Rollback         │  │
│  └──────────────────┘    │  - Monitoring Service        │  │
│                          │  - Rules Engine              │  │
│                          └──────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
        ┌──────────┐   ┌────────────┐  ┌────────────┐
        │  WMI /   │   │  Win32 /   │  │  Hardware  │
        │  CIM     │   │  PowerShell│  │  Libraries │
        └──────────┘   └────────────┘  └────────────┘
```

### 4.1 Frontend (React + TypeScript)
- **Framework:** React 18 + TypeScript + Vite
- **Estado:** Zustand (global) + React Query (server state)
- **UI:** Radix UI + Tailwind CSS (tema oscuro/claro, high contrast)
- **Gráficos:** uPlot (ligero, 60fps, < 30 KB gz)
- **i18n:** react-i18next

### 4.2 Backend (Rust)
- **Core crate:** `wexo-core` — lógica pura, testeable sin Tauri
- **System crates:**
  - `wexo-sysinfo` — wrapper sobre `sysinfo`, `windows-perf`, `wmi`
  - `wexo-optimizer` — motor de optimizaciones, snapshots, rollback
  - `wexo-rules` — rules engine (JSON/DSL para recomendaciones)
  - `wexo-monitor` — servicio background, streaming via Tauri events
- **Seguridad:** Capabilities mínimas (`fs-read`, `fs-write`, `process`, `shell` scoped)

### 4.3 Comunicación Frontend ↔ Backend
- **Commands (request/response):** `invoke("cmd_name", args)` — optimizaciones, snapshots, diagnósticos puntuales
- **Events (streaming):** `listen("monitor://*")` — métricas en tiempo real, progreso de tareas largas
- **Tipado compartido:** `wexo-types` (TS/Rust via `ts-rs` o `specta`)

---

## 5. Core Modules

### 5.1 Diagnóstico Inteligente (`wexo-core::diagnostics`)

**Entradas:**
- Hardware: CPU (modelo, cores, cache, freq base/boost, TDP), GPU (vendor, VRAM, driver), RAM (capacidad, velocidad, timings, canales), Disco (tipo, NVMe/SATA, salud SMART), Red (adaptador, velocidad link)
- Sistema: Windows build, drivers, servicios, tareas programadas, inicio
- Uso actual: Métricas 30s/5m/1h (P95, media, picos)

**Salidas:**
- `SystemProfile` — snapshot estructurado del hardware/config
- `BottleneckReport` — lista priorizada: `{component, severity, evidence, impact_estimate, confidence}`
- `HealthScore` — 0-100 por subsistema + global

**Fuentes de datos:**
- WMI/CIM: `Win32_Processor`, `Win32_VideoController`, `Win32_PhysicalMemory`, `Win32_DiskDrive`, `MSStorageDriver_FailurePredictData`
- Performance Counters (PDH): `\Processor\% Processor Time`, `\Memory\Available MBytes`, `\PhysicalDisk\% Disk Time`, `\Network Interface\Bytes Total/sec`
- `sysinfo` crate para temps (CPU/GPU/NVMe via SMART)
- PowerShell para: `Get-Service`, `Get-ScheduledTask`, `Get-ItemProperty HKLM:\Software\Microsoft\Windows\CurrentVersion\Run`

**Ejemplo de detección:**
- CPU throttling: `% Processor Time` alto + freq actual < base freq sostenido → thermal/power limit
- RAM single-channel: 2 DIMMs pero mismo canal (WMI `BankLabel`) → bandwidth halved
- Disco 100% activo: `% Disk Time` > 90% + cola larga → supervisar `Avg. Disk Queue Length`
- GPU subutilizada en render: `DXGI` frame timing + `nvidia-smi` / `rocm-smi`

### 5.2 Motor de Optimizaciones (`wexo-core::optimizer`)

**Principios:**
1. **Atomicidad:** Cada optimización = una transacción (apply + verify + rollback)
2. **Reversibilidad:** Snapshot previo (registro, archivos, servicios) → rollback en 1 click
3. **Explicabilidad:** `description: string`, `rationale: string`, `risk: Low|Medium|High`, `requires_reboot: bool`
4. **Medibilidad:** `benchmark_before()`, `benchmark_after()`, `delta: BenchmarkDelta`

**Categorías de optimizaciones (ejemplos):**

| Categoría | Ejemplos | Riesgo | Requiere Reboot |
|-----------|----------|--------|-----------------|
| **Power/Performance** | Plan de energía Ultimate, CPU parking, Core parking, Timer resolution | Low | No |
| **Servicios** | Deshabilitar telemetría, SysMain (prefetch), DiagTrack, MapsBroker | Low-Med | No |
| **Inicio** | Deshabilitar apps de inicio innecesarias, Task Scheduler tasks | Low | No |
| **Sistema de archivos** | Disable 8.3 names, LastAccess update, NTFS compression selectiva | Low | Sí (algunas) |
| **Red** | TCP autotuning, RSS, Chimney offload, DNS cache | Low | No |
| **Memoria** | Clear standby list (EmptyWorkingSet), Large System Cache | Med | No |
| **GPU** | Hardware-accelerated GPU scheduling (HAGS), Low latency mode | Low | Sí |
| **Visuales** | Animaciones, transparencias, efectos visuales | Low | No |
| **Privacidad/Telemetría** | Diagnostic data off, tailored experiences, error reporting | Low | No |

**Estructura de una optimización (Rust):**
```rust
pub struct Optimization {
    pub id: String,                    // "disable-sysmain"
    pub category: Category,
    pub title: String,                 // "Deshabilitar SysMain (Superfetch)"
    pub description: String,           // "Detiene el servicio que precarga apps..."
    pub rationale: String,             // "En SSD/NVMe moderno, precarga no ayuda y consume I/O"
    pub risk: RiskLevel,               // Low
    pub requires_reboot: bool,
    pub applies_to: Applicability,     // {os_min, hardware_filters, conflicts_with}
    pub snapshot: SnapshotSpec,        // Qué guardar para rollback
    pub apply: fn(&mut System) -> Result<()>,
    pub verify: fn(&System) -> VerificationResult,
    pub benchmark: Option<BenchmarkSpec>,
    pub rollback: fn(&Snapshot) -> Result<()>,
}
```

**Snapshot/Rollback:**
- Registro: exportar claves afectadas a `.reg` + hash SHA256
- Servicios: estado previo (Running/Stopped/Disabled) + config de recuperación
- Archivos: copia a `%TEMP%\wexo-snapshots\{id}\` + manifest JSON
- Rollback: transacción inversa validada por hash

### 5.3 Safety Layer (`wexo-core::safety`)

- **Pre-flight checks:** Admin rights, disco espacio (> 500 MB), punto de restauración del sistema (opcional), no updates pendientes
- **Transaction log:** SQLite local (`%LOCALAPPDATA%\WexoOptimizer\transactions.db`) — cada acción registrada con timestamp, params, resultado, rollback_available
- **Rollback UI:** Historial cronológico, filtro por categoría, "Deshacer todo" (en orden inverso)
- **Explicaciones:** Generación de texto natural desde `Optimization.rationale` + contexto detectado ("En tu NVMe Samsung 980 Pro, SysMain consume 15% I/O innecesario")

### 5.4 Monitorización Avanzada (`wexo-core::monitor`)

**Arquitectura:** Servicio Rust separado (background) + streaming via Tauri events

**Métricas (frecuencia configurable):**
| Métrica | Fuente | Frecuencia default |
|---------|--------|-------------------|
| CPU % total / por core | PDH / sysinfo | 1s |
| CPU freq / temp / power | sysinfo (MSR/HwMon) | 2s |
| GPU % / VRAM / temp / power | DXGI / NVML / ADL | 2s |
| RAM used / available / standby / modified | sysinfo | 1s |
| Disco I/O / latency / queue / temp (SMART) | PDH / SMART | 2s |
| Red up/down / conexiones / latency | PDH / GetExtendedTcpTable | 1s |
| Procesos top CPU/RAM/GPU/Disco/Red | sysinfo / PDH | 5s |
| Servicios estado | WMI | 30s |

**Overhead objetivo:** < 1% CPU, < 50 MB RAM en idle (sampling 1s)
**Persistencia:** Buffer circular en memoria (última hora) + opcional export CSV/JSON

**Alertas:** Umbrales configurables → toast nativo + log

### 5.5 Motor de Recomendaciones (`wexo-core::rules`)

**Formato reglas (JSON/DSL):**
```json
{
  "id": "reco-disable-sysmain-nvme",
  "title": "Deshabilitar SysMain en SSD/NVMe",
  "condition": "disk.type == 'NVMe' && service.sysmain.running == true",
  "optimization_id": "disable-sysmain",
  "priority": 85,
  "reason": "SysMain precarga apps en RAM, pero en NVMe el acceso aleatorio es ~50μs vs ~10ms HDD. No hay beneficio medible y consume ciclos de I/O."
}
```

**Motor:** Evalúa reglas contra `SystemProfile` + métricas actuales → lista priorizada de `Recommendation`
**Extensibilidad:** Archivo `rules.json` en `%LOCALAPPDATA%\WexoOptimizer\rules\` — usuario puede añadir/editar/deshabilitar

---

## 6. UI/UX Structure

### 6.1 Navegación principal (Sidebar)
1. **Dashboard** — Resumen salud, score, alertas, acciones rápidas
2. **Diagnóstico** — Perfil hardware, cuellos de botella, health scores
3. **Optimizaciones** — Lista categorizada, filtros, búsqueda, "Aplicar seleccionadas"
4. **Monitor** — Tiempo real, gráficos, procesos, alertas
5. **Historial / Rollback** — Transacciones, snapshots, deshacer
6. **Configuración** — General, monitor, reglas, updates, about

### 6.2 Flujos clave

**Flujo "Optimizar ahora" (One-click):**
1. Usuario pulsa "Analizar y optimizar"
2. Diagnóstico completo (10-30s) → `BottleneckReport`
3. Motor de reglas → `Recommendation[]` priorizadas
4. UI muestra: "Se detectaron 7 optimizaciones aplicables. Impacto estimado: +12% score, -5°C CPU, -8% RAM idle"
5. Usuario revisa (expandible: qué hace, por qué, riesgo, benchmark)
6. Usuario pulsa "Aplicar seleccionadas" → barra progreso + log en vivo
7. Auto-benchmark post-aplicación (configurable: rápido/completo)
8. Resultado: "Score 68 → 79. CPU idle 18% → 12%. Rollback disponible en Historial"

**Flujo "Optimización individual":**
- Usuario navega a Optimizaciones → filtra "Red" → ve "TCP Autotuning"
- Expande: explicación, riesgo Low, requiere reboot No, benchmark estimado +3% throughput
- Pulsa "Aplicar" → snapshot → apply → verify → toast "Aplicado. Rollback en Historial"

---

## 7. Distribución y Operaciones

### 7.1 Empaquetado
- **MSIX** (primario): `tauri build --target msix` — Store-ready, auto-update via AppInstaller, sandbox opcional
- **NSIS** (portable/enterprise): Instalador offline, sin Store, crea acceso directo, desinstalador
- **Arquitecturas:** x64 (principal), ARM64 (si CI lo soporta)

### 7.2 Firma y Confianza
- Certificado EV Code Signing (SmartScreen reputation inmediato)
- Timestamp RFC3161
- SBOM (CycloneDX) generado en CI
- Reproducible builds (verificable)

### 7.3 Auto-Update
- Canal: `stable` / `beta` (selector en Configuración)
- Backend: GitHub Releases / servidor propio + AppInstaller XML
- Delta updates (MSIX) / full (NSIS)
- Silent background download → prompt al reiniciar

### 7.4 Telemetría (Opt-in, Off by Default)
- **Qué:** Crash reports (minidump), feature usage (anónimo), performance metrics (aggregado)
- **Dónde:** Endpoint propio (no terceros)
- **Control:** Toggle en Configuración + "Ver datos que se envían" (JSON crudo)
- **Sin:** PII, hardware IDs, rutas de archivos, contenido de discos

### 7.5 Logging y Diagnóstico
- Log rotativo (`%LOCALAPPDATA%\WexoOptimizer\logs\wexo-YYYY-MM-DD.log`)
- Niveles: Error, Warn, Info, Debug (configurable)
- "Exportar logs para soporte" → ZIP con logs + system profile (sanitizado)

---

## 8. Testing Strategy

| Nivel | Herramienta | Qué cubre |
|-------|-------------|-----------|
| **Unit (Rust)** | `cargo test` | `wexo-core`, `wexo-rules`, `wexo-optimizer` logic |
| **Integration (Rust)** | `cargo test --test integration` | WMI wrappers, PDH, snapshot/rollback real |
| **Unit (TS)** | Vitest | Stores, utils, i18n, rules parsing |
| **Component (TS)** | React Testing Library | UI components aislados |
| **E2E** | Playwright + Tauri driver | Flujos completos: diagnóstico → optimización → rollback |
| **Bench** | Criterion (Rust) | Overhead monitor, latencia commands, memoria |
| **Stress** | Custom | 24h monitor running, 100 optimizaciones apply/rollback |

**CI Pipeline (GitHub Actions):**
1. `cargo test --workspace` + `bun test`
2. `cargo clippy -- -D warnings` + `bunx biome check`
3. Build MSIX/NSIS (matrix: x64, ARM64)
4. Test E2E en VM Windows 10/11 clean
5. Firmar artifacts (si secrets disponibles)
6. Publicar Release draft

---

## 9. Project Structure (Monorepo)

```
wexo-optimizer/
├── Cargo.toml                 # Workspace root
├── package.json               # Frontend deps/scripts
├── tauri.conf.json            # Tauri config
├── docs/
│   └── superpowers/specs/     # Este documento
├── src/                       # Frontend (React + TS)
│   ├── components/
│   ├── pages/
│   ├── stores/
│   ├── hooks/
│   ├── i18n/
│   └── main.tsx
├── src-tauri/                 # Backend (Rust)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands.rs        # Tauri commands
│   │   ├── events.rs          # Tauri events
│   │   ├── core/              # wexo-core (lib)
│   │   ├── sysinfo/           # wexo-sysinfo
│   │   ├── optimizer/         # wexo-optimizer
│   │   ├── rules/             # wexo-rules
│   │   ├── monitor/           # wexo-monitor
│   │   └── safety/            # wexo-safety
│   └── build.rs
├── rules/                     # rules.json (embebido + user override)
├── tests/
│   ├── e2e/                   # Playwright
│   └── fixtures/
└── .github/workflows/         # CI
```

---

## 10. What's Left to the Builder

Decisions the builder should make (documented here as "my call" unless user overrides):

| Decision | Default Choice | Rationale |
|----------|----------------|-----------|
| **State management** | Zustand + React Query | Simple, no boilerplate, buena DX |
| **Charts** | uPlot | 60fps, <30KB, sin dependencias pesadas |
| **CSS** | Tailwind + CSS Variables (temas) | Dark/light/HC nativo, tree-shaking |
| **Rules format** | JSON + `serde_json` | Simple, editable por usuario, versionable |
| **Benchmark suite** | Custom (CrystalDiskMark-like para disco, custom CPU/GPU/RAM) | Sin dependencias externas, reproducible |
| **GPU metrics** | `dxgi` crate + `nvml` (NVIDIA) + `adl` (AMD) opcional | Cobertura amplia, fallback graceful |
| **SMART reading** | `smartctl` wrapper (vendored) | Estándar de facto, funciona en RAID básico |
| **Installer default** | MSIX (Store) + NSIS (portable) | Máxima compatibilidad |
| **Logging** | `tracing` + `tracing-appender` (rolling file) | Estructurado, filtrable, bajo overhead |
| **Error handling** | `thiserror` + `anyhow` en commands | Tipado en Rust, serializable a TS |

---

## 11. Milestones (High-Level)

| Phase | Scope | Est. Duration |
|-------|-------|---------------|
| **M0: Foundation** | Tauri setup, Rust workspace, CI, logging, config, i18n | 2-3 semanas |
| **M1: Diagnóstico** | Hardware profiling, WMI/PDH wrappers, bottleneck scoring, health score | 3-4 semanas |
| **M2: Optimizador Core** | Snapshot/rollback engine, 15-20 optimizaciones base, transaction log | 4-5 semanas |
| **M3: Monitor** | Background service, streaming events, UI gráficos, alertas | 3-4 semanas |
| **M4: Reglas & UX** | Rules engine, recomendaciones, dashboard, one-click flow | 2-3 semanas |
| **M5: Polish & Ship** | MSIX/NSIS, firma, auto-update, tests E2E, docs, landing page | 3-4 semanas |

**Total estimado:** 17-23 semanas (4-6 meses) para v1.0 estable

---

## 12. Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Win32/WMI APIs cambian entre builds Windows | Media | Alto | Abstracción `wexo-sysinfo` + tests en VMs Win10/11 insiders |
| Optimización rompe sistema (ej. deshabilitar servicio crítico) | Baja | Crítico | Whitelist/blacklist por versión OS, snapshot obligatorio, dry-run mode |
| Overhead monitor alto en laptops antiguas | Media | Medio | Sampling adaptativo (baja freq si battery), perf testing continuo |
| Falsos positivos en diagnóstico | Media | Alto | Rules versionadas, feedback loop usuario ("¿Fue útil?"), ML ligero futuro |
| Certificado EV costoso/retrasos | Media | Medio | Empezar con Standard, migrar a EV tras tracción |
| Antivirus false positive en optimizaciones | Media | Medio | Firma EV, heurísticas limpias, submit a AV vendors pre-release |

---

## 13. Approval

**Status:** ✅ Approved for planning phase.

**Next step:** Invoke `superpowers:writing-plans` to generate detailed implementation plan with tasks, dependencies, and file-level breakdown.

---

*Document committed to `docs/superpowers/specs/2026-11-07-wexo-optimizer-design.md`*