import { useCallback, useEffect, useState } from "react";
import { api } from "@/lib/api";

export type CredentialItem = {
  id: string;
  name: string;
  type: string;
};

let cache: CredentialItem[] | null = null;
let inflight: Promise<CredentialItem[]> | null = null;

async function fetchCredentials(): Promise<CredentialItem[]> {
  if (cache) return cache;
  if (inflight) return inflight;
  inflight = api<{ credentials: CredentialItem[] }>("/api/credentials")
    .then((res) => {
      cache = res.credentials;
      return cache;
    })
    .finally(() => {
      inflight = null;
    });
  return inflight;
}

export function invalidateCredentialsCache() {
  cache = null;
}

export function useCredentials() {
  const [credentials, setCredentials] = useState<CredentialItem[]>(cache ?? []);
  const [loading, setLoading] = useState(!cache);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      invalidateCredentialsCache();
      const list = await fetchCredentials();
      setCredentials(list);
    } catch {
      setCredentials([]);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    void fetchCredentials()
      .then((list) => {
        if (!cancelled) setCredentials(list);
      })
      .catch(() => {
        if (!cancelled) setCredentials([]);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return { credentials, loading, reload };
}
