import { config as loadEnv } from "dotenv";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "../..");
loadEnv({ path: path.join(root, ".env") });

const host = process.env.HOST ?? "127.0.0.1";
const port = Number(process.env.PORT ?? 8787);

export const config = {
  host,
  port,
  /** Public base URL for webhook links shown in the editor. */
  publicBaseUrl:
    process.env.PUBLIC_URL?.replace(/\/$/, "") ??
    `http://${host === "0.0.0.0" ? "127.0.0.1" : host}:${port}`,
  mongoUri: process.env.MONGODB_URI ?? "mongodb://127.0.0.1:27017/botflow",
  webDist: path.resolve(root, process.env.WEB_DIST ?? "web/dist"),
  redisUrl: process.env.REDIS_URL ?? "",
  rabbitmqUrl: process.env.RABBITMQ_URL ?? "",
  whatsappVerifyToken: process.env.WHATSAPP_VERIFY_TOKEN ?? "",
  /** Default identity when X-DataFlow-Actor is absent (local single-user / fallback). */
  defaultActor: process.env.DATAFLOW_ACTOR ?? "local",
  /** Better Auth secret (required in production). */
  betterAuthSecret:
    process.env.BETTER_AUTH_SECRET ?? "botflow-dev-better-auth-secret-change-me",
  /**
   * Public origin browsers use for auth cookies.
   * With Vite (`npm run dev`), set BETTER_AUTH_URL=http://127.0.0.1:5173.
   */
  betterAuthUrl:
    process.env.BETTER_AUTH_URL?.replace(/\/$/, "") ??
    process.env.PUBLIC_URL?.replace(/\/$/, "") ??
    `http://${host === "0.0.0.0" ? "127.0.0.1" : host}:${port}`,
  /** Bootstrap admin (created once on startup if missing). */
  adminUsername: process.env.DATAFLOW_ADMIN_USER ?? "admin",
  adminPassword: process.env.DATAFLOW_ADMIN_PASSWORD ?? "P@ssw0rd",
  adminEmail: process.env.DATAFLOW_ADMIN_EMAIL ?? "admin@botflow.local",
  /** POST JSON audit events here (Splunk HEC / Datadog / custom). */
  auditWebhookUrl: process.env.AUDIT_WEBHOOK_URL ?? "",
  /** POST JSON run metrics / logs here. */
  metricsWebhookUrl: process.env.METRICS_WEBHOOK_URL ?? "",
  /** OpenTelemetry-compatible HTTP collector base (appends /v1/logs). */
  otelEndpoint: process.env.OTEL_ENDPOINT ?? "",
  /** Upstream Wealth.Api base URL for proxying mirrored endpoints. */
  wealthApiBaseUrl: (process.env.WEALTH_API_BASE_URL ?? "").replace(/\/$/, ""),
  /** Client credentials accepted by mirrored ServiceAuthenticate. */
  wealthClientId: process.env.WEALTH_CLIENT_ID ?? "dataflow-wealth",
  wealthClientSecret: process.env.WEALTH_CLIENT_SECRET ?? "dataflow-wealth-secret",
  /** HS256 secret for wealth API project JWT (align with Wealth when proxying). */
  wealthJwtSecret:
    process.env.WEALTH_JWT_SECRET ?? "dataflow-wealth-jwt-dev-secret",
  /** HS256 secret for Wealth Online API project JWT. */
  wealthOnlineJwtSecret:
    process.env.WEALTH_ONLINE_JWT_SECRET ??
    process.env.WEALTH_JWT_SECRET ??
    "dataflow-wealth-online-jwt-dev-secret",
  /** Upstream Dynamics.Api base URL (Online flows call this). */
  dynamicsApiBaseUrl: (
    process.env.DYNAMICS_API_BASE_URL ??
    "https://uat-online.otto1890.com/Dynamics.Api"
  ).replace(/\/$/, ""),
  /** X-Api-Key accepted by the Dynamics API mirror (matches Dynamics.Api ApiKey). */
  dynamicsApiKey: process.env.DYNAMICS_API_KEY ?? "B3st@pIK3y3v3r",
};
