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

export function formatNumber(value: number): string {
  return new Intl.NumberFormat("es-ES").format(value);
}

/** Maps a 0-100 score to a semantic tone used by badges and rings. */
export function scoreTone(score: number): "ok" | "warn" | "danger" {
  if (score >= 75) return "ok";
  if (score >= 50) return "warn";
  return "danger";
}
