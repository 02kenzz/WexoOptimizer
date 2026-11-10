"use client";

import { useId } from "react";

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
  if (value >= 90) return "var(--danger)";
  if (value >= 75) return "var(--warn)";
  return TONE_COLORS[tone];
}

export function Gauge({
  value,
  label,
  unit = "%",
  size = 170,
  strokeWidth = 10,
  tone = "accent",
  showValue = true,
  animated = true,
  className,
}: GaugeProps) {
  const clamped = Math.max(0, Math.min(100, value));
  const radius = (size - strokeWidth) / 2 - 4;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference * (1 - clamped / 100);
  const color = gaugeColor(clamped, tone);

  const rawId = useId();
  const gradientId = `gauge-${rawId.replace(/[^a-zA-Z0-9_-]/g, "")}`;

  return (
    <div
      className={cn("gauge", className)}
      style={{ width: size }}
      role="img"
      aria-label={`${label}: ${clamped}${unit}`}
    >
      <div className="gauge-ring" style={{ width: size, height: size }}>
        <svg viewBox={`0 0 ${size} ${size}`} width={size} height={size} className="-rotate-90">
          <defs>
            <linearGradient id={gradientId} x1="0%" y1="0%" x2="100%" y2="100%">
              <stop offset="0%" stopColor={color} stopOpacity="0.55" />
              <stop offset="100%" stopColor={color} />
            </linearGradient>
          </defs>
          <circle
            cx={size / 2}
            cy={size / 2}
            r={radius}
            fill="none"
            stroke="rgba(255,255,255,0.07)"
            strokeWidth={strokeWidth}
          />
          <circle
            cx={size / 2}
            cy={size / 2}
            r={radius}
            fill="none"
            stroke={`url(#${gradientId})`}
            strokeWidth={strokeWidth}
            strokeLinecap="round"
            strokeDasharray={circumference}
            strokeDashoffset={offset}
            style={{
              transition: animated ? "stroke-dashoffset 1s cubic-bezier(0.22, 1, 0.36, 1)" : "none",
              filter: `drop-shadow(0 0 9px ${color})`,
            }}
          />
        </svg>

        {showValue ? (
          <div className="gauge-center">
            <span
              style={{
                fontSize: size * 0.2,
                fontWeight: 650,
                letterSpacing: "-0.04em",
                fontVariantNumeric: "tabular-nums",
                lineHeight: 1,
              }}
            >
              {clamped.toFixed(clamped < 10 ? 1 : 0)}
            </span>
            <span
              style={{
                marginTop: 3,
                fontSize: 9,
                letterSpacing: "0.16em",
                textTransform: "uppercase",
                color: "var(--dim)",
              }}
            >
              {unit}
            </span>
          </div>
        ) : null}
      </div>

      <p className="gauge-label">{label}</p>
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

export function GaugeCluster({ gauges, size = 150, gap = 24, className }: GaugeClusterProps) {
  return (
    <div
      className={cn("gauges", className)}
      style={{ gap }}
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
          strokeWidth={9}
        />
      ))}
    </div>
  );
}
