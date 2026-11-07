import { Database, History as HistoryIcon, ShieldCheck, Undo2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/Badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";

export function History() {
  const { t } = useTranslation();

  return (
    <div className="grid gap-4 lg:grid-cols-[1.5fr_0.9fr]">
      <Card className="p-5">
        <EmptyState
          icon={HistoryIcon}
          title={t("history.emptyTitle")}
          description={t("history.emptyHint")}
          className="border-0 py-16"
        />
      </Card>

      <div className="space-y-3">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Undo2 className="size-4 text-accent" aria-hidden />
              {t("history.rollbackTitle")}
            </CardTitle>
            <Badge tone="accent">M2</Badge>
          </CardHeader>
          <CardContent>
            <p className="text-xs leading-relaxed text-muted">{t("history.rollbackHint")}</p>
            <div className="mt-3 flex items-center gap-2 rounded-lg border border-ok/25 bg-ok/5 px-3 py-2 text-[11px] text-ok">
              <ShieldCheck className="size-3.5" aria-hidden />
              SHA256 · SQLite · dry-run
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Database className="size-4 text-accent" aria-hidden />
              Transaction log
            </CardTitle>
            <Badge tone="muted">%LOCALAPPDATA%</Badge>
          </CardHeader>
          <CardContent>
            <p className="font-mono text-[10px] leading-relaxed text-muted">
              WexoOptimizer/transactions.db
            </p>
            <p className="mt-2 text-[11px] leading-relaxed text-muted">
              Cada acción se registra con timestamp, parámetros, resultado y disponibilidad de
              rollback.
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
