import { describe, expect, it } from "vitest";

import type { SystemProfile } from "@/types";

import { computeHealth, computeInsights, healthLabel } from "./insights";

function profile(overrides: Partial<SystemProfile> = {}): SystemProfile {
  return {
    cpu: {
      brand: "Test CPU",
      vendor: "Test",
      physicalCores: 8,
      logicalCores: 16,
      currentFrequencyMhz: 3600,
      usagePercent: 10,
    },
    memory: {
      totalBytes: 32 * 1024 ** 3,
      usedBytes: 8 * 1024 ** 3,
      availableBytes: 24 * 1024 ** 3,
      swapTotalBytes: 4 * 1024 ** 3,
      swapUsedBytes: 0,
      channels: 2,
    },
    disks: [
      {
        name: "Test NVMe",
        mountPoint: "C:\\",
        totalBytes: 1000 * 1024 ** 3,
        availableBytes: 500 * 1024 ** 3,
        kind: "nvme",
        isRemovable: false,
      },
    ],
    networks: [
      { name: "Ethernet", macAddress: "00:00:00:00:00:00", receivedBytes: 0, transmittedBytes: 0 },
    ],
    os: {
      name: "Windows",
      version: "11",
      build: "",
      kernelVersion: "10.0",
      hostname: "test",
      arch: "x86_64",
    },
    collectedAtMs: 0,
    ...overrides,
  };
}

describe("computeHealth", () => {
  it("scores a healthy machine highly", () => {
    const health = computeHealth(profile());
    expect(health.global).toBeGreaterThan(85);
    expect(health.disk).toBe(96);
  });

  it("penalizes a nearly full system disk", () => {
    const health = computeHealth(
      profile({
        disks: [
          {
            name: "Full disk",
            mountPoint: "C:\\",
            totalBytes: 1000,
            availableBytes: 40,
            kind: "hdd",
            isRemovable: false,
          },
        ],
      }),
    );
    expect(health.disk).toBeLessThan(50);
    expect(health.global).toBeLessThan(85);
  });
});

describe("computeInsights", () => {
  it("reports low disk space with translated keys", () => {
    const insights = computeInsights(
      profile({
        disks: [
          {
            name: "C",
            mountPoint: "C:\\",
            totalBytes: 1000,
            availableBytes: 50,
            kind: "nvme",
            isRemovable: false,
          },
        ],
      }),
    );

    expect(insights.some((insight) => insight.titleKey === "insights.diskLow.title")).toBe(true);
  });

  it("sorts findings by severity", () => {
    const insights = computeInsights(
      profile({
        cpu: { ...profile().cpu, usagePercent: 99 },
        memory: { ...profile().memory, channels: 1 },
      }),
    );

    expect(insights[0]?.severity).toBe("high");
    expect(insights.map((insight) => insight.severity)).not.toContain("critical");
  });
});

describe("healthLabel", () => {
  it("maps score ranges", () => {
    expect(healthLabel(92)).toBe("excellent");
    expect(healthLabel(75)).toBe("good");
    expect(healthLabel(60)).toBe("fair");
    expect(healthLabel(20)).toBe("poor");
  });
});
