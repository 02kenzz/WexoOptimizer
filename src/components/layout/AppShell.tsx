import { Outlet, useLocation } from "react-router-dom";

import { Sidebar } from "./Sidebar";

export function AppShell() {
  const location = useLocation();
  const viewMap: Record<string, string> = {
    "/dashboard": "inicio",
    "/diagnostics": "diagnostico",
    "/optimizations": "optimizacion",
    "/monitor": "monitor",
    "/tools": "herramientas",
    "/settings": "configuracion",
    "/history": "historial",
  };
  const currentView = viewMap[location.pathname] || "inicio";

  return (
    <div className="flex h-screen overflow-hidden">
      <Sidebar currentView={currentView} />
      <main className="flex-1 min-w-0">
        <Outlet />
      </main>
    </div>
  );
}