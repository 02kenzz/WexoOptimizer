import { Languages, Monitor, Moon, ShieldAlert, ShieldCheck, Sun } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useLocation } from "react-router-dom";

import { Button } from "@/components/ui/Button";
import { Tooltip } from "@/components/ui/Switch";
import { isDesktop } from "@/lib/ipc";
import { useAppStore } from "@/stores/appStore";

const PAGE_META: Record<string, { title: string; subtitle: string }> = {
  "/dashboard": { title: "dashboard.title", subtitle: "dashboard.subtitle" },
  "/diagnostics": { title: "diagnostics.title", subtitle: "diagnostics.subtitle" },
  "/optimizations": { title: "optimizations.title", subtitle: "optimizations.subtitle" },
  "/monitor": { title: "monitor.title", subtitle: "monitor.subtitle" },
  "/history": { title: "history.title", subtitle: "history.subtitle" },
  "/settings": { title: "settings.title", subtitle: "settings.subtitle" },
};

export function TopBar() {
  const { t } = useTranslation();
  const location = useLocation();
  const settings = useAppStore((state) => state.settings);
  const appInfo = useAppStore((state) => state.appInfo);
  const updateSettings = useAppStore((state) => state.updateSettings);

  const meta = PAGE_META[location.pathname] ?? PAGE_META["/dashboard"];

  const isDark = settings.theme === "dark" || settings.theme === "system";

  return (
    <header className="z-10 flex h-18 shrink-0 items-center justify-between gap-4 border-b border-border/30 bg-bg-elevated/60 px-5 backdrop-blur-2xl lg:px-8">
      <div className="min-w-0">
        <h1 className="truncate text-lg font-semibold tracking-tight text-text">{t(meta.title)}</h1>
        <p className="truncate text-xs text-text-dim">{t(meta.subtitle)}</p>
      </div>

      <div className="flex items-center gap-2">
        {!isDesktop ? (
          <Tooltip label={t("common.browserHint")}>
            <span className="hidden rounded-full border border-warn/30 bg-warn/10 px-2.5 py-1 text-[11px] text-warn sm:inline-flex">
              {t("app.browserMode")}
            </span>
          </Tooltip>
        ) : null}

        <span className="hidden items-center gap-1.5 rounded-full border border-border/30 bg-panel/50 px-2.5 py-1 text-[11px] text-text-dim sm:inline-flex">
          {appInfo?.isAdmin ? (
            <ShieldCheck className="size-3.5 text-ok" aria-hidden />
          ) : (
            <ShieldAlert className="size-3.5 text-warn" aria-hidden />
          )}
          {appInfo?.isAdmin ? t("app.admin") : t("app.standard")}
        </span>

        <Tooltip label={settings.language === "es" ? "Switch to English" : "Cambiar a español"}>
          <Button
            variant="ghost"
            size="sm"
            aria-label={t("settings.language")}
            onClick={() =>
              void updateSettings({ language: settings.language === "es" ? "en" : "es" })
            }
          >
            <Languages className="size-4" aria-hidden />
            <span className="font-mono text-[11px] uppercase text-text-muted">{settings.language}</span>
          </Button>
        </Tooltip>

        <Tooltip label={t("settings.theme")}>
          <Button
            variant="ghost"
            size="sm"
            aria-label={t("settings.theme")}
            onClick={() => void updateSettings({ theme: isDark ? "light" : "dark" })}
          >
            {settings.theme === "high-contrast" ? (
              <Monitor className="size-4" aria-hidden />
            ) : isDark ? (
              <Sun className="size-4" aria-hidden />
            ) : (
              <Moon className="size-4" aria-hidden />
            )}
          </Button>
        </Tooltip>
      </div>
    </header>
  );
}