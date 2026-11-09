import { useTranslation } from "react-i18next";
import { Link, useNavigate } from "react-router-dom";
import { Activity, Database, Sparkles } from "lucide-react";

import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";
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
import { ipc } from "@/lib/ipc";
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

function Sparkline({ data, color = "#cfcfd6" }: { data: number[]; color?: string }) {
  const points = data.map((v, i) => `${(i * 90) / 19},${22 - v * 0.2}`).join(" ");
  return (
    <svg viewBox="0 0 90 24" preserveAspectRatio="none">
      <polyline fill="none" stroke={color} strokeWidth={1.6} points={points} />
    </svg>
  );
}

interface MetricRowProps {
  label: string;
  key: string;
  unit: string;
  sparkColor?: string;
}

function MetricRow({ label, key, unit, sparkColor }: MetricRowProps) {
  const { latest } = useLiveMetrics(120);
  const history = latest ? Array(20).fill(latest[key as keyof typeof latest] as number) : Array(20).fill(0);

  return (
    <div className="m">
      <b>{label}</b>
      <em data-k={key} data-u={unit}>—</em>
      <Sparkline data={history} color={sparkColor} />
    </div>
  );
}

function TemperatureRow({ label, key, unit }: { label: string; key: string; unit: string }) {
  const { latest } = useLiveMetrics(120);

  return (
    <div className="m t">
      <span>{label}</span>
      <em data-k={key} data-u={unit}>—</em>
    </div>
  );
}

function ActionCard({ icon: Icon, action, title, description }: {
  icon: React.ComponentType<{ className?: string }>;
  action: string;
  title: string;
  description: string;
}) {
  const navigate = useNavigate();

  const handleClick = () => {
    if (action === "diagnose") navigate("/diagnostics");
    else if (action === "smart-optimize") navigate("/optimizations");
    else if (action === "clean") navigate("/tools");
    else if (action === "gaming") navigate("/optimizations");
  };

  return (
    <button className="card act" data-action={action} onClick={handleClick}>
      <span>
        <Icon className="ic" aria-hidden />
        <strong>{title}</strong>
        <small>{description}</small>
      </span>
      <span className="go"><svg className="ic"><use href="#arrow" /></svg></span>
    </button>
  );
}

function ActivityLog() {
  const { t } = useTranslation();

  return (
    <Card>
      <h3>{t("dashboard.recentActivity")}</h3>
      <ul className="log">
        <li>
          <i><svg className="ic"><use href="#check" /></svg></i>
          <span>{t("dashboard.activityDiagnosticCompleted")}</span>
          <time>Hace 2 min</time>
        </li>
        <li>
          <i><svg className="ic"><use href="#check" /></svg></i>
          <span>{t("dashboard.activityTempCleaned")}</span>
          <time>Hace 5 min</time>
        </li>
        <li>
          <i><svg className="ic"><use href="#check" /></svg></i>
          <span>{t("dashboard.activityStartupOptimized")}</span>
          <time>Hace 10 min</time>
        </li>
      </ul>
    </Card>
  );
}

function GamingModeCard() {
  const { t } = useTranslation();

  return (
    <Card className="game">
      <div>
        <h3>{t("dashboard.gamingMode")}</h3>
        <p>{t("dashboard.gamingModeDesc")}</p>
      </div>
      <Button className="btn w" data-action="gaming" onClick={() => navigate("/optimizations")}>
        <svg className="ic"><use href="#bolt" /></svg>
        {t("dashboard.activateGaming")}
      </Button>
    </Card>
  );
}

function HardwareCard({ report }: { report: DiagnosticReport }) {
  const { t } = useTranslation();
  const { profile } = report;
  const mainDisk = systemDisk(profile);
  const memoryUsed = profile.memory.totalBytes
    ? (profile.memory.usedBytes / profile.memory.totalBytes) * 100
    : 0;
  const diskUsed = usedPercent(mainDisk);

  return (
    <Card>
      <h3>{t("dashboard.hardwareStatus")}</h3>
      <div className="m">
        <b>{t("dashboard.cpu")}</b>
        <em data-k="cpu" data-u="%">—</em>
        <svg viewBox="0 0 90 24" preserveAspectRatio="none"><polyline data-spark="cpu" /></svg>
      </div>
      <div className="m">
        <b>{t("dashboard.memory")}</b>
        <em data-k="mem" data-u="%">—</em>
        <svg viewBox="0 0 90 24" preserveAspectRatio="none"><polyline data-spark="mem" /></svg>
      </div>
      <div className="m">
        <b>{t("dashboard.gpu")}</b>
        <em data-k="gpu" data-u="%">—</em>
        <svg viewBox="0 0 90 24" preserveAspectRatio="none"><polyline data-spark="gpu" /></svg>
      </div>
      <div className="m">
        <b>{t("dashboard.disk")}</b>
        <em data-k="dsk" data-u="%">—</em>
        <svg viewBox="0 0 90 24" preserveAspectRatio="none"><polyline data-spark="dsk" /></svg>
      </div>
      <div className="m t">
        <span>{t("dashboard.cpuTemp")}</span>
        <em data-k="tc" data-u=" °C">—</em>
      </div>
      <div className="m t">
        <span>{t("dashboard.gpuTemp")}</span>
        <em data-k="tg" data-u=" °C">—</em>
      </div>
    </Card>
  );
}

function Skeleton() {
  return (
    <div className="space-y-6 animate-pulse">
      <div className="grid gap-6 lg:grid-cols-[1.5fr_1fr_1.1fr]">
        <div className="min-h-[300px] rounded-[22px] border border-line bg-card" />
        <div className="min-h-[300px] rounded-[22px] border border-line bg-card" />
        <div className="min-h-[300px] rounded-[22px] border border-line bg-card" />
      </div>
      <div className="grid gap-18 lg:grid-cols-3">
        <div className="min-h-[180px] rounded-[22px] border border-line bg-card" />
        <div className="min-h-[180px] rounded-[22px] border border-line bg-card" />
        <div className="min-h-[180px] rounded-[22px] border border-line bg-card" />
      </div>
      <div className="grid gap-18 lg:grid-cols-[1.6fr_1fr]">
        <div className="min-h-[200px] rounded-[22px] border border-line bg-card" />
        <div className="min-h-[200px] rounded-[22px] border border-line bg-card" />
      </div>
    </div>
  );
}

export function Dashboard() {
  const { t } = useTranslation();
  const { data: report, isLoading, isError, refetch } = useDiagnostics(5000);
  const { latest } = useLiveMetrics(120);
  const navigate = useNavigate();

  if (isLoading) return <Skeleton />;

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

  const cpuPercent = latest?.cpuPercent ?? profile.cpu.usagePercent;
  const memoryPercent = latest
    ? latest.memoryPercent
    : profile.memory.totalBytes
      ? (profile.memory.usedBytes / profile.memory.totalBytes) * 100
      : 0;
  const diskUsed = usedPercent(systemDisk(profile));

  const subsystems = [
    { label: t("dashboard.cpu"), score: health.cpu, tone: "accent" as const },
    { label: t("dashboard.memory"), score: health.memory, tone: "accent-2" as const },
    { label: t("dashboard.storage"), score: health.disk, tone: "accent-3" as const },
    { label: t("dashboard.network"), score: health.network, tone: "ok" as const },
  ];

  return (
    <section className="view on" id="inicio">
      <div className="top">
        <label className="search">
          <svg className="ic"><use href="#search" /></svg>
          <input placeholder={t("dashboard.searchPlaceholder")} />
        </label>
        <div className="avatar" />
      </div>

      <div className="hero">
        <div>
          <small>{t("dashboard.systemReady")}</small>
          <h1>{t("dashboard.maxPerformance")}</h1>
          <p>{t("dashboard.systemReadyDesc")}</p>
          <div className="acts">
            <Button className="btn w" data-action="optimize" onClick={() => navigate("/optimizations")}>
              <svg className="ic"><use href="#bolt" /></svg>
              {t("dashboard.optimizeNow")}
            </Button>
            <Button className="btn g" data-action="analyze" onClick={() => navigate("/diagnostics")}>
              {t("dashboard.viewAnalysis")}
            </Button>
          </div>
        </div>

        <div className="ring">
          <svg viewBox="0 0 100 100">
            <circle className="t" cx="50" cy="50" r="44" pathLength="100" />
            <circle className="p" cx="50" cy="50" r="44" pathLength="100" strokeDasharray={`${health.global * 0.98} 100`} />
          </svg>
          <div>
            <strong>{health.global}%</strong>
            <span>{t("dashboard.systemHealth")}</span>
          </div>
        </div>

        <HardwareCard report={report} />
      </div>

      <div className="row3">
        <ActionCard icon={Activity} action="diagnose" title={t("dashboard.fullDiagnostic")} description={t("dashboard.fullDiagnosticDesc")} />
        <ActionCard icon={Sparkles} action="smart-optimize" title={t("dashboard.smartOptimization")} description={t("dashboard.smartOptimizationDesc")} />
        <ActionCard icon={Database} action="clean" title={t("dashboard.cleanFiles")} description={t("dashboard.cleanFilesDesc")} />
      </div>

      <div className="bottom">
        <ActivityLog />
        <GamingModeCard />
      </div>
    </section>
  );
}