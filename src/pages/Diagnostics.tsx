import {
  Activity,
  Cpu,
  Gauge,
  HardDrive,
  Info,
  MemoryStick,
  Monitor,
  RefreshCw,
  Timer,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { InsightCard } from "@/components/InsightCard";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";
import { Meter } from "@/components/ui/Meter";
import { useDiagnostics } from "@/hooks/useDiagnostics";
import { bottleneckKey } from "@/lib/bottlenecks";
import { usageTone } from "@/lib/tone";
import {
  formatBytes,
  formatDuration,
  formatMhz,
  formatPercent,
  formatRate,
  scoreTone,
} from "@/lib/utils";
import type { DiagnosticReport, DiskInfo } from "@/types";

function StatLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3 border-b border-border/50 py-2 text-xs last:border-0">
      <span className="text-muted">{label}</span>
      <span className="truncate text-right font-medium text-text">{value}</span>
    </div>
  );
}

function DiskRow({ disk }: { disk: DiskInfo }) {
  const { t } = useTranslation();
  const usedPercent = disk.totalBytes > 0 ? 100 - (disk.availableBytes / disk.totalBytes) * 100 : 0;

  return (
    <div className="border-b border-border/50 py-3 last:border-0">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="flex items-center gap-2 text-xs font-medium">
            <span className="font-mono text-[11px] text-muted">{disk.mountPoint}</span>
            <span className="truncate">{disk.name}</span>
          </p>
          <p className="mt-0.5 text-[11px] text-muted">
            {t("diagnostics.freeOf", {
              free: formatBytes(disk.availableBytes),
              total: formatBytes(disk.totalBytes),
            })}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          {disk.isRemovable ? <Badge tone="muted">{t("diagnostics.removable")}</Badge> : null}
          {disk.busType ? <Badge tone="muted">{disk.busType}</Badge> : null}
          <Badge tone={disk.kind === "hdd" ? "warn" : "accent"}>{disk.kind.toUpperCase()}</Badge>
        </div>
      </div>
      <Meter
        value={usedPercent}
        tone={usageTone(usedPercent)}
        className="mt-2.5"
        label={disk.mountPoint}
      />
    </div>
  );
}

function WindowSummaryCard({ report }: { report: DiagnosticReport }) {
  const { t } = useTranslation();
  const window = report.window;

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Timer className="size-4 text-accent" aria-hidden />
          {t("diagnostics.window")}
        </CardTitle>
        <Badge tone={window.sampleCount >= 5 ? "accent" : "muted"}>
          {t("diagnostics.sampleCount", { value: window.sampleCount })}
        </Badge>
      </CardHeader>
      <CardContent>
        {window.sampleCount < 5 ? (
          <p className="py-2 text-xs leading-relaxed text-muted">{t("diagnostics.windowHint")}</p>
        ) : (
          <div>
            <StatLine
              label={t("diagnostics.windowDuration")}
              value={formatDuration(window.durationMs / 1000)}
            />
            <StatLine label={t("diagnostics.cpuP50")} value={formatPercent(window.cpuP50, 1)} />
            <StatLine label={t("diagnostics.cpuP95")} value={formatPercent(window.cpuP95, 1)} />
            <StatLine
              label={t("diagnostics.memoryP95")}
              value={formatPercent(window.memoryP95, 1)}
            />
            <StatLine
              label={t("diagnostics.diskBusyP95")}
              value={formatPercent(window.diskBusyP95, 1)}
            />
            <StatLine
              label={t("diagnostics.diskQueueP95")}
              value={window.diskQueueP95.toFixed(1)}
            />
            <StatLine label={t("diagnostics.networkRx")} value={formatRate(window.networkRxAvg)} />
            <StatLine label={t("diagnostics.networkTx")} value={formatRate(window.networkTxAvg)} />
          </div>
        )}
      </CardContent>
    </Card>
  );
}

export function Diagnostics() {
  const { t } = useTranslation();
  const { data: report, isLoading, isError, refetch, dataUpdatedAt } = useDiagnostics(4000);

  if (isLoading) {
    return <Card className="h-72 animate-pulse bg-panel-2/40" />;
  }

  if (isError || !report) {
    return (
      <EmptyState
        icon={Activity}
        title={t("common.error")}
        description={t("common.backendUnavailable")}
        action={
          <Button size="sm" onClick={() => void refetch()}>
            {t("common.retry")}
          </Button>
        }
      />
    );
  }

  const { profile, health, bottlenecks } = report;
  const memoryUsed = profile.memory.totalBytes
    ? (profile.memory.usedBytes / profile.memory.totalBytes) * 100
    : 0;
  const secondsAgo = Math.max(0, Math.round((Date.now() - dataUpdatedAt) / 1000));
  const systemDisk =
    profile.disks.find((disk) => disk.mountPoint.toUpperCase().startsWith("C:")) ??
    profile.disks[0];

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border/70 bg-panel/60 px-4 py-3">
        <div className="flex items-center gap-2 text-xs text-muted">
          <Info className="size-3.5 text-accent" aria-hidden />
          {t("dashboard.updated", { seconds: secondsAgo })} · {profile.os.hostname}
        </div>
        <div className="flex items-center gap-2">
          <Badge tone={scoreTone(health.global)} dot>
            {t("dashboard.health")} {health.global}
          </Badge>
          <Button variant="ghost" size="sm" onClick={() => void refetch()}>
            <RefreshCw className="size-3.5" aria-hidden />
            {t("common.refresh")}
          </Button>
        </div>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Cpu className="size-4 text-accent" aria-hidden />
              {t("diagnostics.processor")}
            </CardTitle>
            <Badge tone={scoreTone(health.cpu)}>{health.cpu}</Badge>
          </CardHeader>
          <CardContent>
            <p className="truncate text-sm font-medium">{profile.cpu.brand || "—"}</p>
            <p className="mt-0.5 text-[11px] text-muted">{profile.cpu.vendor || "—"}</p>
            <div className="mt-3">
              <StatLine
                label={t("diagnostics.cores")}
                value={t("dashboard.cores", {
                  physical: profile.cpu.physicalCores,
                  logical: profile.cpu.logicalCores,
                })}
              />
              <StatLine
                label={t("diagnostics.baseFrequency")}
                value={formatMhz(profile.cpu.baseFrequencyMhz)}
              />
              <StatLine
                label={t("dashboard.frequency")}
                value={formatMhz(profile.cpu.currentFrequencyMhz)}
              />
              <StatLine
                label={t("diagnostics.usage")}
                value={formatPercent(profile.cpu.usagePercent, 1)}
              />
            </div>
            <Meter
              value={profile.cpu.usagePercent}
              tone={usageTone(profile.cpu.usagePercent)}
              className="mt-2"
              label={t("diagnostics.usage")}
            />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <MemoryStick className="size-4 text-accent" aria-hidden />
              {t("diagnostics.memory")}
            </CardTitle>
            <Badge tone={scoreTone(health.memory)}>{health.memory}</Badge>
          </CardHeader>
          <CardContent>
            <p className="text-sm font-medium">{formatBytes(profile.memory.totalBytes)}</p>
            <p className="mt-0.5 text-[11px] text-muted">
              {t("dashboard.usedOf", {
                used: formatBytes(profile.memory.usedBytes),
                total: formatBytes(profile.memory.totalBytes),
              })}
            </p>
            <Meter
              value={memoryUsed}
              tone={usageTone(memoryUsed)}
              className="mt-3"
              label={t("diagnostics.memory")}
            />
            <div className="mt-3">
              <StatLine
                label={t("diagnostics.swap")}
                value={t("dashboard.usedOf", {
                  used: formatBytes(profile.memory.swapUsedBytes),
                  total: formatBytes(profile.memory.swapTotalBytes),
                })}
              />
              <StatLine
                label={t("diagnostics.channels")}
                value={
                  profile.memory.channels === 2
                    ? t("diagnostics.dualChannel")
                    : profile.memory.channels === 1
                      ? t("diagnostics.singleChannel")
                      : t("common.unknown")
                }
              />
              <StatLine
                label={t("diagnostics.modules")}
                value={
                  profile.memory.moduleCount > 0
                    ? t("diagnostics.moduleCount", { value: profile.memory.moduleCount })
                    : t("common.unknown")
                }
              />
              <StatLine
                label={t("diagnostics.memorySpeed")}
                value={formatMhz(profile.memory.speedMhz)}
              />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <HardDrive className="size-4 text-accent" aria-hidden />
              {t("diagnostics.storage")}
            </CardTitle>
            <Badge tone={scoreTone(health.disk)}>{health.disk}</Badge>
          </CardHeader>
          <CardContent>
            {profile.disks.length === 0 ? (
              <p className="py-6 text-center text-xs text-muted">{t("common.unknown")}</p>
            ) : (
              profile.disks.map((disk) => (
                <DiskRow key={`${disk.mountPoint}-${disk.name}`} disk={disk} />
              ))
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Activity className="size-4 text-accent" aria-hidden />
              {t("diagnostics.system")}
            </CardTitle>
            <Badge tone="muted">{profile.os.arch || "—"}</Badge>
          </CardHeader>
          <CardContent>
            <p className="text-sm font-medium">
              {profile.os.name} {profile.os.version}
            </p>
            <div className="mt-3">
              <StatLine label={t("diagnostics.kernel")} value={profile.os.kernelVersion || "—"} />
              <StatLine label={t("diagnostics.build")} value={profile.os.build || "—"} />
              <StatLine label={t("diagnostics.hostname")} value={profile.os.hostname || "—"} />
              <StatLine label={t("diagnostics.architecture")} value={profile.os.arch || "—"} />
              <StatLine
                label={t("diagnostics.uptime")}
                value={formatDuration(profile.uptimeSeconds)}
              />
              <StatLine
                label={t("diagnostics.systemDisk")}
                value={
                  systemDisk
                    ? `${systemDisk.mountPoint} · ${systemDisk.kind.toUpperCase()}`
                    : t("common.unknown")
                }
              />
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Monitor className="size-4 text-accent" aria-hidden />
              {t("diagnostics.gpus")}
            </CardTitle>
            <Badge tone={scoreTone(health.network)}>
              {t("dashboard.network")} {health.network}
            </Badge>
          </CardHeader>
          <CardContent>
            {profile.gpus.length === 0 ? (
              <p className="py-6 text-center text-xs text-muted">{t("diagnostics.noGpu")}</p>
            ) : (
              profile.gpus.map((gpu) => (
                <div
                  key={`${gpu.name}-${gpu.driverVersion}`}
                  className="border-b border-border/50 py-3 first:pt-0 last:border-0 last:pb-0"
                >
                  <p className="truncate text-xs font-medium">{gpu.name || "—"}</p>
                  <p className="mt-0.5 text-[11px] text-muted">
                    {[
                      gpu.vendor,
                      gpu.vramBytes ? formatBytes(gpu.vramBytes) : null,
                      gpu.driverVersion ? `v${gpu.driverVersion}` : null,
                    ]
                      .filter(Boolean)
                      .join(" · ")}
                  </p>
                </div>
              ))
            )}
          </CardContent>
        </Card>

        <WindowSummaryCard report={report} />
      </div>

      <section>
        <div className="mb-3 flex items-end justify-between">
          <div>
            <h2 className="text-sm font-semibold tracking-tight">{t("diagnostics.bottlenecks")}</h2>
            <p className="text-xs text-muted">
              {t("common.severity")} · {t("common.confidence")} · {t("common.evidence")}
            </p>
          </div>
          <Badge tone={bottlenecks.length === 0 ? "ok" : "warn"}>{bottlenecks.length}</Badge>
        </div>

        {bottlenecks.length === 0 ? (
          <EmptyState
            icon={Gauge}
            title={t("diagnostics.noBottlenecks")}
            description={t("diagnostics.noBottlenecksHint")}
          />
        ) : (
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {bottlenecks.map((bottleneck) => (
              <InsightCard key={bottleneckKey(bottleneck)} bottleneck={bottleneck} />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
