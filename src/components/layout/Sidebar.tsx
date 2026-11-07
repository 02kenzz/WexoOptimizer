import {
  Activity,
  History,
  LayoutDashboard,
  LineChart,
  type LucideIcon,
  Settings,
  Sparkles,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { NavLink } from "react-router-dom";

import { isDesktop } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";

interface NavItem {
  to: string;
  labelKey: string;
  icon: LucideIcon;
}

const NAV_ITEMS: NavItem[] = [
  { to: "/dashboard", labelKey: "nav.dashboard", icon: LayoutDashboard },
  { to: "/diagnostics", labelKey: "nav.diagnostics", icon: Activity },
  { to: "/optimizations", labelKey: "nav.optimizations", icon: Sparkles },
  { to: "/monitor", labelKey: "nav.monitor", icon: LineChart },
  { to: "/history", labelKey: "nav.history", icon: History },
  { to: "/settings", labelKey: "nav.settings", icon: Settings },
];

export function Sidebar() {
  const { t } = useTranslation();
  const appInfo = useAppStore((state) => state.appInfo);

  return (
    <aside className="hidden w-[248px] shrink-0 flex-col border-r border-border bg-panel/55 backdrop-blur-xl md:flex">
      <div className="flex items-center gap-3 px-5 pb-5 pt-6">
        <div className="grid size-10 place-items-center rounded-2xl bg-gradient-to-br from-accent to-accent-2 text-lg font-black text-accent-contrast shadow-[0_10px_30px_-12px_var(--accent)]">
          W
        </div>
        <div className="min-w-0">
          <p className="truncate text-sm font-semibold tracking-tight">Wexo Optimizer</p>
          <p className="truncate text-[10px] uppercase tracking-[0.14em] text-muted">
            {t("app.tagline")}
          </p>
        </div>
      </div>

      <nav className="flex-1 space-y-1 px-3" aria-label={t("app.name")}>
        {NAV_ITEMS.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            className={({ isActive }) =>
              cn(
                "group relative flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm transition-colors",
                isActive
                  ? "bg-accent-soft font-medium text-accent"
                  : "text-muted hover:bg-panel-2 hover:text-text",
              )
            }
          >
            {({ isActive }) => (
              <>
                {isActive ? (
                  <span className="absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-full bg-accent" />
                ) : null}
                <item.icon className="size-4 shrink-0" strokeWidth={1.9} aria-hidden />
                <span className="truncate">{t(item.labelKey)}</span>
              </>
            )}
          </NavLink>
        ))}
      </nav>

      <div className="space-y-3 border-t border-border/70 p-4">
        <div className="flex items-center justify-between text-[11px] text-muted">
          <span className="font-mono">{appInfo?.version ?? "0.1.0"}</span>
          <span className="rounded-full border border-border px-2 py-0.5 text-[10px] uppercase tracking-wider">
            {t("app.foundation")}
          </span>
        </div>
        <div className="flex items-center gap-2 rounded-lg border border-border/70 bg-panel-2/60 px-2.5 py-2 text-[11px] text-muted">
          <span
            className={cn(
              "size-1.5 rounded-full",
              isDesktop ? "bg-ok animate-pulse-soft" : "bg-warn",
            )}
            aria-hidden
          />
          <span className="truncate">{isDesktop ? "Tauri conectado" : t("app.browserMode")}</span>
        </div>
      </div>
    </aside>
  );
}
