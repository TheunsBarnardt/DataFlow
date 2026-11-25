import { MongoClient, Db, Collection, ObjectId } from "mongodb";
import type {
  WorkflowDoc,
  RunDoc,
  ScheduleDoc,
  WebhookDoc,
  ThreadDoc,
  McpServerDoc,
  VariableDoc,
  CredentialDoc,
  AuditEventDoc,
  ApiProjectDoc,
  ApiEndpointDoc,
} from "@dataflow/shared";
import { config } from "./config.js";

let client: MongoClient | null = null;
let db: Db | null = null;

export async function connectDb(): Promise<Db> {
  if (db) return db;
  client = new MongoClient(config.mongoUri);
  await client.connect();
  db = client.db();
  await Promise.all([
    db.collection("workflows").createIndex({ name: 1 }),
    db.collection("workflows").createIndex({ apiProjectId: 1, kind: 1 }),
    db.collection("runs").createIndex({ workflowId: 1, startedAt: -1 }),
    db.collection("runs").createIndex({ startedAt: -1 }),
    db.collection("schedules").createIndex({ nextRunAt: 1 }),
    db.collection("webhooks").createIndex({ path: 1 }, { unique: true }),
    db.collection("api_projects").createIndex({ slug: 1 }, { unique: true }),
    db
      .collection("api_endpoints")
      .createIndex(
        { apiProjectId: 1, method: 1, path: 1 },
        { unique: true },
      ),
    db.collection("api_endpoints").createIndex({ workflowId: 1 }),
    db.collection("variables").createIndex({ key: 1 }, { unique: true }),
    db.collection("credentials").createIndex({ name: 1 }),
    db.collection("audit_events").createIndex({ at: -1 }),
    db.collection("audit_events").createIndex({ resourceId: 1, at: -1 }),
  ]);
  return db;
}

export function getDb(): Db {
  if (!db) throw new Error("Database not connected");
  return db;
}

export function getMongoClient(): MongoClient {
  if (!client) throw new Error("Database not connected");
  return client;
}

export async function closeDb(): Promise<void> {
  await client?.close();
  client = null;
  db = null;
}

/** Mongo stored docs: ObjectId _id, not the API string _id. */
export type Stored<T> = Omit<T, "_id"> & { _id: ObjectId };

export type WorkflowColl = Collection<Stored<WorkflowDoc>>;
export type RunColl = Collection<Stored<RunDoc>>;
export type ScheduleColl = Collection<Stored<ScheduleDoc>>;
export type WebhookColl = Collection<Stored<WebhookDoc>>;
export type ThreadColl = Collection<Stored<ThreadDoc>>;
export type McpColl = Collection<Stored<McpServerDoc>>;
export type VariableColl = Collection<Stored<VariableDoc>>;
export type CredentialColl = Collection<Stored<CredentialDoc>>;
export type AuditColl = Collection<Stored<AuditEventDoc>>;
export type ApiProjectColl = Collection<Stored<ApiProjectDoc>>;
export type ApiEndpointColl = Collection<Stored<ApiEndpointDoc>>;

export function workflows(): WorkflowColl {
  return getDb().collection("workflows") as WorkflowColl;
}
export function runs(): RunColl {
  return getDb().collection("runs") as RunColl;
}
export function schedules(): ScheduleColl {
  return getDb().collection("schedules") as ScheduleColl;
}
export function webhooks(): WebhookColl {
  return getDb().collection("webhooks") as WebhookColl;
}
export function threads(): ThreadColl {
  return getDb().collection("threads") as ThreadColl;
}
export function mcpServers(): McpColl {
  return getDb().collection("mcp_servers") as McpColl;
}
export function variables(): VariableColl {
  return getDb().collection("variables") as VariableColl;
}
export function credentials(): CredentialColl {
  return getDb().collection("credentials") as CredentialColl;
}
export function auditEvents(): AuditColl {
  return getDb().collection("audit_events") as AuditColl;
}
export function apiProjects(): ApiProjectColl {
  return getDb().collection("api_projects") as ApiProjectColl;
}
export function apiEndpoints(): ApiEndpointColl {
  return getDb().collection("api_endpoints") as ApiEndpointColl;
}

export function toId(id: string | ObjectId): ObjectId {
  return typeof id === "string" ? new ObjectId(id) : id;
}

export function serialize<T extends { _id?: ObjectId | string }>(
  doc: T,
): T & { _id: string } {
  return { ...doc, _id: String(doc._id) };
}
