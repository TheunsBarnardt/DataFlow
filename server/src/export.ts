import { config } from "./config.js";

/** Fire-and-forget JSON event to configured sinks (webhook / OTLP-ish HTTP). */
export function emitExternal(
  kind: "audit" | "log" | "metric",
  payload: Record<string, unknown>,
): void {
  const body = {
    source: "dataflow",
    kind,
    at: new Date().toISOString(),
    ...payload,
  };
  const targets: string[] = [];
  if (config.auditWebhookUrl) targets.push(config.auditWebhookUrl);
  if (config.metricsWebhookUrl && (kind === "metric" || kind === "log")) {
    targets.push(config.metricsWebhookUrl);
  }
  if (config.otelEndpoint) {
    // Generic HTTP JSON sink — compatible with collectors that accept JSON POSTs
    targets.push(config.otelEndpoint.replace(/\/$/, "") + "/v1/logs");
  }
  for (const url of [...new Set(targets)]) {
    void fetch(url, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(body),
    }).catch(() => {
      /* sink failures must not break the app */
    });
  }
}
