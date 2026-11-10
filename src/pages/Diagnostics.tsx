import { Activity, Cpu, HardDrive, MemoryStick, Monitor } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { InsightCard } from "@/components/InsightCard";
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

function StatLine({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className={mono ? "kv mono" : "kv"}>
      <span>{label}</span>
      <b title={value}>{value}</b>
    </div>
  );
}

function DiskRow({ disk }: { disk: DiskInfo }) {
  const { t } = useTranslation();
  const used = disk.totalBytes > 0 ? 100 - (disk.availableBytes / disk.totalBytes) * 100 : 0;

  return (
    <div style={{ padding: "10px 0", borderBottom: "1px solid rgba(255,255,255,0.05)" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 10 }}>
        <div style={{ minWidth: 0 }}>
          <p style={{ display: "flex", alignItems: "center", gap: 7, fontSize: 12 }}>
            <span style={{ fontFamily: "var(--mono)", fontSize: 10.5, color: "var(--accent)" }}>
              {disk.mountPoint}
            </span>
            <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {disk.name}
            </span>
          </p>
          <p style={{ marginTop: 2, fontSize: 10.5, color: "var(--dim)" }}>
            {t("diagnostics.freeOf", {
              free: formatBytes(disk.availableBytes),
              total: formatBytes(disk.totalBytes),
            })}
          </p>
        </div>
        <div style={{ display: "flex", flexShrink: 0, alignItems: "center", gap: 5 }}>
          {disk.isRemovable ? <Badge tone="muted">{t("diagnostics.removable")}</Badge> : null}
          {disk.busType ? <Badge tone="muted">{disk.busType}</Badge> : null}
          <Badge tone={disk.kind === "hdd" ? "warn" : "accent"}>{disk.kind.toUpperCase()}</Badge>
        </div>
      </div>
      <Meter value={used} tone={usageTone(used)} className="mt-2" height={6} label={disk.mountPoint} />
    </div>
  );
}

function WindowSummaryCard({ report }: { report: DiagnosticReport }) {
  const { t } = useTranslation();
  const window = report.window;

  return (
    <Card>
      <div className="panel-head">
        <h3 className="panel-title">
          <Activity size={14} strokeWidth={2} aria-hidden />
          {t("diagnostics.window")}
        </h3>
        <Badge tone={window.sampleCount >= 5 ? "accent" : "muted"}>
          {t("diagnostics.sampleCount", { value: window.sampleCount })}
        </Badge>
      </div>

      {window.sampleCount < 5 ? (
        <p style={{ fontSize: 11.5, lineHeight: 1.6, color: "var(--dim)" }}>
          {t("diagnostics.windowHint")}
        </p>
      ) : (
        <div>
          <StatLine label={t("diagnostics.windowDuration")} value={formatDuration(window.durationMs / 1000)} />
          <StatLine label={t("diagnostics.cpuP50")} value={formatPercent(window.cpuP50, 1)} />
          <StatLine label={t("diagnostics.cpuP95")} value={formatPercent(window.cpuP95, 1)} />
          <StatLine label={t("diagnostics.memoryP95")} value={formatPercent(window.memoryP95, 1)} />
          <StatLine label={t("diagnostics.diskBusyP95")} value={formatPercent(window.diskBusyP95, 1)} />
          <StatLine label={t("diagnostics.diskQueueP95")} value={window.diskQueueP95.toFixed(1)} />
          <StatLine label={t("diagnostics.networkRx")} value={formatRate(window.networkRxAvg)} />
          <StatLine label={t("diagnostics.networkTx")} value={formatRate(window.networkTxAvg)} />
        </div>
      )}
    </Card>
  );
}

function HardwareCard({
  icon: Icon,
  title,
  score,
  children,
}: {
  icon: typeof Cpu;
  title: string;
  score: number;
  children: ReactNode;
}) {
  return (
    <Card>
      <div className="panel-head">
        <h3 className="panel-title">
          <Icon size={14} strokeWidth={2} aria-hidden />
          {title}
        </h3>
        <span className={`chip chip-${scoreTone(score)}`}>{score}</span>
      </div>
      {children}
    </Card>
  );
}

export function Diagnostics() {
  const { t } = useTranslation();
  const { data: report, isLoading, isError, refetch, dataUpdatedAt } = useDiagnostics(4000);

  if (isLoading) {
    return (
      <div className="grid cols-2">
        <div className="panel" style={{ height: 220, opacity: 0.5 }} />
        <div className="panel" style={{ height: 220, opacity: 0.4 }} />
        <div className="panel" style={{ height: 220, opacity: 0.3 }} />
        <div className="panel" style={{ height: 220, opacity: 0.2 }} />
      </div>
    );
  }

  if (isError || !report) {
    return (
      <EmptyState
        icon={Activity}
        title={t("common.error")}
        description={t("common.backendUnavailable")}
        action={
          <Button size="sm" variant="ghost" onClick={() => void refetch()}>
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
    profile.disks.find((disk) => disk.mountPoint.toUpperCase().startsWith("C:")) ?? profile.disks[0];

  return (
    <section className="view" id="diagnostico">
      <div className="strip">
        <div className="strip-left">
          <span className="pulse-dot" />
          {t("dashboard.updated", { seconds: secondsAgo })}
          <span style={{ opacity: 0.4 }}>·</span>
          <strong>{profile.os.hostname}</strong>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <Badge tone={scoreTone(health.global)} dot>
            {t("dashboard.health")} {health.global}
          </Badge>
          <Button variant="ghost" size="sm" onClick={() => void refetch()}>
            {t("common.refresh")}
          </Button>
        </div>
      </div>

      <div className="grid cols-2">
        <HardwareCard icon={Cpu} title={t("diagnostics.processor")} score={health.cpu}>
          <p style={{ fontSize: 13, fontWeight: 550 }}>{profile.cpu.brand || "—"}</p>
          <p style={{ marginTop: 2, fontSize: 10.5, color: "var(--dim)" }}>{profile.cpu.vendor || "—"}</p>
          <div style={{ marginTop: 10 }}>
            <StatLine
              label={t("diagnostics.cores")}
              value={t("dashboard.cores", {
                physical: profile.cpu.physicalCores,
                logical: profile.cpu.logicalCores,
              })}
            />
            <StatLine label={t("diagnostics.baseFrequency")} value={formatMhz(profile.cpu.baseFrequencyMhz)} />
            <StatLine label={t("dashboard.frequency")} value={formatMhz(profile.cpu.currentFrequencyMhz)} />
            <StatLine label={t("diagnostics.usage")} value={formatPercent(profile.cpu.usagePercent, 1)} />
          </div>
          <Meter
            value={profile.cpu.usagePercent}
            tone={usageTone(profile.cpu.usagePercent)}
            className="mt-2"
            label={t("diagnostics.usage")}
            showValue
            height={6}
          />
        </HardwareCard>

        <HardwareCard icon={MemoryStick} title={t("diagnostics.memory")} score={health.memory}>
          <p style={{ fontSize: 13, fontWeight: 550 }}>{formatBytes(profile.memory.totalBytes)}</p>
          <p style={{ marginTop: 2, fontSize: 10.5, color: "var(--dim)" }}>
            {t("dashboard.usedOf", {
              used: formatBytes(profile.memory.usedBytes),
              total: formatBytes(profile.memory.totalBytes),
            })}
          </p>
          <Meter value={memoryUsed} tone={usageTone(memoryUsed)} className="mt-2" label={t("diagnostics.memory")} showValue height={6} />
          <div style={{ marginTop: 10 }}>
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
            <StatLine label={t("diagnostics.memorySpeed")} value={formatMhz(profile.memory.speedMhz)} />
          </div>
        </HardwareCard>

        <HardwareCard icon={HardDrive} title={t("diagnostics.storage")} score={health.disk}>
          {profile.disks.length === 0 ? (
            <p style={{ padding: "18px 0", textAlign: "center", fontSize: 11, color: "var(--dim)" }}>
              {t("common.unknown")}
            </p>
          ) : (
            profile.disks.map((disk) => <DiskRow key={`${disk.mountPoint}-${disk.name}`} disk={disk} />)
          )}
        </HardwareCard>

        <HardwareCard icon={Monitor} title={t("diagnostics.system")} score={health.network}>
          <p style={{ fontSize: 13, fontWeight: 550 }}>
            {profile.os.name} {profile.os.version}
          </p>
          <div style={{ marginTop: 10 }}>
            <StatLine label={t("diagnostics.kernel")} value={profile.os.kernelVersion || "—"} mono />
            <StatLine label={t("diagnostics.build")} value={profile.os.build || "—"} mono />
            <StatLine label={t("diagnostics.hostname")} value={profile.os.hostname || "—"} />
            <StatLine label={t("diagnostics.architecture")} value={profile.os.arch || "—"} mono />
            <StatLine label={t("diagnostics.uptime")} value={formatDuration(profile.uptimeSeconds)} />
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

        <HardwareCard icon={Activity} title={t("diagnostics.gpus")} score={health.network}>
          {profile.gpus.length === 0 ? (
            <p style={{ padding: "18px 0", textAlign: "center", fontSize: 11, color: "var(--dim)" }}>
              {t("diagnostics.noGpu")}
            </p>
          ) : (
            profile.gpus.map((gpu) => (
              <div
                key={`${gpu.name}-${gpu.driverVersion}`}
                style={{ padding: "10px 0", borderBottom: "1px solid rgba(255,255,255,0.05)" }}
              >
                <p style={{ fontSize: 12, fontWeight: 550 }}>{gpu.name || "—"}</p>
                <p style={{ marginTop: 2, fontSize: 10.5, color: "var(--dim)" }}>
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
        <div className="split-head" style={{ marginBottom: 12 }}>
          <div>
            <h2 className="section-title">{t("diagnostics.bottlenecks")}</h2>
            <p className="section-sub">
              {t("common.severity")} · {t("common.confidence")} · {t("common.evidence")}
            </p>
          </div>
          <Badge tone={bottlenecks.length === 0 ? "ok" : "warn"}>
            {bottlenecks.length}
          </Badge>
        </div>

        {bottlenecks.length === 0 ? (
          <EmptyState
            icon={Activity}
            title={t("diagnostics.noBottlenecks")}
            description={t("diagnostics.noBottlenecksHint")}
          />
        ) : (
          <div className="grid cols-2">
            {bottlenecks.map((bottleneck) => (
              <InsightCard key={bottleneckKey(bottleneck)} bottleneck={bottleneck} />
            ))}
          </div>
        )}
      </section>
    </section>
  );
}
