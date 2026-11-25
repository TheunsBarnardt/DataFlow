export type CapturedWebhook = {
  method: string;
  headers: Record<string, unknown>;
  query: unknown;
  body: unknown;
  path: string;
  capturedAt: string;
};

type ListenSession = {
  workflowId: string;
  path: string;
  method: string;
  expiresAt: number;
  resolve: (payload: CapturedWebhook) => void;
  reject: (err: Error) => void;
  promise: Promise<CapturedWebhook>;
};

const sessions = new Map<string, ListenSession>();

function key(path: string): string {
  return path.trim().toLowerCase();
}

const LISTEN_MS = 120_000;

export function startWebhookListen(opts: {
  workflowId: string;
  path: string;
  method?: string;
  /** When true, replace any existing session for this path */
  force?: boolean;
}): { expiresAt: string; waitMs: number; reused: boolean } {
  const k = key(opts.path);
  const existing = getListenSession(opts.path);
  if (existing && !opts.force) {
    return {
      expiresAt: new Date(existing.expiresAt).toISOString(),
      waitMs: Math.max(0, existing.expiresAt - Date.now()),
      reused: true,
    };
  }
  if (existing) {
    existing.reject(new Error("Superseded by a new listen session"));
    sessions.delete(k);
  }

  let resolve!: (payload: CapturedWebhook) => void;
  let reject!: (err: Error) => void;
  const promise = new Promise<CapturedWebhook>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  // Prevent unhandled rejection if client disconnects without waiting
  void promise.catch(() => undefined);

  const expiresAt = Date.now() + LISTEN_MS;
  sessions.set(k, {
    workflowId: opts.workflowId,
    path: opts.path,
    method: String(opts.method ?? "POST").toUpperCase(),
    expiresAt,
    resolve,
    reject,
    promise,
  });

  setTimeout(() => {
    const s = sessions.get(k);
    if (s && s.expiresAt === expiresAt) {
      s.reject(new Error("Listen timed out"));
      sessions.delete(k);
    }
  }, LISTEN_MS + 50);

  return {
    expiresAt: new Date(expiresAt).toISOString(),
    waitMs: LISTEN_MS,
    reused: false,
  };
}

export function getListenSession(path: string): ListenSession | undefined {
  const s = sessions.get(key(path));
  if (!s) return undefined;
  if (Date.now() > s.expiresAt) {
    s.reject(new Error("Listen timed out"));
    sessions.delete(key(path));
    return undefined;
  }
  return s;
}

export function captureTestWebhook(
  path: string,
  payload: Omit<CapturedWebhook, "path" | "capturedAt">,
): CapturedWebhook | null {
  const s = getListenSession(path);
  if (!s) return null;
  // During test listen, accept any HTTP method so users can probe freely.
  // Production hooks still enforce method via the node/registry.
  const captured: CapturedWebhook = {
    ...payload,
    path: s.path,
    capturedAt: new Date().toISOString(),
  };
  s.resolve(captured);
  sessions.delete(key(path));
  return captured;
}

export function awaitListen(path: string): Promise<CapturedWebhook> {
  const s = getListenSession(path);
  if (!s) return Promise.reject(new Error("No active listen session"));
  return s.promise;
}

export function cancelListen(path: string): boolean {
  const s = sessions.get(key(path));
  if (!s) return false;
  s.reject(new Error("Listen cancelled"));
  sessions.delete(key(path));
  return true;
}
