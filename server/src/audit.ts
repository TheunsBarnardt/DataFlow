import { ObjectId } from "mongodb";
import type { AuditAction, AuditEventDoc } from "@dataflow/shared";
import { auditEvents, serialize, type Stored } from "./db.js";
import { emitExternal } from "./export.js";
import { config } from "./config.js";

export function resolveActor(header?: string | string[]): string {
  const fromHeader = Array.isArray(header) ? header[0] : header;
  const trimmed = fromHeader?.trim();
  if (trimmed) return trimmed;
  return config.defaultActor;
}

export async function recordAudit(input: {
  action: AuditAction;
  resourceType: "workflow" | "run";
  resourceId: string;
  resourceName?: string;
  actor?: string;
  details?: Record<string, unknown>;
}): Promise<AuditEventDoc & { _id: string }> {
  const doc: Stored<AuditEventDoc> = {
    _id: new ObjectId(),
    action: input.action,
    resourceType: input.resourceType,
    resourceId: input.resourceId,
    resourceName: input.resourceName,
    actor: input.actor?.trim() || config.defaultActor,
    at: new Date().toISOString(),
    details: input.details,
  };
  await auditEvents().insertOne(doc);
  const serialized = serialize(doc);
  emitExternal("audit", {
    event: serialized,
  });
  return serialized;
}
