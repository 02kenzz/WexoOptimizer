import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/Badge";
import { evidenceParams, ruleKey } from "@/lib/bottlenecks";
import { severityTone } from "@/lib/tone";
import type { Bottleneck } from "@/types";

/**
 * Renders a Rust `Bottleneck`. Prose lives in the locales; Rust only ships the
 * stable rule id plus numeric evidence.
 */
export function InsightCard({ bottleneck }: { bottleneck: Bottleneck }) {
  const { t } = useTranslation();
  const rule = `insights.rules.${ruleKey(bottleneck.id)}`;
  const params = evidenceParams(bottleneck);

  return (
    <article className="rounded-xl border border-border/30 bg-panel/50 p-4 transition-colors hover:border-border/50">
      <div className="flex items-center justify-between gap-2">
        <Badge tone={severityTone(bottleneck.severity)} dot>
          {t(`severity.${bottleneck.severity}`)}
        </Badge>
        <span className="text-[10px] uppercase tracking-wider text-text-dim">
          {t(`components.${bottleneck.component}`)}
        </span>
      </div>
      <h3 className="mt-3 text-sm font-medium leading-snug text-text">
        {t(`${rule}.title`, params)}
      </h3>
      <p className="mt-1.5 text-xs leading-relaxed text-text-dim">{t(`${rule}.desc`, params)}</p>

      {bottleneck.evidence.length > 0 ? (
        <ul className="mt-3 flex flex-wrap gap-x-3 gap-y-1">
          {bottleneck.evidence.map((item) => (
            <li
              key={item.code}
              className="text-[10px] tabular-nums text-text-dim before:mr-1 before:text-text-dim/50 before:content-['·']"
            >
              {t(`insights.evidence.${item.code}`, { value: item.value })}
            </li>
          ))}
        </ul>
      ) : null}

      <p className="mt-3 text-[10px] uppercase tracking-wider text-text-dim/80">
        {t("diagnostics.confidence", { value: t(`confidence.${bottleneck.confidence}`) })}
      </p>
    </article>
  );
}