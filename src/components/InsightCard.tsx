import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/Badge";
import type { Insight } from "@/lib/insights";
import { severityTone } from "@/lib/tone";

export function InsightCard({ insight }: { insight: Insight }) {
  const { t } = useTranslation();

  return (
    <article className="rounded-xl border border-border/80 bg-panel-2/45 p-4 transition-colors hover:border-border">
      <div className="flex items-center justify-between gap-2">
        <Badge tone={severityTone(insight.severity)} dot>
          {t(`severity.${insight.severity}`)}
        </Badge>
        <span className="text-[10px] uppercase tracking-wider text-muted">{insight.impact}</span>
      </div>
      <h3 className="mt-3 text-sm font-medium leading-snug text-text">
        {t(insight.titleKey, insight.params)}
      </h3>
      <p className="mt-1.5 text-xs leading-relaxed text-muted">
        {t(insight.descriptionKey, insight.params)}
      </p>
      <p className="mt-3 text-[10px] uppercase tracking-wider text-muted/80">
        {t("diagnostics.confidence", { value: t(`confidence.${insight.confidence}`) })}
      </p>
    </article>
  );
}
