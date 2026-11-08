import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

const BYTE_UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

/** Formats a byte count with one decimal for GB/TB and no decimals for B/KB/MB. */
export function formatBytes(bytes: number, fractionDigits = 1): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return "0 B";
  }
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), BYTE_UNITS.length - 1);
  const value = bytes / 1024 ** exponent;
  const digits = exponent >= 3 ? fractionDigits : 0;
  return `${value.toFixed(digits)} ${BYTE_UNITS[exponent]}`;
}

export function formatMhz(mhz: number): string {
  if (!Number.isFinite(mhz) || mhz <= 0) {
    return "—";
  }
  return mhz >= 1000 ? `${(mhz / 1000).toFixed(2)} GHz` : `${Math.round(mhz)} MHz`;
}

export function formatPercent(value: number, fractionDigits = 0): string {
  if (!Number.isFinite(value)) {
    return "—";
  }
  return `${value.toFixed(fractionDigits)}%`;
}

/** Formats a throughput in bytes per second, e.g. `"1.2 MB/s"`. */
export function formatRate(bytesPerSecond: number): string {
  if (!Number.isFinite(bytesPerSecond) || bytesPerSecond < 0) {
    return "—";
  }
  return `${formatBytes(bytesPerSecond)}/s`;
}

/** Formats an uptime in seconds as `Xd Yh Zm`, `Yh Zm`, `Zm` or `Ns`. */
export function formatDuration(totalSeconds: number): string {
  if (!Number.isFinite(totalSeconds) || totalSeconds <= 0) {
    return "—";
  }
  const seconds = Math.floor(totalSeconds);
  const days = Math.floor(seconds / 86_400);
  const hours = Math.floor((seconds % 86_400) / 3_600);
  const minutes = Math.floor((seconds % 3_600) / 60);

  if (days > 0) return `${days}d ${hours}h ${minutes}m`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m`;
  return `${seconds}s`;
}

export function formatNumber(value: number): string {
  return new Intl.NumberFormat("es-ES").format(value);
}

/** Maps a 0-100 score to a semantic tone used by badges and rings. */
export function scoreTone(score: number): "ok" | "warn" | "danger" {
  if (score >= 75) return "ok";
  if (score >= 50) return "warn";
  return "danger";
}

/** Qualitative label for the global health score, translated by the caller. */
export function healthLabel(score: number): "excellent" | "good" | "fair" | "poor" {
  if (score >= 85) return "excellent";
  if (score >= 70) return "good";
  if (score >= 50) return "fair";
  return "poor";
}
