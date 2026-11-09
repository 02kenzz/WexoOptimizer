"use client";

import { cn } from "@/lib/utils";

type Tone = "accent" | "accent-2" | "accent-3" | "ok" | "warn" | "danger";

const TONE_COLORS: Record<Tone, string> = {
  accent: "var(--accent)",
  "accent-2": "var(--accent-2)",
  "accent-3": "var(--accent-3)",
  ok: "var(--ok)",
  warn: "var(--warn)",
  danger: "var(--danger)",
};

interface GaugeProps {
  value: number;
  label: string;
  unit?: string;
  size?: number;
  strokeWidth?: number;
  tone?: Tone;
  showValue?: boolean;
  animated?: boolean;
  className?: string;
}

function gaugeColor(value: number, tone: Tone): string {
  if (tone !== "accent" && tone !== "accent-2" && tone !== "accent-3") {
    return TONE_COLORS[tone];
  }
  if (value >= 75) return "var(--danger)";
  if (value >= 50) return "var(--warn)";
  return TONE_COLORS[tone];
}

export function Gauge({
  value,
  label,
  unit = "%",
  size = 160,
  strokeWidth = 10,
  tone = "accent",
  showValue = true,
  animated = true,
  className,
}: GaugeProps) {
  const clamped = Math.max(0, Math.min(100, value));
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference * (1 - clamped / 100);
  const color = gaugeColor(clamped, tone);

  const ringStyle: React.CSSProperties = {
    "--circumference": `${circumference}px`,
    "--target-offset": `${offset}px`,
    transition: animated ? "stroke-dashoffset 1.2s cubic-bezier(0.22, 1, 0.36, 1)" : "none",
  };

  return (
    <div
      className={cn("relative inline-flex flex-col items-center", className)}
      style={{ width: size, height: size + 48 }}
      role="img"
      aria-label={`${label}: ${clamped}${unit}`}
    >
      <div className="relative" style={{ width: size, height: size }}>
        <svg viewBox={`0 0 ${size} ${size}`} className="size-full -rotate-90" aria-hidden="true">
          <circle
            cx={size / 2}
            cy={size / 2}
            r={radius}
            fill="none"
            stroke="var(--line)"
            strokeWidth={strokeWidth}
          />
          <circle
            cx={size / 2}
            cy={size / 2}
            r={radius}
            fill="none"
            stroke={color}
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            strokeDasharray={circumference}
            strokeDashoffset={animated ? circumference : offset}
            style={ringStyle}
            className={animated ? "animate-ring-draw" : ""}
          />
          {clamped > 95 && (
            <circle
              cx={size / 2}
              cy={size / 2}
              r={radius + 2}
              fill="none"
              stroke={color}
              strokeWidth={2}
              strokeDasharray="4 8"
              opacity={0.6}
              className="animate-spin"
            />
          )}
        </svg>

        {showValue && (
          <div className="absolute inset-0 flex flex-col items-center justify-center">
            <span className="text-3xl font-bold tabular-nums tracking-tight text-text">
              {clamped.toFixed(clamped < 10 ? 1 : 0)}
            </span>
            <span className="text-[10px] font-medium uppercase tracking-[0.16em] text-text-dim">
              {unit}
            </span>
          </div>
        )}
      </div>

      <div className="mt-3 text-center">
        <p className="text-xs font-semibold uppercase tracking-[0.14em] text-text-dim">{label}</p>
      </div>
    </div>
  );
}

interface GaugeClusterProps {
  gauges: Array<{
    value: number;
    label: string;
    unit?: string;
    tone?: Tone;
    key: string;
  }>;
  size?: number;
  gap?: number;
  className?: string;
}

export function GaugeCluster({
  gauges,
  size = 140,
  gap = 24,
  className,
}: GaugeClusterProps) {
  return (
    <div
      className={cn("flex items-center justify-center gap-4 flex-wrap", className)}
      style={{ gap, "--gauge-size": `${size}px` }}
      role="list"
      aria-label="System metrics"
    >
      {gauges.map((gauge) => (
        <Gauge
          key={gauge.key}
          value={gauge.value}
          label={gauge.label}
          unit={gauge.unit}
          size={size}
          tone={gauge.tone}
          strokeWidth={8}
        />
      ))}
    </div>
  );
}