import { Activity } from "lucide-react";
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
    <article className="panel panel-pad">
      <div className="panel-head">
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <span className="tool-icon" style={{ width: 32, height: 32, borderRadius: 10 }}>
            <Activity size={15} strokeWidth={2} aria-hidden />
          </span>
          <div>
            <h3 style={{ fontSize: 12.5, fontWeight: 600, lineHeight: 1.35 }}>
              {t(`${rule}.title`, params)}
            </h3>
            <p
              style={{
                marginTop: 3,
                fontSize: 9.5,
                letterSpacing: "0.12em",
                textTransform: "uppercase",
                color: "var(--dim)",
              }}
            >
              {t(`components.${bottleneck.component}`)}
            </p>
          </div>
        </div>
        <Badge tone={severityTone(bottleneck.severity)} dot>
          {t(`severity.${bottleneck.severity}`)}
        </Badge>
      </div>

      <p style={{ fontSize: 11.5, lineHeight: 1.6, color: "var(--mut)" }}>
        {t(`${rule}.desc`, params)}
      </p>

      {bottleneck.evidence.length > 0 ? (
        <div style={{ display: "flex", flexWrap: "wrap", gap: 5, marginTop: 10 }}>
          {bottleneck.evidence.map((item) => (
            <span key={item.code} className="chip chip-muted">
              {t(`insights.evidence.${item.code}`, { value: item.value })}
            </span>
          ))}
        </div>
      ) : null}

      <p
        style={{
          marginTop: 10,
          fontSize: 9.5,
          letterSpacing: "0.12em",
          textTransform: "uppercase",
          color: "var(--dim)",
        }}
      >
        {t("diagnostics.confidence", { value: t(`confidence.${bottleneck.confidence}`) })}
      </p>
    </article>
  );
}
