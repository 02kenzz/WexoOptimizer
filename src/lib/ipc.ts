import type { AppInfo, AppSettings, SystemProfile } from "@/types";

/**
 * True when running inside the Tauri webview. In a plain browser (`npm run dev`)
 * the IPC layer falls back to demo data so the UI can be designed without Rust.
 */
export const isDesktop =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in (window as object);

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

const demoProfile: SystemProfile = {
  cpu: {
    brand: "AMD Ryzen 9 5900X 12-Core Processor",
    vendor: "AuthenticAMD",
    physicalCores: 12,
    logicalCores: 24,
    currentFrequencyMhz: 3700,
    usagePercent: 13.4,
  },
  memory: {
    totalBytes: 34_359_738_368,
    usedBytes: 13_743_895_347,
    availableBytes: 20_615_843_021,
    swapTotalBytes: 4_294_967_296,
    swapUsedBytes: 0,
    channels: 2,
  },
  disks: [
    {
      name: "Samsung SSD 980 PRO 1TB",
      mountPoint: "C:\\",
      totalBytes: 1_000_204_886_016,
      availableBytes: 512_110_190_592,
      kind: "nvme",
      isRemovable: false,
    },
    {
      name: "ST4000DM004",
      mountPoint: "D:\\",
      totalBytes: 4_000_787_030_016,
      availableBytes: 1_811_939_377_152,
      kind: "hdd",
      isRemovable: false,
    },
  ],
  networks: [
    {
      name: "Ethernet",
      macAddress: "A8:5E:45:11:22:33",
      receivedBytes: 42_949_672_960,
      transmittedBytes: 8_589_934_592,
    },
  ],
  os: {
    name: "Windows",
    version: "11 Home",
    build: "22631",
    kernelVersion: "10.0.22631",
    hostname: "DESKTOP-WEXO",
    arch: "x86_64",
  },
  collectedAtMs: Date.now(),
};

/**
 * Typed IPC surface. Every function mirrors a `#[tauri::command]` in `src-tauri`.
 */
export const ipc = {
  ping(): Promise<string> {
    return isDesktop ? invoke("ping") : Promise.resolve("pong (browser)");
  },

  getAppInfo(): Promise<AppInfo> {
    if (!isDesktop) {
      return Promise.resolve({
        name: "Wexo Optimizer",
        version: "0.1.0-dev",
        os: "Windows 11 (modo navegador)",
        arch: "x86_64",
        isAdmin: false,
        logDir: "%LOCALAPPDATA%\\WexoOptimizer\\logs",
      });
    }
    return invoke("get_app_info");
  },

  getSystemProfile(): Promise<SystemProfile> {
    return isDesktop
      ? invoke("get_system_profile")
      : Promise.resolve({ ...demoProfile, collectedAtMs: Date.now() });
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
};

export { defaultSettings };
