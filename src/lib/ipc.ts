import type { AppInfo, AppSettings, DiagnosticReport, LiveMetrics, SystemProfile, Optimization, VerificationResult, BenchmarkDelta, TransactionStatus } from "@/types";

export interface OptimizationResult {
  optimization_id: string;
  applied: boolean;
  verification: VerificationResult;
  message?: string;
  benchmark_delta?: BenchmarkDelta;
}

export interface ApplyResult {
  transaction_id: string;
  results: OptimizationResult[];
}

export interface TransactionRecord {
  id: string;
  timestamp_ms: number;
  optimization_ids: string[];
  status: TransactionStatus;
  rollback_available: boolean;
  snapshot_id?: string;
  message?: string;
}

/**
 * True when running inside the Tauri webview. In a plain browser (`npm run dev`)
 * there is no backend: data commands reject instead of returning fake metrics.
 */
export const isDesktop =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in (window as object);

/** Must match `events::MONITOR_METRICS` in `src-tauri/src/events.rs`. */
export const MONITOR_METRICS_EVENT = "monitor://metrics";

const BACKEND_UNAVAILABLE =
  "El backend de Wexo Optimizer no está disponible. Abre la aplicación de escritorio.";

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke: tauriInvoke } = await import("@tauri-apps/api/core");
  return tauriInvoke<T>(command, args);
}

const SETTINGS_KEY = "wexo.settings";

const defaultSettings: AppSettings = {
  language: "es",
  theme: "system",
  monitorIntervalMs: 1000,
  minimizeToTray: false,
  updateChannel: "stable",
  telemetryEnabled: false,
};

const browserAppInfo: AppInfo = {
  name: "Wexo Optimizer",
  version: "0.1.0-dev",
  os: "Navegador (sin backend)",
  arch: "x86_64",
  isAdmin: false,
  logDir: "%LOCALAPPDATA%\\WexoOptimizer\\logs",
};

function backendUnavailable<T>(): Promise<T> {
  return Promise.reject(new Error(BACKEND_UNAVAILABLE));
}

/**
 * Typed IPC surface. Every function mirrors a `#[tauri::command]` in `src-tauri`.
 */
export const ipc = {
  ping(): Promise<string> {
    return isDesktop ? invoke("ping") : Promise.resolve("pong (browser)");
  },

  getAppInfo(): Promise<AppInfo> {
    return isDesktop ? invoke("get_app_info") : Promise.resolve(browserAppInfo);
  },

  getSystemProfile(): Promise<SystemProfile> {
    return isDesktop ? invoke("get_system_profile") : backendUnavailable();
  },

  /** Re-collects static hardware data (WMI) and returns the refreshed profile. */
  refreshProfile(): Promise<SystemProfile> {
    return isDesktop ? invoke("refresh_profile") : backendUnavailable();
  },

  getLiveMetrics(): Promise<LiveMetrics | null> {
    return isDesktop ? invoke("get_live_metrics") : backendUnavailable();
  },

  getMetricsHistory(): Promise<LiveMetrics[]> {
    return isDesktop ? invoke("get_metrics_history") : backendUnavailable();
  },

  getMonitorInterval(): Promise<number | null> {
    return isDesktop ? invoke("get_monitor_interval") : backendUnavailable();
  },

  runDiagnostics(): Promise<DiagnosticReport> {
    return isDesktop ? invoke("run_diagnostics") : backendUnavailable();
  },

  /**
   * Subscribes to live samples. Resolves to an unsubscribe function; in a plain
   * browser it resolves to a no-op.
   */
  async onMonitorSample(handler: (sample: LiveMetrics) => void): Promise<() => void> {
    if (!isDesktop) return () => {};
    const { listen } = await import("@tauri-apps/api/event");
    return listen<LiveMetrics>(MONITOR_METRICS_EVENT, (event) => handler(event.payload));
  },

  getSettings(): Promise<AppSettings> {
    if (isDesktop) {
      return invoke("get_settings");
    }
    try {
      const raw = localStorage.getItem(SETTINGS_KEY);
      return Promise.resolve(raw ? { ...defaultSettings, ...JSON.parse(raw) } : defaultSettings);
    } catch {
      return Promise.resolve(defaultSettings);
    }
  },

  saveSettings(settings: AppSettings): Promise<AppSettings> {
    if (isDesktop) {
      return invoke("save_settings", { settings });
    }
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
    return Promise.resolve(settings);
  },

  getOptimizations(): Promise<Optimization[]> {
    return isDesktop ? invoke("get_optimizations") : backendUnavailable();
  },

  applyOptimizations(optimization_ids: string[], dry_run: boolean): Promise<ApplyResult> {
    return isDesktop ? invoke("apply_optimizations", { optimization_ids, dry_run }) : backendUnavailable();
  },

  getTransactionHistory(limit: number): Promise<TransactionRecord[]> {
    return isDesktop ? invoke("get_transaction_history", { limit }) : backendUnavailable();
  },

  rollbackTransaction(transaction_id: string): Promise<void> {
    return isDesktop ? invoke("rollback_transaction", { transaction_id }) : backendUnavailable();
  },
};

export { BACKEND_UNAVAILABLE, defaultSettings };
