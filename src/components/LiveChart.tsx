import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import uPlot from "uplot";

import "uplot/dist/uPlot.min.css";

import { useAppStore } from "@/stores/appStore";
import type { LiveMetrics } from "@/types";

const CHART_HEIGHT = 250;

function cssVar(name: string, fallback: string): string {
  if (typeof document === "undefined") return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function buildData(samples: LiveMetrics[]): uPlot.AlignedData {
  const timestamps: number[] = [];
  const cpu: (number | null)[] = [];
  const memory: (number | null)[] = [];
  const disk: (number | null)[] = [];

  for (const sample of samples) {
    timestamps.push(sample.timestampMs / 1000);
    cpu.push(sample.cpuPercent);
    memory.push(sample.memoryPercent);
    disk.push(sample.diskBusyPercent);
  }

  return [timestamps, cpu, memory, disk];
}

/**
 * uPlot chart for CPU, memory and disk busy time. Percentages share a fixed
 * 0-100 scale so the shape of the data is comparable at a glance.
 */
export function LiveChart({ samples }: { samples: LiveMetrics[] }) {
  const { t } = useTranslation();
  const theme = useAppStore((state) => state.settings.theme);

  const containerRef = useRef<HTMLDivElement>(null);
  const plotRef = useRef<uPlot | null>(null);
  const samplesRef = useRef(samples);
  samplesRef.current = samples;

  // Recreate the plot when the theme or language changes: axis colours and
  // series labels are baked into the options.
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    // Fallbacks only apply before the theme stylesheet is ready; `theme` is a
    // dependency so the plot is rebuilt when the palette changes.
    const text = cssVar("--dim", "#7b756c");
    const grid = cssVar("--line", "rgba(255, 255, 255, 0.075)");
    const accent = cssVar("--accent", "#d3b483");
    const blue = cssVar("--accent-2", "#c08552");
    const warn = cssVar("--warn", "#ffc266");

    const options: uPlot.Options = {
      width: Math.max(container.clientWidth, 320),
      height: CHART_HEIGHT,
      padding: [14, 14, 0, 0],
      legend: { show: false },
      cursor: {
        show: true,
        x: true,
        y: false,
        points: { show: false },
        drag: { x: false, y: false, setScale: false },
      },
      scales: {
        x: { time: true },
        y: { auto: false, range: [0, 100] },
      },
      axes: [
        {
          stroke: text,
          font: "10px Inter, system-ui, sans-serif",
          grid: { stroke: grid, width: 1 },
          ticks: { stroke: grid, width: 1, size: 4 },
        },
        {
          stroke: text,
          font: "10px Inter, system-ui, sans-serif",
          size: 40,
          grid: { stroke: grid, width: 1 },
          ticks: { stroke: grid, width: 1, size: 4 },
          values: (_self, splits) => splits.map((value) => `${value}%`),
        },
      ],
      series: [
        { label: t("monitor.chartTime") },
        {
          label: t("monitor.chartCpu"),
          stroke: accent,
          width: 2,
          points: { show: false },
          fill: "rgba(211, 180, 131, 0.10)",
        },
        {
          label: t("monitor.chartMemory"),
          stroke: blue,
          width: 1.6,
          points: { show: false },
          fill: "rgba(176, 112, 63, 0.06)",
        },
        { label: t("monitor.chartDisk"), stroke: warn, width: 1.4, points: { show: false } },
      ],
    };

    const plot = new uPlot(options, buildData(samplesRef.current), container);
    plotRef.current = plot;

    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width;
      if (width && width > 0) {
        plot.setSize({ width, height: CHART_HEIGHT });
      }
    });
    observer.observe(container);

    return () => {
      observer.disconnect();
      plot.destroy();
      plotRef.current = null;
    };
  }, [theme, t]);

  useEffect(() => {
    plotRef.current?.setData(buildData(samples));
  }, [samples]);

  return <div ref={containerRef} style={{ width: "100%" }} />;
}
