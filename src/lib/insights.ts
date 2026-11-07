import type {
  BottleneckComponent,
  Confidence,
  HealthScore,
  Severity,
  SystemProfile,
} from "@/types";

import { clamp } from "./utils";

/**
 * An insight is a language-agnostic finding: the UI translates `titleKey` and
 * `descriptionKey` and interpolates `params`.
 */
export interface Insight {
  id: string;
  component: BottleneckComponent;
  severity: Severity;
  confidence: Confidence;
  titleKey: string;
  descriptionKey: string;
  params: Record<string, string | number>;
  impact: string;
}

const SEVERITY_WEIGHT: Record<Severity, number> = {
  critical: 4,
  high: 3,
  medium: 2,
  low: 1,
};

function ratioPercent(value: number, total: number): number {
  if (total <= 0) return 0;
  return (value / total) * 100;
}

export function computeHealth(profile: SystemProfile): HealthScore {
  const memoryUsed = ratioPercent(profile.memory.usedBytes, profile.memory.totalBytes);
  const memory = clamp(100 - Math.max(0, memoryUsed - 40) * 1.2, 20, 100);

  const cpu = clamp(100 - Math.max(0, profile.cpu.usagePercent - 15) * 1.1, 20, 100);

  const fixedDisks = profile.disks.filter((disk) => !disk.isRemovable);
  const worstFree = fixedDisks.length
    ? Math.min(...fixedDisks.map((disk) => ratioPercent(disk.availableBytes, disk.totalBytes)))
    : 100;

  let disk = 96;
  if (worstFree < 8) disk = 38;
  else if (worstFree < 15) disk = 64;
  else if (worstFree < 25) disk = 82;
  if (fixedDisks.some((entry) => entry.kind === "hdd")) {
    disk = Math.min(disk, 78);
  }

  const network = profile.networks.length > 0 ? 94 : 60;

  return {
    global: Math.round(cpu * 0.3 + memory * 0.25 + disk * 0.3 + network * 0.15),
    cpu: Math.round(cpu),
    memory: Math.round(memory),
    disk: Math.round(disk),
    network: Math.round(network),
  };
}

export function computeInsights(profile: SystemProfile): Insight[] {
  const insights: Insight[] = [];
  const memoryUsed = ratioPercent(profile.memory.usedBytes, profile.memory.totalBytes);
  const systemMemoryGb = profile.memory.totalBytes / 1024 ** 3;

  if (profile.memory.totalBytes > 0 && memoryUsed >= 85) {
    insights.push({
      id: "memory-high",
      component: "memory",
      severity: memoryUsed >= 93 ? "critical" : "high",
      confidence: "high",
      titleKey: "insights.memoryHigh.title",
      descriptionKey: "insights.memoryHigh.desc",
      params: { percent: Math.round(memoryUsed) },
      impact: "-8% RAM idle",
    });
  }

  if (profile.memory.totalBytes > 0 && systemMemoryGb < 8) {
    insights.push({
      id: "memory-low-capacity",
      component: "memory",
      severity: "medium",
      confidence: "high",
      titleKey: "insights.ramLow.title",
      descriptionKey: "insights.ramLow.desc",
      params: { total: `${systemMemoryGb.toFixed(0)} GB` },
      impact: "Recomendado: 16 GB",
    });
  }

  if (profile.memory.channels === 1) {
    insights.push({
      id: "memory-single-channel",
      component: "memory",
      severity: "medium",
      confidence: "high",
      titleKey: "insights.singleChannel.title",
      descriptionKey: "insights.singleChannel.desc",
      params: {},
      impact: "+15% ancho de banda",
    });
  }

  for (const disk of profile.disks.filter((entry) => !entry.isRemovable)) {
    const free = ratioPercent(disk.availableBytes, disk.totalBytes);
    if (free < 12) {
      insights.push({
        id: `disk-free-${disk.mountPoint}`,
        component: "disk",
        severity: free < 6 ? "critical" : "high",
        confidence: "high",
        titleKey: "insights.diskLow.title",
        descriptionKey: "insights.diskLow.desc",
        params: {
          mount: disk.mountPoint,
          free: `${(disk.availableBytes / 1024 ** 3).toFixed(1)} GB`,
          percent: Math.round(free),
        },
        impact: "Riesgo de actualizaciones fallidas",
      });
    }
    if (disk.kind === "hdd" && disk.mountPoint.toUpperCase().startsWith("C:")) {
      insights.push({
        id: "disk-hdd-system",
        component: "disk",
        severity: "medium",
        confidence: "medium",
        titleKey: "insights.hddSystem.title",
        descriptionKey: "insights.hddSystem.desc",
        params: {},
        impact: "-40% tiempo de arranque",
      });
    }
  }

  if (!profile.disks.some((disk) => disk.kind === "nvme")) {
    insights.push({
      id: "no-nvme",
      component: "disk",
      severity: "low",
      confidence: "high",
      titleKey: "insights.noNvme.title",
      descriptionKey: "insights.noNvme.desc",
      params: {},
      impact: "Mejora de almacenamiento",
    });
  }

  if (profile.cpu.usagePercent >= 80) {
    insights.push({
      id: "cpu-high",
      component: "cpu",
      severity: profile.cpu.usagePercent >= 95 ? "high" : "medium",
      confidence: "medium",
      titleKey: "insights.cpuHigh.title",
      descriptionKey: "insights.cpuHigh.desc",
      params: { percent: Math.round(profile.cpu.usagePercent) },
      impact: "Revisar procesos",
    });
  }

  return insights.sort(
    (left, right) => SEVERITY_WEIGHT[right.severity] - SEVERITY_WEIGHT[left.severity],
  );
}

export function healthLabel(score: number): "excellent" | "good" | "fair" | "poor" {
  if (score >= 85) return "excellent";
  if (score >= 70) return "good";
  if (score >= 50) return "fair";
  return "poor";
}
