import { ObjectId } from "mongodb";
import { credentials } from "../db.js";

export type CredentialPayload = Record<string, unknown>;

export function decryptCredentialData(data: string | undefined): CredentialPayload {
  if (!data) return {};
  try {
    return JSON.parse(Buffer.from(data, "base64").toString("utf8")) as CredentialPayload;
  } catch {
    try {
      return JSON.parse(data) as CredentialPayload;
    } catch {
      return {};
    }
  }
}

function isObjectIdString(value: string): boolean {
  return /^[a-fA-F0-9]{24}$/.test(value);
}

/** Load stored credential by Mongo id. */
export async function loadCredentialById(
  id: string,
): Promise<{ type: string; data: CredentialPayload } | null> {
  if (!id || !isObjectIdString(id)) return null;
  try {
    const doc = await credentials().findOne({ _id: new ObjectId(id) });
    if (!doc) return null;
    return { type: doc.type, data: decryptCredentialData(doc.data) };
  } catch {
    return null;
  }
}

/**
 * Resolve a credential key:
 * 1. Env var by exact key
 * 2. If key looks like ObjectId → load credential and return JSON string of data
 *    or a preferred field (token, accessToken, password, apiKey, connectionString)
 * 3. If key is `id.field` → load id and return that field
 */
export async function resolveCredentialValue(
  key: string,
): Promise<string | undefined> {
  if (!key) return undefined;
  const fromEnv = process.env[key];
  if (fromEnv !== undefined && fromEnv !== "") return fromEnv;

  const dot = key.indexOf(".");
  if (dot > 0) {
    const id = key.slice(0, dot);
    const field = key.slice(dot + 1);
    const cred = await loadCredentialById(id);
    if (!cred) return undefined;
    const v = cred.data[field];
    return v == null ? undefined : String(v);
  }

  const cred = await loadCredentialById(key);
  if (!cred) return undefined;

  const preferred = [
    "token",
    "accessToken",
    "apiKey",
    "password",
    "secret",
    "connectionString",
    "bearerToken",
    "value",
  ];
  for (const p of preferred) {
    if (cred.data[p] != null && String(cred.data[p]) !== "") {
      return String(cred.data[p]);
    }
  }
  // Fall back to JSON for multi-field consumers that parse it
  return JSON.stringify(cred.data);
}

export async function resolveCredentialFields(
  credentialId: string | undefined,
): Promise<CredentialPayload> {
  if (!credentialId) return {};
  const cred = await loadCredentialById(credentialId);
  return cred?.data ?? {};
}
