import type { RiskLevel, Severity } from "@/types";

export type Tone = "neutral" | "accent" | "ok" | "warn" | "danger" | "muted";

export function severityTone(severity: Severity): Tone {
  switch (severity) {
    case "low":
      return "ok";
    case "medium":
      return "warn";
    default:
      return "danger";
  }
}

export function riskTone(risk: RiskLevel): Tone {
  switch (risk) {
    case "low":
      return "ok";
    case "medium":
      return "warn";
    default:
      return "danger";
  }
}

export function usageTone(percent: number): "accent" | "warn" | "danger" {
  if (percent >= 90) return "danger";
  if (percent >= 75) return "warn";
  return "accent";
}

/** Temperature tone: a CPU is fine up to ~80 °C and throttles near 95 °C. */
export function thermalTone(celsius: number): "accent" | "warn" | "danger" {
  if (celsius >= 90) return "danger";
  if (celsius >= 80) return "warn";
  return "accent";
}
