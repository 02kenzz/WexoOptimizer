import type {
  AppInfo,
  AppSettings,
  BenchmarkDelta,
  DiagnosticReport,
  LiveMetrics,
  Optimization,
  OverlayStats,
  SystemProfile,
  TransactionStatus,
  VerificationResult,
} from "@/types";

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

/** Result of a cleanup scan or cleanup run (mirrors `wexo_tools::CleanReport`). */
export interface CleanReport {
  bytes: number;
  files: number;
  skipped: number;
  roots: string[];
}

/** State of a long-running maintenance command. */
export type JobStatus = "running" | "succeeded" | "failed" | "cancelled";

export interface JobSnapshot {
  id: string;
  kind: string;
  label: string;
  status: JobStatus;
  startedMs: number;
  finishedMs: number | null;
  exitCode: number | null;
  lines: string[];
  droppedLines: number;
}

/** Size before/after of a database maintenance pass. */
export interface DatabaseReport {
  beforeBytes: number;
  afterBytes: number;
  transactions: number;
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
  overlayEnabled: false,
  dataDir: null,
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

  /** Snapshot for the floating performance overlay. */
  getOverlayStats(): Promise<OverlayStats> {
    return isDesktop ? invoke("get_overlay_stats") : backendUnavailable();
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

  openLogsDir(): Promise<void> {
    return isDesktop ? invoke("open_logs_dir") : backendUnavailable();
  },

  relaunchAsAdmin(): Promise<void> {
    return isDesktop ? invoke("relaunch_as_admin") : backendUnavailable();
  },

  getOptimizations(): Promise<Optimization[]> {
    return isDesktop ? invoke("get_optimizations") : backendUnavailable();
  },

  applyOptimizations(optimization_ids: string[], dry_run: boolean): Promise<ApplyResult> {
    return isDesktop
      ? invoke("apply_optimizations", { optimizationIds: optimization_ids, dryRun: dry_run })
      : backendUnavailable();
  },

  getTransactionHistory(limit: number): Promise<TransactionRecord[]> {
    return isDesktop ? invoke("get_transaction_history", { limit }) : backendUnavailable();
  },

  rollbackTransaction(transaction_id: string): Promise<void> {
    return isDesktop
      ? invoke("rollback_transaction", { transactionId: transaction_id })
      : backendUnavailable();
  },

  /** Measures what a cleanup would free. Deletes nothing. */
  scanCleanup(kind: string): Promise<CleanReport> {
    return isDesktop ? invoke("scan_cleanup", { kind }) : backendUnavailable();
  },

  /** Deletes the files of one cleanup kind. */
  runCleanup(kind: string): Promise<CleanReport> {
    return isDesktop ? invoke("run_cleanup", { kind }) : backendUnavailable();
  },

  /** Starts a long-running maintenance command (`sfc`, `dism`, `defrag`...). */
  startToolJob(tool: string): Promise<JobSnapshot> {
    return isDesktop ? invoke("start_tool_job", { tool }) : backendUnavailable();
  },

  getToolJob(id: string): Promise<JobSnapshot | null> {
    return isDesktop ? invoke("get_tool_job", { id }) : backendUnavailable();
  },

  getToolJobs(): Promise<JobSnapshot[]> {
    return isDesktop ? invoke("get_tool_jobs") : backendUnavailable();
  },

  cancelToolJob(id: string): Promise<boolean> {
    return isDesktop ? invoke("cancel_tool_job", { id }) : backendUnavailable();
  },

  /** `vacuum` compacts the database, `reindex` rebuilds its indexes. */
  maintainDatabase(action: "vacuum" | "reindex"): Promise<DatabaseReport> {
    return isDesktop ? invoke("maintain_database", { action }) : backendUnavailable();
  },
};

export { BACKEND_UNAVAILABLE, defaultSettings };
