import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";

interface NavItem {
  view: string;
  labelKey: string;
  icon: string;
  path: string;
}

const NAV_ITEMS: NavItem[] = [
  { view: "inicio", labelKey: "nav.dashboard", icon: "home", path: "/dashboard" },
  { view: "diagnostico", labelKey: "nav.diagnostics", icon: "search", path: "/diagnostics" },
  { view: "optimizacion", labelKey: "nav.optimizations", icon: "bolt", path: "/optimizations" },
  { view: "monitor", labelKey: "nav.monitor", icon: "mon", path: "/monitor" },
  { view: "herramientas", labelKey: "nav.tools", icon: "tools", path: "/tools" },
  { view: "historial", labelKey: "nav.history", icon: "folder", path: "/history" },
  { view: "configuracion", labelKey: "nav.settings", icon: "gear", path: "/settings" },
];

interface SidebarProps {
  currentView: string;
}

export function Sidebar({ currentView }: SidebarProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const appInfo = useAppStore((state) => state.appInfo);

  return (
    <aside>
      <div className="brand">
        <div className="logo" />
        <div>
          <b>Optimizer</b>
          <span>Rendimiento sin límites</span>
        </div>
      </div>
      <nav>
        {NAV_ITEMS.map((item) => (
          <button
            key={item.view}
            data-view={item.view}
            className={cn(
              "flex gap-14 items-center px-[14px] py-[13px] rounded-[14px] text-left border border-transparent transition-[background,color] duration-200",
              currentView === item.view
                ? "text-text bg-[rgba(255,255,255,.08)] border-line"
                : "text-mut hover:text-text hover:bg-[rgba(255,255,255,.04)]"
            )}
            onClick={() => navigate(item.path)}
          >
            <svg className="ic"><use href={`#${item.icon}`} /></svg>
            {t(item.labelKey)}
          </button>
        ))}
      </nav>
    </aside>
  );
}