import { useEffect, useId, useRef, useState } from "react";

import { cn } from "@/lib/utils";

export interface ScoreRingProps {
  value: number;
  label: string;
  size?: number;
  className?: string;
}

function toneColor(value: number): { from: string; to: string; glow: string } {
  if (value >= 75) {
    return { from: "#d3b483", to: "#f3e2c3", glow: "rgba(211, 180, 131, 0.7)" };
  }
  if (value >= 50) {
    return { from: "#ffc266", to: "#ffe0a8", glow: "rgba(255, 194, 102, 0.65)" };
  }
  return { from: "#e0705f", to: "#f2b0a4", glow: "rgba(224, 112, 95, 0.65)" };
}

/** Counts from the previous value to the next one for a live feel. */
function useCountUp(target: number): number {
  const [display, setDisplay] = useState(0);
  const fromRef = useRef(0);

  useEffect(() => {
    const reduce =
      typeof window !== "undefined" &&
      window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

    if (reduce) {
      setDisplay(target);
      fromRef.current = target;
      return;
    }

    const from = fromRef.current;
    const startedAt = performance.now();
    const duration = 950;
    let frame = 0;

    const tick = (now: number) => {
      const progress = Math.min(1, (now - startedAt) / duration);
      const eased = 1 - (1 - progress) ** 3;
      setDisplay(Math.round(from + (target - from) * eased));
      if (progress < 1) {
        frame = requestAnimationFrame(tick);
      } else {
        fromRef.current = target;
      }
    };

    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  }, [target]);

  return display;
}

/** Large glass score ring with a gradient stroke and soft glow. */
export function ScoreRing({ value, label, size = 208, className }: ScoreRingProps) {
  const clamped = Math.max(0, Math.min(100, value));
  const display = useCountUp(clamped);
  const color = toneColor(clamped);

  const rawId = useId();
  const gradientId = `score-${rawId.replace(/[^a-zA-Z0-9_-]/g, "")}`;

  const stroke = Math.max(9, size * 0.052);
  const radius = size / 2 - stroke - 6;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference * (1 - clamped / 100);

  return (
    <div
      className={cn("score-ring", className)}
      style={{ width: size, height: size }}
      role="img"
      aria-label={`${label}: ${clamped}`}
    >
      <svg viewBox={`0 0 ${size} ${size}`} width={size} height={size} aria-hidden="true">
        <defs>
          <linearGradient id={gradientId} x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stopColor={color.from} />
            <stop offset="100%" stopColor={color.to} />
          </linearGradient>
        </defs>
        <circle
          className="ticks"
          cx={size / 2}
          cy={size / 2}
          r={radius + stroke / 2 + 3}
        />
        <circle className="track" cx={size / 2} cy={size / 2} r={radius} strokeWidth={stroke} />
        <circle
          className="prog"
          cx={size / 2}
          cy={size / 2}
          r={radius}
          stroke={`url(#${gradientId})`}
          strokeWidth={stroke}
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          style={{ filter: `drop-shadow(0 0 10px ${color.glow})` }}
        />
      </svg>
      <div className="score-center">
        <div className="score-value">
          {display}
          <span>%</span>
        </div>
        <div className="score-label">{label}</div>
      </div>
    </div>
  );
}
