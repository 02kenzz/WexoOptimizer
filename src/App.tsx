import { useEffect } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";

import { AppShell } from "@/components/layout/AppShell";
import { ToastProvider } from "@/hooks/useToast";
import { Dashboard } from "@/pages/Dashboard";
import { Diagnostics } from "@/pages/Diagnostics";
import { History } from "@/pages/History";
import { Monitor } from "@/pages/Monitor";
import { Optimizations } from "@/pages/Optimizations";
import { Overlay } from "@/pages/Overlay";
import { Settings } from "@/pages/Settings";
import { Tools } from "@/pages/Tools";
import { useAppStore } from "@/stores/appStore";

/** Keeps `<html data-theme>` in sync with the stored preference. */
function ThemeSync() {
  const theme = useAppStore((state) => state.settings.theme);

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: light)");

    const apply = () => {
      const resolved = theme === "system" ? (media.matches ? "light" : "dark") : theme;
      document.documentElement.dataset.theme = resolved;
      document.documentElement.style.colorScheme = resolved === "light" ? "light" : "dark";
    };

    apply();
    if (theme !== "system") return;

    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);

  return null;
}

/** True inside the overlay window created by `src-tauri/src/overlay.rs`. */
function isOverlayWindow(): boolean {
  try {
    const internals = (
      window as {
        __TAURI_INTERNALS__?: { metadata?: { currentWindow?: { label?: string } } };
      }
    ).__TAURI_INTERNALS__;
    return internals?.metadata?.currentWindow?.label === "overlay";
  } catch {
    return false;
  }
}

export default function App() {
  const bootstrap = useAppStore((state) => state.bootstrap);
  const overlay = isOverlayWindow();

  useEffect(() => {
    if (!overlay) void bootstrap();
  }, [bootstrap, overlay]);

  if (overlay) {
    return <Overlay />;
  }

  return (
    <>
      <ThemeSync />
      <ToastProvider>
        <HashRouter>
          <Routes>
            <Route element={<AppShell />}>
              <Route index element={<Navigate to="/dashboard" replace />} />
              <Route path="/dashboard" element={<Dashboard />} />
              <Route path="/diagnostics" element={<Diagnostics />} />
              <Route path="/optimizations" element={<Optimizations />} />
              <Route path="/monitor" element={<Monitor />} />
              <Route path="/tools" element={<Tools />} />
              <Route path="/history" element={<History />} />
              <Route path="/settings" element={<Settings />} />
              <Route path="*" element={<Navigate to="/dashboard" replace />} />
            </Route>
          </Routes>
        </HashRouter>
      </ToastProvider>
    </>
  );
}
