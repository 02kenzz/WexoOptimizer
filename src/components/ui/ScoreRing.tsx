import { cn } from "@/lib/utils";

export interface ScoreRingProps {
  value: number;
  label: string;
  size?: number;
  className?: string;
}

function ringColor(value: number): string {
  if (value >= 75) return "var(--ok)";
  if (value >= 50) return "var(--warn)";
  return "var(--danger)";
}

/** SVG score ring with a smooth stroke transition. */
export function ScoreRing({ value, label, size = 156, className }: ScoreRingProps) {
  const clamped = Math.max(0, Math.min(100, value));
  const radius = 54;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference * (1 - clamped / 100);

  return (
    <div
      className={cn("relative inline-flex items-center justify-center", className)}
      style={{ width: size, height: size }}
      role="img"
      aria-label={`${label}: ${clamped}`}
    >
      <svg viewBox="0 0 140 140" className="size-full -rotate-90" aria-hidden="true">
        <circle cx="70" cy="70" r={radius} fill="none" stroke="var(--border)" strokeWidth="9" />
        <circle
          cx="70"
          cy="70"
          r={radius}
          fill="none"
          stroke={ringColor(clamped)}
          strokeWidth="9"
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          style={{ transition: "stroke-dashoffset 700ms cubic-bezier(0.22, 1, 0.36, 1)" }}
        />
      </svg>
      <div className="absolute flex flex-col items-center">
        <span className="text-4xl font-semibold tabular-nums tracking-tight">{clamped}</span>
        <span className="mt-0.5 text-[10px] font-medium uppercase tracking-[0.16em] text-muted">
          {label}
        </span>
      </div>
    </div>
  );
}
