import { Activity, Cpu, HardDrive, MemoryStick, Network, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";

import { LiveChart } from "@/components/LiveChart";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";
import { Gauge } from "@/components/ui/Gauge";
import { Meter } from "@/components/ui/Meter";
import { useLiveMetrics } from "@/hooks/useLiveMetrics";
import { usageTone } from "@/lib/tone";
import { clamp, cn, formatBytes, formatDuration, formatRate } from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";

function LiveTile({
  icon: Icon,
  label,
  value,
  unit,
  percent,
  hint,
}: {
  icon: typeof Cpu;
  label: string;
  value: string;
  unit?: string;
  percent: number;
  hint?: string;
}) {
  return (
    <div className="tile">
      <div className="tile-head">
        <span>
          <Icon size={13} strokeWidth={1.9} aria-hidden />
          {label}
        </span>
        <span className="pulse-dot" />
      </div>
      <div className="tile-value">
        {value}
        {unit ? <small>{unit}</small> : null}
      </div>
      <div className="tile-hint">{hint ?? "\u00a0"}</div>
      <Meter value={percent} tone={usageTone(percent)} height={5} />
    </div>
  );
}

function CoreGrid({ values }: { values: number[] }) {
  const { t } = useTranslation();

  if (values.length === 0) {
    return (
      <p style={{ padding: "16px 0", textAlign: "center", fontSize: 11, color: "var(--dim)" }}>
        {t("monitor.noCores")}
      </p>
    );
  }

  return (
    <div className="core-grid">
      {values.map((value, index) => {
        const percent = clamp(value, 0, 100);
        return (
          <div
            key={index.toString()}
            className={cn("core", usageTone(percent))}
            title={`${t("monitor.core")} ${index + 1}: ${percent.toFixed(0)}%`}
          >
            <span>{index + 1}</span>
            <i style={{ height: `${percent}%` }} />
          </div>
        );
      })}
    </div>
  );
}

function LegendDot({ color, label }: { color: string; label: string }) {
  return (
    <span>
      <i style={{ backgroundColor: color }} aria-hidden />
      {label}
    </span>
  );
}

export function Monitor() {
  const { t } = useTranslation();
  const interval = useAppStore((state) => state.settings.monitorIntervalMs);
  const { samples, latest, loading, error } = useLiveMetrics();

  if (loading && !latest) {
    return (
      <div className="tiles">
        <div className="panel" style={{ height: 130, opacity: 0.5 }} />
        <div className="panel" style={{ height: 130, opacity: 0.4 }} />
        <div className="panel" style={{ height: 130, opacity: 0.3 }} />
        <div className="panel" style={{ height: 130, opacity: 0.2 }} />
      </div>
    );
  }

  if (error && !latest) {
    return (
      <EmptyState
        icon={Activity}
        title={t("common.error")}
        description={t("common.backendUnavailable")}
        action={
          <Button size="sm" variant="ghost" onClick={() => window.location.reload()}>
            <RefreshCw size={13} aria-hidden />
            {t("common.retry")}
          </Button>
        }
      />
    );
  }

  const cpu = latest?.cpuPercent ?? 0;
  const memory = latest?.memoryPercent ?? 0;
  const diskBusy = latest?.diskBusyPercent ?? 0;

  return (
    <section className="view" id="monitor">
      <div className="tiles">
        <LiveTile
          icon={Cpu}
          label={t("monitor.liveCpu")}
          value={cpu.toFixed(0)}
          unit="%"
          percent={cpu}
          hint={latest ? `${latest.cpuFrequencyMhz.toFixed(0)} MHz` : undefined}
        />
        <LiveTile
          icon={MemoryStick}
          label={t("monitor.liveMemory")}
          value={memory.toFixed(0)}
          unit="%"
          percent={memory}
          hint={latest ? formatBytes(latest.memoryUsedBytes) : undefined}
        />
        <LiveTile
          icon={HardDrive}
          label={t("monitor.liveDisk")}
          value={diskBusy.toFixed(0)}
          unit="%"
          percent={diskBusy}
          hint={
            latest
              ? t("monitor.diskThroughput", {
                  read: formatBytes(latest.diskReadBytesPerSec),
                  write: formatBytes(latest.diskWriteBytesPerSec),
                })
              : undefined
          }
        />
        <LiveTile
          icon={Network}
          label={t("monitor.liveNetwork")}
          value={latest ? formatRate(latest.networkRxBytesPerSec) : "—"}
          percent={Math.min(100, (latest?.networkRxBytesPerSec ?? 0) / 1_000_000)}
          hint={latest ? t("monitor.upload", { value: formatRate(latest.networkTxBytesPerSec) }) : undefined}
        />
      </div>

      <Card>
        <div className="panel-head">
          <div>
            <h3 className="panel-title">
              <Activity size={14} strokeWidth={2} aria-hidden />
              {t("monitor.chartTitle")}
            </h3>
            <p className="panel-sub">
              {latest ? t("monitor.liveSince", { value: formatDuration(latest.uptimeSeconds) }) : t("monitor.waiting")}
              {" · "}
              {t("monitor.sampleCount", { value: samples.length })}
            </p>
          </div>
          <div className="legend">
            <LegendDot color="var(--accent)" label={t("monitor.chartCpu")} />
            <LegendDot color="var(--accent-2)" label={t("monitor.chartMemory")} />
            <LegendDot color="var(--warn)" label={t("monitor.chartDisk")} />
          </div>
        </div>
        <div className="chart-shell">
          <LiveChart samples={samples} />
        </div>
      </Card>

      <div className="grid cols-hero">
        <Card>
          <div className="panel-head">
            <h3 className="panel-title">
              <Cpu size={14} strokeWidth={2} aria-hidden />
              {t("monitor.cores", { value: latest?.perCorePercent.length ?? 0 })}
            </h3>
            <Badge tone="muted">{t("monitor.perCore")}</Badge>
          </div>
          <CoreGrid values={latest?.perCorePercent ?? []} />
        </Card>

        <Card>
          <div className="panel-head">
            <h3 className="panel-title">
              <Activity size={14} strokeWidth={2} aria-hidden />
              {t("monitor.systemStatus")}
            </h3>
            <span className="chip chip-ok">
              <span className="dot" />
              {t("dashboard.live")}
            </span>
          </div>

          <div className="gauges">
            <Gauge value={cpu} label={t("dashboard.cpu")} size={132} strokeWidth={8} tone="accent" />
            <Gauge
              value={memory}
              label={t("dashboard.memory")}
              size={132}
              strokeWidth={8}
              tone="accent-2"
            />
          </div>

          <div>
            <div className="kv">
              <span>{t("monitor.sampling")}</span>
              <b>{interval} ms</b>
            </div>
            <div className="kv">
              <span>{t("dashboard.uptime")}</span>
              <b>{latest ? formatDuration(latest.uptimeSeconds) : "—"}</b>
            </div>
            <div className="kv">
              <span>{t("dashboard.storage")}</span>
              <b>
                {latest
                  ? `${formatBytes(latest.diskReadBytesPerSec)}/s`
                  : "—"}
              </b>
            </div>
          </div>

          <p className="panel-sub" style={{ marginTop: 10 }}>
            {t("monitor.targets")}
          </p>
        </Card>
      </div>
    </section>
  );
}
