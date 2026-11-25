import fs from "node:fs";
import path from "node:path";
import Fastify from "fastify";
import cors from "@fastify/cors";
import fastifyStatic from "@fastify/static";
import { config } from "./config.js";
import { connectDb, closeDb } from "./db.js";
import { registerApiRoutes } from "./routes/api.js";
import { registerApiProjectRoutes } from "./apis/runtime.js";
import { ensureWealthApiMirror } from "./apis/wealth/seed.js";
import { ensureWealthOnlineApiMirror } from "./apis/wealthOnline/seed.js";
import { ensureDynamicsApiMirror } from "./apis/dynamics/seed.js";
import { refreshSchedules } from "./scheduler.js";
import {
  startTriggerListeners,
  stopTriggerListeners,
} from "./triggers/listeners.js";
import { getAuth } from "./auth/auth.js";
import { ensureBootstrapAdmin } from "./auth/seedAdmin.js";
import {
  registerAuthRoutes,
  requireSession,
} from "./auth/registerAuth.js";
import { registerMeRoutes } from "./auth/meRoutes.js";
import { registerAuthenticationRoutes } from "./auth/authenticationRoutes.js";

async function main() {
  await connectDb();
  // Initialize Better Auth after DB is ready
  getAuth();
  await ensureBootstrapAdmin();
  await refreshSchedules();
  await ensureWealthApiMirror();
  await ensureWealthOnlineApiMirror();
  await ensureDynamicsApiMirror();

  const app = Fastify({
    logger: true,
    // Allow webhook "Listen for test event" to wait up to 2 minutes
    connectionTimeout: 0,
    requestTimeout: 130_000,
  });
  await app.register(cors, {
    origin: true,
    credentials: true,
  });
  // Tolerate empty / missing content-type on POST (PowerShell, curl -X POST)
  app.removeAllContentTypeParsers();
  app.addContentTypeParser(
    ["application/json", "text/json", "*"],
    { parseAs: "string" },
    (_req, body, done) => {
      try {
        const raw = body === "" || body === undefined ? "{}" : String(body);
        const trimmed = raw.trim();
        if (!trimmed) return done(null, {});
        done(null, JSON.parse(trimmed));
      } catch (err) {
        done(err as Error, undefined);
      }
    },
  );

  await registerAuthRoutes(app);
  await registerMeRoutes(app);
  await registerAuthenticationRoutes(app);

  app.addHook("preHandler", async (req, reply) => {
    const pathOnly = req.url.split("?")[0] ?? "";
    if (!pathOnly.startsWith("/api/")) return;
    if (pathOnly === "/api/health" || pathOnly.startsWith("/api/auth")) return;
    if (
      pathOnly === "/api/me" ||
      pathOnly.startsWith("/api/users") ||
      pathOnly.startsWith("/api/settings/authentication")
    ) {
      // Handled inside those routes (still require session there)
      return;
    }
    const user = await requireSession(req, reply);
    if (!user) return;
  });

  await registerApiRoutes(app);
  await registerApiProjectRoutes(app);
  await startTriggerListeners();

  const webDist = config.webDist;
  if (fs.existsSync(webDist)) {
    await app.register(fastifyStatic, {
      root: webDist,
      prefix: "/",
    });
    app.setNotFoundHandler((req, reply) => {
      if (
        req.url.startsWith("/api") ||
        req.url.startsWith("/apis") ||
        req.url.startsWith("/hooks") ||
        req.url.startsWith("/hooks-test")
      ) {
        return reply.code(404).send({ error: "Not found" });
      }
      return reply.sendFile("index.html");
    });
  } else {
    app.get("/", async () => ({
      name: "DataFlow",
      message: "API up. Build the web UI with npm run build -w web",
      health: "/api/health",
    }));
  }

  await app.listen({ host: config.host, port: config.port });
  console.log(`DataFlow listening on http://${config.host}:${config.port}`);

  const shutdown = async () => {
    await stopTriggerListeners();
    await app.close();
    await closeDb();
    process.exit(0);
  };
  process.on("SIGINT", () => void shutdown());
  process.on("SIGTERM", () => void shutdown());
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
