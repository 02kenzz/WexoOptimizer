# Wexo Optimizer

Centro de control para Windows que diagnostica, optimiza y monitoriza el sistema
de forma segura y medible: cada cambio es atómico, reversible y verificable.

> Estado: **M0 · Foundation**. El shell de la aplicación funciona (Tauri v2 + React),
> con diagnóstico básico de hardware en vivo. El motor de optimizaciones llega en M2.

## Stack

| Capa | Tecnología |
|------|------------|
| Shell | Tauri v2 (WebView2) |
| Backend | Rust (workspace de crates) |
| Frontend | React 18 + TypeScript + Vite |
| Estilos | Tailwind CSS 4 + tokens CSS (dark/light/high-contrast) |
| Estado | Zustand + TanStack Query |
| IPC | Tauri commands tipados con `ts-rs` |
| Tests | Vitest + Testing Library · `cargo test` |

## Requisitos

- Node 20+ (probado con 24) y npm
- Rust stable con el toolchain MSVC
- WebView2 Runtime (incluido en Windows 11)
- Visual Studio Build Tools con "Desktop development with C++"

## Desarrollo

```bash
npm install
npm run tauri dev      # app de escritorio con backend Rust
npm run dev            # solo frontend (datos simulados en el navegador)
```

## Scripts

| Comando | Descripción |
|---------|-------------|
| `npm run dev` | Vite en `localhost:1420` |
| `npm run build` | Build de producción del frontend |
| `npm run typecheck` | TypeScript sin emitir |
| `npm test` | Vitest (una pasada) |
| `npm run lint` | Biome |
| `npm run bindings` | Regenera los tipos TS desde Rust |
| `npm run tauri build` | Instaladores MSIX/NSIS (M5) |

## Estructura

```
crates/wexo-core       Tipos de dominio puros + lógica testeable (sin Windows)
crates/wexo-sysinfo    Colectores de hardware/OS (sysinfo, WMI en M1)
crates/wexo-optimizer  Registro de optimizaciones y motor apply/rollback (M2)
crates/wexo-safety     Preflight, snapshots y transaction log (M2)
crates/wexo-rules      Motor de reglas y recomendaciones (M4)
crates/wexo-monitor    Muestreo y buffer circular de métricas (M3)
src-tauri              Shell Tauri, commands, config, logging
src/                   Frontend React
docs/superpowers       Spec y plan de implementación
```

## Documentación

- [Diseño](docs/superpowers/specs/2026-11-07-wexo-optimizer-design.md)
- [Plan de implementación](docs/superpowers/plans/2026-11-07-wexo-optimizer-implementation-plan.md)

## Privacidad

Sin telemetría por defecto. Sin conexiones de red ocultas. Todo se guarda en
`%LOCALAPPDATA%\WexoOptimizer`.
