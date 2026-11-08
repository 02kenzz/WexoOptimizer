import { useEffect, useRef, useState } from "react";

import { ipc } from "@/lib/ipc";
import type { LiveMetrics } from "@/types";

/** Samples kept in the UI. At 1 Hz this is five minutes of history. */
export const MAX_CHART_SAMPLES = 300;

export interface LiveMetricsState {
  samples: LiveMetrics[];
  latest: LiveMetrics | null;
  loading: boolean;
  error: string | null;
}

/**
 * Live monitor data: backfills from the backend ring buffer, then appends each
 * `monitor://metrics` event. Bounded so a long-running window stays cheap.
 */
export function useLiveMetrics(maxSamples = MAX_CHART_SAMPLES): LiveMetricsState {
  const [samples, setSamples] = useState<LiveMetrics[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const maxSamplesRef = useRef(maxSamples);
  maxSamplesRef.current = maxSamples;

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    const append = (sample: LiveMetrics) => {
      setSamples((previous) => {
        const next = [...previous, sample];
        return next.length > maxSamplesRef.current
          ? next.slice(next.length - maxSamplesRef.current)
          : next;
      });
    };

    void (async () => {
      try {
        const history = await ipc.getMetricsHistory();
        if (cancelled) return;
        setSamples(history.slice(-maxSamplesRef.current));
        setError(null);
      } catch (cause) {
        if (!cancelled) {
          setError(cause instanceof Error ? cause.message : String(cause));
        }
      } finally {
        if (!cancelled) setLoading(false);
      }

      if (cancelled) return;
      try {
        const stop = await ipc.onMonitorSample(append);
        if (cancelled) {
          stop();
          return;
        }
        unlisten = stop;
      } catch (cause) {
        if (!cancelled) {
          setError(cause instanceof Error ? cause.message : String(cause));
        }
      }
    })();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return {
    samples,
    latest: samples.at(-1) ?? null,
    loading,
    error,
  };
}
