import { describe, expect, it } from "vitest";

import type { Bottleneck } from "@/types";

import { bottleneckKey, evidenceParams, ruleKey } from "./bottlenecks";

describe("ruleKey", () => {
  it("converts kebab-case ids to camelCase translation segments", () => {
    expect(ruleKey("cpu-sustained")).toBe("cpuSustained");
    expect(ruleKey("memory-single-channel")).toBe("memorySingleChannel");
    expect(ruleKey("disk-space")).toBe("diskSpace");
    expect(ruleKey("network-no-adapter")).toBe("networkNoAdapter");
    expect(ruleKey("disk-saturated")).toBe("diskSaturated");
  });

  it("leaves ids without dashes untouched", () => {
    expect(ruleKey("cpu")).toBe("cpu");
  });
});

describe("evidenceParams", () => {
  it("keys values by evidence code", () => {
    const bottleneck: Bottleneck = {
      id: "disk-space",
      component: "disk",
      severity: "high",
      confidence: "high",
      evidence: [
        { code: "mount", value: "C:\\" },
        { code: "freeGb", value: "2.4" },
        { code: "freePercent", value: "3" },
      ],
    };

    expect(evidenceParams(bottleneck)).toEqual({
      mount: "C:\\",
      freeGb: "2.4",
      freePercent: "3",
    });
  });

  it("returns an empty object without evidence", () => {
    const bottleneck: Bottleneck = {
      id: "network-no-adapter",
      component: "network",
      severity: "low",
      confidence: "high",
      evidence: [],
    };
    expect(evidenceParams(bottleneck)).toEqual({});
  });
});

describe("bottleneckKey", () => {
  it("disambiguates disk rules by mount point", () => {
    const bottleneck: Bottleneck = {
      id: "disk-space",
      component: "disk",
      severity: "high",
      confidence: "high",
      evidence: [{ code: "mount", value: "D:\\" }],
    };
    expect(bottleneckKey(bottleneck)).toBe("disk-space:D:\\");
  });

  it("falls back to the rule id without a mount", () => {
    const bottleneck: Bottleneck = {
      id: "cpu-sustained",
      component: "cpu",
      severity: "high",
      confidence: "high",
      evidence: [{ code: "cpuP95", value: "94" }],
    };
    expect(bottleneckKey(bottleneck)).toBe("cpu-sustained");
  });
});
