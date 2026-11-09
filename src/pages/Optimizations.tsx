import { ChevronDown, Circle, CircleCheck, Info, Sparkles, Wand2, AlertCircle, Loader2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";
import { sampleOptimizations } from "@/data/optimizations";
import { riskTone } from "@/lib/tone";
import { cn } from "@/lib/utils";
import { ipc } from "@/lib/ipc";
import type { Category, Optimization } from "@/types";

type Filter = "all" | Category;

export function Optimizations() {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<Filter>("all");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [expanded, setExpanded] = useState<string | null>(null);
  const [optimizations, setOptimizations] = useState<Optimization[]>([]);
  const [loading, setLoading] = useState(true);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    ipc.getOptimizations()
      .then((opts) => mounted && setOptimizations(opts))
      .catch((e) => mounted && setError(e.message))
      .finally(() => mounted && setLoading(false));
    return () => { mounted = false; };
  }, []);

  const categories = useMemo(
    () =>
      ["all", ...Array.from(new Set(optimizations.map((item) => item.category)))] as Filter[],
    [optimizations],
  );

  const visible = useMemo(
    () =>
      filter === "all"
        ? optimizations
        : optimizations.filter((item) => item.category === filter),
    [filter, optimizations],
  );

  function toggleSelected(id: string) {
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  async function handleApply() {
    if (selected.size === 0 || applying) return;
    setApplying(true);
    setError(null);
    try {
      await ipc.applyOptimizations(Array.from(selected), false);
      setSelected(new Set());
      setExpanded(null);
      const opts = await ipc.getOptimizations();
      setOptimizations(opts);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setApplying(false);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="size-8 animate-spin text-accent" aria-hidden />
        <span className="ml-3 text-mut">{t("common.loading")}</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center gap-2 text-xs text-warn bg-warn/10 border border-warn/20 rounded-xl p-3">
        <AlertCircle className="size-4 shrink-0" aria-hidden />
        <span>{error}</span>
      </div>
    );
  }

  return (
    <section className="view" id="optimizacion">
      <div className="flex flex-wrap items-start justify-between gap-3 rounded-xl border border-line bg-card px-4 py-3">
        <div className="flex items-start gap-2.5 text-xs text-mut">
          <Info className="mt-0.5 size-4 shrink-0 text-accent" aria-hidden />
          <p className="max-w-3xl leading-relaxed">{t("optimizations.previewNote")}</p>
        </div>
        <Button
          variant="primary"
          size="sm"
          disabled={selected.size === 0 || applying}
          onClick={handleApply}
          className="btn w"
        >
          {applying ? (
            <>
              <Loader2 className="size-3.5 animate-spin" aria-hidden />
              {t("common.applying")}
            </>
          ) : (
            <>
              <Wand2 className="size-3.5" aria-hidden />
              {t("common.applySelected")}
              {selected.size > 0 ? ` (${selected.size})` : ""}
            </>
          )}
        </Button>
      </div>

      <div className="flex flex-wrap items-center gap-1.5">
        {categories.map((category) => (
          <button
            key={category}
            type="button"
            onClick={() => setFilter(category)}
            className={cn(
              "rounded-full border px-3 py-1.5 text-xs transition-colors",
              filter === category
                ? "border-accent/50 bg-accent-soft font-medium text-accent"
                : "border-line bg-card text-mut hover:border-accent/35 hover:text-text"
            )}
          >
            {category === "all" ? t("optimizations.all") : t(`categories.${category}`)}
          </button>
        ))}
        <span className="ml-auto text-[11px] text-mut">
          {visible.length} / {optimizations.length}
        </span>
      </div>

      {visible.length === 0 ? (
        <EmptyState icon={Sparkles} title={t("optimizations.empty")} description="" />
      ) : (
        <div className="space-y-2">
          {visible.map((item) => {
            const isSelected = selected.has(item.id);
            const isOpen = expanded === item.id;

            return (
              <Card
                key={item.id}
                className={cn(
                  "overflow-hidden transition-colors",
                  isSelected ? "border-accent/45" : undefined,
                )}
              >
                <div className="flex items-start gap-3 p-4">
                  <button
                    type="button"
                    aria-pressed={isSelected}
                    aria-label={item.title}
                    onClick={() => toggleSelected(item.id)}
                    className="mt-0.5 rounded-full text-mut transition-colors hover:text-accent"
                  >
                    {isSelected ? (
                      <CircleCheck className="size-5 text-accent" aria-hidden />
                    ) : (
                      <Circle className="size-5" aria-hidden />
                    )}
                  </button>

                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <h3 className="text-sm font-medium leading-snug text-text">{item.title}</h3>
                      <Badge tone="muted">{t(`categories.${item.category}`)}</Badge>
                      <Badge tone={riskTone(item.risk)}>
                        {t("common.risk")}: {t(`risk.${item.risk}`)}
                      </Badge>
                      <Badge tone={item.requiresReboot ? "warn" : "muted"}>
                        {item.requiresReboot ? t("common.rebootRequired") : t("common.noReboot")}
                      </Badge>
                      {!item.available ? (
                        <Badge tone="neutral">{t("optimizations.unavailable")}</Badge>
                      ) : null}
                    </div>

                    <p className="mt-1.5 text-xs leading-relaxed text-mut">{item.description}</p>

                    {isOpen ? (
                      <div className="mt-3 grid gap-3 rounded-lg border border-line bg-card p-3 sm:grid-cols-2">
                        <div>
                          <p className="text-[10px] font-semibold uppercase tracking-wider text-accent">
                            {t("optimizations.why")}
                          </p>
                          <p className="mt-1 text-[11px] leading-relaxed text-mut">
                            {item.rationale}
                          </p>
                        </div>
                        <div>
                          <p className="text-[10px] font-semibold uppercase tracking-wider text-accent">
                            {t("optimizations.what")}
                          </p>
                          <ul className="mt-1 space-y-1 text-[11px] leading-relaxed text-mut">
                            {item.snapshot.services.length > 0 ? (
                              <li>Servicios: {item.snapshot.services.join(", ")}</li>
                            ) : null}
                            {item.snapshot.registryKeys.map((key) => (
                              <li key={key} className="truncate font-mono text-[10px]">
                                {key}
                              </li>
                            ))}
                            {item.appliesTo.hardwareFilters.map((rule) => (
                              <li key={rule} className="font-mono text-[10px]">
                                {rule}
                              </li>
                            ))}
                          </ul>
                        </div>
                      </div>
                    ) : null}
                  </div>

                  <Button
                    variant="ghost"
                    size="sm"
                    aria-expanded={isOpen}
                    aria-label={t("optimizations.expand")}
                    onClick={() => setExpanded(isOpen ? null : item.id)}
                  >
                    <ChevronDown
                      className={cn("size-4 transition-transform", isOpen && "rotate-180")}
                      aria-hidden
                    />
                  </Button>
                </div>
              </Card>
            );
          })}
        </div>
      )}
    </section>
  );
}