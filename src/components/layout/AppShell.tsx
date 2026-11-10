import { Outlet, useLocation } from "react-router-dom";

import { Sidebar } from "./Sidebar";
import { TopBar } from "./TopBar";

const VIEW_BY_PATH: Record<string, string> = {
  "/dashboard": "inicio",
  "/diagnostics": "diagnostico",
  "/optimizations": "optimizacion",
  "/monitor": "monitor",
  "/tools": "herramientas",
  "/settings": "configuracion",
  "/history": "historial",
};

export function AppShell() {
  const location = useLocation();
  const currentView = VIEW_BY_PATH[location.pathname] ?? "inicio";

  return (
    <>
      <div className="app-bg" aria-hidden="true">
        <span className="aurora a1" />
        <span className="aurora a2" />
        <span className="aurora a3" />
      </div>

      <div className="shell">
        <Sidebar currentView={currentView} />
        <main className="main">
          <TopBar />
          <div className="page" key={location.pathname}>
            <Outlet />
          </div>
        </main>
      </div>
    </>
  );
}
