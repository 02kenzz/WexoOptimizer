import { Activity, Cpu, Gauge, HardDrive, MemoryStick, Network, RefreshCw, Timer } from "lucide-react";
import { useTranslation } from "react-i18next";

import { LiveChart } from "@/components/LiveChart";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";
import { Meter } from "@/components/ui/Meter";
import { useLiveMetrics } from "@/hooks/useLiveMetrics";
import { usageTone } from "@/lib/tone";
import {
  clamp,
  cn,
  formatBytes,
  formatDuration,
  formatMhz,
  formatPercent,
  formatRate,
} from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";

const CORE_TONE: Record<"accent" | "warn" | "danger", string> = {
  accent: "bg-accent",
  warn: "bg-warn",
  danger: "bg-danger",
};

function LiveTile({
  icon: Icon,
  label,
  value,
  percent,
  hint,
}: {
  icon: typeof Cpu;
  label: string;
  value: string;
  percent?: number;
  hint?: string;
}) {
  return (
    <Card>
      <div className="flex items-center justify-between text-[11px] uppercase tracking-wider text-mut">
        <span className="flex items-center gap-1.5">
          <Icon className="size-3.5" aria-hidden />
          {label}
        </span>
        <span className="size-1.5 rounded-full bg-ok animate-pulse-glow" aria-hidden />
      </div>
      <p className="mt-2 text-2xl font-semibold tabular-nums tracking-tight text-text">{value}</p>
      {hint ? <p className="mt-0.5 truncate text-[11px] text-mut">{hint}</p> : null}
      {percent !== undefined ? (
        <Meter value={percent} tone={usageTone(percent)} className="mt-3" />
      ) : null}
    </Card>
  );
}

function CoreGrid({ values }: { values: number[] }) {
  const { t } = useTranslation();

  if (values.length === 0) {
    return <p className="py-6 text-center text-xs text-mut">{t("monitor.noCores")}</p>;
  }

  return (
    <div className="grid grid-cols-8 gap-1.5 sm:grid-cols-12 lg:grid-cols-16">
      {values.map((value, index) => {
        const percent = clamp(value, 0, 100);
        return (
          <div
            key={index.toString()}
            className="flex h-14 items-end overflow-hidden rounded-sm bg-card"
            title={`${t("monitor.core")} ${index + 1}: ${formatPercent(percent, 0)}`}
          >
            <div
              className={cn("w-full rounded-sm transition-[height]", CORE_TONE[usageTone(percent)])}
              style={{ height: `${percent}%` }}
            />
          </div>
        );
      })}
    </div>
  );
}

function LegendDot({ color, label }: { color: string; label: string }) {
  return (
    <span className="flex items-center gap-1.5">
      <span className="size-1.5 rounded-full" style={{ backgroundColor: color }} aria-hidden />
      {label}
    </span>
  );
}

export function Monitor() {
  const { t } = useTranslation();
  const interval = useAppStore((state) => state.settings.monitorIntervalMs);
  const { samples, latest, loading, error } = useLiveMetrics();

  if (loading && !latest) {
    return <Card className="h-72 animate-pulse" />;
  }

  if (error && !latest) {
    return (
      <EmptyState
        icon={Activity}
        title={t("common.error")}
        description={t("common.backendUnavailable")}
        action={
          <Button size="sm" onClick={() => window.location.reload()}>
            <RefreshCw className="size-3.5" aria-hidden />
            {t("common.retry")}
          </Button>
        }
      />
    );
  }

  const diskBusy = latest?.diskBusyPercent ?? null;
  const memoryPercent = latest?.memoryPercent ?? 0;

  return (
    <section className="view" id="monitor">
      <div className="hud">
        <div className="stat">
          {t("monitor.systemStatus")}<i />
        </div>
        <div className="duo">
          <div className="unit cpu">
            <div>
              <h3>{t("monitor.cpuCore")}</h3>
              <small>{t("monitor.tempAndUsage")}</small>
            </div>
            <div className="big">
              <svg viewBox="0 0 100 100">
                <circle className="t" cx="50" cy="50" r="46" pathLength="100" strokeDasharray="75 25" />
                <circle className="p" data-arc="cpu" cx="50" cy="50" r="46" pathLength="100" strokeDasharray={`${(latest?.cpuPercent ?? 0) * 0.75} 100`} />
              </svg>
              <div className="orb" data-k="tc">—</div>
            </div>
            <div>Uso <span data-k="cpu" data-u=" %">—</span></div>
            <div className="mini">
              <svg viewBox="0 0 100 100">
                <circle className="t" cx="50" cy="50" r="42" pathLength="100" strokeDasharray="75 25" />
                <circle className="p" data-arc="mem" cx="50" cy="50" r="42" pathLength="100" strokeDasharray={`${(latest?.memoryPercent ?? 0) * 0.75} 100`} />
              </svg>
              <div data-k="mem" data-u="%">—</div>
            </div>
            <b>RAM</b>
          </div>
          <div className="wire" />
          <div className="unit gpu">
            <div>
              <h3>{t("monitor.gpuCore")}</h3>
              <small>{t("monitor.tempAndUsage")}</small>
            </div>
            <div className="big">
              <svg viewBox="0 0 100 100">
                <circle className="t" cx="50" cy="50" r="46" pathLength="100" strokeDasharray="75 25" />
                <circle className="p" data-arc="gpu" cx="50" cy="50" r="46" pathLength="100" strokeDasharray={`${(latest?.gpuPercent ?? 0) * 0.75} 100`} />
              </svg>
              <div className="orb" data-k="tg">—</div>
            </div>
            <div>Uso <span data-k="gpu" data-u=" %">—</span></div>
            <div className="mini">
              <svg viewBox="0 0 100 100">
                <circle className="t" cx="50" cy="50" r="42" pathLength="100" strokeDasharray="75 25" />
                <circle className="p" data-arc="vram" cx="50" cy="50" r="42" pathLength="100" strokeDasharray={`${(latest?.vramPercent ?? 0) * 0.75} 100`} />
              </svg>
              <div data-k="vram" data-u="%">—</div>
            </div>
            <b>VRAM</b>
          </div>
        </div>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Activity className="size-4 text-accent" aria-hidden />
              {t("monitor.chartTitle")}
            </CardTitle>
            <div className="flex flex-wrap items-center gap-3 text-[10px] uppercase tracking-wider text-mut">
              <LegendDot color="var(--accent)" label={t("monitor.chartCpu")} />
              <LegendDot color="var(--accent-2)" label={t("monitor.chartMemory")} />
              <LegendDot color="var(--warn)" label={t("monitor.chartDisk")} />
            </div>
          </CardHeader>
          <CardContent className="pb-4">
            <LiveChart samples={samples} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Gauge className="size-4 text-accent" aria-hidden />
              {t("monitor.cores", { value: latest?.perCorePercent.length ?? 0 })}
            </CardTitle>
            <Badge tone="muted">{t("monitor.perCore")}</Badge>
          </CardHeader>
          <CardContent className="pb-5">
            <CoreGrid values={latest?.perCorePercent ?? []} />
          </CardContent>
        </Card>
      </div>
    </section>
  );
}