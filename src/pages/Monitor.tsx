import {
  Activity,
  Cpu,
  Gauge,
  HardDrive,
  MemoryStick,
  Network,
  RefreshCw,
  Timer,
} from "lucide-react";
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
    <Card className="p-4">
      <div className="flex items-center justify-between text-[11px] uppercase tracking-wider text-muted">
        <span className="flex items-center gap-1.5">
          <Icon className="size-3.5" aria-hidden />
          {label}
        </span>
        <span className="size-1.5 rounded-full bg-ok animate-pulse-soft" aria-hidden />
      </div>
      <p className="mt-2 text-2xl font-semibold tabular-nums tracking-tight">{value}</p>
      {hint ? <p className="mt-0.5 truncate text-[11px] text-muted">{hint}</p> : null}
      {percent !== undefined ? (
        <Meter value={percent} tone={usageTone(percent)} className="mt-3" />
      ) : null}
    </Card>
  );
}

function CoreGrid({ values }: { values: number[] }) {
  const { t } = useTranslation();

  if (values.length === 0) {
    return <p className="py-6 text-center text-xs text-muted">{t("monitor.noCores")}</p>;
  }

  return (
    <div className="grid grid-cols-8 gap-1.5 sm:grid-cols-12 lg:grid-cols-16">
      {values.map((value, index) => {
        const percent = clamp(value, 0, 100);
        return (
          <div
            key={index.toString()}
            className="flex h-14 items-end overflow-hidden rounded-sm bg-panel-2/70"
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

export function Monitor() {
  const { t } = useTranslation();
  const interval = useAppStore((state) => state.settings.monitorIntervalMs);
  const { samples, latest, loading, error } = useLiveMetrics();

  if (loading && !latest) {
    return <Card className="h-72 animate-pulse bg-panel-2/40" />;
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
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border/70 bg-panel/60 px-4 py-3">
        <div className="flex items-center gap-2 text-xs text-muted">
          <span className="size-1.5 rounded-full bg-ok animate-pulse-soft" aria-hidden />
          {latest
            ? t("monitor.liveSince", { value: formatDuration(latest.uptimeSeconds) })
            : t("monitor.waiting")}
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Badge tone="muted">{t("monitor.sampleCount", { value: samples.length })}</Badge>
          <Badge tone="accent">
            <Timer className="size-3" aria-hidden />
            {interval} ms
          </Badge>
        </div>
      </div>

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <LiveTile
          icon={Cpu}
          label={t("monitor.liveCpu")}
          value={formatPercent(latest?.cpuPercent ?? 0, 1)}
          percent={latest?.cpuPercent ?? 0}
          hint={latest ? formatMhz(latest.cpuFrequencyMhz) : undefined}
        />
        <LiveTile
          icon={MemoryStick}
          label={t("monitor.liveMemory")}
          value={formatPercent(memoryPercent, 0)}
          percent={memoryPercent}
          hint={
            latest
              ? t("dashboard.usedOf", {
                  used: formatBytes(latest.memoryUsedBytes),
                  total: formatBytes(latest.memoryTotalBytes),
                })
              : undefined
          }
        />
        <LiveTile
          icon={HardDrive}
          label={t("monitor.liveDisk")}
          value={diskBusy === null ? t("common.unknown") : formatPercent(diskBusy, 1)}
          percent={diskBusy ?? undefined}
          hint={
            latest
              ? t("monitor.diskThroughput", {
                  read: formatRate(latest.diskReadBytesPerSec),
                  write: formatRate(latest.diskWriteBytesPerSec),
                })
              : undefined
          }
        />
        <LiveTile
          icon={Network}
          label={t("monitor.liveNetwork")}
          value={formatRate(latest?.networkRxBytesPerSec ?? 0)}
          hint={t("monitor.upload", { value: formatRate(latest?.networkTxBytesPerSec ?? 0) })}
        />
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="size-4 text-accent" aria-hidden />
            {t("monitor.chartTitle")}
          </CardTitle>
          <div className="flex flex-wrap items-center gap-3 text-[10px] uppercase tracking-wider text-muted">
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
