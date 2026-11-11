import { Activity, Cpu, MemoryStick, Wifi } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import i18n from "@/i18n";
import { defaultSettings, ipc } from "@/lib/ipc";
import { clamp, formatBytes, formatRate } from "@/lib/utils";
import type { OverlaySettings, OverlayStats } from "@/types";

/** Refresh cadence. Fast enough to feel live, cheap enough to ignore. */
const POLL_MS = 500;
/** Preferences change rarely; a slow poll keeps the panel in sync. */
const SETTINGS_POLL_MS = 1500;
/** Consecutive failures before the header admits it lost the backend. */
const FAILURES_BEFORE_STALE = 3;
/** Samples kept for the frame-rate sparkline. */
const HISTORY_POINTS = 48;

type Level = "none" | "ok" | "warn" | "danger";

/** Rounds a 0-100 metric, or renders an em dash when the machine has no value. */
function percent(value: number | null): string {
  if (value === null || !Number.isFinite(value)) return "\u2014";
  return `${Math.round(clamp(value, 0, 100))}%`;
}

/** Frame rate as a whole number; a paused or unavailable meter is not `0`. */
function frames(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value) || value <= 0)
    return "\u2014";
  return Math.round(value).toString();
}

/** Bar tint: calm until the component is actually busy. */
function level(value: number | null): Level {
  if (value === null || !Number.isFinite(value)) return "none";
  if (value >= 90) return "danger";
  if (value >= 70) return "warn";
  return "ok";
}

function barWidth(value: number | null): number {
  if (value === null || !Number.isFinite(value)) return 0;
  return clamp(value, 0, 100);
}

/** Frame-rate trend from the last samples; hidden until there is a trend. */
function Sparkline({ values }: { values: number[] }) {
  if (values.length < 2) return null;
  const peak = Math.max(...values, 1);
  const step = 78 / (values.length - 1);
  const points = values
    .map((value, index) => `${(index * step).toFixed(1)},${(22 - (value / peak) * 20).toFixed(1)}`)
    .join(" ");
  return (
    <svg
      className="ov-spark"
      viewBox="0 0 78 24"
      preserveAspectRatio="none"
      aria-hidden
      focusable="false"
    >
      <polygon className="ov-spark-area" points={`0,24 ${points} 78,24`} />
      <polyline className="ov-spark-line" points={points} vectorEffect="non-scaling-stroke" />
    </svg>
  );
}

function Metric({
  icon: Icon,
  name,
  value,
  detail,
}: {
  icon: typeof Cpu;
  name: string;
  value: number | null;
  detail?: string;
}) {
  return (
    <div className="ov-metric" data-level={level(value)}>
      <div className="ov-metric-head">
        <Icon size={12} strokeWidth={2} aria-hidden />
        <span className="ov-name">{name}</span>
        <span className="ov-figure">
          <b>{percent(value)}</b>
        </span>
      </div>
      <span className="ov-bar">
        <i style={{ width: `${barWidth(value)}%` }} />
      </span>
      {detail ? <span className="ov-caption">{detail}</span> : null}
    </div>
  );
}

function Network({ rx, tx }: { rx: number | null; tx: number | null }) {
  return (
    <div className="ov-metric ov-metric-net">
      <div className="ov-metric-head">
        <Wifi size={12} strokeWidth={2} aria-hidden />
        <span className="ov-name">NET</span>
        <span className="ov-net">
          <span>{`\u2193 ${rx === null ? "\u2014" : formatRate(rx)}`}</span>
          <span>{`\u2191 ${tx === null ? "\u2014" : formatRate(tx)}`}</span>
        </span>
      </div>
    </div>
  );
}

/**
 * Body of the floating overlay window.
 *
 * A compact glass card anchored to the corner picked in Settings: frame rate
 * and its 1% low on top with the trend, then one block per component. Which
 * rows exist, the corner and the opacity come from the saved preferences. A
 * metric the machine cannot provide is rendered as `—`, never as an invented
 * zero.
 */
export function Overlay() {
  const { t } = useTranslation();
  const [stats, setStats] = useState<OverlayStats | null>(null);
  const [stale, setStale] = useState(false);
  const [settings, setSettings] = useState<OverlaySettings>(defaultSettings.overlay);
  const [fpsHistory, setFpsHistory] = useState<number[]>([]);

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
        const fps = next.fps;
        if (fps !== null && fps > 0) {
          setFpsHistory((previous) => [...previous, fps].slice(-HISTORY_POINTS));
        }
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

  // The overlay window has its own store instance, so preferences are polled
  // instead of shared: a change in Settings reaches the panel in ~1.5 s.
  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      try {
        const current = await ipc.getSettings();
        if (cancelled) return;
        setSettings(current.overlay);
        if (i18n.language !== current.language) {
          void i18n.changeLanguage(current.language);
        }
      } catch {
        // Keep the defaults: measurements still render without preferences.
      }
    };

    void load();
    const timer = window.setInterval(() => void load(), SETTINGS_POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  const memoryDetail =
    stats && stats.memoryTotalBytes > 0
      ? `${formatBytes(stats.memoryUsedBytes)} / ${formatBytes(stats.memoryTotalBytes)}`
      : undefined;

  const fpsLabel = frames(stats?.fps);
  const lowLabel = frames(stats?.fpsLow1Percent);
  const hasRows = settings.showCpu || settings.showGpu || settings.showRam || settings.showNetwork;

  return (
    <div className="ov-stage" data-corner={settings.position}>
      <div
        className="ov-card"
        data-orientation={settings.orientation}
        style={{ opacity: settings.opacity }}
      >
        <div className="ov-head">
          <span className="ov-mark">Wexo</span>
          <span className="ov-state" data-stale={stale ? "true" : "false"} role="status">
            {stale ? t("overlay.stale") : t("overlay.live")}
          </span>
        </div>

        <div className="ov-body">
          {settings.showFps ? (
            <div className="ov-fps">
              <div className="ov-hero">
                <span
                  className="ov-hero-value"
                  data-empty={fpsLabel === "\u2014" ? "true" : "false"}
                >
                  {fpsLabel}
                </span>
                <span className="ov-hero-unit">FPS</span>
                <span className="ov-low">
                  <span>1% low</span>
                  <b>{lowLabel}</b>
                </span>
              </div>
              <Sparkline values={fpsHistory} />
            </div>
          ) : null}

          {hasRows ? (
            <div className="ov-metrics">
              {settings.showCpu ? (
                <Metric icon={Cpu} name="CPU" value={stats?.cpuPercent ?? null} />
              ) : null}
              {settings.showGpu ? (
                <Metric icon={Activity} name="GPU" value={stats?.gpuPercent ?? null} />
              ) : null}
              {settings.showRam ? (
                <Metric
                  icon={MemoryStick}
                  name="RAM"
                  value={stats?.memoryPercent ?? null}
                  detail={memoryDetail}
                />
              ) : null}
              {settings.showNetwork ? (
                <Network
                  rx={stats ? stats.networkRxBytesPerSec : null}
                  tx={stats ? stats.networkTxBytesPerSec : null}
                />
              ) : null}
            </div>
          ) : null}
        </div>

        {!settings.showFps && !hasRows ? <p className="ov-empty">{t("overlay.empty")}</p> : null}
      </div>
    </div>
  );
}
