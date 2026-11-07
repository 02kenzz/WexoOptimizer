import { useQuery } from "@tanstack/react-query";

import { ipc } from "@/lib/ipc";
import type { SystemProfile } from "@/types";

/**
 * Polls the Rust collector. The first sample may report 0% CPU because usage is
 * computed as a delta between two reads; subsequent polls are accurate.
 */
export function useSystemProfile(intervalMs = 3000) {
  return useQuery<SystemProfile>({
    queryKey: ["system-profile"],
    queryFn: () => ipc.getSystemProfile(),
    refetchInterval: intervalMs,
    refetchIntervalInBackground: false,
  });
}
