import {
  Contrast,
  Gauge,
  Info,
  Languages,
  Monitor as MonitorIcon,
  Moon,
  ShieldCheck,
  Sun,
} from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/Badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { Switch } from "@/components/ui/Switch";
import { isDesktop } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";
import type { Theme } from "@/types";

const THEMES: { value: Theme; icon: typeof Sun; labelKey: string }[] = [
  { value: "system", icon: MonitorIcon, labelKey: "settings.themeSystem" },
  { value: "light", icon: Sun, labelKey: "settings.themeLight" },
  { value: "dark", icon: Moon, labelKey: "settings.themeDark" },
  { value: "high-contrast", icon: Contrast, labelKey: "settings.themeHighContrast" },
];

function Section({
  title,
  icon: Icon,
  children,
}: {
  title: string;
  icon: typeof Sun;
  children: ReactNode;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Icon className="size-4 text-accent" aria-hidden />
          {title}
        </CardTitle>
      </CardHeader>
      <CardContent className="divide-y divide-border/60 pt-0">{children}</CardContent>
    </Card>
  );
}

function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-4 py-4">
      <div className="min-w-[180px] flex-1">
        <p className="text-xs font-medium text-text">{label}</p>
        {hint ? <p className="mt-0.5 text-[11px] leading-relaxed text-muted">{hint}</p> : null}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

export function Settings() {
  const { t } = useTranslation();
  const settings = useAppStore((state) => state.settings);
  const appInfo = useAppStore((state) => state.appInfo);
  const updateSettings = useAppStore((state) => state.updateSettings);

  return (
    <div className="grid gap-4 xl:grid-cols-2">
      <Section title={t("settings.appearance")} icon={Sun}>
        <Row label={t("settings.language")} hint={t("settings.saved")}>
          <div className="inline-flex rounded-lg border border-border bg-panel-2/60 p-0.5">
            {(["es", "en"] as const).map((language) => (
              <button
                key={language}
                type="button"
                onClick={() => void updateSettings({ language })}
                className={cn(
                  "rounded-md px-3 py-1.5 text-xs transition-colors",
                  settings.language === language
                    ? "bg-accent text-accent-contrast font-medium"
                    : "text-muted hover:text-text",
                )}
              >
                {language === "es" ? "Español" : "English"}
              </button>
            ))}
          </div>
        </Row>

        <Row label={t("settings.theme")} hint={t("settings.saved")}>
          <div className="grid grid-cols-2 gap-1.5 sm:grid-cols-4">
            {THEMES.map((theme) => (
              <button
                key={theme.value}
                type="button"
                onClick={() => void updateSettings({ theme: theme.value })}
                className={cn(
                  "flex w-[74px] flex-col items-center gap-1.5 rounded-lg border px-2 py-2 text-[10px] transition-colors",
                  settings.theme === theme.value
                    ? "border-accent/60 bg-accent-soft text-accent"
                    : "border-border bg-panel-2/40 text-muted hover:text-text",
                )}
              >
                <theme.icon className="size-4" aria-hidden />
                {t(theme.labelKey)}
              </button>
            ))}
          </div>
        </Row>

        <Row label={t("settings.tray")}>
          <Switch
            checked={settings.minimizeToTray}
            onCheckedChange={(checked) => void updateSettings({ minimizeToTray: checked })}
          />
        </Row>
      </Section>

      <Section title={t("settings.monitor")} icon={Gauge}>
        <Row
          label={t("settings.monitorInterval")}
          hint={t("settings.monitorIntervalHint", { ms: settings.monitorIntervalMs })}
        >
          <div className="flex w-56 items-center gap-3">
            <input
              type="range"
              min={500}
              max={5000}
              step={500}
              value={settings.monitorIntervalMs}
              onChange={(event) =>
                void updateSettings({ monitorIntervalMs: Number(event.target.value) })
              }
              className="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-border"
              style={{ accentColor: "var(--accent)" }}
              aria-label={t("settings.monitorInterval")}
            />
            <span className="w-14 text-right font-mono text-[11px] text-muted">
              {settings.monitorIntervalMs} ms
            </span>
          </div>
        </Row>

        <Row label={t("settings.telemetry")} hint={t("settings.telemetryHint")}>
          <Switch
            checked={settings.telemetryEnabled}
            onCheckedChange={(checked) => void updateSettings({ telemetryEnabled: checked })}
          />
        </Row>

        <Row label={t("monitor.targets")}>
          <Badge tone="accent">M3</Badge>
        </Row>
      </Section>

      <Section title={t("settings.updates")} icon={Languages}>
        <Row label={t("settings.channel")}>
          <div className="inline-flex rounded-lg border border-border bg-panel-2/60 p-0.5">
            {(["stable", "beta"] as const).map((channel) => (
              <button
                key={channel}
                type="button"
                onClick={() => void updateSettings({ updateChannel: channel })}
                className={cn(
                  "rounded-md px-3 py-1.5 text-xs capitalize transition-colors",
                  settings.updateChannel === channel
                    ? "bg-accent text-accent-contrast font-medium"
                    : "text-muted hover:text-text",
                )}
              >
                {channel === "stable" ? t("settings.stable") : t("settings.beta")}
              </button>
            ))}
          </div>
        </Row>
        <Row label="Auto-update">
          <Badge tone="muted">M5</Badge>
        </Row>
      </Section>

      <Section title={t("settings.about")} icon={Info}>
        <Row label={t("settings.version")}>
          <span className="font-mono text-xs text-muted">{appInfo?.version ?? "0.1.0"}</span>
        </Row>
        <Row label={t("settings.os")}>
          <span className="text-xs text-muted">{appInfo?.os ?? "Windows"}</span>
        </Row>
        <Row label={t("settings.arch")}>
          <span className="font-mono text-xs text-muted">{appInfo?.arch ?? "x86_64"}</span>
        </Row>
        <Row label={t("settings.privileges")}>
          <span className="flex items-center gap-1.5 text-xs text-muted">
            <ShieldCheck
              className={cn("size-3.5", appInfo?.isAdmin ? "text-ok" : "text-warn")}
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
  );
}
