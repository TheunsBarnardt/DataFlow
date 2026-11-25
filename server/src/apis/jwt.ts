import { createHmac, timingSafeEqual } from "node:crypto";

export interface JwtPayload {
  [key: string]: unknown;
  exp?: number;
  nbf?: number;
  iat?: number;
}

function b64url(buf: Buffer | string): string {
  const b = typeof buf === "string" ? Buffer.from(buf, "utf8") : buf;
  return b.toString("base64url");
}

export function signJwt(
  payload: JwtPayload,
  secret: string,
  expiresInSeconds = 86400,
): string {
  const now = Math.floor(Date.now() / 1000);
  const body: JwtPayload = {
    ...payload,
    iat: payload.iat ?? now,
    nbf: payload.nbf ?? now,
    exp: payload.exp ?? now + expiresInSeconds,
  };
  const header = b64url(JSON.stringify({ alg: "HS256", typ: "JWT" }));
  const mid = b64url(JSON.stringify(body));
  const sig = createHmac("sha256", secret)
    .update(`${header}.${mid}`)
    .digest("base64url");
  return `${header}.${mid}.${sig}`;
}

export function verifyJwt(
  token: string,
  secret: string,
): { ok: true; payload: JwtPayload } | { ok: false; error: string } {
  const [h, p, s] = token.split(".");
  if (!h || !p || !s) return { ok: false, error: "Invalid JWT" };
  const expected = createHmac("sha256", secret)
    .update(`${h}.${p}`)
    .digest("base64url");
  const a = Buffer.from(s);
  const b = Buffer.from(expected);
  if (a.length !== b.length || !timingSafeEqual(a, b)) {
    return { ok: false, error: "JWT signature invalid" };
  }
  try {
    const payload = JSON.parse(
      Buffer.from(p, "base64url").toString("utf8"),
    ) as JwtPayload;
    const now = Math.floor(Date.now() / 1000);
    if (typeof payload.exp === "number" && now > payload.exp) {
      return { ok: false, error: "JWT expired" };
    }
    if (typeof payload.nbf === "number" && now < payload.nbf) {
      return { ok: false, error: "JWT not yet valid" };
    }
    return { ok: true, payload };
  } catch {
    return { ok: false, error: "Invalid JWT payload" };
  }
}

export function bearerFromRequest(headers: Record<string, unknown>): string {
  const auth = String(headers.authorization ?? headers.Authorization ?? "");
  const m = /^Bearer\s+(.+)$/i.exec(auth.trim());
  return m?.[1]?.trim() ?? "";
}

/** Dynamics.Api-style key from X-Api-Key (case-insensitive header lookup). */
export function apiKeyFromRequest(headers: Record<string, unknown>): string {
  for (const [k, v] of Object.entries(headers)) {
    if (k.toLowerCase() === "x-api-key") return String(v ?? "").trim();
  }
  return "";
}
