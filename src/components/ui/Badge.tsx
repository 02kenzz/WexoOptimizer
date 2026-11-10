import type { HTMLAttributes } from "react";

import { cn } from "@/lib/utils";

type Tone = "neutral" | "accent" | "ok" | "warn" | "danger" | "muted";

const TONES: Record<Tone, string> = {
  neutral: "",
  accent: "chip-accent",
  ok: "chip-ok",
  warn: "chip-warn",
  danger: "chip-danger",
  muted: "chip-muted",
};

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  tone?: Tone;
  dot?: boolean;
}

export function Badge({ className, tone = "neutral", dot = false, children, ...props }: BadgeProps) {
  return (
    <span className={cn("chip", TONES[tone], className)} {...props}>
      {dot ? <span className="dot" aria-hidden /> : null}
      {children}
    </span>
  );
}
