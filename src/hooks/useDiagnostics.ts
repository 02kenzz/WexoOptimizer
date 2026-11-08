import { useQuery } from "@tanstack/react-query";

import { ipc } from "@/lib/ipc";
import type { DiagnosticReport } from "@/types";

/**
 * Runs the Rust diagnostic engine. Each call re-collects the profile and
 * analyses the monitor window, so a short interval still yields fresh data.
 */
export function useDiagnostics(refetchIntervalMs?: number) {
  return useQuery<DiagnosticReport>({
    queryKey: ["diagnostics"],
    queryFn: () => ipc.runDiagnostics(),
    refetchInterval: refetchIntervalMs,
    refetchIntervalInBackground: false,
    staleTime: 0,
  });
}
