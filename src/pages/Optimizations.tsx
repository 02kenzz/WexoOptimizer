import {
  AlertCircle,
  Check,
  ChevronDown,
  Info,
  Loader2,
  RotateCcw,
  ShieldAlert,
  Sparkles,
  Wand2,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Card";
import { EmptyState } from "@/components/ui/EmptyState";
import { riskTone } from "@/lib/tone";
import { cn } from "@/lib/utils";
import { ipc } from "@/lib/ipc";
import type { TransactionRecord } from "@/lib/ipc";
import { useAppStore } from "@/stores/appStore";
import type { Category, Optimization } from "@/types";

type Filter = "all" | Category;

const STATUS_KEY: Record<string, string> = {
  applied: "common.applied",
  rolled_back: "optimizations.reverted",
  rolledback: "optimizations.reverted",
  failed: "common.failed",
};

export function Optimizations() {
  const { t } = useTranslation();
  const isAdmin = useAppStore((state) => state.appInfo?.isAdmin ?? false);
  const [filter, setFilter] = useState<Filter>("all");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [expanded, setExpanded] = useState<string | null>(null);
  const [optimizations, setOptimizations] = useState<Optimization[]>([]);
  const [lastChange, setLastChange] = useState<TransactionRecord | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<"apply" | "preview" | "rollback" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [outcome, setOutcome] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const [items, history] = await Promise.all([
      ipc.getOptimizations(),
      ipc.getTransactionHistory(1).catch(() => [] as TransactionRecord[]),
    ]);
    setOptimizations(items);
    setLastChange(history[0] ?? null);
  }, []);

  useEffect(() => {
    let mounted = true;
    ipc
      .getOptimizations()
      .then((items) => mounted && setOptimizations(items))
      .catch((cause) => mounted && setError(cause.message))
      .finally(() => mounted && setLoading(false));
    ipc
      .getTransactionHistory(1)
      .then((history) => mounted && setLastChange(history[0] ?? null))
      .catch(() => undefined);
    return () => {
      mounted = false;
    };
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

  const isSelectable = (item: Optimization) =>
    item.available && (!item.requiresAdmin || isAdmin);

  function toggleSelected(item: Optimization) {
    if (!isSelectable(item)) return;
    setSelected((current) => {
      const next = new Set(current);
      if (next.has(item.id)) next.delete(item.id);
      else next.add(item.id);
      return next;
    });
  }

  async function run(dryRun: boolean) {
    if (selected.size === 0 || busy) return;
    setBusy(dryRun ? "preview" : "apply");
    setError(null);
    setOutcome(null);
    try {
      const result = await ipc.applyOptimizations(Array.from(selected), dryRun);
      if (dryRun) {
        setOutcome(t("optimizations.previewOk", { count: result.results.length }));
      } else {
        const applied = result.results.filter((entry) => entry.applied).length;
        setOutcome(t("optimizations.appliedOk", { count: applied }));
        setSelected(new Set());
        setExpanded(null);
        await refresh();
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function rollback() {
    if (!lastChange || busy) return;
    setBusy("rollback");
    setError(null);
    setOutcome(null);
    try {
      await ipc.rollbackTransaction(lastChange.id);
      setOutcome(t("optimizations.reverted"));
      await refresh();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  if (loading) {
    return (
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: 240, gap: 10, color: "var(--mut)" }}>
        <Loader2 className="animate-spin" size={22} aria-hidden />
        <span style={{ fontSize: 12 }}>{t("common.loading")}</span>
      </div>
    );
  }

  if (error && optimizations.length === 0) {
    return (
      <div className="alert">
        <AlertCircle size={15} aria-hidden />
        <span>{error}</span>
      </div>
    );
  }

  return (
    <section className="view" id="optimizacion">
      <Card>
        <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", justifyContent: "space-between", gap: 14 }}>
          <div style={{ display: "flex", alignItems: "flex-start", gap: 10, maxWidth: "62ch" }}>
            <Info size={16} style={{ marginTop: 2, color: "var(--accent)", flex: "none" }} aria-hidden />
            <p style={{ fontSize: 11.5, lineHeight: 1.6, color: "var(--mut)" }}>
              {t("optimizations.note")}
            </p>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Button
              variant="ghost"
              disabled={selected.size === 0 || busy !== null}
              onClick={() => void run(true)}
            >
              {busy === "preview" ? <Loader2 size={14} className="animate-spin" aria-hidden /> : null}
              {t("optimizations.preview")}
            </Button>
            <Button
              variant="primary"
              disabled={selected.size === 0 || busy !== null}
              onClick={() => void run(false)}
            >
              {busy === "apply" ? (
                <Loader2 size={14} className="animate-spin" aria-hidden />
              ) : (
                <Wand2 size={14} aria-hidden />
              )}
              {t("common.applySelected")}
              {selected.size > 0 ? ` (${selected.size})` : ""}
            </Button>
          </div>
        </div>
      </Card>

      {!isAdmin ? (
        <Card>
          <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", justifyContent: "space-between", gap: 12 }}>
            <p style={{ display: "flex", alignItems: "center", gap: 9, fontSize: 11.5, color: "var(--mut)" }}>
              <ShieldAlert size={15} style={{ color: "var(--warn)" }} aria-hidden />
              {t("optimizations.adminHint")}
            </p>
            <Button
              variant="ghost"
              size="sm"
              disabled={busy !== null}
              onClick={() =>
                void ipc
                  .relaunchAsAdmin()
                  .catch((cause) => setError(cause instanceof Error ? cause.message : String(cause)))
              }
            >
              {t("settings.restartAdmin")}
            </Button>
          </div>
        </Card>
      ) : null}

      {lastChange ? (
        <Card>
          <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", justifyContent: "space-between", gap: 12 }}>
            <div style={{ minWidth: 0 }}>
              <p style={{ fontSize: 12, fontWeight: 600 }}>{t("optimizations.lastChange")}</p>
              <p style={{ marginTop: 3, fontSize: 10.5, color: "var(--dim)" }}>
                {new Date(lastChange.timestamp_ms).toLocaleString()} ·{" "}
                {lastChange.optimization_ids.join(", ")}
              </p>
            </div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Badge tone={lastChange.rollback_available ? "accent" : "muted"}>
                {t(STATUS_KEY[lastChange.status] ?? "common.unknown")}
              </Badge>
              {lastChange.rollback_available ? (
                <Button variant="ghost" size="sm" disabled={busy !== null} onClick={() => void rollback()}>
                  {busy === "rollback" ? (
                    <Loader2 size={13} className="animate-spin" aria-hidden />
                  ) : (
                    <RotateCcw size={13} aria-hidden />
                  )}
                  {t("optimizations.revert")}
                </Button>
              ) : null}
            </div>
          </div>
        </Card>
      ) : null}

      {error ? (
        <div className="alert">
          <AlertCircle size={15} aria-hidden />
          <span>{error}</span>
        </div>
      ) : null}

      {outcome ? (
        <div className="alert">
          <Check size={15} aria-hidden />
          <span>{outcome}</span>
        </div>
      ) : null}

      <div className="split-head">
        <div className="chips">
          {categories.map((category) => (
            <button
              key={category}
              type="button"
              onClick={() => setFilter(category)}
              className={cn("chip-toggle", filter === category && "active")}
            >
              {category === "all" ? t("optimizations.all") : t(`categories.${category}`)}
            </button>
          ))}
        </div>
        <span style={{ fontSize: 10.5, color: "var(--dim)", fontVariantNumeric: "tabular-nums" }}>
          {visible.length} / {optimizations.length}
        </span>
      </div>

      {visible.length === 0 ? (
        <EmptyState icon={Sparkles} title={t("optimizations.empty")} description="" />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 9 }}>
          {visible.map((item) => {
            const isSelected = selected.has(item.id);
            const isOpen = expanded === item.id;
            const tone = riskTone(item.risk);
            const selectable = isSelectable(item);

            return (
              <div key={item.id} className={cn("opt", isSelected && "selected", !selectable && "disabled")}>
                <button
                  type="button"
                  aria-pressed={isSelected}
                  aria-label={item.title}
                  disabled={!selectable}
                  onClick={() => toggleSelected(item)}
                  className={cn("opt-check", isSelected && "on")}
                >
                  {isSelected ? <Check size={14} strokeWidth={3} aria-hidden /> : null}
                </button>

                <div style={{ minWidth: 0, flex: 1 }}>
                  <div className="opt-head">
                    <h3>{item.title}</h3>
                    <Badge tone="muted">{t(`categories.${item.category}`)}</Badge>
                    <Badge tone={tone}>
                      {t("common.risk")}: {t(`risk.${item.risk}`)}
                    </Badge>
                    <Badge tone={item.requiresReboot ? "warn" : "muted"}>
                      {item.requiresReboot ? t("common.rebootRequired") : t("common.noReboot")}
                    </Badge>
                    {!item.available ? (
                      <Badge tone="muted">{t("optimizations.incompatible")}</Badge>
                    ) : item.requiresAdmin && !isAdmin ? (
                      <Badge tone="warn">{t("optimizations.requiresAdmin")}</Badge>
                    ) : null}
                  </div>

                  <p className="opt-desc">{item.description}</p>

                  {isOpen ? (
                    <div className="opt-detail">
                      <div>
                        <h4>{t("optimizations.why")}</h4>
                        <p>{item.rationale}</p>
                      </div>
                      <div>
                        <h4>{t("optimizations.what")}</h4>
                        <ul>
                          {item.snapshot.services.length > 0 ? (
                            <li>Servicios: {item.snapshot.services.join(", ")}</li>
                          ) : null}
                          {item.snapshot.registryKeys.map((key) => (
                            <li key={key}>
                              <code>{key}</code>
                            </li>
                          ))}
                          {item.appliesTo.hardwareFilters.map((rule) => (
                            <li key={rule}>
                              <code>{rule}</code>
                            </li>
                          ))}
                        </ul>
                      </div>
                    </div>
                  ) : null}
                </div>

                <button
                  type="button"
                  className={cn("opt-toggle", isOpen && "open")}
                  aria-expanded={isOpen}
                  aria-label={t("optimizations.expand")}
                  onClick={() => setExpanded(isOpen ? null : item.id)}
                >
                  <ChevronDown size={15} aria-hidden />
                </button>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
