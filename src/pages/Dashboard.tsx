import { Activity, ArrowRight, Cpu, HardDrive, MemoryStick, Network, Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router-dom";

import { InsightCard } from "@/components/InsightCard";
import { Badge } from "@/components/ui/Badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";
import { Meter } from "@/components/ui/Meter";
import { ScoreRing } from "@/components/ui/ScoreRing";
import { useDiagnostics } from "@/hooks/useDiagnostics";
import { useLiveMetrics } from "@/hooks/useLiveMetrics";
import { bottleneckKey } from "@/lib/bottlenecks";
import { usageTone } from "@/lib/tone";
import {
  cn,
  formatBytes,
  formatDuration,
  formatMhz,
  formatPercent,
  formatRate,
  healthLabel,
  scoreTone,
} from "@/lib/utils";
import type { DiagnosticReport, DiskInfo } from "@/types";

function systemDisk(profile: DiagnosticReport["profile"]): DiskInfo | undefined {
  return (
    profile.disks.find((disk) => disk.mountPoint.toUpperCase().startsWith("C:")) ?? profile.disks[0]
  );
}

function usedPercent(disk: DiskInfo | undefined): number {
  if (!disk || disk.totalBytes === 0) return 0;
  return 100 - (disk.availableBytes / disk.totalBytes) * 100;
}

function LiveRow({
  icon: Icon,
  label,
  value,
  percent,
  tone = "accent",
}: {
  icon: typeof Cpu;
  label: string;
  value: string;
  percent?: number;
  tone?: "accent" | "ok" | "warn" | "danger";
}) {
  return (
    <div>
      <div className="flex items-center justify-between gap-3 text-xs">
        <span className="flex items-center gap-2 text-muted">
          <Icon className="size-3.5" aria-hidden />
          {label}
        </span>
        <span className="font-medium tabular-nums text-text">{value}</span>
      </div>
      {percent !== undefined ? <Meter value={percent} tone={tone} className="mt-2" /> : null}
    </div>
  );
}

function Skeleton() {
  return (
    <div className="space-y-4">
      <div className="grid gap-4 xl:grid-cols-[1.15fr_0.85fr]">
        <Card className="h-64 animate-pulse bg-panel-2/40" />
        <Card className="h-64 animate-pulse bg-panel-2/40" />
      </div>
      <Card className="h-40 animate-pulse bg-panel-2/40" />
    </div>
  );
}

export function Dashboard() {
  const { t } = useTranslation();
  const { data: report, isLoading, isError, refetch } = useDiagnostics(5000);
  const { latest } = useLiveMetrics(120);

  if (isLoading) return <Skeleton />;

  if (isError || !report) {
    return (
      <EmptyState
        icon={Activity}
        title={t("common.error")}
        description={t("common.backendUnavailable")}
        action={
          <button
            type="button"
            onClick={() => void refetch()}
            className="text-xs font-medium text-accent hover:underline"
          >
            {t("common.retry")}
          </button>
        }
      />
    );
  }

  const { profile, health, bottlenecks } = report;
  const mainDisk = systemDisk(profile);
  const label = healthLabel(health.global);

  const cpuPercent = latest?.cpuPercent ?? profile.cpu.usagePercent;
  const memoryPercent = latest
    ? latest.memoryPercent
    : profile.memory.totalBytes
      ? (profile.memory.usedBytes / profile.memory.totalBytes) * 100
      : 0;
  const memoryUsed = latest?.memoryUsedBytes ?? profile.memory.usedBytes;
  const memoryTotal = latest?.memoryTotalBytes ?? profile.memory.totalBytes;
  const diskUsed = usedPercent(mainDisk);
  const networkName = profile.networks[0]?.name ?? t("dashboard.network");

  const subsystems = [
    { label: t("dashboard.cpu"), score: health.cpu },
    { label: t("dashboard.memory"), score: health.memory },
    { label: t("dashboard.storage"), score: health.disk },
    { label: t("dashboard.network"), score: health.network },
  ];

  return (
    <div className="space-y-4">
      <div className="grid gap-4 xl:grid-cols-[1.15fr_0.85fr]">
        <Card className="p-6">
          <div className="flex flex-wrap items-center gap-7">
            <ScoreRing value={health.global} label={t("dashboard.health")} />
            <div className="min-w-[220px] flex-1">
              <div className="flex flex-wrap items-center gap-2">
                <Badge tone={scoreTone(health.global)} dot>
                  {t(`dashboard.${label}`)}
                </Badge>
                <span className="text-[11px] text-muted">
                  {bottlenecks.length === 0
                    ? t("dashboard.allGood")
                    : t("dashboard.healthSummary", { count: bottlenecks.length })}
                </span>
              </div>

              <div className="mt-6 grid grid-cols-1 gap-4 sm:grid-cols-2">
                {subsystems.map((subsystem) => (
                  <div key={subsystem.label}>
                    <div className="flex items-baseline justify-between gap-2">
                      <span className="text-xs text-muted">{subsystem.label}</span>
                      <span className="text-xs font-semibold tabular-nums">{subsystem.score}</span>
                    </div>
                    <Meter
                      value={subsystem.score}
                      tone={scoreTone(subsystem.score)}
                      className="mt-1.5"
                      label={subsystem.label}
                    />
                  </div>
                ))}
              </div>
            </div>
          </div>
        </Card>

        <Card className="p-5">
          <div className="flex items-center justify-between">
            <CardTitle>{t("dashboard.live")}</CardTitle>
            <span className="flex items-center gap-1.5 text-[10px] uppercase tracking-[0.14em] text-muted">
              <span className="size-1.5 rounded-full bg-ok animate-pulse-soft" aria-hidden />
              live
            </span>
          </div>

          <div className="mt-5 space-y-4">
            <LiveRow
              icon={Cpu}
              label={t("dashboard.cpu")}
              value={formatPercent(cpuPercent, 1)}
              percent={cpuPercent}
              tone={usageTone(cpuPercent)}
            />
            <LiveRow
              icon={MemoryStick}
              label={t("dashboard.memory")}
              value={t("dashboard.usedOf", {
                used: formatBytes(memoryUsed),
                total: formatBytes(memoryTotal),
              })}
              percent={memoryPercent}
              tone={usageTone(memoryPercent)}
            />
            <LiveRow
              icon={HardDrive}
              label={mainDisk?.mountPoint ?? t("dashboard.storage")}
              value={t("dashboard.free", { value: formatBytes(mainDisk?.availableBytes ?? 0) })}
              percent={diskUsed}
              tone={usageTone(diskUsed)}
            />
            <LiveRow
              icon={Network}
              label={networkName}
              value={formatRate(latest?.networkRxBytesPerSec ?? 0)}
            />
          </div>

          <p className="mt-4 flex items-center gap-1.5 text-[10px] text-muted">
            <span className="size-1 rounded-full bg-accent" aria-hidden />
            {latest
              ? `${t("dashboard.frequency")}: ${formatMhz(latest.cpuFrequencyMhz)} · ${t(
                  "dashboard.uptime",
                )}: ${formatDuration(latest.uptimeSeconds)}`
              : `${t("dashboard.frequency")}: ${formatMhz(profile.cpu.currentFrequencyMhz)}`}
          </p>
        </Card>
      </div>

      <section>
        <div className="mb-3 flex items-end justify-between gap-4">
          <div>
            <h2 className="text-sm font-semibold tracking-tight">
              {t("dashboard.recommendations")}
            </h2>
            <p className="text-xs text-muted">{t("dashboard.viewDiagnostics")}</p>
          </div>
          <Link
            to="/diagnostics"
            className="flex items-center gap-1 text-xs font-medium text-accent hover:underline"
          >
            {t("common.viewAll")}
            <ArrowRight className="size-3.5" aria-hidden />
          </Link>
        </div>

        {bottlenecks.length === 0 ? (
          <Card className="p-6">
            <div className="flex items-center gap-3">
              <div className="grid size-10 place-items-center rounded-xl border border-ok/30 bg-ok/10 text-ok">
                <Sparkles className="size-4" aria-hidden />
              </div>
              <div>
                <p className="text-sm font-medium">{t("dashboard.noRecommendations")}</p>
                <p className="text-xs text-muted">{t("dashboard.noRecommendationsHint")}</p>
              </div>
            </div>
          </Card>
        ) : (
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {bottlenecks.slice(0, 3).map((bottleneck) => (
              <InsightCard key={bottleneckKey(bottleneck)} bottleneck={bottleneck} />
            ))}
          </div>
        )}
      </section>

      <div className="grid gap-4 xl:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>{t("dashboard.hardware")}</CardTitle>
            <Badge tone="muted">{profile.os.arch}</Badge>
          </CardHeader>
          <CardContent className="space-y-0">
            <HardwareRow
              icon={Cpu}
              label={t("diagnostics.processor")}
              value={profile.cpu.brand}
              detail={t("dashboard.cores", {
                physical: profile.cpu.physicalCores,
                logical: profile.cpu.logicalCores,
              })}
            />
            <HardwareRow
              icon={MemoryStick}
              label={t("diagnostics.memory")}
              value={formatBytes(profile.memory.totalBytes)}
              detail={t("dashboard.usedOf", {
                used: formatBytes(profile.memory.usedBytes),
                total: formatBytes(profile.memory.totalBytes),
              })}
            />
            <HardwareRow
              icon={HardDrive}
              label={t("diagnostics.storage")}
              value={mainDisk?.name ?? t("common.unknown")}
              detail={
                mainDisk
                  ? `${mainDisk.kind.toUpperCase()} · ${t("dashboard.free", {
                      value: formatBytes(mainDisk.availableBytes),
                    })}`
                  : t("common.unknown")
              }
            />
            <HardwareRow
              icon={Activity}
              label={t("diagnostics.gpus")}
              value={profile.gpus[0]?.name ?? t("diagnostics.noGpu")}
              detail={profile.gpus[0]?.vramBytes ? formatBytes(profile.gpus[0].vramBytes) : "—"}
            />
            <HardwareRow
              icon={Activity}
              label={t("dashboard.operatingSystem")}
              value={`${profile.os.name} ${profile.os.version}`.trim()}
              detail={profile.os.kernelVersion || t("common.unknown")}
            />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t("dashboard.quickActions")}</CardTitle>
            <Badge tone="accent">{t("app.foundation")}</Badge>
          </CardHeader>
          <CardContent className="space-y-2.5">
            <QuickAction
              to="/optimizations"
              icon={Sparkles}
              title={t("dashboard.optimizeNow")}
              hint={t("dashboard.optimizeNowHint")}
            />
            <QuickAction
              to="/monitor"
              icon={Activity}
              title={t("dashboard.openMonitor")}
              hint={t("dashboard.openMonitorHint")}
            />
            <p className="pt-1 text-[11px] leading-relaxed text-muted">
              {t("dashboard.engineNote")}
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

function HardwareRow({
  icon: Icon,
  label,
  value,
  detail,
}: {
  icon: typeof Cpu;
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <div className="flex items-center gap-3 border-b border-border/60 py-3 last:border-0">
      <div className="grid size-8 shrink-0 place-items-center rounded-lg border border-border/70 bg-panel-2 text-muted">
        <Icon className="size-3.5" aria-hidden />
      </div>
      <div className="min-w-0 flex-1">
        <p className="text-[10px] uppercase tracking-wider text-muted">{label}</p>
        <p className="truncate text-xs font-medium text-text">{value}</p>
      </div>
      <span className="max-w-[45%] truncate text-right text-[11px] text-muted">{detail}</span>
    </div>
  );
}

function QuickAction({
  to,
  icon: Icon,
  title,
  hint,
}: {
  to: string;
  icon: typeof Cpu;
  title: string;
  hint: string;
}) {
  return (
    <Link
      to={to}
      className={cn(
        "group flex items-center gap-3 rounded-xl border border-border/80 bg-panel-2/40 p-3.5",
        "transition-colors hover:border-accent/50 hover:bg-panel-2",
      )}
    >
      <div className="grid size-9 shrink-0 place-items-center rounded-lg bg-accent-soft text-accent">
        <Icon className="size-4" aria-hidden />
      </div>
      <div className="min-w-0 flex-1">
        <p className="text-xs font-medium text-text">{title}</p>
        <p className="truncate text-[11px] text-muted">{hint}</p>
      </div>
      <ArrowRight
        className="size-4 shrink-0 text-muted transition-transform group-hover:translate-x-0.5 group-hover:text-accent"
        aria-hidden
      />
    </Link>
  );
}
