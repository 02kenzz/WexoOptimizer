import {
  CheckCircle2,
  Database,
  Download,
  HardDrive,
  Loader2,
  MonitorUp,
  RefreshCw,
  ShieldCheck,
  Square,
  Trash2,
  Wrench,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { useToast } from "@/hooks/useToast";
import {
  type CleanReport,
  type GpuDriverInfo,
  type GpuDriverStatus,
  ipc,
  isDesktop,
  type JobSnapshot,
  type JobStatus,
} from "@/lib/ipc";
import { formatBytes } from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";

type CleanKind = "temp" | "logs" | "cache";
type JobTool = "sfc" | "dism" | "defrag" | "trim" | "check-disk";

interface ToolCardProps {
  icon: typeof Trash2;
  title: string;
  description: string;
  actions: Array<{ id: string; label: string }>;
  busy: string | null;
  disabled: boolean;
  onAction: (id: string) => void;
}

const STATUS_TONE: Record<JobStatus, "accent" | "ok" | "danger" | "warn"> = {
  running: "accent",
  succeeded: "ok",
  failed: "danger",
  cancelled: "warn",
};

const DRIVER_TONE: Record<GpuDriverStatus, "ok" | "warn" | "muted"> = {
  upToDate: "ok",
  outdated: "warn",
  unknown: "muted",
};

function formatElapsed(startedMs: number, finishedMs: number | null, nowMs: number): string {
  const totalSeconds = Math.max(0, Math.floor(((finishedMs ?? nowMs) - startedMs) / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

/** Formats an ISO `YYYY-MM-DD` date in the active UI language. */
function formatDriverDate(value: string | null, locale: string): string | null {
  if (!value) return null;
  const date = new Date(`${value}T00:00:00`);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleDateString(locale);
}

function ToolCard({
  icon: Icon,
  title,
  description,
  actions,
  busy,
  disabled,
  onAction,
}: ToolCardProps) {
  return (
    <div className="panel tool-card">
      <div className="tool-head">
        <span className="tool-icon">
          <Icon size={18} strokeWidth={1.8} aria-hidden />
        </span>
        <div>
          <h3>{title}</h3>
          <p>{description}</p>
        </div>
      </div>
      <div className="tool-actions">
        {actions.map((action) => (
          <Button
            key={action.id}
            variant="glass"
            size="sm"
            disabled={disabled || busy !== null}
            onClick={() => onAction(action.id)}
          >
            {busy === action.id ? <Loader2 size={13} className="animate-spin" aria-hidden /> : null}
            {action.label}
          </Button>
        ))}
      </div>
    </div>
  );
}

export function Tools() {
  const { t, i18n } = useTranslation();
  const { showToast } = useToast();
  const appInfo = useAppStore((state) => state.appInfo);

  const [busy, setBusy] = useState<string | null>(null);
  const [preview, setPreview] = useState<{
    kind: CleanKind;
    label: string;
    report: CleanReport;
  } | null>(null);
  const [job, setJob] = useState<JobSnapshot | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [drivers, setDrivers] = useState<GpuDriverInfo[] | null>(null);
  const [driversLoading, setDriversLoading] = useState(false);
  const [driversError, setDriversError] = useState<string | null>(null);

  const jobId = job?.id ?? null;
  const jobStatus = job?.status ?? null;
  const logRef = useRef<HTMLPreElement>(null);
  const previousStatus = useRef<JobStatus | null>(null);

  const toastError = useCallback(
    (title: string, cause: unknown) => {
      showToast({
        title,
        description: cause instanceof Error ? cause.message : String(cause),
        tone: "danger",
      });
    },
    [showToast],
  );

  // Restore the console after navigating away and back while a job runs.
  useEffect(() => {
    if (!isDesktop) return;
    let cancelled = false;

    void ipc
      .getToolJobs()
      .then((jobs) => {
        if (cancelled || jobs.length === 0) return;
        const active = jobs.find((entry) => entry.status === "running");
        setJob(active ?? jobs[jobs.length - 1]);
      })
      .catch(() => {
        // No jobs yet, or the backend went away: nothing to restore.
      });

    return () => {
      cancelled = true;
    };
  }, []);

  // Poll the running job instead of streaming events: one request every
  // 800 ms is cheap and survives the window being hidden.
  useEffect(() => {
    if (!jobId || jobStatus !== "running") return;
    let cancelled = false;

    const timer = window.setInterval(() => {
      void ipc
        .getToolJob(jobId)
        .then((next) => {
          if (!cancelled && next) setJob(next);
        })
        .catch(() => {
          // Keep the last known snapshot; the next tick retries.
        });
    }, 800);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [jobId, jobStatus]);

  // Ticks the elapsed clock while a job runs.
  useEffect(() => {
    if (jobStatus !== "running") return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [jobStatus]);

  // Reports the real outcome once, when a job transitions out of `running`.
  useEffect(() => {
    if (!job) return;
    if (job.status === previousStatus.current) return;

    const previous = previousStatus.current;
    previousStatus.current = job.status;
    if (previous !== "running") return;

    if (job.status === "succeeded") {
      showToast({ title: t("tools.jobSucceeded"), description: job.label, tone: "ok" });
    } else if (job.status === "cancelled") {
      showToast({ title: t("tools.jobCancelled"), description: job.label, tone: "warn" });
    } else {
      showToast({
        title: t("tools.jobError"),
        description:
          job.exitCode !== null
            ? `${job.label} · ${t("tools.exitCode")} ${job.exitCode}`
            : job.label,
        tone: "danger",
      });
    }
  }, [job, showToast, t]);

  // Keeps the console pinned to the newest line.
  // biome-ignore lint/correctness/useExhaustiveDependencies: re-scroll whenever a new line arrives
  useEffect(() => {
    const element = logRef.current;
    if (element) element.scrollTop = element.scrollHeight;
  }, [job?.lines.length]);

  const requireDesktop = () => {
    if (isDesktop) return true;
    showToast({
      title: t("tools.browserMode"),
      description: t("tools.browserModeDesc"),
      tone: "warn",
    });
    return false;
  };

  const scanCleanup = async (kind: CleanKind, label: string) => {
    if (!requireDesktop()) return;
    setBusy(`clean-${kind}`);
    try {
      const report = await ipc.scanCleanup(kind);
      if (report.files === 0) {
        setPreview(null);
        showToast({ title: t("tools.cleanNothing"), tone: "accent" });
        return;
      }
      setPreview({ kind, label, report });
    } catch (cause) {
      toastError(t("tools.scanFailed"), cause);
    } finally {
      setBusy(null);
    }
  };

  const confirmCleanup = async () => {
    if (!preview) return;
    setBusy("clean");
    try {
      const report = await ipc.runCleanup(preview.kind);
      setPreview(null);
      showToast({
        title: t("tools.cleanDone"),
        description: t("tools.cleanResult", {
          size: formatBytes(report.bytes),
          files: report.files,
          skipped: report.skipped,
        }),
        tone: report.skipped > 0 ? "warn" : "ok",
      });
    } catch (cause) {
      toastError(t("tools.cleanFailed"), cause);
    } finally {
      setBusy(null);
    }
  };

  const maintainDatabase = async (action: "vacuum" | "reindex") => {
    if (!requireDesktop()) return;
    setBusy(`db-${action}`);
    try {
      const report = await ipc.maintainDatabase(action);
      showToast({
        title: action === "vacuum" ? t("tools.dbVacuumDone") : t("tools.dbReindexDone"),
        description: t("tools.dbResult", {
          before: formatBytes(report.beforeBytes),
          after: formatBytes(report.afterBytes),
          count: report.transactions,
        }),
        tone: "ok",
      });
    } catch (cause) {
      toastError(t("tools.dbFailed"), cause);
    } finally {
      setBusy(null);
    }
  };

  const startJob = async (tool: JobTool) => {
    if (!requireDesktop()) return;
    setBusy(`job-${tool}`);
    try {
      const snapshot = await ipc.startToolJob(tool);
      previousStatus.current = snapshot.status;
      setJob(snapshot);
      setNow(Date.now());
    } catch (cause) {
      toastError(t("tools.jobFailed"), cause);
    } finally {
      setBusy(null);
    }
  };

  const stopJob = async () => {
    if (!job) return;
    try {
      const stopped = await ipc.cancelToolJob(job.id);
      if (!stopped) {
        showToast({ title: t("tools.jobNotRunning"), tone: "warn" });
      }
    } catch (cause) {
      toastError(t("tools.jobFailed"), cause);
    }
  };

  const refreshDrivers = useCallback(async () => {
    if (!isDesktop) return;
    setDriversLoading(true);
    setDriversError(null);
    try {
      setDrivers(await ipc.checkGpuDrivers());
    } catch (cause) {
      setDriversError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setDriversLoading(false);
    }
  }, []);

  // Checks once when the page opens. The backend caches the vendor lookup for
  // a few hours, so revisiting the page does not hammer the server.
  useEffect(() => {
    void refreshDrivers();
  }, [refreshDrivers]);

  const openDriverUpdate = async (driver: GpuDriverInfo) => {
    if (!requireDesktop()) return;
    try {
      const target = driver.detailsUrl ?? driver.updateUrl;
      const opened = await ipc.openGpuDriverUpdate(driver.vendor, target);
      showToast({
        title:
          opened === "browser"
            ? t("tools.driversOpenedBrowser")
            : t("tools.driversOpened", { name: opened }),
        tone: "ok",
      });
    } catch (cause) {
      toastError(t("tools.driversOpenFailed"), cause);
    }
  };

  const cards: ToolCardProps[] = [
    {
      icon: Trash2,
      title: t("tools.cleanup"),
      description: t("tools.cleanupDesc"),
      actions: [
        { id: "clean-temp", label: t("tools.cleanTemp") },
        { id: "clean-logs", label: t("tools.cleanLogs") },
        { id: "clean-cache", label: t("tools.cleanCache") },
      ],
      busy,
      disabled: preview !== null,
      onAction: (id) => {
        if (id === "clean-temp") void scanCleanup("temp", t("tools.cleanTemp"));
        if (id === "clean-logs") void scanCleanup("logs", t("tools.cleanLogs"));
        if (id === "clean-cache") void scanCleanup("cache", t("tools.cleanCache"));
      },
    },
    {
      icon: ShieldCheck,
      title: t("tools.repair"),
      description: t("tools.repairDesc"),
      actions: [
        { id: "job-sfc", label: t("tools.repairSystem") },
        { id: "job-dism", label: t("tools.repairImage") },
      ],
      busy,
      disabled: job?.status === "running",
      onAction: (id) => {
        if (id === "job-sfc") void startJob("sfc");
        if (id === "job-dism") void startJob("dism");
      },
    },
    {
      icon: Database,
      title: t("tools.database"),
      description: t("tools.databaseDesc"),
      actions: [
        { id: "db-vacuum", label: t("tools.vacuumDb") },
        { id: "db-reindex", label: t("tools.reindexDb") },
      ],
      busy,
      disabled: false,
      onAction: (id) => {
        if (id === "db-vacuum") void maintainDatabase("vacuum");
        if (id === "db-reindex") void maintainDatabase("reindex");
      },
    },
    {
      icon: HardDrive,
      title: t("tools.disk"),
      description: t("tools.diskDesc"),
      actions: [
        { id: "job-defrag", label: t("tools.defrag") },
        { id: "job-trim", label: t("tools.trim") },
        { id: "job-check-disk", label: t("tools.checkDisk") },
      ],
      busy,
      disabled: job?.status === "running",
      onAction: (id) => {
        if (id === "job-defrag") void startJob("defrag");
        if (id === "job-trim") void startJob("trim");
        if (id === "job-check-disk") void startJob("check-disk");
      },
    },
  ];

  const elapsed = job ? formatElapsed(job.startedMs, job.finishedMs, now) : "";
  const running = job?.status === "running";
  const outdatedCount = drivers?.filter((driver) => driver.status === "outdated").length ?? 0;

  return (
    <section className="view" id="herramientas">
      <div className="grid tools-grid">
        {cards.map((card) => (
          <ToolCard key={card.title} {...card} />
        ))}
      </div>

      <Card>
        <div className="panel-head">
          <h3 className="panel-title">
            <MonitorUp size={14} strokeWidth={2} aria-hidden />
            {t("tools.drivers")}
          </h3>
          <div className="tool-console-meta">
            {drivers && drivers.length > 0 ? (
              <Badge tone={outdatedCount > 0 ? "warn" : "ok"} dot>
                {outdatedCount > 0
                  ? t("tools.driversOutdatedCount", { count: outdatedCount })
                  : t("tools.driversAllCurrent")}
              </Badge>
            ) : null}
            <Button
              variant="ghost"
              size="sm"
              disabled={driversLoading || !isDesktop}
              onClick={() => void refreshDrivers()}
            >
              <RefreshCw
                size={13}
                className={driversLoading ? "animate-spin" : undefined}
                aria-hidden
              />
              {t("tools.driversRefresh")}
            </Button>
          </div>
        </div>

        {!isDesktop ? (
          <p className="tool-preview">{t("tools.browserModeDesc")}</p>
        ) : driversError ? (
          <p className="tool-preview">
            {t("tools.driversFailed")}: {driversError}
          </p>
        ) : drivers === null || (driversLoading && drivers.length === 0) ? (
          <p className="tool-preview driver-loading">
            <Loader2 size={13} className="animate-spin" aria-hidden />
            {t("tools.driversChecking")}
          </p>
        ) : drivers.length === 0 ? (
          <p className="tool-preview">{t("tools.driversEmpty")}</p>
        ) : (
          <>
            <div className="driver-list">
              {drivers.map((driver) => {
                const released = formatDriverDate(driver.releaseDate, i18n.language);
                const outdated = driver.status === "outdated";
                return (
                  <div className="driver-row" key={`${driver.name}-${driver.vendor}`}>
                    <div className="driver-main">
                      <div className="driver-title">
                        <strong>{driver.name}</strong>
                        <Badge tone={DRIVER_TONE[driver.status]} dot>
                          {t(`tools.driverStatus.${driver.status}`)}
                        </Badge>
                      </div>
                      <div className="driver-meta">
                        <span>
                          {t("tools.driversInstalled")}: <b>{driver.installedVersion || "—"}</b>
                        </span>
                        <span>
                          {t("tools.driversLatest")}: <b>{driver.latestVersion ?? "—"}</b>
                        </span>
                        {released ? (
                          <span>
                            {t("tools.driversReleased")}: <b>{released}</b>
                          </span>
                        ) : null}
                      </div>
                    </div>
                    <Button
                      variant={outdated ? "primary" : "glass"}
                      size="sm"
                      onClick={() => void openDriverUpdate(driver)}
                    >
                      <Download size={13} aria-hidden />
                      {driver.softwareInstalled
                        ? t("tools.driversOpenSoftware", {
                            name: driver.softwareName ?? driver.vendorLabel,
                          })
                        : t("tools.driversDownload")}
                    </Button>
                  </div>
                );
              })}
            </div>
            {drivers.some((driver) => driver.status === "unknown") ? (
              <p className="driver-note">{t("tools.driversUnknownNote")}</p>
            ) : null}
          </>
        )}
      </Card>

      {preview ? (
        <Card>
          <div className="panel-head">
            <h3 className="panel-title">
              <Trash2 size={14} strokeWidth={2} aria-hidden />
              {t("tools.previewTitle", { kind: preview.label })}
            </h3>
            <Badge tone="accent">{t("tools.previewBadge")}</Badge>
          </div>

          <p className="tool-preview">
            {t("tools.previewBody", {
              size: formatBytes(preview.report.bytes),
              files: preview.report.files,
            })}
          </p>

          <div className="tool-actions">
            <Button
              variant="primary"
              size="sm"
              disabled={busy !== null}
              onClick={() => void confirmCleanup()}
            >
              {busy === "clean" ? (
                <Loader2 size={13} className="animate-spin" aria-hidden />
              ) : (
                <Trash2 size={13} aria-hidden />
              )}
              {t("tools.cleanConfirm")}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={busy !== null}
              onClick={() => setPreview(null)}
            >
              {t("common.cancel")}
            </Button>
          </div>
        </Card>
      ) : null}

      {job ? (
        <Card>
          <div className="panel-head">
            <h3 className="panel-title">
              {job.status === "succeeded" ? (
                <CheckCircle2 size={14} strokeWidth={2} aria-hidden />
              ) : (
                <Wrench size={14} strokeWidth={2} aria-hidden />
              )}
              {t("tools.console")} · {job.label}
            </h3>
            <div className="tool-console-meta">
              <Badge tone={STATUS_TONE[job.status]} dot>
                {t(`tools.status.${job.status}`)}
              </Badge>
              <span className="tool-meta-item">
                {t("tools.elapsed")} {elapsed}
              </span>
              {job.exitCode !== null ? (
                <span className="tool-meta-item">
                  {t("tools.exitCode")} {job.exitCode}
                </span>
              ) : null}
              {running ? (
                <Button variant="danger" size="sm" onClick={() => void stopJob()}>
                  <Square size={11} aria-hidden />
                  {t("tools.stop")}
                </Button>
              ) : null}
            </div>
          </div>

          {job.droppedLines > 0 ? (
            <p className="tool-console-note">
              {t("tools.droppedLines", { count: job.droppedLines })}
            </p>
          ) : null}

          <pre className="tool-log" ref={logRef}>
            {job.lines.length > 0 ? job.lines.join("\n") : t("tools.waitingOutput")}
          </pre>
        </Card>
      ) : null}

      <Card>
        <div className="panel-head">
          <h3 className="panel-title">
            <Wrench size={14} strokeWidth={2} aria-hidden />
            {t("tools.systemInfo")}
          </h3>
          <Badge tone="accent">{appInfo?.isAdmin ? t("app.admin") : t("app.standard")}</Badge>
        </div>

        <div className="grid tool-stats">
          <div className="stat-tile">
            <span>{t("tools.windowsVersion")}</span>
            <b>{appInfo?.os ?? (isDesktop ? "—" : t("tools.browserValue"))}</b>
          </div>
          <div className="stat-tile">
            <span>{t("tools.architecture")}</span>
            <b>{appInfo?.arch ?? (isDesktop ? "—" : "WASM")}</b>
          </div>
          <div className="stat-tile">
            <span>{t("tools.appVersion")}</span>
            <b>{appInfo?.version ?? "—"}</b>
          </div>
          <div className="stat-tile">
            <span>{t("tools.runtime")}</span>
            <b>{isDesktop ? "Tauri 2" : "Web"}</b>
          </div>
        </div>
      </Card>
    </section>
  );
}
