import {
  Contrast,
  FolderOpen,
  Gauge as GaugeIcon,
  Info,
  Monitor as MonitorIcon,
  Moon,
  RotateCcw,
  Settings2,
  ShieldAlert,
  ShieldCheck,
  Sun,
} from "lucide-react";
import type { ReactNode } from "react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { OverlayConfigDialog } from "@/components/ui/OverlayConfigDialog";
import { Switch, Tooltip } from "@/components/ui/Switch";
import { ipc, isDesktop } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";
import type { Language, Theme } from "@/types";

const THEMES: { value: Theme; icon: typeof Sun; labelKey: string }[] = [
  { value: "system", icon: MonitorIcon, labelKey: "settings.themeSystem" },
  { value: "light", icon: Sun, labelKey: "settings.themeLight" },
  { value: "dark", icon: Moon, labelKey: "settings.themeDark" },
  { value: "high-contrast", icon: Contrast, labelKey: "settings.themeHighContrast" },
];

/** The seven most spoken languages, each shown in its own script. */
const LANGUAGES: { value: Language; label: string }[] = [
  { value: "es", label: "Español" },
  { value: "en", label: "English" },
  { value: "zh", label: "中文" },
  { value: "hi", label: "हिन्दी" },
  { value: "fr", label: "Français" },
  { value: "ar", label: "العربية" },
  { value: "bn", label: "বাংলা" },
];

function Section({
  title,
  icon: Icon,
  badge,
  children,
}: {
  title: string;
  icon: typeof Sun;
  badge?: string;
  children: ReactNode;
}) {
  return (
    <Card>
      <div className="panel-head">
        <h3 className="panel-title">
          <Icon size={14} strokeWidth={2} aria-hidden />
          {title}
        </h3>
        {badge ? <Badge tone="muted">{badge}</Badge> : null}
      </div>
      {children}
    </Card>
  );
}

function Row({
  label,
  hint,
  wide = false,
  children,
}: {
  label: string;
  hint?: string;
  wide?: boolean;
  children: ReactNode;
}) {
  return (
    <div className={cn("row", wide && "row-wide")}>
      <div className="row-label">
        <b>{label}</b>
        {hint ? <span>{hint}</span> : null}
      </div>
      <div className="row-control" style={wide ? undefined : { flexShrink: 0 }}>
        {children}
      </div>
    </div>
  );
}

export function Settings() {
  const { t } = useTranslation();
  const settings = useAppStore((state) => state.settings);
  const appInfo = useAppStore((state) => state.appInfo);
  const updateSettings = useAppStore((state) => state.updateSettings);
  const resetSettings = useAppStore((state) => state.resetSettings);
  const [feedback, setFeedback] = useState<string | null>(null);
  const [overlayOpen, setOverlayOpen] = useState(false);

  const openLogs = () => {
    setFeedback(null);
    void ipc
      .openLogsDir()
      .catch((cause) => setFeedback(cause instanceof Error ? cause.message : String(cause)));
  };

  const reset = () => {
    setFeedback(null);
    void resetSettings()
      .then(() => setFeedback(t("settings.resetDone")))
      .catch((cause) => setFeedback(cause instanceof Error ? cause.message : String(cause)));
  };

  const restartElevated = () => {
    setFeedback(null);
    void ipc
      .relaunchAsAdmin()
      .catch((cause) => setFeedback(cause instanceof Error ? cause.message : String(cause)));
  };

  return (
    <section className="view" id="configuracion">
      <div className="grid cols-2">
        <Section title={t("settings.appearance")} icon={Sun}>
          <Row label={t("settings.language")} hint={t("settings.saved")} wide>
            <div className="lang-grid">
              {LANGUAGES.map((language) => (
                <button
                  key={language.value}
                  type="button"
                  lang={language.value}
                  dir={language.value === "ar" ? "rtl" : undefined}
                  onClick={() => void updateSettings({ language: language.value })}
                  className={cn("lang-btn", settings.language === language.value && "active")}
                >
                  {language.label}
                </button>
              ))}
            </div>
          </Row>

          <Row label={t("settings.theme")} hint={t("settings.saved")}>
            <div className="theme-grid">
              {THEMES.map((theme) => {
                const Icon = theme.icon;
                return (
                  <button
                    key={theme.value}
                    type="button"
                    onClick={() => void updateSettings({ theme: theme.value })}
                    className={cn("theme-btn", settings.theme === theme.value && "active")}
                  >
                    <Icon size={15} aria-hidden />
                    {t(theme.labelKey)}
                  </button>
                );
              })}
            </div>
          </Row>

          <Row label={t("settings.tray")}>
            <Switch
              checked={settings.minimizeToTray}
              onCheckedChange={(checked) => void updateSettings({ minimizeToTray: checked })}
            />
          </Row>
        </Section>

        <Section title={t("settings.monitor")} icon={GaugeIcon}>
          <Row
            label={t("settings.monitorInterval")}
            hint={t("settings.monitorIntervalHint", { ms: settings.monitorIntervalMs })}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <input
                type="range"
                min={500}
                max={5000}
                step={500}
                value={settings.monitorIntervalMs}
                onChange={(event) =>
                  void updateSettings({ monitorIntervalMs: Number(event.target.value) })
                }
                className="range"
                aria-label={t("settings.monitorInterval")}
              />
              <span
                style={{
                  width: 56,
                  textAlign: "right",
                  fontFamily: "var(--mono)",
                  fontSize: 10.5,
                  color: "var(--accent)",
                }}
              >
                {settings.monitorIntervalMs} ms
              </span>
            </div>
          </Row>

          <Row label={t("settings.overlay")} hint={t("settings.overlayHint")}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Tooltip label={t("settings.overlayConfigure")}>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setOverlayOpen(true)}
                  aria-label={t("settings.overlayConfigure")}
                >
                  <Settings2 size={14} aria-hidden />
                </Button>
              </Tooltip>
              <Switch
                checked={settings.overlayEnabled}
                onCheckedChange={(checked) => void updateSettings({ overlayEnabled: checked })}
              />
            </div>
          </Row>
        </Section>

        <Section title={t("settings.actions")} icon={FolderOpen}>
          <Row label={t("settings.openLogs")} hint={appInfo?.logDir ?? undefined}>
            <Button variant="ghost" size="sm" disabled={!isDesktop} onClick={openLogs}>
              <FolderOpen size={13} aria-hidden />
              {t("settings.open")}
            </Button>
          </Row>

          <Row label={t("settings.reset")} hint={t("settings.resetHint")}>
            <Button variant="ghost" size="sm" onClick={reset}>
              <RotateCcw size={13} aria-hidden />
              {t("settings.resetAction")}
            </Button>
          </Row>

          <Row
            label={t("settings.restartAdmin")}
            hint={appInfo?.isAdmin ? t("settings.adminAlready") : t("settings.restartAdminHint")}
          >
            {appInfo?.isAdmin ? (
              <Badge tone="accent">
                <ShieldCheck size={12} aria-hidden /> {t("settings.admin")}
              </Badge>
            ) : (
              <Button variant="ghost" size="sm" disabled={!isDesktop} onClick={restartElevated}>
                <ShieldAlert size={13} aria-hidden />
                {t("settings.restartAdmin")}
              </Button>
            )}
          </Row>

          {feedback ? (
            <p style={{ marginTop: 10, fontSize: 10.5, color: "var(--mut)" }}>{feedback}</p>
          ) : null}
        </Section>

        <Section title={t("settings.about")} icon={Info}>
          <Row label={t("settings.version")}>
            <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--mut)" }}>
              {appInfo?.version ?? "0.1.0"}
            </span>
          </Row>
          <Row label={t("settings.os")}>
            <span style={{ fontSize: 11, color: "var(--mut)" }}>{appInfo?.os ?? "—"}</span>
          </Row>
          <Row label={t("settings.arch")}>
            <span style={{ fontFamily: "var(--mono)", fontSize: 11, color: "var(--mut)" }}>
              {appInfo?.arch ?? "x86_64"}
            </span>
          </Row>
          <Row label={t("settings.privileges")}>
            <span
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                fontSize: 11,
                color: "var(--mut)",
              }}
            >
              <ShieldCheck
                size={13}
                style={{ color: appInfo?.isAdmin ? "var(--ok)" : "var(--warn)" }}
                aria-hidden
              />
              {appInfo?.isAdmin ? t("settings.admin") : t("settings.standard")}
            </span>
          </Row>
          <Row label={t("settings.logs")} hint={isDesktop ? appInfo?.logDir : t("app.browserMode")}>
            <Badge tone="muted">{isDesktop ? "LOCALAPPDATA" : "browser"}</Badge>
          </Row>
        </Section>
      </div>

      <OverlayConfigDialog open={overlayOpen} onOpenChange={setOverlayOpen} />
    </section>
  );
}
