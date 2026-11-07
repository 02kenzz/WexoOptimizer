import { Activity, Cpu, HardDrive, Info, MemoryStick, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";

import { InsightCard } from "@/components/InsightCard";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";
import { Meter } from "@/components/ui/Meter";
import { useSystemProfile } from "@/hooks/useSystemProfile";
import { computeHealth, computeInsights } from "@/lib/insights";
import { usageTone } from "@/lib/tone";
import { formatBytes, formatMhz, formatPercent } from "@/lib/utils";
import type { SystemProfile } from "@/types";

function StatLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-3 border-b border-border/50 py-2 text-xs last:border-0">
      <span className="text-muted">{label}</span>
      <span className="truncate text-right font-medium text-text">{value}</span>
    </div>
  );
}

function DiskRow({ disk }: { disk: SystemProfile["disks"][number] }) {
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

export function Diagnostics() {
  const { t } = useTranslation();
  const { data: profile, isLoading, isError, refetch, dataUpdatedAt } = useSystemProfile(4000);

  if (isLoading) {
    return <Card className="h-72 animate-pulse bg-panel-2/40" />;
  }

  if (isError || !profile) {
    return (
      <EmptyState
        icon={Activity}
        title={t("common.error")}
        description={t("common.browserHint")}
        action={
          <Button size="sm" onClick={() => void refetch()}>
            {t("common.retry")}
          </Button>
        }
      />
    );
  }

  const health = computeHealth(profile);
  const insights = computeInsights(profile);
  const memoryUsed = profile.memory.totalBytes
    ? (profile.memory.usedBytes / profile.memory.totalBytes) * 100
    : 0;
  const secondsAgo = Math.max(0, Math.round((Date.now() - dataUpdatedAt) / 1000));

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border/70 bg-panel/60 px-4 py-3">
        <div className="flex items-center gap-2 text-xs text-muted">
          <Info className="size-3.5 text-accent" aria-hidden />
          {t("dashboard.updated", { seconds: secondsAgo })} · {profile.os.hostname}
        </div>
        <div className="flex items-center gap-2">
          <Badge tone={health.global >= 75 ? "ok" : health.global >= 50 ? "warn" : "danger"} dot>
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
            <Badge tone={health.cpu >= 75 ? "ok" : health.cpu >= 50 ? "warn" : "danger"}>
              {health.cpu}
            </Badge>
          </CardHeader>
          <CardContent>
            <p className="truncate text-sm font-medium">{profile.cpu.brand}</p>
            <p className="mt-0.5 text-[11px] text-muted">{profile.cpu.vendor}</p>
            <div className="mt-3">
              <StatLine
                label={t("dashboard.cores", {
                  physical: profile.cpu.physicalCores,
                  logical: profile.cpu.logicalCores,
                })}
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
            <Badge tone={health.memory >= 75 ? "ok" : health.memory >= 50 ? "warn" : "danger"}>
              {health.memory}
            </Badge>
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
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <HardDrive className="size-4 text-accent" aria-hidden />
              {t("diagnostics.storage")}
            </CardTitle>
            <Badge tone={health.disk >= 75 ? "ok" : health.disk >= 50 ? "warn" : "danger"}>
              {health.disk}
            </Badge>
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
            <Badge tone="muted">{profile.os.arch}</Badge>
          </CardHeader>
          <CardContent>
            <p className="text-sm font-medium">
              {profile.os.name} {profile.os.version}
            </p>
            <div className="mt-3">
              <StatLine label={t("diagnostics.kernel")} value={profile.os.kernelVersion || "—"} />
              <StatLine label={t("diagnostics.hostname")} value={profile.os.hostname || "—"} />
              <StatLine label={t("diagnostics.architecture")} value={profile.os.arch || "—"} />
              <StatLine
                label={t("dashboard.network")}
                value={`${profile.networks.length} · ${profile.networks[0]?.name ?? "—"}`}
              />
            </div>
          </CardContent>
        </Card>
      </div>

      <section>
        <div className="mb-3 flex items-end justify-between">
          <div>
            <h2 className="text-sm font-semibold tracking-tight">{t("diagnostics.bottlenecks")}</h2>
            <p className="text-xs text-muted">
              {t("common.severity")} · {t("common.confidence")} · {t("common.evidence")}
            </p>
          </div>
          <Badge tone={insights.length === 0 ? "ok" : "warn"}>{insights.length}</Badge>
        </div>

        {insights.length === 0 ? (
          <EmptyState
            icon={Activity}
            title={t("diagnostics.noBottlenecks")}
            description={t("diagnostics.noBottlenecksHint")}
          />
        ) : (
          <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
            {insights.map((insight) => (
              <InsightCard key={insight.id} insight={insight} />
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
