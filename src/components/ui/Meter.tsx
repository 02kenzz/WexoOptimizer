import * as ProgressPrimitive from "@radix-ui/react-progress";

import { cn } from "@/lib/utils";

type Tone = "accent" | "ok" | "warn" | "danger";

const TONES: Record<Tone, string> = {
  accent: "bg-accent",
  ok: "bg-ok",
  warn: "bg-warn",
  danger: "bg-danger",
};

export interface MeterProps {
  value: number;
  tone?: Tone;
  label?: string;
  height?: number;
  showValue?: boolean;
  animated?: boolean;
  className?: string;
}

/** Thin progress meter with glass styling and smooth animation. */
export function Meter({
  value,
  tone = "accent",
  label,
  height = 6,
  showValue = false,
  animated = true,
  className,
}: MeterProps) {
  const clamped = Math.max(0, Math.min(100, value));

  return (
    <div className={cn("w-full", className)}>
      {label && (
        <div className="flex items-center justify-between mb-1.5 text-[10px] font-medium uppercase tracking-[0.1em] text-mut">
          <span>{label}</span>
          {showValue && <span className="tabular-nums text-text">{clamped.toFixed(0)}%</span>}
        </div>
      )}
      <ProgressPrimitive.Root
        value={clamped}
        aria-label={label}
        className={cn(
          "relative w-full overflow-hidden rounded-full bg-line",
          animated ? "transition-all duration-700" : "",
        )}
        style={{ height }}
      >
        <ProgressPrimitive.Indicator
          className={cn(
            "h-full rounded-full transition-[width] duration-1000 cubic-bezier(0.22, 1, 0.36, 1)",
            TONES[tone],
          )}
          style={{ width: `${clamped}%` }}
        />
        {clamped > 0 && (
          <div
            className="absolute inset-0 bg-gradient-to-r from-transparent via-white/10 to-transparent"
            style={{
              maskImage: `linear-gradient(90deg, black ${clamped}%, transparent ${clamped}%)`,
              WebkitMaskImage: `linear-gradient(90deg, black ${clamped}%, transparent ${clamped}%)`,
            }}
            aria-hidden="true"
          />
        )}
      </ProgressPrimitive.Root>
    </div>
  );
}