export type NodeType =
  // Triggers
  | "webhook"
  | "schedule"
  | "manual"
  | "rabbitmqTrigger"
  | "redisTrigger"
  | "outlookTrigger"
  | "teamsTrigger"
  | "whatsappTrigger"
  | "formTrigger"
  | "errorTrigger"
  | "localFileTrigger"
  | "sseTrigger"
  | "rssFeedTrigger"
  | "emailTriggerImap"
  | "executeWorkflowTrigger"
  | "workflowTrigger"
  | "n8nTrigger"
  | "interval"
  | "activationTrigger"
  | "simulateTrigger"
  | "mcpServerTrigger"
  | "evaluationTrigger"
  | "apiEndpoint"
  // Control / flow
  | "if"
  | "switch"
  | "merge"
  | "wait"
  | "mcpCall"
  | "noOp"
  | "stopAndError"
  | "stickyNote"
  | "executeWorkflow"
  | "executeCommand"
  // API / helpers
  | "httpRequest"
  | "webhookRespond"
  | "apiRespond"
  | "graphql"
  | "jwt"
  | "n8nApi"
  | "form"
  | "rssRead"
  | "sendEmail"
  | "ftp"
  | "ssh"
  | "git"
  | "ldap"
  // Microsoft / Dynamics
  | "mssql"
  | "microsoftGraph"
  | "outlook"
  | "excelOnline"
  | "sharepoint"
  | "dynamics"
  // Files
  | "readFile"
  | "writeFile"
  | "listFiles"
  | "moveFile"
  | "copyFile"
  | "deleteFile"
  | "csv"
  | "json"
  | "xml"
  | "excel"
  | "zip"
  | "compression"
  | "convertToFile"
  | "extractFromFile"
  | "readPdf"
  | "moveBinaryData"
  | "editImage"
  | "icalendar"
  // Data
  | "set"
  | "editFields"
  | "filter"
  | "sort"
  | "limit"
  | "splitInBatches"
  | "splitOut"
  | "aggregate"
  | "summarize"
  | "itemLists"
  | "renameKeys"
  | "removeDuplicates"
  | "compareDatasets"
  | "code"
  | "aiTransform"
  | "dateTime"
  | "crypto"
  | "html"
  | "htmlExtract"
  | "markdown"
  | "totp"
  | "timeSaved"
  | "dataTable"
  | "executionData"
  | "simulate"
  | "debugHelper"
  | "evaluation";

export interface NodeSettings {
  alwaysOutputData?: boolean;
  executeOnce?: boolean;
  retryOnFail?: boolean;
  /** Max attempts when retryOnFail is true (default 3). */
  maxTries?: number;
  /** Milliseconds between retry attempts (default 1000). */
  waitBetweenTries?: number;
  onError?: "stopWorkflow" | "continue" | "continueErrorOutput";
  notes?: string;
  displayNoteInFlow?: boolean;
  /** Soft-disable without deleting the node */
  disabled?: boolean;
}

export interface WorkflowNode {
  id: string;
  type: NodeType;
  name: string;
  position: { x: number; y: number };
  parameters: Record<string, unknown>;
  settings?: NodeSettings;
}

export interface WorkflowEdge {
  id: string;
  source: string;
  target: string;
  sourceHandle?: string;
  targetHandle?: string;
}

export type WorkflowKind = "workflow" | "apiEndpoint";

export interface WorkflowDoc {
  _id?: string;
  name: string;
  description?: string;
  enabled: boolean;
  version: number;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  /** Labels for filtering / organizing workflows. */
  tags?: string[];
  /** Fixed minutes saved per successful production run (ROI). */
  timeSavedMinutes?: number;
  /** Default workflow; apiEndpoint belongs to an ApiProject. */
  kind?: WorkflowKind;
  /** Parent API project when kind === "apiEndpoint". */
  apiProjectId?: string;
  createdBy?: string;
  updatedBy?: string;
  /** Better Auth user id of the owner (personal flows). */
  ownerId?: string;
  visibility?: "personal" | "shared";
  createdAt: string;
  updatedAt: string;
}

/** JSON Schema subset used for OpenAPI request/response docs. */
export type JsonSchema = Record<string, unknown>;

export type ApiHttpMethod =
  | "GET"
  | "POST"
  | "PUT"
  | "PATCH"
  | "DELETE"
  | "HEAD";

export type ApiAuthMode = "none" | "jwt" | "headerBearer" | "apiKey";

export interface ApiProjectAuth {
  mode: "none" | "jwt";
  /** HS256 secret for JWT (project-level). */
  jwtSecret?: string;
  /** OpenAPI security scheme name (default Bearer). */
  schemeName?: string;
}

export interface ApiProjectDoc {
  _id?: string;
  name: string;
  /** URL slug under /apis/:slug */
  slug: string;
  description?: string;
  version: string;
  enabled: boolean;
  auth: ApiProjectAuth;
  /** Shared named JSON schemas for OpenAPI components.schemas */
  schemas?: Record<string, JsonSchema>;
  createdBy?: string;
  updatedBy?: string;
  createdAt: string;
  updatedAt: string;
}

/** Synced registry row for one API endpoint flow. */
export interface ApiEndpointDoc {
  _id?: string;
  apiProjectId: string;
  workflowId: string;
  nodeId?: string;
  /** Relative path under the project (e.g. Auth/ServiceAuthenticate). */
  path: string;
  method: ApiHttpMethod;
  summary?: string;
  tag?: string;
  auth: ApiAuthMode;
  operationId?: string;
  requestQuerySchema?: JsonSchema;
  requestBodySchema?: JsonSchema;
  responseSchema?: JsonSchema;
  responseStatus?: number;
}

export type RunStatus =
  | "queued"
  | "running"
  | "waiting"
  | "completed"
  | "failed"
  | "cancelled";

export interface NodeRunLog {
  nodeId: string;
  nodeName: string;
  status: RunStatus;
  startedAt?: string;
  finishedAt?: string;
  input?: unknown;
  output?: unknown;
  error?: string;
}

export type RunTrigger =
  | "manual"
  | "step"
  | "schedule"
  | "interval"
  | "webhook"
  | "mcp"
  | "agent"
  | "rabbitmq"
  | "redis"
  | "outlook"
  | "teams"
  | "whatsapp"
  | "form"
  | "error"
  | "sse"
  | "localFile"
  | "rss"
  | "email"
  | "executeWorkflow"
  | "activation"
  | "simulate"
  | "mcpServer"
  | "evaluation"
  | "api"
  | "n8n"
  | "workflow";

export interface RunDoc {
  _id?: string;
  workflowId: string;
  workflowName: string;
  status: RunStatus;
  trigger: RunTrigger;
  startedAt: string;
  finishedAt?: string;
  input?: unknown;
  output?: unknown;
  error?: string;
  logs: NodeRunLog[];
  /** Minutes saved for this run (fixed workflow setting and/or timeSaved nodes). */
  timeSavedMinutes?: number;
  actor?: string;
}

export type AuditAction =
  | "workflow.created"
  | "workflow.updated"
  | "workflow.deleted"
  | "run.started"
  | "run.completed"
  | "run.failed";

export interface AuditEventDoc {
  _id?: string;
  action: AuditAction;
  resourceType: "workflow" | "run";
  resourceId: string;
  resourceName?: string;
  actor: string;
  at: string;
  details?: Record<string, unknown>;
}

export interface InsightsMetric {
  value: number;
  previous?: number;
  unit: "count" | "ratio" | "minute" | "millisecond";
}

export interface InsightsByWorkflow {
  workflowId: string;
  workflowName: string;
  total: number;
  failed: number;
  failureRate: number;
  timeSavedMinutes: number;
  averageRunTimeMs: number;
}

export interface InsightsByTime {
  /** ISO date (UTC day start) */
  date: string;
  total: number;
  failed: number;
  failureRate: number;
  timeSavedMinutes: number;
  averageRunTimeMs: number;
}

export interface InsightsSummary {
  from: string;
  to: string;
  total: InsightsMetric;
  failed: InsightsMetric;
  failureRate: InsightsMetric;
  timeSaved: InsightsMetric;
  averageRunTime: InsightsMetric;
  byWorkflow: InsightsByWorkflow[];
  byTime: InsightsByTime[];
}

export interface ScheduleDoc {
  _id?: string;
  workflowId: string;
  /** Canvas schedule/interval node that owns this schedule row */
  nodeId?: string;
  name: string;
  cron?: string;
  intervalMinutes?: number;
  paused: boolean;
  lastRunAt?: string;
  nextRunAt?: string;
}

export interface WebhookDoc {
  _id?: string;
  workflowId: string;
  /** Canvas node id that owns this production hook */
  nodeId?: string;
  path: string;
  secret: string;
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD";
  /** none = open; headerBearer = require Bearer / ?secret= */
  authentication?: "none" | "headerBearer";
  /** Optional inbound kind for Graph/Meta/form triggers */
  kind?:
    | "webhook"
    | "outlook"
    | "teams"
    | "whatsapp"
    | "form"
    | "sse";
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  toolName?: string;
  createdAt: string;
}

export interface ThreadDoc {
  _id?: string;
  title: string;
  messages: ChatMessage[];
  createdAt: string;
  updatedAt: string;
}

export interface McpServerDoc {
  _id?: string;
  name: string;
  transport: "stdio" | "http";
  command?: string;
  args?: string[];
  url?: string;
  env?: Record<string, string>;
  enabled: boolean;
}

export interface VariableDoc {
  _id?: string;
  key: string;
  value: string;
  type: "string" | "number" | "boolean" | "json";
  description?: string;
  updatedAt: string;
  createdAt: string;
}

/** Stored credential — `data` is encrypted-at-rest and never returned in list APIs. */
export interface CredentialDoc {
  _id?: string;
  name: string;
  type: string;
  /** Encrypted payload (server-only; omit from list responses). */
  data?: string;
  createdAt: string;
  updatedAt: string;
}

export const DAY_ONE_NODE_TYPES: {
  type: NodeType;
  label: string;
  category: string;
}[] = [
  // Triggers
  { type: "webhook", label: "Webhook", category: "trigger" },
  { type: "schedule", label: "Schedule", category: "trigger" },
  { type: "manual", label: "Manual", category: "trigger" },
  { type: "rabbitmqTrigger", label: "RabbitMQ Trigger", category: "trigger" },
  { type: "redisTrigger", label: "Redis Trigger", category: "trigger" },
  { type: "outlookTrigger", label: "Microsoft Outlook Trigger", category: "trigger" },
  { type: "teamsTrigger", label: "Microsoft Teams Trigger", category: "trigger" },
  { type: "whatsappTrigger", label: "WhatsApp Trigger", category: "trigger" },
  { type: "formTrigger", label: "Form Trigger", category: "trigger" },
  { type: "errorTrigger", label: "Error Trigger", category: "trigger" },
  { type: "localFileTrigger", label: "Local File Trigger", category: "trigger" },
  { type: "sseTrigger", label: "SSE Trigger", category: "trigger" },
  { type: "rssFeedTrigger", label: "RSS Feed Trigger", category: "trigger" },
  { type: "emailTriggerImap", label: "Email Trigger (IMAP)", category: "trigger" },
  { type: "executeWorkflowTrigger", label: "Execute Workflow Trigger", category: "trigger" },
  { type: "workflowTrigger", label: "Workflow Trigger", category: "trigger" },
  { type: "n8nTrigger", label: "n8n Trigger", category: "trigger" },
  { type: "interval", label: "Interval", category: "trigger" },
  { type: "activationTrigger", label: "Activation Trigger", category: "trigger" },
  { type: "simulateTrigger", label: "Simulate Trigger", category: "trigger" },
  { type: "mcpServerTrigger", label: "MCP Server Trigger", category: "trigger" },
  { type: "evaluationTrigger", label: "Evaluation Trigger", category: "trigger" },
  { type: "apiEndpoint", label: "API Endpoint", category: "trigger" },
  // Control
  { type: "if", label: "IF", category: "control" },
  { type: "switch", label: "Switch", category: "control" },
  { type: "merge", label: "Merge", category: "control" },
  { type: "wait", label: "Wait", category: "control" },
  { type: "mcpCall", label: "MCP Call", category: "control" },
  { type: "noOp", label: "No Operation", category: "control" },
  { type: "stopAndError", label: "Stop and Error", category: "control" },
  { type: "stickyNote", label: "Sticky Note", category: "control" },
  { type: "executeWorkflow", label: "Execute Sub-workflow", category: "control" },
  { type: "executeCommand", label: "Execute Command", category: "control" },
  // API / helpers
  { type: "httpRequest", label: "HTTP Request", category: "api" },
  { type: "webhookRespond", label: "Respond to Webhook", category: "api" },
  { type: "apiRespond", label: "Respond to API", category: "api" },
  { type: "graphql", label: "GraphQL", category: "api" },
  { type: "jwt", label: "JWT", category: "api" },
  { type: "n8nApi", label: "n8n", category: "api" },
  { type: "form", label: "Form", category: "api" },
  { type: "rssRead", label: "RSS Read", category: "api" },
  { type: "sendEmail", label: "Send Email", category: "api" },
  { type: "ftp", label: "FTP", category: "api" },
  { type: "ssh", label: "SSH", category: "api" },
  { type: "git", label: "Git", category: "api" },
  { type: "ldap", label: "LDAP", category: "api" },
  // Microsoft
  { type: "mssql", label: "MSSQL", category: "microsoft" },
  { type: "microsoftGraph", label: "Microsoft Graph", category: "microsoft" },
  { type: "outlook", label: "Outlook", category: "microsoft" },
  { type: "excelOnline", label: "Excel Online", category: "microsoft" },
  { type: "sharepoint", label: "SharePoint", category: "microsoft" },
  { type: "dynamics", label: "Dataverse", category: "microsoft" },
  // Files
  { type: "readFile", label: "Read File", category: "files" },
  { type: "writeFile", label: "Write File", category: "files" },
  { type: "listFiles", label: "List Files", category: "files" },
  { type: "moveFile", label: "Move File", category: "files" },
  { type: "copyFile", label: "Copy File", category: "files" },
  { type: "deleteFile", label: "Delete File", category: "files" },
  { type: "csv", label: "CSV", category: "files" },
  { type: "json", label: "JSON", category: "files" },
  { type: "xml", label: "XML", category: "files" },
  { type: "excel", label: "Excel File", category: "files" },
  { type: "zip", label: "Zip", category: "files" },
  { type: "compression", label: "Compression", category: "files" },
  { type: "convertToFile", label: "Convert to File", category: "files" },
  { type: "extractFromFile", label: "Extract from File", category: "files" },
  { type: "readPdf", label: "Read PDF", category: "files" },
  { type: "moveBinaryData", label: "Move Binary Data", category: "files" },
  { type: "editImage", label: "Edit Image", category: "files" },
  { type: "icalendar", label: "iCalendar", category: "files" },
  // Data
  { type: "set", label: "Edit Fields (Set)", category: "data" },
  { type: "editFields", label: "Edit Fields", category: "data" },
  { type: "filter", label: "Filter", category: "data" },
  { type: "sort", label: "Sort", category: "data" },
  { type: "limit", label: "Limit", category: "data" },
  { type: "splitInBatches", label: "Loop Over Items", category: "data" },
  { type: "splitOut", label: "Split Out", category: "data" },
  { type: "aggregate", label: "Aggregate", category: "data" },
  { type: "summarize", label: "Summarize", category: "data" },
  { type: "itemLists", label: "Item Lists", category: "data" },
  { type: "renameKeys", label: "Rename Keys", category: "data" },
  { type: "removeDuplicates", label: "Remove Duplicates", category: "data" },
  { type: "compareDatasets", label: "Compare Datasets", category: "data" },
  { type: "code", label: "Code", category: "data" },
  { type: "aiTransform", label: "AI Transform", category: "data" },
  { type: "dateTime", label: "Date & Time", category: "data" },
  { type: "crypto", label: "Crypto", category: "data" },
  { type: "html", label: "HTML", category: "data" },
  { type: "htmlExtract", label: "HTML Extract", category: "data" },
  { type: "markdown", label: "Markdown", category: "data" },
  { type: "totp", label: "TOTP", category: "data" },
  { type: "timeSaved", label: "Track Time Saved", category: "data" },
  { type: "dataTable", label: "Data Table", category: "data" },
  { type: "executionData", label: "Execution Data", category: "data" },
  { type: "simulate", label: "Simulate", category: "data" },
  { type: "debugHelper", label: "Debug Helper", category: "data" },
  { type: "evaluation", label: "Evaluation", category: "data" },
];
