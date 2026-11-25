import { useCallback, useEffect, useState } from "react";
import type { VariableDoc } from "@dataflow/shared";
import { api } from "@/lib/api";

export type VariableItem = VariableDoc & { _id: string };

let cache: VariableItem[] | null = null;
let inflight: Promise<VariableItem[]> | null = null;

async function fetchVariables(): Promise<VariableItem[]> {
  if (cache) return cache;
  if (inflight) return inflight;
  inflight = api<{ variables: VariableItem[] }>("/api/variables")
    .then((res) => {
      cache = res.variables;
      return cache;
    })
    .finally(() => {
      inflight = null;
    });
  return inflight;
}

export function invalidateVariablesCache() {
  cache = null;
}

export function useVariables() {
  const [variables, setVariables] = useState<VariableItem[]>(cache ?? []);
  const [loading, setLoading] = useState(!cache);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      invalidateVariablesCache();
      const list = await fetchVariables();
      setVariables(list);
    } catch {
      setVariables([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    void fetchVariables()
      .then((list) => {
        if (!cancelled) setVariables(list);
      })
      .catch(() => {
        if (!cancelled) setVariables([]);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return { variables, loading, reload };
}
