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
  className?: string;
}

/** Thin progress meter used for subsystem usage and scores. */
export function Meter({ value, tone = "accent", label, className }: MeterProps) {
  const clamped = Math.max(0, Math.min(100, value));
  return (
    <ProgressPrimitive.Root
      value={clamped}
      aria-label={label}
      className={cn("relative h-1.5 w-full overflow-hidden rounded-full bg-border/70", className)}
    >
      <ProgressPrimitive.Indicator
        className={cn("h-full rounded-full transition-[width] duration-500", TONES[tone])}
        style={{ width: `${clamped}%` }}
      />
    </ProgressPrimitive.Root>
  );
}
