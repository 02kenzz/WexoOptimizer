import { Database, HardDrive, ShieldCheck, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useToast } from "@/hooks/useToast";

import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { cn } from "@/lib/utils";
import { ipc, isDesktop } from "@/lib/ipc";

export function Tools() {
  const { t } = useTranslation();
  const { showToast } = useToast();

  const handleAction = async (action: string) => {
    try {
      // For now, show a toast since these IPC commands don't exist yet
      // In the future, these would call actual Tauri commands
      if (!isDesktop) {
        showToast({ title: t("tools.browserMode"), description: t("tools.browserModeDesc"), tone: "warn" });
        return;
      }

      // These commands would need to be implemented in the Rust backend
      showToast({ title: t("tools.comingSoon"), description: `${action} - ${t("tools.needsBackend")}`, tone: "accent" });
    } catch (error) {
      showToast({ title: t("common.error"), description: String(error), tone: "danger" });
    }
  };

  return (
    <section className="view" id="herramientas">
      <div className="space-y-4">
        <div className="grid gap-4 lg:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Trash2 className="size-4 text-accent" aria-hidden />
                {t("tools.cleanup")}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-sm text-mut">{t("tools.cleanupDesc")}</p>
              <div className="flex flex-wrap gap-2">
                <Button variant="glass" size="sm" onClick={() => handleAction("clean-temp")}>
                  {t("tools.cleanTemp")}
                </Button>
                <Button variant="glass" size="sm" onClick={() => handleAction("clean-logs")}>
                  {t("tools.cleanLogs")}
                </Button>
                <Button variant="glass" size="sm" onClick={() => handleAction("clean-cache")}>
                  {t("tools.cleanCache")}
                </Button>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <ShieldCheck className="size-4 text-accent" aria-hidden />
                {t("tools.repair")}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-sm text-mut">{t("tools.repairDesc")}</p>
              <div className="flex flex-wrap gap-2">
                <Button variant="glass" size="sm" onClick={() => handleAction("repair-system")}>
                  {t("tools.repairSystem")}
                </Button>
                <Button variant="glass" size="sm" onClick={() => handleAction("repair-permissions")}>
                  {t("tools.repairPermissions")}
                </Button>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Database className="size-4 text-accent" aria-hidden />
                {t("tools.database")}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-sm text-mut">{t("tools.databaseDesc")}</p>
              <div className="flex flex-wrap gap-2">
                <Button variant="glass" size="sm" onClick={() => handleAction("vacuum-db")}>
                  {t("tools.vacuumDb")}
                </Button>
                <Button variant="glass" size="sm" onClick={() => handleAction("reindex-db")}>
                  {t("tools.reindexDb")}
                </Button>
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <HardDrive className="size-4 text-accent" aria-hidden />
                {t("tools.disk")}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-sm text-mut">{t("tools.diskDesc")}</p>
              <div className="flex flex-wrap gap-2">
                <Button variant="glass" size="sm" onClick={() => handleAction("defrag")}>
                  {t("tools.defrag")}
                </Button>
                <Button variant="glass" size="sm" onClick={() => handleAction("trim")}>
                  {t("tools.trim")}
                </Button>
                <Button variant="glass" size="sm" onClick={() => handleAction("check-disk")}>
                  {t("tools.checkDisk")}
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <ShieldCheck className="size-4 text-accent" aria-hidden />
              {t("tools.systemInfo")}
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
              <div className="p-3 rounded-xl border border-line bg-card">
                <p className="text-xs text-mut">{t("tools.windowsVersion")}</p>
                <p className="font-mono text-sm text-text">{isDesktop ? "Windows 11" : "Browser"}</p>
              </div>
              <div className="p-3 rounded-xl border border-line bg-card">
                <p className="text-xs text-mut">{t("tools.architecture")}</p>
                <p className="font-mono text-sm text-text">{isDesktop ? "x64" : "WASM"}</p>
              </div>
              <div className="p-3 rounded-xl border border-line bg-card">
                <p className="text-xs text-mut">{t("tools.appVersion")}</p>
                <p className="font-mono text-sm text-text">1.0.0</p>
              </div>
              <div className="p-3 rounded-xl border border-line bg-card">
                <p className="text-xs text-mut">{t("tools.runtime")}</p>
                <p className="font-mono text-sm text-text">{isDesktop ? "Tauri" : "Web"}</p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    </section>
  );
}