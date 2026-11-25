import type { FastifyRequest } from "fastify";
import { resolveActor } from "../audit.js";

/** Prefer authenticated session user; fall back to legacy actor header. */
export function actorFromRequest(req: FastifyRequest): string {
  const u = req.user;
  if (u?.username) return u.username;
  if (u?.email) return u.email;
  if (u?.name) return u.name;
  if (u?.id) return u.id;
  return resolveActor(req.headers["x-dataflow-actor"]);
}
