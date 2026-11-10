import { Database, History as HistoryIcon, ShieldCheck, Undo2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/Badge";
import { Card } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";

export function History() {
  const { t } = useTranslation();

  return (
    <section className="view" id="historial">
      <div className="grid cols-hero">
        <Card style={{ padding: 0 }}>
          <div style={{ padding: "40px 24px" }}>
            <EmptyState
              icon={HistoryIcon}
              title={t("history.emptyTitle")}
              description={t("history.emptyHint")}
            />
          </div>
        </Card>

        <div className="grid" style={{ gap: 14 }}>
          <Card>
            <div className="panel-head">
              <h3 className="panel-title">
                <Undo2 size={14} strokeWidth={2} aria-hidden />
                {t("history.rollbackTitle")}
              </h3>
              <Badge tone="accent">M2</Badge>
            </div>
            <p style={{ fontSize: 11.5, lineHeight: 1.6, color: "var(--mut)" }}>
              {t("history.rollbackHint")}
            </p>
            <div
              style={{
                marginTop: 12,
                display: "flex",
                alignItems: "center",
                gap: 8,
                padding: "9px 11px",
                borderRadius: 11,
                border: "1px solid rgba(211,180,131,0.25)",
                background: "rgba(211,180,131,0.06)",
                fontSize: 10.5,
                color: "var(--accent)",
              }}
            >
              <ShieldCheck size={14} aria-hidden />
              SHA256 · SQLite · dry-run
            </div>
          </Card>

          <Card>
            <div className="panel-head">
              <h3 className="panel-title">
                <Database size={14} strokeWidth={2} aria-hidden />
                Transaction log
              </h3>
              <Badge tone="muted">%LOCALAPPDATA%</Badge>
            </div>
            <p style={{ fontFamily: "var(--mono)", fontSize: 10, color: "var(--dim)" }}>
              WexoOptimizer/transactions.db
            </p>
            <p style={{ marginTop: 8, fontSize: 10.5, lineHeight: 1.6, color: "var(--dim)" }}>
              Cada acción se registra con timestamp, parámetros, resultado y disponibilidad de
              rollback.
            </p>
          </Card>
        </div>
      </div>
    </section>
  );
}
