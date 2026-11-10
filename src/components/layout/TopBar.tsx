import {
  Activity,
  ArrowUpRight,
  Languages,
  LayoutDashboard,
  Monitor,
  Moon,
  ScanSearch,
  Search,
  ShieldAlert,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
  Sun,
  Wrench,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";

import { Tooltip } from "@/components/ui/Switch";
import { isDesktop } from "@/lib/ipc";
import { useAppStore } from "@/stores/appStore";

const PAGE_META: Record<string, { titleKey: string; subtitleKey: string }> = {
  "/dashboard": { titleKey: "nav.dashboard", subtitleKey: "dashboard.subtitle" },
  "/diagnostics": { titleKey: "nav.diagnostics", subtitleKey: "diagnostics.subtitle" },
  "/optimizations": { titleKey: "nav.optimizations", subtitleKey: "optimizations.subtitle" },
  "/monitor": { titleKey: "nav.monitor", subtitleKey: "monitor.subtitle" },
  "/tools": { titleKey: "nav.tools", subtitleKey: "app.tagline" },
  "/history": { titleKey: "nav.history", subtitleKey: "history.subtitle" },
  "/settings": { titleKey: "nav.settings", subtitleKey: "settings.subtitle" },
};

const COMMANDS = [
  { path: "/dashboard", labelKey: "nav.dashboard", icon: LayoutDashboard },
  { path: "/diagnostics", labelKey: "nav.diagnostics", icon: ScanSearch },
  { path: "/optimizations", labelKey: "nav.optimizations", icon: Sparkles },
  { path: "/monitor", labelKey: "nav.monitor", icon: Activity },
  { path: "/tools", labelKey: "nav.tools", icon: Wrench },
  { path: "/history", labelKey: "nav.history", icon: Activity },
  { path: "/settings", labelKey: "nav.settings", icon: SlidersHorizontal },
];

export function TopBar() {
  const { t } = useTranslation();
  const location = useLocation();
  const navigate = useNavigate();
  const settings = useAppStore((state) => state.settings);
  const appInfo = useAppStore((state) => state.appInfo);
  const updateSettings = useAppStore((state) => state.updateSettings);

  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  const meta = PAGE_META[location.pathname] ?? PAGE_META["/dashboard"];
  const isDark = settings.theme !== "light";

  const results = useMemo(() => {
    const term = query.trim().toLowerCase();
    if (!term) return [];
    return COMMANDS.filter((command) =>
      t(command.labelKey).toLowerCase().includes(term),
    ).slice(0, 5);
  }, [query, t]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        inputRef.current?.focus();
      }
      if (event.key === "Escape") {
        setOpen(false);
        inputRef.current?.blur();
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const go = (path: string) => {
    navigate(path);
    setQuery("");
    setOpen(false);
    inputRef.current?.blur();
  };

  return (
    <header className="topbar">
      <div className="page-id">
        <h1>{t(meta.titleKey)}</h1>
        <p>{t(meta.subtitleKey)}</p>
      </div>

      <div className="topbar-tools">
        <label className="search">
          <Search size={15} aria-hidden />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setOpen(true);
            }}
            onFocus={() => setOpen(true)}
            onBlur={() => window.setTimeout(() => setOpen(false), 120)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && results[0]) go(results[0].path);
            }}
            placeholder={t("dashboard.searchPlaceholder")}
            aria-label={t("dashboard.searchPlaceholder")}
            spellCheck={false}
          />
          <kbd>Ctrl K</kbd>

          {open && results.length > 0 ? (
            <div className="palette" role="listbox">
              {results.map((command) => {
                const Icon = command.icon;
                return (
                  <button
                    key={command.path}
                    type="button"
                    className="palette-item"
                    onMouseDown={(event) => event.preventDefault()}
                    onClick={() => go(command.path)}
                  >
                    <Icon size={14} aria-hidden />
                    {t(command.labelKey)}
                    <ArrowUpRight size={13} style={{ marginLeft: "auto", opacity: 0.5 }} />
                  </button>
                );
              })}
            </div>
          ) : null}
        </label>

        <Tooltip label={settings.language === "es" ? "Switch to English" : "Cambiar a español"}>
          <button
            type="button"
            className="icon-btn"
            aria-label={t("settings.language")}
            onClick={() =>
              void updateSettings({ language: settings.language === "es" ? "en" : "es" })
            }
          >
            <Languages size={15} aria-hidden />
            <span className="code">{settings.language}</span>
          </button>
        </Tooltip>

        <Tooltip label={t("settings.theme")}>
          <button
            type="button"
            className="icon-btn"
            aria-label={t("settings.theme")}
            onClick={() => void updateSettings({ theme: isDark ? "light" : "dark" })}
          >
            {settings.theme === "high-contrast" ? (
              <Monitor size={15} aria-hidden />
            ) : isDark ? (
              <Sun size={15} aria-hidden />
            ) : (
              <Moon size={15} aria-hidden />
            )}
          </button>
        </Tooltip>

        <span className={isDesktop ? "status-chip" : "status-chip warn"}>
          {isDesktop ? <ShieldCheck size={13} aria-hidden /> : <ShieldAlert size={13} aria-hidden />}
          {isDesktop ? t("dashboard.live") : t("app.browserMode")}
        </span>

        <span className="icon-btn admin-chip" style={{ pointerEvents: "none" }}>
          {appInfo?.isAdmin ? (
            <ShieldCheck size={15} aria-hidden />
          ) : (
            <ShieldAlert size={15} aria-hidden />
          )}
          <span className="code">
            {appInfo?.isAdmin ? t("app.admin") : t("app.standard")}
          </span>
        </span>
      </div>
    </header>
  );
}
