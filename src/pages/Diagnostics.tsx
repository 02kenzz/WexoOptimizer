import { Cpu, HardDrive, MemoryStick, Monitor, Activity } from "lucide-react";
import { useTranslation } from "react-i18next";

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
    <div className="flex items-center justify-between gap-3 border-b border-line py-2 text-xs last:border-0">
      <span className="text-mut">{label}</span>
      <span className="truncate text-right font-medium text-text">{value}</span>
    </div>
  );
}

function DiskRow({ disk }: { disk: DiskInfo }) {
  const { t } = useTranslation();
  const usedPercent = disk.totalBytes > 0 ? 100 - (disk.availableBytes / disk.totalBytes) * 100 : 0;

  return (
    <div className="border-b border-line py-3 last:border-0">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <p className="flex items-center gap-2 text-xs font-medium">
            <span className="font-mono text-[11px] text-mut">{disk.mountPoint}</span>
            <span className="truncate text-text">{disk.name}</span>
          </p>
          <p className="mt-0.5 text-[11px] text-mut">
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
          <Activity className="size-4 text-accent" aria-hidden />
          {t("diagnostics.window")}
        </CardTitle>
        <Badge tone={window.sampleCount >= 5 ? "accent" : "muted"}>
          {t("diagnostics.sampleCount", { value: window.sampleCount })}
        </Badge>
      </CardHeader>
      <CardContent>
        {window.sampleCount < 5 ? (
          <p className="py-2 text-xs leading-relaxed text-mut">{t("diagnostics.windowHint")}</p>
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

function HardwareCard({ icon: Icon, title, score, children }: {
  icon: typeof Cpu;
  title: string;
  score: number;
  children: React.ReactNode;
}) {
  const { t } = useTranslation();

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Icon className="size-4 text-accent" aria-hidden />
          {title}
        </CardTitle>
        <Badge tone={scoreTone(score)}>{score}</Badge>
      </CardHeader>
      <CardContent>{children}</CardContent>
    </Card>
  );
}

export function Diagnostics() {
  const { t } = useTranslation();
  const { data: report, isLoading, isError, refetch, dataUpdatedAt } = useDiagnostics(4000);

  if (isLoading) {
    return <Card className="h-72 animate-pulse" />;
  }

  if (isError || !report) {
    return (
      <EmptyState
        icon={Activity}
        title={t("common.error")}
        description={t("common.backendUnavailable")}
        action={
          <Button size="sm" onClick={() => void refetch()}>
            {t("common.refresh")}
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
    <section className="view" id="diagnostico">
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-line bg-card px-4 py-3">
        <div className="flex items-center gap-2 text-xs text-mut">
          <Activity className="size-3.5 text-accent" aria-hidden />
          {t("dashboard.updated", { seconds: secondsAgo })} · {profile.os.hostname}
        </div>
        <div className="flex items-center gap-2">
          <Badge tone={scoreTone(health.global)} dot>
            {t("dashboard.health")} {health.global}
          </Badge>
          <Button variant="ghost" size="sm" onClick={() => void refetch()}>
            {t("common.refresh")}
          </Button>
        </div>
      </div>

      <div className="grid gap-4 lg:grid-cols-2">
        <HardwareCard
          icon={Cpu}
          title={t("diagnostics.processor")}
          score={health.cpu}
        >
          <p className="truncate text-sm font-medium text-text">{profile.cpu.brand || "—"}</p>
          <p className="mt-0.5 text-[11px] text-mut">{profile.cpu.vendor || "—"}</p>
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
        </HardwareCard>

        <HardwareCard
          icon={MemoryStick}
          title={t("diagnostics.memory")}
          score={health.memory}
        >
          <p className="text-sm font-medium text-text">{formatBytes(profile.memory.totalBytes)}</p>
          <p className="mt-0.5 text-[11px] text-mut">
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
        </HardwareCard>

        <HardwareCard
          icon={HardDrive}
          title={t("diagnostics.storage")}
          score={health.disk}
        >
          {profile.disks.length === 0 ? (
            <p className="py-6 text-center text-xs text-mut">{t("common.unknown")}</p>
          ) : (
            profile.disks.map((disk) => (
              <DiskRow key={`${disk.mountPoint}-${disk.name}`} disk={disk} />
            ))
          )}
        </HardwareCard>

        <HardwareCard
          icon={Monitor}
          title={t("diagnostics.system")}
          score={health.network}
        >
          <p className="text-sm font-medium text-text">
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
        </HardwareCard>

        <HardwareCard
          icon={Activity}
          title={t("diagnostics.gpus")}
          score={health.network}
        >
          {profile.gpus.length === 0 ? (
            <p className="py-6 text-center text-xs text-mut">{t("diagnostics.noGpu")}</p>
          ) : (
            profile.gpus.map((gpu) => (
              <div
                key={`${gpu.name}-${gpu.driverVersion}`}
                className="border-b border-line py-3 first:pt-0 last:border-0 last:pb-0"
              >
                <p className="truncate text-xs font-medium text-text">{gpu.name || "—"}</p>
                <p className="mt-0.5 text-[11px] text-mut">
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
        </HardwareCard>

        <WindowSummaryCard report={report} />
      </div>

      <section>
        <div className="mb-3 flex items-end justify-between">
          <div>
            <h2 className="text-sm font-semibold tracking-tight text-text">{t("diagnostics.bottlenecks")}</h2>
            <p className="text-xs text-mut">
              {t("common.severity")} · {t("common.confidence")} · {t("common.evidence")}
            </p>
          </div>
          <Badge tone={bottlenecks.length === 0 ? "ok" : "warn"}>{bottlenecks.length}</Badge>
        </div>

        {bottlenecks.length === 0 ? (
          <EmptyState
            icon={Activity}
            title={t("diagnostics.noBottlenecks")}
            description={t("diagnostics.noBottlenecksHint")}
          />
        ) : (
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {bottlenecks.map((bottleneck) => (
              <div key={bottleneckKey(bottleneck)} className="card">
                <div className="flex items-start justify-between gap-3 mb-3">
                  <div className="flex items-center gap-2.5">
                    <span className="size-10 rounded-xl bg-card flex items-center justify-center">
                      <Activity className="size-4.5 text-accent" aria-hidden />
                    </span>
                    <h4 className="text-sm font-semibold text-text">{bottleneck.title}</h4>
                  </div>
                  <Badge
                    tone={bottleneck.risk === "low" ? "ok" : bottleneck.risk === "medium" ? "warn" : "danger"}
                    className="text-[10px]"
                  >
                    {t(`risk.${bottleneck.risk}`)}
                  </Badge>
                </div>
                <p className="text-xs leading-relaxed text-mut mb-4">{bottleneck.description}</p>
                <div className="flex items-center gap-2 text-[10px] text-mut">
                  <span className="flex items-center gap-1">
                    <span className="size-1.5 rounded-full" style={{ backgroundColor: bottleneck.risk === "low" ? "var(--ok)" : bottleneck.risk === "medium" ? "var(--warn)" : "var(--danger)" }} aria-hidden />
                    {t("common.impact")}: {bottleneck.impact}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>
    </section>
  );
}