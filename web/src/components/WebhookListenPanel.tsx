import { useEffect, useMemo, useState } from "react";
import { Check, Copy, Loader2, Radio } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { toast } from "sonner";
import { cn } from "@/lib/utils";

export type WebhookSample = {
  headers: Record<string, unknown>;
  query: unknown;
  body: unknown;
  method: string;
  path: string;
  capturedAt: string;
};

function publicOrigin(): string {
  const env = (import.meta as { env?: { VITE_PUBLIC_URL?: string } }).env
    ?.VITE_PUBLIC_URL?.replace(/\/$/, "");
  if (env) return env;
  if (typeof window === "undefined") return "http://127.0.0.1:8787";
  // Dev UI is proxied; prefer current origin so /hooks-test hits Vite → API
  return window.location.origin;
}

export function WebhookListenPanel({
  workflowId,
  path,
  method,
  workflowEnabled,
  sample,
  onSample,
}: {
  workflowId: string;
  path: string;
  method: string;
  workflowEnabled: boolean;
  sample: WebhookSample | null;
  onSample: (sample: WebhookSample | null) => void;
}) {
  const [mode, setMode] = useState<"test" | "production">("test");
  const [listening, setListening] = useState(false);
  const [expiresAt, setExpiresAt] = useState<number | null>(null);
  const [now, setNow] = useState(Date.now());
  const [copied, setCopied] = useState(false);

  const absoluteUrl = useMemo(() => {
    if (!path) return "";
    const suffix = mode === "test" ? `/hooks-test/${path}` : `/hooks/${path}`;
    return `${publicOrigin()}${suffix}`;
  }, [mode, path]);

  useEffect(() => {
    if (!listening || !expiresAt) return;
    const id = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(id);
  }, [listening, expiresAt]);

  const remainingSec =
    listening && expiresAt
      ? Math.max(0, Math.ceil((expiresAt - now) / 1000))
      : 0;

  async function copyUrl() {
    try {
      await navigator.clipboard.writeText(absoluteUrl);
      setCopied(true);
      toast.success("URL copied");
      window.setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error("Could not copy URL");
    }
  }

  async function startListen() {
    if (!path.trim()) {
      toast.error("Set a webhook path before listening");
      return;
    }
    setListening(true);
    onSample(null);
    const startedAt = Date.now() + 120_000;
    setExpiresAt(startedAt);
    toast.message("Listening for test event", {
      description: "Send a request to the Test URL",
    });
    try {
      const res = await api<{
        event?: WebhookSample;
        expiresAt?: string;
        error?: string;
      }>("/api/webhooks/listen", {
        method: "POST",
        body: JSON.stringify({
          workflowId,
          path: path.trim(),
          method,
          wait: true,
        }),
      });
      if (res.event) {
        onSample(res.event);
        toast.success("Test event received");
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Listen failed");
    } finally {
      setListening(false);
      setExpiresAt(null);
    }
  }

  return (
    <div className="flex flex-col gap-3 rounded-lg border bg-muted/20 p-3">
      <div className="flex items-center gap-1 rounded-md bg-muted/50 p-0.5">
        <button
          type="button"
          className={cn(
            "flex-1 rounded-md px-2 py-1.5 text-xs font-medium",
            mode === "test" ? "bg-card shadow-sm" : "text-muted-foreground",
          )}
          onClick={() => setMode("test")}
        >
          Test URL
        </button>
        <button
          type="button"
          className={cn(
            "flex-1 rounded-md px-2 py-1.5 text-xs font-medium",
            mode === "production"
              ? "bg-card shadow-sm"
              : "text-muted-foreground",
          )}
          onClick={() => setMode("production")}
        >
          Production URL
        </button>
      </div>

      <div className="flex items-stretch gap-1">
        <code className="min-w-0 flex-1 truncate rounded-md border bg-background px-2 py-1.5 font-mono text-[11px]">
          {absoluteUrl || "Save path to generate URL"}
        </code>
        <Button
          type="button"
          size="icon-xs"
          variant="outline"
          disabled={!path}
          onClick={() => void copyUrl()}
          aria-label="Copy URL"
        >
          {copied ? (
            <Check className="h-3.5 w-3.5" />
          ) : (
            <Copy className="h-3.5 w-3.5" />
          )}
        </Button>
      </div>

      {mode === "production" && !workflowEnabled ? (
        <p className="text-[11px] text-amber-600 dark:text-amber-400">
          Enable the workflow to accept production webhook calls.
        </p>
      ) : null}

      {mode === "test" ? (
        <Button
          type="button"
          size="sm"
          variant={listening ? "secondary" : "default"}
          disabled={!path || listening}
          onClick={() => void startListen()}
        >
          {listening ? (
            <>
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              Listening… {remainingSec}s
            </>
          ) : (
            <>
              <Radio className="h-3.5 w-3.5" />
              Listen for test event
            </>
          )}
        </Button>
      ) : (
        <p className="text-[11px] text-muted-foreground">
          Production calls run the workflow and appear under Executions.
        </p>
      )}

      {sample ? (
        <div className="rounded-md border bg-background p-2">
          <div className="mb-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
            Last test event
          </div>
          <pre className="max-h-40 overflow-auto font-mono text-[10px] leading-relaxed">
            {JSON.stringify(
              {
                method: sample.method,
                headers: sample.headers,
                query: sample.query,
                body: sample.body,
              },
              null,
              2,
            )}
          </pre>
        </div>
      ) : null}
    </div>
  );
}
