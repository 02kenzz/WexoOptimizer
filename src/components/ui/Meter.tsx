import { cn } from "@/lib/utils";

type Tone = "accent" | "ok" | "warn" | "danger";

export interface MeterProps {
  value: number;
  tone?: Tone;
  label?: string;
  height?: number;
  showValue?: boolean;
  animated?: boolean;
  className?: string;
}

/** Slim glass meter with a glowing gradient fill. */
export function Meter({
  value,
  tone = "accent",
  label,
  height = 7,
  showValue = false,
  animated = true,
  className,
}: MeterProps) {
  const clamped = Math.max(0, Math.min(100, value));

  return (
    <div className={cn("meter-wrap", className)}>
      {label ? (
        <div className="meter-label">
          <span>{label}</span>
          {showValue ? <b>{Math.round(clamped)}%</b> : null}
        </div>
      ) : null}
      <div
        className={cn("meter", tone)}
        role="progressbar"
        aria-label={label}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(clamped)}
        style={{ height }}
      >
        <i
          style={{
            width: `${clamped}%`,
            transition: animated ? undefined : "none",
          }}
        />
      </div>
    </div>
  );
}
