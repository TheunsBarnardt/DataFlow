import type { FastifyInstance } from "fastify";
import { fromNodeHeaders } from "better-auth/node";
import { getDb } from "../db.js";
import { getAuth } from "./auth.js";
import { requireSession, requireAdmin, attachSession } from "./registerAuth.js";
import { createMcpToken } from "./mcpTokens.js";

function headersOf(req: { headers: Record<string, unknown> | object }) {
  return fromNodeHeaders(req.headers as Parameters<typeof fromNodeHeaders>[0]);
}

export async function registerMeRoutes(app: FastifyInstance): Promise<void> {
  app.get("/api/me", async (req, reply) => {
    const user = await requireSession(req, reply);
    if (!user) return;
    return { user };
  });

  app.patch<{
    Body: { name?: string; companyName?: string };
  }>("/api/me", async (req, reply) => {
    const user = await requireSession(req, reply);
    if (!user) return;
    const name = req.body?.name?.trim();
    const companyName = req.body?.companyName?.trim();
    const $set: Record<string, unknown> = {};
    if (name) $set.name = name;
    if (companyName !== undefined) $set.companyName = companyName;
    if (Object.keys($set).length === 0) {
      return { user };
    }
    await getDb()
      .collection("user")
      .updateOne({ id: user.id }, { $set });
    const updated = await attachSession(req);
    return { user: updated ?? { ...user, ...$set } };
  });

  app.get("/api/users", async (req, reply) => {
    const user = await requireSession(req, reply);
    if (!user) return;
    if (!requireAdmin(req, reply)) return;

    const auth = getAuth();
    const result = await auth.api.listUsers({
      query: {
        limit: 100,
        sortBy: "createdAt",
        sortDirection: "desc",
      },
      headers: headersOf(req),
    });
    return { users: result?.users ?? [] };
  });

  app.post<{
    Body: {
      email: string;
      password: string;
      name: string;
      username: string;
      role?: string;
      companyName?: string;
    };
  }>("/api/users", async (req, reply) => {
    const user = await requireSession(req, reply);
    if (!user) return;
    if (!requireAdmin(req, reply)) return;

    const body = req.body;
    if (!body?.email || !body?.password || !body?.name || !body?.username) {
      return reply.code(400).send({ error: "Missing required fields" });
    }

    const auth = getAuth();
    const role = body.role === "admin" ? "admin" : "user";
    try {
      const created = await auth.api.createUser({
        body: {
          email: body.email,
          password: body.password,
          name: body.name,
          role,
          data: {
            username: body.username,
            companyName: body.companyName ?? "",
            status: "active",
          },
        },
        headers: headersOf(req),
      });

      // Ensure username + DataFlow fields on the user document
      await getDb()
        .collection("user")
        .updateOne(
          { id: created.user.id },
          {
            $set: {
              username: body.username.trim().toLowerCase(),
              displayUsername: body.username.trim(),
              companyName: body.companyName ?? "",
              status: "active",
              role: body.role ?? "editor",
            },
          },
        );

      return { user: created.user };
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      return reply.code(400).send({ error: msg });
    }
  });

  app.patch<{
    Params: { id: string };
    Body: {
      role?: string;
      status?: string;
      companyName?: string;
      name?: string;
    };
  }>("/api/users/:id", async (req, reply) => {
    const user = await requireSession(req, reply);
    if (!user) return;
    if (!requireAdmin(req, reply)) return;

    const $set: Record<string, unknown> = {};
    if (req.body.role) $set.role = req.body.role;
    if (req.body.status) $set.status = req.body.status;
    if (req.body.companyName !== undefined)
      $set.companyName = req.body.companyName;
    if (req.body.name) $set.name = req.body.name.trim();

    if (Object.keys($set).length === 0) {
      return reply.code(400).send({ error: "No updates" });
    }

    await getDb()
      .collection("user")
      .updateOne({ id: req.params.id }, { $set });

    if (req.body.role === "admin" || req.body.role) {
      try {
        const auth = getAuth();
        await auth.api.setRole({
          body: {
            userId: req.params.id,
            role: req.body.role === "admin" ? "admin" : "user",
          },
          headers: headersOf(req),
        });
      } catch {
        /* DataFlow role stored on user doc; admin plugin role is best-effort */
      }
    }

    const updated = await getDb()
      .collection("user")
      .findOne({ id: req.params.id });
    return { user: updated };
  });

  app.get("/api/mcp-tokens", async (req, reply) => {
    const user = await requireSession(req, reply);
    if (!user) return;
    if (!requireAdmin(req, reply)) return;
    const list = await getDb()
      .collection("mcp_tokens")
      .find({})
      .project({ keyHash: 0 })
      .sort({ createdAt: -1 })
      .toArray();
    return { tokens: list };
  });

  app.post<{
    Body: {
      name: string;
      companyName?: string;
      toolAllowlist?: string[];
      readonly?: boolean;
    };
  }>("/api/mcp-tokens", async (req, reply) => {
    const user = await requireSession(req, reply);
    if (!user) return;
    if (!requireAdmin(req, reply)) return;
    const name = req.body?.name?.trim();
    if (!name) return reply.code(400).send({ error: "name required" });
    const { token, secret } = await createMcpToken({
      name,
      createdBy: user.id,
      companyName: req.body.companyName,
      toolAllowlist: req.body.toolAllowlist,
      readonly: req.body.readonly ?? true,
    });
    const { keyHash: _, ...safe } = token;
    return { token: safe, secret };
  });

  app.delete<{ Params: { id: string } }>(
    "/api/mcp-tokens/:id",
    async (req, reply) => {
      const user = await requireSession(req, reply);
      if (!user) return;
      if (!requireAdmin(req, reply)) return;
      await getDb().collection("mcp_tokens").deleteOne({ id: req.params.id });
      return { ok: true };
    },
  );
}
