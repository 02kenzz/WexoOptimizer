import type { Bottleneck } from "@/types";

/**
 * Converts a stable rule id to its translation key segment.
 *
 * `cpu-sustained` -> `cpuSustained`, so the UI can resolve
 * `insights.rules.cpuSustained.title` without shipping prose from Rust.
 */
export function ruleKey(id: string): string {
  return id.replace(/-([a-z0-9])/g, (_, character: string) => character.toUpperCase());
}

/** Evidence values keyed by code, ready for i18next interpolation. */
export function evidenceParams(bottleneck: Bottleneck): Record<string, string> {
  return Object.fromEntries(bottleneck.evidence.map((item) => [item.code, item.value]));
}

/**
 * Stable React key for a finding. Disk rules can fire once per volume, so the
 * mount point disambiguates them.
 */
export function bottleneckKey(bottleneck: Bottleneck): string {
  const mount = bottleneck.evidence.find((item) => item.code === "mount")?.value;
  return mount ? `${bottleneck.id}:${mount}` : bottleneck.id;
}
