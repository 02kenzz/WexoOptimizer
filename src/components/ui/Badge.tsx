import type { HTMLAttributes } from "react";

import { cn } from "@/lib/utils";

type Tone = "neutral" | "accent" | "ok" | "warn" | "danger" | "muted";

const TONES: Record<Tone, string> = {
  neutral: "border-line bg-card text-text",
  accent: "border-accent bg-accent/10 text-accent",
  ok: "border-ok bg-ok/10 text-ok",
  warn: "border-warn bg-warn/10 text-warn",
  danger: "border-danger bg-danger/10 text-danger",
  muted: "border-line bg-card text-mut",
};

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  tone?: Tone;
  dot?: boolean;
}

export function Badge({
  className,
  tone = "neutral",
  dot = false,
  children,
  ...props
}: BadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-medium whitespace-nowrap",
        TONES[tone],
        className,
      )}
      {...props}
    >
      {dot ? <span className="size-1.5 rounded-full bg-current" aria-hidden /> : null}
      {children}
    </span>
  );
}