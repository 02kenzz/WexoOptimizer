import {
  Activity,
  History,
  LayoutDashboard,
  ScanSearch,
  SlidersHorizontal,
  Sparkles,
  Wrench,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";

interface NavItem {
  view: string;
  labelKey: string;
  icon: typeof LayoutDashboard;
  path: string;
}

const NAV_ITEMS: NavItem[] = [
  { view: "inicio", labelKey: "nav.dashboard", icon: LayoutDashboard, path: "/dashboard" },
  { view: "diagnostico", labelKey: "nav.diagnostics", icon: ScanSearch, path: "/diagnostics" },
  { view: "optimizacion", labelKey: "nav.optimizations", icon: Sparkles, path: "/optimizations" },
  { view: "monitor", labelKey: "nav.monitor", icon: Activity, path: "/monitor" },
  { view: "herramientas", labelKey: "nav.tools", icon: Wrench, path: "/tools" },
  { view: "historial", labelKey: "nav.history", icon: History, path: "/history" },
  { view: "configuracion", labelKey: "nav.settings", icon: SlidersHorizontal, path: "/settings" },
];

interface SidebarProps {
  currentView: string;
}

export function Sidebar({ currentView }: SidebarProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const appInfo = useAppStore((state) => state.appInfo);

  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark" aria-hidden="true">
          <img src="/logo.svg" alt="" draggable={false} />
        </div>
        <div className="brand-text">
          <b>{t("app.name")}</b>
          <span>{t("app.tagline")}</span>
        </div>
      </div>

      <nav className="nav" aria-label={t("app.name")}>
        {NAV_ITEMS.map((item) => {
          const Icon = item.icon;
          const active = currentView === item.view;

          return (
            <button
              key={item.view}
              type="button"
              data-view={item.view}
              aria-current={active ? "page" : undefined}
              className={cn("nav-item", active && "active")}
              onClick={() => navigate(item.path)}
            >
              <span className="nav-icon">
                <Icon size={17} strokeWidth={1.8} aria-hidden />
              </span>
              <span className="nav-label">{t(item.labelKey)}</span>
            </button>
          );
        })}
      </nav>

      <div className="side-spacer" />

      <div className="side-card">
        <div className="side-card-head">
          <span className="pulse-dot" />
          <span>{appInfo?.isAdmin ? t("app.admin") : t("app.standard")}</span>
        </div>
        <small>{t("app.version", { version: appInfo?.version ?? "0.1.0" })}</small>
        <div className="side-meta">
          <span>Win</span>
          <span>{appInfo?.arch ?? "x86_64"}</span>
        </div>
      </div>
    </aside>
  );
}
