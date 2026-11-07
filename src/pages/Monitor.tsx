import { Activity, Cpu, HardDrive, LineChart, MemoryStick, Timer } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/Badge";
import { Card, CardHeader, CardTitle } from "@/components/ui/Card";
import { Meter } from "@/components/ui/Meter";
import { useSystemProfile } from "@/hooks/useSystemProfile";
import { usageTone } from "@/lib/tone";
import { formatBytes, formatPercent } from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";

function LiveTile({
  icon: Icon,
  label,
  value,
  percent,
}: {
  icon: typeof Cpu;
  label: string;
  value: string;
  percent?: number;
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
      {percent !== undefined ? (
        <Meter value={percent} tone={usageTone(percent)} className="mt-3" />
      ) : null}
    </Card>
  );
}

export function Monitor() {
  const { t } = useTranslation();
  const interval = useAppStore((state) => state.settings.monitorIntervalMs);
  const { data: profile } = useSystemProfile(Math.max(1500, interval));

  const memoryPercent = profile?.memory.totalBytes
    ? (profile.memory.usedBytes / profile.memory.totalBytes) * 100
    : 0;
  const mainDisk = profile?.disks.find((disk) => disk.mountPoint.toUpperCase().startsWith("C:"));
  const diskUsed = mainDisk?.totalBytes
    ? 100 - (mainDisk.availableBytes / mainDisk.totalBytes) * 100
    : 0;

  return (
    <div className="space-y-4">
      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <LiveTile
          icon={Cpu}
          label={t("monitor.liveCpu")}
          value={formatPercent(profile?.cpu.usagePercent ?? 0, 1)}
          percent={profile?.cpu.usagePercent ?? 0}
        />
        <LiveTile
          icon={MemoryStick}
          label={t("monitor.liveMemory")}
          value={formatPercent(memoryPercent, 0)}
          percent={memoryPercent}
        />
        <LiveTile
          icon={HardDrive}
          label={t("monitor.liveDisk")}
          value={formatBytes(mainDisk?.availableBytes ?? 0)}
          percent={diskUsed}
        />
        <LiveTile icon={Timer} label={t("monitor.sampling")} value={`${interval} ms`} />
      </div>

      <Card className="relative overflow-hidden">
        <div className="grid-backdrop absolute inset-0 opacity-30" aria-hidden />
        <div className="relative flex flex-col items-center justify-center px-6 py-24 text-center">
          <div className="grid size-14 place-items-center rounded-2xl border border-accent/30 bg-accent-soft text-accent">
            <LineChart className="size-6" strokeWidth={1.8} aria-hidden />
          </div>
          <h2 className="mt-4 text-sm font-semibold">{t("monitor.placeholderTitle")}</h2>
          <p className="mt-1.5 max-w-md text-xs leading-relaxed text-muted">
            {t("monitor.placeholderHint")}
          </p>
          <div className="mt-4 flex flex-wrap items-center justify-center gap-2">
            <Badge tone="accent">{t("common.milestone", { milestone: "M3" })}</Badge>
            <Badge tone="muted">{t("monitor.targets")}</Badge>
          </div>
        </div>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="size-4 text-accent" aria-hidden />
            {t("monitor.subtitle")}
          </CardTitle>
          <Badge tone="muted">uPlot · 60 fps</Badge>
        </CardHeader>
        <div className="grid grid-cols-12 items-end gap-1 px-5 pb-4" aria-hidden>
          {Array.from({ length: 48 }).map((_, index) => {
            const height = 12 + Math.round(28 * Math.abs(Math.sin(index / 3.1)));
            return (
              <div key={index.toString()} className="rounded-sm bg-accent/25" style={{ height }} />
            );
          })}
        </div>
      </Card>
    </div>
  );
}
