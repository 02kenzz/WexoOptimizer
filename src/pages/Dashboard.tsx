import {
  Activity,
  Cpu,
  HardDrive,
  MemoryStick,
  Network,
  ScanSearch,
  Sparkles,
  Trash2,
  Zap,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import { Button } from "@/components/ui/Button";
import { EmptyState } from "@/components/ui/EmptyState";
import { ScoreRing } from "@/components/ui/ScoreRing";
import { useDiagnostics } from "@/hooks/useDiagnostics";
import { useLiveMetrics } from "@/hooks/useLiveMetrics";
import { formatBytes, formatDuration, formatPercent, formatRate, healthLabel, scoreTone } from "@/lib/utils";
import type { DiagnosticReport, DiskInfo, LiveMetrics } from "@/types";

function systemDisk(profile: DiagnosticReport["profile"]): DiskInfo | undefined {
  return (
    profile.disks.find((disk) => disk.mountPoint.toUpperCase().startsWith("C:")) ?? profile.disks[0]
  );
}

function usedPercent(disk: DiskInfo | undefined): number {
  if (!disk || disk.totalBytes === 0) return 0;
  return 100 - (disk.availableBytes / disk.totalBytes) * 100;
}

function Spark({ data }: { data: number[] }) {
  const series = data.length > 1 ? data : [0, 0];
  const max = Math.max(...series, 1);
  const min = Math.min(...series, 0);
  const span = max - min || 1;
  const points = series
    .map(
      (value, index) =>
        `${((index * 72) / (series.length - 1)).toFixed(1)},${(
          24 - ((value - min) / span) * 19
        ).toFixed(1)}`,
    )
    .join(" ");

  return (
    <svg className="spark" viewBox="0 0 72 26" preserveAspectRatio="none" aria-hidden="true">
      <polyline points={points} />
    </svg>
  );
}

interface MetricRowProps {
  icon: LucideIcon;
  label: string;
  value: string;
  data: number[];
}

function MetricRow({ icon: Icon, label, value, data }: MetricRowProps) {
  return (
    <div className="metric">
      <span className="metric-icon">
        <Icon size={15} strokeWidth={1.8} />
      </span>
      <span className="metric-name">{label}</span>
      <span className="metric-value">{value}</span>
      <Spark data={data} />
    </div>
  );
}

function seriesFrom(samples: LiveMetrics[], pick: (sample: LiveMetrics) => number): number[] {
  const slice = samples.slice(-24);
  return slice.length > 1 ? slice.map(pick) : [0, 0];
}

function QuickAction({
  icon: Icon,
  title,
  description,
  onClick,
}: {
  icon: LucideIcon;
  title: string;
  description: string;
  onClick: () => void;
}) {
  return (
    <button type="button" className="quick" onClick={onClick}>
      <span className="quick-top">
        <span className="quick-icon">
          <Icon size={19} strokeWidth={1.8} />
        </span>
        <span className="quick-arrow">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <path d="M5 12h14M13 6l6 6-6 6" />
          </svg>
        </span>
      </span>
      <span>
        <b>{title}</b>
        <small>{description}</small>
      </span>
    </button>
  );
}

function GamingBanner() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <div
      className="banner"
      style={{
        flexDirection: "row",
        alignItems: "center",
        justifyContent: "space-between",
        flexWrap: "wrap",
        gap: 20,
        padding: "18px 22px",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 16, minWidth: 0 }}>
        <span className="banner-badge" style={{ margin: 0, flex: "none" }}>
          <Zap size={18} strokeWidth={2} aria-hidden />
        </span>
        <div style={{ minWidth: 0 }}>
          <h3>{t("dashboard.gamingMode")}</h3>
          <p style={{ marginTop: 4 }}>{t("dashboard.gamingModeDesc")}</p>
        </div>
      </div>
      <Button variant="ghost" size="sm" onClick={() => navigate("/optimizations")}>
        {t("dashboard.activateGaming")}
      </Button>
    </div>
  );
}

export function Dashboard() {
  const { t } = useTranslation();
  const { data: report, isLoading, isError, refetch } = useDiagnostics(5000);
  const { samples, latest } = useLiveMetrics(120);
  const navigate = useNavigate();

  if (isLoading) {
    return (
      <div className="view">
        <div className="grid cols-3">
          <div className="panel" style={{ height: 260, opacity: 0.5 }} />
          <div className="panel" style={{ height: 260, opacity: 0.35 }} />
          <div className="panel" style={{ height: 260, opacity: 0.25 }} />
        </div>
        <div className="grid cols-4">
          <div className="panel" style={{ height: 120, opacity: 0.3 }} />
          <div className="panel" style={{ height: 120, opacity: 0.25 }} />
          <div className="panel" style={{ height: 120, opacity: 0.2 }} />
          <div className="panel" style={{ height: 120, opacity: 0.15 }} />
        </div>
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
          <Button variant="ghost" size="sm" onClick={() => void refetch()}>
            {t("common.retry")}
          </Button>
        }
      />
    );
  }

  const { profile, health, bottlenecks } = report;
  const label = healthLabel(health.global);
  const diskUsed = usedPercent(systemDisk(profile));

  const cpuPercent = latest?.cpuPercent ?? profile.cpu.usagePercent;
  const memoryPercent = latest
    ? latest.memoryPercent
    : profile.memory.totalBytes
      ? (profile.memory.usedBytes / profile.memory.totalBytes) * 100
      : 0;

  return (
    <section className="view" id="inicio">
      <div className="panel hero">
        <div className="hero-copy">
          <span className="eyebrow">
            <span className="pulse-dot" />
            {t("dashboard.live")}
          </span>
          <h2>
            {health.global >= 75 ? t("dashboard.maxPerformance") : t("dashboard.systemReady")}
          </h2>
          <p>
            {bottlenecks.length > 0
              ? t("dashboard.healthSummary", { count: bottlenecks.length })
              : t("dashboard.allGood")}
            {" · "}
            {t("dashboard.systemReadyDesc")}
          </p>

          <div className="hero-actions">
            <Button variant="primary" size="lg" onClick={() => navigate("/optimizations")}>
              <Zap size={16} aria-hidden />
              {t("dashboard.optimizeNow")}
            </Button>
            <Button variant="ghost" size="lg" onClick={() => navigate("/diagnostics")}>
              {t("dashboard.viewAnalysis")}
            </Button>
          </div>

          <div className="hero-foot">
            <span>
              <Cpu size={13} aria-hidden />
              {profile.cpu.brand || "CPU"}
            </span>
            <span>
              <MemoryStick size={13} aria-hidden />
              {formatBytes(profile.memory.totalBytes)}
            </span>
            <span>
              <HardDrive size={13} aria-hidden />
              {t("dashboard.uptime")} {formatDuration(profile.uptimeSeconds)}
            </span>
          </div>
        </div>

        <div className="hero-score">
          <ScoreRing value={health.global} label={t("dashboard.health")} size={208} />
          <span className={`chip chip-${scoreTone(health.global)}`}>
            <span className="dot" />
            {t(`dashboard.${label}`)}
          </span>
        </div>

        <div className="hero-metrics">
          <div className="metric-list">
            <MetricRow
              icon={Cpu}
              label={t("dashboard.cpu")}
              value={formatPercent(cpuPercent, 0)}
              data={seriesFrom(samples, (sample) => sample.cpuPercent)}
            />
            <MetricRow
              icon={MemoryStick}
              label={t("dashboard.memory")}
              value={formatPercent(memoryPercent, 0)}
              data={seriesFrom(samples, (sample) => sample.memoryPercent)}
            />
            <MetricRow
              icon={HardDrive}
              label={t("dashboard.storage")}
              value={formatPercent(latest?.diskBusyPercent ?? diskUsed, 0)}
              data={seriesFrom(samples, (sample) => sample.diskBusyPercent ?? 0)}
            />
            <MetricRow
              icon={Network}
              label={t("dashboard.network")}
              value={formatRate(latest?.networkRxBytesPerSec ?? 0)}
              data={seriesFrom(samples, (sample) => sample.networkRxBytesPerSec)}
            />
          </div>
        </div>
      </div>

      <div className="grid cols-4">
        <QuickAction
          icon={ScanSearch}
          title={t("dashboard.fullDiagnostic")}
          description={t("dashboard.fullDiagnosticDesc")}
          onClick={() => navigate("/diagnostics")}
        />
        <QuickAction
          icon={Sparkles}
          title={t("dashboard.smartOptimization")}
          description={t("dashboard.smartOptimizationDesc")}
          onClick={() => navigate("/optimizations")}
        />
        <QuickAction
          icon={Trash2}
          title={t("dashboard.cleanFiles")}
          description={t("dashboard.cleanFilesDesc")}
          onClick={() => navigate("/tools")}
        />
        <QuickAction
          icon={Activity}
          title={t("dashboard.openMonitor")}
          description={t("dashboard.openMonitorHint")}
          onClick={() => navigate("/monitor")}
        />
      </div>

      <GamingBanner />
    </section>
  );
}
