import { describe, expect, it } from "vitest";

import { clamp, formatBytes, formatMhz, formatPercent, scoreTone } from "./utils";

describe("formatBytes", () => {
  it("handles zero and invalid values", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(Number.NaN)).toBe("0 B");
    expect(formatBytes(-1)).toBe("0 B");
  });

  it("scales units and keeps one decimal for large units", () => {
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(1024 ** 2)).toBe("1 MB");
    expect(formatBytes(1_000_204_886_016)).toBe("931.5 GB");
  });
});

describe("formatMhz", () => {
  it("switches to GHz above 1000 MHz", () => {
    expect(formatMhz(800)).toBe("800 MHz");
    expect(formatMhz(3700)).toBe("3.70 GHz");
    expect(formatMhz(0)).toBe("—");
  });
});

describe("formatPercent and clamp", () => {
  it("rounds percentages", () => {
    expect(formatPercent(13.44, 1)).toBe("13.4%");
    expect(formatPercent(Number.NaN)).toBe("—");
  });

  it("clamps values", () => {
    expect(clamp(120, 0, 100)).toBe(100);
    expect(clamp(-4, 0, 100)).toBe(0);
  });
});

describe("scoreTone", () => {
  it("maps scores to semantic tones", () => {
    expect(scoreTone(90)).toBe("ok");
    expect(scoreTone(60)).toBe("warn");
    expect(scoreTone(20)).toBe("danger");
  });
});
