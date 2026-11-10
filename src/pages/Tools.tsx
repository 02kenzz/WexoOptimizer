import { Database, HardDrive, ShieldCheck, Trash2, Wrench } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { useToast } from "@/hooks/useToast";
import { isDesktop } from "@/lib/ipc";
import { useAppStore } from "@/stores/appStore";

interface ToolCardProps {
  icon: typeof Trash2;
  title: string;
  description: string;
  actions: Array<{ id: string; label: string }>;
  onAction: (id: string) => void;
}

function ToolCard({ icon: Icon, title, description, actions, onAction }: ToolCardProps) {
  return (
    <div className="panel tool-card">
      <div className="tool-head">
        <span className="tool-icon">
          <Icon size={18} strokeWidth={1.8} aria-hidden />
        </span>
        <div>
          <h3>{title}</h3>
          <p>{description}</p>
        </div>
      </div>
      <div className="tool-actions">
        {actions.map((action) => (
          <Button key={action.id} variant="glass" size="sm" onClick={() => onAction(action.id)}>
            {action.label}
          </Button>
        ))}
      </div>
    </div>
  );
}

export function Tools() {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const appInfo = useAppStore((state) => state.appInfo);

  const handleAction = async (action: string) => {
    try {
      // For now, show a toast since these IPC commands don't exist yet
      // In the future, these would call actual Tauri commands
      if (!isDesktop) {
        showToast({
          title: t("tools.browserMode"),
          description: t("tools.browserModeDesc"),
          tone: "warn",
        });
        return;
      }

      // These commands would need to be implemented in the Rust backend
      showToast({
        title: t("tools.comingSoon"),
        description: `${action} - ${t("tools.needsBackend")}`,
        tone: "accent",
      });
    } catch (error) {
      showToast({ title: t("common.error"), description: String(error), tone: "danger" });
    }
  };

  const cards: ToolCardProps[] = [
    {
      icon: Trash2,
      title: t("tools.cleanup"),
      description: t("tools.cleanupDesc"),
      actions: [
        { id: "clean-temp", label: t("tools.cleanTemp") },
        { id: "clean-logs", label: t("tools.cleanLogs") },
        { id: "clean-cache", label: t("tools.cleanCache") },
      ],
      onAction: handleAction,
    },
    {
      icon: ShieldCheck,
      title: t("tools.repair"),
      description: t("tools.repairDesc"),
      actions: [
        { id: "repair-system", label: t("tools.repairSystem") },
        { id: "repair-permissions", label: t("tools.repairPermissions") },
      ],
      onAction: handleAction,
    },
    {
      icon: Database,
      title: t("tools.database"),
      description: t("tools.databaseDesc"),
      actions: [
        { id: "vacuum-db", label: t("tools.vacuumDb") },
        { id: "reindex-db", label: t("tools.reindexDb") },
      ],
      onAction: handleAction,
    },
    {
      icon: HardDrive,
      title: t("tools.disk"),
      description: t("tools.diskDesc"),
      actions: [
        { id: "defrag", label: t("tools.defrag") },
        { id: "trim", label: t("tools.trim") },
        { id: "check-disk", label: t("tools.checkDisk") },
      ],
      onAction: handleAction,
    },
  ];

  return (
    <section className="view" id="herramientas">
      <div className="grid tools-grid">
        {cards.map((card) => (
          <ToolCard key={card.title} {...card} />
        ))}
      </div>

      <Card>
        <div className="panel-head">
          <h3 className="panel-title">
            <Wrench size={14} strokeWidth={2} aria-hidden />
            {t("tools.systemInfo")}
          </h3>
          <Badge tone="accent">
            {appInfo?.isAdmin ? t("app.admin") : t("app.standard")}
          </Badge>
        </div>

        <div className="grid tool-stats">
          <div className="stat-tile">
            <span>{t("tools.windowsVersion")}</span>
            <b>{isDesktop ? "Windows 11" : "Browser"}</b>
          </div>
          <div className="stat-tile">
            <span>{t("tools.architecture")}</span>
            <b>{isDesktop ? "x64" : "WASM"}</b>
          </div>
          <div className="stat-tile">
            <span>{t("tools.appVersion")}</span>
            <b>1.0.0</b>
          </div>
          <div className="stat-tile">
            <span>{t("tools.runtime")}</span>
            <b>{isDesktop ? "Tauri" : "Web"}</b>
          </div>
        </div>
      </Card>
    </section>
  );
}
