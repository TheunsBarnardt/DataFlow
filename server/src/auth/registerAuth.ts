import type { FastifyInstance, FastifyReply, FastifyRequest } from "fastify";
import { fromNodeHeaders } from "better-auth/node";
import { getAuth } from "./auth.js";

function toWebHeaders(headers: FastifyRequest["headers"]): Headers {
  return fromNodeHeaders(headers);
}

async function forwardAuth(
  request: FastifyRequest,
  reply: FastifyReply,
): Promise<void> {
  const auth = getAuth();
  const url = new URL(request.url, `http://${request.headers.host}`);

  const hasBody =
    request.method !== "GET" &&
    request.method !== "HEAD" &&
    request.body !== undefined &&
    request.body !== null;

  const req = new Request(url.toString(), {
    method: request.method,
    headers: toWebHeaders(request.headers),
    ...(hasBody ? { body: JSON.stringify(request.body) } : {}),
  });

  const response = await auth.handler(req);
  reply.status(response.status);
  response.headers.forEach((value, key) => {
    reply.header(key, value);
  });
  const text = await response.text();
  if (text) {
    try {
      reply.send(JSON.parse(text));
    } catch {
      reply.send(text);
    }
  } else {
    reply.send(null);
  }
}

export async function registerAuthRoutes(app: FastifyInstance): Promise<void> {
  app.route({
    method: ["GET", "POST"],
    url: "/api/auth/*",
    handler: forwardAuth,
  });
  // Fastify sometimes needs the exact prefix without splat for nested paths
  app.route({
    method: ["GET", "POST"],
    url: "/api/auth",
    handler: forwardAuth,
  });
}

export type SessionUser = {
  id: string;
  name: string;
  email: string;
  username?: string | null;
  role?: string | null;
  companyName?: string | null;
  status?: string | null;
  image?: string | null;
};

declare module "fastify" {
  interface FastifyRequest {
    user?: SessionUser | null;
    sessionToken?: string | null;
  }
}

export async function attachSession(
  request: FastifyRequest,
): Promise<SessionUser | null> {
  const auth = getAuth();
  const session = await auth.api.getSession({
    headers: toWebHeaders(request.headers),
  });
  if (!session?.user) {
    request.user = null;
    return null;
  }
  const u = session.user as SessionUser & { username?: string };
  request.user = {
    id: u.id,
    name: u.name,
    email: u.email,
    username: u.username ?? null,
    role: (u as { role?: string }).role ?? "editor",
    companyName: (u as { companyName?: string }).companyName ?? null,
    status: (u as { status?: string }).status ?? "active",
    image: u.image ?? null,
  };
  return request.user;
}

export async function requireSession(
  request: FastifyRequest,
  reply: FastifyReply,
): Promise<SessionUser | null> {
  const user = await attachSession(request);
  if (!user) {
    reply.code(401).send({ error: "Unauthorized" });
    return null;
  }
  if (user.status && user.status !== "active") {
    reply.code(403).send({ error: "Account disabled" });
    return null;
  }
  return user;
}

export function requireAdmin(
  request: FastifyRequest,
  reply: FastifyReply,
): boolean {
  if (request.user?.role !== "admin") {
    reply.code(403).send({ error: "Admin only" });
    return false;
  }
  return true;
}
