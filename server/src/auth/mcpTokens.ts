import { createHash, randomBytes } from "node:crypto";
import { getDb } from "../db.js";

export type McpTokenDoc = {
  id: string;
  name: string;
  keyHash: string;
  prefix: string;
  companyName?: string;
  toolAllowlist: string[];
  readonly: boolean;
  createdBy: string;
  createdAt: string;
  expiresAt?: string;
};

function hashKey(raw: string): string {
  return createHash("sha256").update(raw).digest("hex");
}

export async function createMcpToken(input: {
  name: string;
  createdBy: string;
  companyName?: string;
  toolAllowlist?: string[];
  readonly?: boolean;
  expiresAt?: string;
}): Promise<{ token: McpTokenDoc; secret: string }> {
  const secret = `bfmcp_${randomBytes(24).toString("base64url")}`;
  const doc: McpTokenDoc = {
    id: randomBytes(12).toString("hex"),
    name: input.name,
    keyHash: hashKey(secret),
    prefix: secret.slice(0, 12),
    companyName: input.companyName,
    toolAllowlist: input.toolAllowlist ?? [],
    readonly: input.readonly ?? true,
    createdBy: input.createdBy,
    createdAt: new Date().toISOString(),
    expiresAt: input.expiresAt,
  };
  await getDb().collection("mcp_tokens").insertOne(doc);
  return { token: doc, secret };
}

export async function verifyMcpToken(
  raw: string,
): Promise<McpTokenDoc | null> {
  if (!raw) return null;
  const keyHash = hashKey(raw);
  const doc = await getDb().collection("mcp_tokens").findOne({ keyHash });
  if (!doc) return null;
  if (doc.expiresAt && new Date(doc.expiresAt).getTime() < Date.now()) {
    return null;
  }
  return doc as unknown as McpTokenDoc;
}

const MUTATING = new Set([
  "create_workflow",
  "update_workflow",
  "run_workflow",
  "delete_workflow",
]);

export function mcpToolAllowed(
  token: McpTokenDoc,
  toolName: string,
): boolean {
  if (token.readonly && MUTATING.has(toolName)) return false;
  if (!token.toolAllowlist.length) return !token.readonly || !MUTATING.has(toolName);
  return token.toolAllowlist.includes(toolName) || token.toolAllowlist.includes("*");
}
