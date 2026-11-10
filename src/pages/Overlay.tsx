import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { ipc } from "@/lib/ipc";
import type { OverlayStats } from "@/types";

/** Refresh cadence. Fast enough to feel live, cheap enough to ignore. */
const POLL_MS = 500;
/** Consecutive failures before the bar admits it lost the backend. */
const FAILURES_BEFORE_STALE = 3;

/** Rounds a 0-100 metric, or renders an em dash when the machine has no value. */
function percent(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value)) return "—";
  return `${Math.round(Math.min(100, Math.max(0, value)))}%`;
}

/** Frame rate as a whole number; a paused or unavailable meter is not `0`. */
function frames(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value) || value <= 0) return "—";
  return Math.round(value).toString();
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="overlay-metric">
      <span className="overlay-label">{label}</span>
      <span className="overlay-value">{value}</span>
    </div>
  );
}

/**
 * Body of the floating overlay window.
 *
 * A single monochrome bar: every metric is measured, the labels stay quiet and
 * the values carry the weight. A metric the machine cannot provide is rendered
 * as `—`, never as an invented zero.
 */
export function Overlay() {
  const { t } = useTranslation();
  const [stats, setStats] = useState<OverlayStats | null>(null);
  const [stale, setStale] = useState(false);

  useEffect(() => {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
    document.body.style.overflow = "hidden";
    document.body.style.userSelect = "none";

    return () => {
      document.documentElement.style.background = "";
      document.body.style.background = "";
      document.body.style.overflow = "";
      document.body.style.userSelect = "";
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    let failures = 0;

    const tick = async () => {
      try {
        const next = await ipc.getOverlayStats();
        if (cancelled) return;
        failures = 0;
        setStats(next);
        setStale(false);
      } catch {
        if (cancelled) return;
        failures += 1;
        if (failures >= FAILURES_BEFORE_STALE) setStale(true);
      }
    };

    void tick();
    const timer = window.setInterval(() => void tick(), POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  const metrics = [
    { label: "FPS", value: frames(stats?.fps) },
    { label: "GPU", value: percent(stats?.gpuPercent) },
    { label: "CPU", value: percent(stats?.cpuPercent) },
    { label: "RAM", value: percent(stats?.memoryPercent) },
  ];

  return (
    <div className="overlay-stage">
      <div className="overlay-bar" data-stale={stale ? "true" : "false"}>
        {metrics.map((metric) => (
          <Metric key={metric.label} label={metric.label} value={metric.value} />
        ))}
        <span className="overlay-status" role="status">
          {stale ? t("overlay.stale") : t("overlay.live")}
        </span>
      </div>
    </div>
  );
}
