import type { NodeType } from "@dataflow/shared";

export type FieldKind =
  | "string"
  | "text"
  | "number"
  | "boolean"
  | "select"
  | "json"
  | "jsonSchema"
  | "code"
  | "keyValue"
  | "stringList"
  | "conditions"
  | "assignments"
  | "switchRules";

export type FieldLanguage = "sql" | "json" | "js" | "text";

export type FieldDef = {
  key: string;
  label: string;
  kind: FieldKind;
  description?: string;
  placeholder?: string;
  default?: unknown;
  options?: { value: string; label: string }[];
  /** Language for code fields (enables Format) */
  language?: FieldLanguage;
  /** Show when another field matches */
  when?: { key: string; equals: string | string[] };
  /** When set, CredentialsPicker filters to this credential type */
  credentialType?: string;
};

export type NodeSchema = {
  type: NodeType;
  description: string;
  fields: FieldDef[];
};

const METHOD_OPTIONS = [
  { value: "GET", label: "GET" },
  { value: "POST", label: "POST" },
  { value: "PUT", label: "PUT" },
  { value: "PATCH", label: "PATCH" },
  { value: "DELETE", label: "DELETE" },
  { value: "HEAD", label: "HEAD" },
];

export const COMPARE_OPS = [
  { value: "equals", label: "Equals" },
  { value: "notEquals", label: "Not equals" },
  { value: "contains", label: "Contains" },
  { value: "notContains", label: "Not contains" },
  { value: "startsWith", label: "Starts with" },
  { value: "endsWith", label: "Ends with" },
  { value: "regex", label: "Regex" },
  { value: "exists", label: "Exists" },
  { value: "notExists", label: "Does not exist" },
  { value: "empty", label: "Is empty" },
  { value: "notEmpty", label: "Is not empty" },
  { value: "truthy", label: "Truthy" },
  { value: "falsy", label: "Falsy" },
  { value: "gt", label: "Greater than" },
  { value: "gte", label: "Greater or equal" },
  { value: "lt", label: "Less than" },
  { value: "lte", label: "Less or equal" },
];

const FILTER_OPS = COMPARE_OPS;

const SCHEMAS: Partial<Record<NodeType, Omit<NodeSchema, "type">>> = {
  // —— Triggers ——
  apiEndpoint: {
    description:
      "HTTP endpoint trigger for an API project. Synced into OpenAPI/Swagger.",
    fields: [
      {
        key: "path",
        label: "Path",
        kind: "string",
        placeholder: "Auth/ServiceAuthenticate",
        description: "Relative path under /apis/:slug/",
        default: "",
      },
      {
        key: "method",
        label: "HTTP Method",
        kind: "select",
        options: METHOD_OPTIONS,
        default: "GET",
      },
      {
        key: "auth",
        label: "Authentication",
        kind: "select",
        options: [
          { value: "none", label: "None (AllowAnonymous)" },
          { value: "jwt", label: "JWT Bearer" },
          { value: "headerBearer", label: "Shared Bearer secret" },
          { value: "apiKey", label: "API Key (X-Api-Key)" },
        ],
        default: "jwt",
        description:
          "JWT = signed-in user session; API Key = X-Api-Key (scoped keys / project secret); Shared Bearer = project secret.",
      },
      {
        key: "tag",
        label: "Swagger tag",
        kind: "string",
        placeholder: "Auth",
        default: "",
      },
      {
        key: "summary",
        label: "Summary",
        kind: "string",
        default: "",
      },
      {
        key: "operationId",
        label: "Operation ID",
        kind: "string",
        default: "",
      },
      {
        key: "requestQuerySchema",
        label: "Query Params",
        kind: "jsonSchema",
        description:
          "Postman-style query params (Key / Type / Value). Values appear in Input as $json.query.* for mapping.",
        default: { type: "object", properties: {}, required: [] },
      },
      {
        key: "requestBodySchema",
        label: "Body",
        kind: "jsonSchema",
        description:
          "Postman-style body fields (Key / Type / Value). Values appear in Input as $json.body.* for mapping.",
        default: { type: "object", properties: {}, required: [] },
      },
    ],
  },
  webhook: {
    description: "Starts the workflow when an HTTP webhook is hit.",
    fields: [
      {
        key: "path",
        label: "Path",
        kind: "string",
        placeholder: "my-hook",
        description: "URL path segment. Leave blank to auto-generate.",
        default: "",
      },
      {
        key: "method",
        label: "HTTP Method",
        kind: "select",
        options: METHOD_OPTIONS,
        default: "POST",
      },
      {
        key: "authentication",
        label: "Authentication",
        kind: "select",
        options: [
          { value: "none", label: "None" },
          { value: "headerBearer", label: "Header auth (Bearer)" },
          { value: "basic", label: "Basic auth" },
        ],
        default: "none",
      },
      {
        key: "secret",
        label: "Secret",
        kind: "string",
        description: "Required when using Header auth. Pass as Bearer token or ?secret=.",
        default: "",
        when: { key: "authentication", equals: "headerBearer" },
      },
      {
        key: "respond",
        label: "Respond",
        kind: "select",
        options: [
          { value: "immediately", label: "Immediately" },
          { value: "lastNode", label: "When Last Node Finishes" },
          { value: "respondNode", label: "Using Respond to Webhook Node" },
        ],
        default: "immediately",
      },
      {
        key: "responseCode",
        label: "Response Code",
        kind: "number",
        default: 200,
        when: { key: "respond", equals: ["immediately", "lastNode"] },
      },
      {
        key: "responseData",
        label: "Response data",
        kind: "select",
        options: [
          { value: "firstEntryJson", label: "First entry JSON" },
          { value: "allEntries", label: "All entries" },
          { value: "noData", label: "No data" },
        ],
        default: "firstEntryJson",
        when: { key: "respond", equals: ["immediately", "lastNode"] },
      },
      {
        key: "responseHeaders",
        label: "Response headers",
        kind: "keyValue",
        default: {},
      },
    ],
  },
  schedule: {
    description: "Runs on a cron expression or fixed interval.",
    fields: [
      { key: "cron", label: "Cron", kind: "string", placeholder: "*/5 * * * *", default: "" },
      {
        key: "intervalMinutes",
        label: "Interval (minutes)",
        kind: "number",
        default: 5,
      },
    ],
  },
  manual: {
    description: "Starts when you click Execute workflow.",
    fields: [],
  },
  rabbitmqTrigger: {
    description: "Consumes messages from a RabbitMQ queue.",
    fields: [
      { key: "queue", label: "Queue", kind: "string", default: "dataflow", placeholder: "dataflow" },
    ],
  },
  redisTrigger: {
    description: "Subscribes to a Redis pub/sub channel.",
    fields: [
      {
        key: "channel",
        label: "Channel",
        kind: "string",
        default: "dataflow",
        placeholder: "dataflow",
      },
    ],
  },
  outlookTrigger: {
    description: "Fires on Outlook mailbox events (via inbound webhook).",
    fields: [
      {
        key: "credentialId",
        label: "Microsoft credential",
        kind: "string",
        default: "",
        credentialType: "microsoftOutlookOAuth2Api",
      },
      {
        key: "event",
        label: "Event",
        kind: "select",
        options: [
          { value: "messageReceived", label: "Message received" },
          { value: "messageUpdated", label: "Message updated" },
        ],
        default: "messageReceived",
      },
      {
        key: "path",
        label: "Webhook path",
        kind: "string",
        default: "",
        description: "Auto-generated if blank; used for Graph subscription URL",
      },
    ],
  },
  teamsTrigger: {
    description: "Fires on Microsoft Teams channel events.",
    fields: [
      {
        key: "credentialId",
        label: "Microsoft credential",
        kind: "string",
        default: "",
        credentialType: "microsoftTeamsOAuth2Api",
      },
      {
        key: "event",
        label: "Event",
        kind: "select",
        options: [
          { value: "newChannelMessage", label: "New channel message" },
          { value: "memberAdded", label: "Member added" },
        ],
        default: "newChannelMessage",
      },
      {
        key: "path",
        label: "Webhook path",
        kind: "string",
        default: "",
      },
    ],
  },
  whatsappTrigger: {
    description: "Receives WhatsApp / Meta webhook events.",
    fields: [
      {
        key: "event",
        label: "Event",
        kind: "select",
        options: [{ value: "messages", label: "Messages" }],
        default: "messages",
      },
    ],
  },
  formTrigger: {
    description: "Starts when a form submission is received.",
    fields: [
      { key: "formId", label: "Form ID", kind: "string", default: "form" },
    ],
  },
  errorTrigger: {
    description: "Runs when another workflow execution errors.",
    fields: [],
  },
  localFileTrigger: {
    description: "Watches a local path for file changes.",
    fields: [
      { key: "path", label: "Watch path", kind: "string", placeholder: "C:\\data\\inbox", default: "" },
    ],
  },
  sseTrigger: {
    description: "Starts from a Server-Sent Events inbound hook.",
    fields: [],
  },
  rssFeedTrigger: {
    description: "Polls an RSS feed for new items.",
    fields: [
      { key: "url", label: "Feed URL", kind: "string", placeholder: "https://…/feed.xml", default: "" },
    ],
  },
  emailTriggerImap: {
    description: "Triggers on new IMAP email (configured via credentials).",
    fields: [
      { key: "credentialId", label: "IMAP credential", kind: "string", default: "" },
      { key: "mailbox", label: "Mailbox", kind: "string", default: "INBOX" },
    ],
  },
  executeWorkflowTrigger: {
    description: "Entry point when this workflow is called by Execute Sub-workflow.",
    fields: [],
  },
  workflowTrigger: {
    description: "Starts when another workflow signals this one.",
    fields: [],
  },
  n8nTrigger: {
    description: "Bridge trigger for inbound n8n events.",
    fields: [],
  },
  interval: {
    description: "Fires repeatedly every N minutes.",
    fields: [
      {
        key: "intervalMinutes",
        label: "Interval (minutes)",
        kind: "number",
        default: 1,
      },
    ],
  },
  activationTrigger: {
    description: "Fires when the workflow is activated (enabled).",
    fields: [
      {
        key: "event",
        label: "Event",
        kind: "select",
        options: [
          { value: "activate", label: "On activate" },
          { value: "update", label: "On update" },
        ],
        default: "activate",
      },
    ],
  },
  simulateTrigger: {
    description: "Manual-style trigger that emits fixed JSON output.",
    fields: [
      {
        key: "output",
        label: "Output items (JSON array)",
        kind: "json",
        default: [{ simulated: true }],
      },
    ],
  },
  mcpServerTrigger: {
    description: "Starts when an MCP client calls a tool exposed by this workflow.",
    fields: [
      {
        key: "toolName",
        label: "Tool name",
        kind: "string",
        default: "run",
        placeholder: "run",
      },
    ],
  },
  evaluationTrigger: {
    description: "Entry point for evaluation / test-dataset runs.",
    fields: [
      {
        key: "datasetId",
        label: "Dataset ID",
        kind: "string",
        default: "",
      },
    ],
  },

  // —— Control ——
  if: {
    description: "Routes each item to true or false using conditions (AND/OR).",
    fields: [
      {
        key: "conditions",
        label: "Conditions",
        kind: "conditions",
        default: { combinator: "AND", conditions: [] },
      },
      {
        key: "ignoreCase",
        label: "Ignore case",
        kind: "boolean",
        default: false,
      },
      { key: "field", label: "Field path (legacy)", kind: "string", default: "" },
      {
        key: "operator",
        label: "Operator (legacy)",
        kind: "select",
        options: COMPARE_OPS,
        default: "equals",
      },
      { key: "value", label: "Compare value (legacy)", kind: "string", default: "" },
    ],
  },
  switch: {
    description: "Routes by rules, expression output index, or value cases.",
    fields: [
      {
        key: "mode",
        label: "Mode",
        kind: "select",
        options: [
          { value: "rules", label: "Rules" },
          { value: "expression", label: "Expression" },
          { value: "value", label: "Value matching" },
        ],
        default: "rules",
      },
      {
        key: "rules",
        label: "Rules",
        kind: "switchRules",
        default: [],
        when: { key: "mode", equals: "rules" },
      },
      {
        key: "output",
        label: "Output index expression",
        kind: "string",
        default: "={{ 0 }}",
        when: { key: "mode", equals: "expression" },
      },
      {
        key: "field",
        label: "Field path",
        kind: "string",
        default: "value",
        when: { key: "mode", equals: "value" },
      },
      {
        key: "cases",
        label: "Cases",
        kind: "stringList",
        description: "One value per line",
        default: [],
        when: { key: "mode", equals: "value" },
      },
      {
        key: "fallbackOutput",
        label: "Fallback output",
        kind: "string",
        default: "default",
      },
      {
        key: "allMatchingOutputs",
        label: "Send to all matching outputs",
        kind: "boolean",
        default: false,
        when: { key: "mode", equals: "rules" },
      },
    ],
  },
  merge: {
    description:
      "Merges two inputs (handles in1/in2). Modes: append, combine by fields/position/all, choose branch.",
    fields: [
      {
        key: "mode",
        label: "Mode",
        kind: "select",
        options: [
          { value: "append", label: "Append" },
          { value: "combineByFields", label: "Combine by fields" },
          { value: "combineByPosition", label: "Combine by position" },
          { value: "combineAll", label: "Combine all (cartesian)" },
          { value: "chooseBranch", label: "Choose branch" },
        ],
        default: "append",
      },
      {
        key: "joinMode",
        label: "Join",
        kind: "select",
        options: [
          { value: "keepMatches", label: "Keep matches" },
          { value: "keepNonMatches", label: "Keep non-matches" },
          { value: "enrichInput1", label: "Enrich Input 1" },
          { value: "enrichInput2", label: "Enrich Input 2" },
          { value: "keepEverything", label: "Keep everything" },
        ],
        default: "keepMatches",
        when: { key: "mode", equals: "combineByFields" },
      },
      {
        key: "fields1",
        label: "Input 1 fields",
        kind: "string",
        default: "id",
        description: "Comma-separated field paths",
        when: { key: "mode", equals: "combineByFields" },
      },
      {
        key: "fields2",
        label: "Input 2 fields",
        kind: "string",
        default: "id",
        when: { key: "mode", equals: "combineByFields" },
      },
      {
        key: "chooseBranch",
        label: "Branch",
        kind: "select",
        options: [
          { value: "input1", label: "Input 1" },
          { value: "input2", label: "Input 2" },
        ],
        default: "input1",
        when: { key: "mode", equals: "chooseBranch" },
      },
    ],
  },
  wait: {
    description:
      "Pauses execution for a duration, until a date, or until a resume webhook.",
    fields: [
      {
        key: "resume",
        label: "Resume",
        kind: "select",
        options: [
          { value: "timeAmount", label: "After time interval" },
          { value: "specificTime", label: "At specified time" },
          { value: "webhook", label: "On webhook call" },
        ],
        default: "timeAmount",
      },
      {
        key: "amount",
        label: "Wait amount",
        kind: "number",
        default: 5,
        when: { key: "resume", equals: "timeAmount" },
      },
      {
        key: "unit",
        label: "Unit",
        kind: "select",
        options: [
          { value: "seconds", label: "Seconds" },
          { value: "minutes", label: "Minutes" },
          { value: "hours", label: "Hours" },
          { value: "milliseconds", label: "Milliseconds" },
        ],
        default: "seconds",
        when: { key: "resume", equals: "timeAmount" },
      },
      {
        key: "dateTime",
        label: "Date & time (ISO)",
        kind: "string",
        default: "",
        when: { key: "resume", equals: "specificTime" },
      },
      {
        key: "ms",
        label: "Wait (ms, legacy)",
        kind: "number",
        default: 1000,
        when: { key: "resume", equals: "timeAmount" },
      },
    ],
  },
  mcpCall: {
    description:
      "Calls a local agent/MCP tool, or POSTs to an optional HTTP endpoint.",
    fields: [
      { key: "tool", label: "Tool name", kind: "string", default: "" },
      { key: "arguments", label: "Arguments (JSON)", kind: "json", default: {} },
      {
        key: "endpoint",
        label: "HTTP endpoint (optional)",
        kind: "string",
        description: "If set, POST { tool, arguments } here instead of local tools",
        default: "",
      },
    ],
  },
  noOp: {
    description: "Passes items through unchanged.",
    fields: [],
  },
  stopAndError: {
    description: "Stops the run and throws an error.",
    fields: [
      {
        key: "message",
        label: "Error message",
        kind: "text",
        default: "Stopped by Stop and Error node",
      },
    ],
  },
  stickyNote: {
    description: "Canvas annotation only — does not affect data.",
    fields: [
      { key: "content", label: "Note", kind: "text", default: "" },
      { key: "color", label: "Color", kind: "string", default: "#fef3c7" },
    ],
  },
  executeWorkflow: {
    description: "Runs another workflow and returns its output.",
    fields: [
      { key: "workflowId", label: "Workflow ID", kind: "string", default: "" },
      {
        key: "input",
        label: "Input override (JSON)",
        kind: "json",
        description: "Leave empty to pass current items",
        default: null,
      },
    ],
  },
  executeCommand: {
    description: "Runs a shell command on the server.",
    fields: [
      { key: "command", label: "Command", kind: "string", default: "echo dataflow" },
      { key: "timeoutMs", label: "Timeout (ms)", kind: "number", default: 30000 },
    ],
  },

  // —— API ——
  httpRequest: {
    description: "Performs an HTTP request for each input item.",
    fields: [
      { key: "url", label: "URL", kind: "string", placeholder: "https://api.example.com", default: "" },
      {
        key: "method",
        label: "Method",
        kind: "select",
        options: METHOD_OPTIONS,
        default: "GET",
      },
      {
        key: "authentication",
        label: "Authentication",
        kind: "select",
        options: [
          { value: "none", label: "None" },
          { value: "bearer", label: "Bearer token" },
          { value: "basicAuth", label: "Basic auth" },
          { value: "headerAuth", label: "Header auth" },
          { value: "predefinedCredentialType", label: "Credential" },
        ],
        default: "none",
      },
      {
        key: "credentialId",
        label: "Credential",
        kind: "string",
        default: "",
        when: {
          key: "authentication",
          equals: ["predefinedCredentialType", "bearer", "basicAuth", "headerAuth"],
        },
        credentialType: "httpHeaderAuth",
      },
      {
        key: "bearerToken",
        label: "Bearer token",
        kind: "string",
        default: "",
        when: { key: "authentication", equals: "bearer" },
      },
      {
        key: "user",
        label: "User",
        kind: "string",
        default: "",
        when: { key: "authentication", equals: "basicAuth" },
      },
      {
        key: "password",
        label: "Password",
        kind: "string",
        default: "",
        when: { key: "authentication", equals: "basicAuth" },
      },
      {
        key: "headerName",
        label: "Header name",
        kind: "string",
        default: "Authorization",
        when: { key: "authentication", equals: "headerAuth" },
      },
      {
        key: "headerValue",
        label: "Header value",
        kind: "string",
        default: "",
        when: { key: "authentication", equals: "headerAuth" },
      },
      { key: "queryParameters", label: "Query parameters", kind: "keyValue", default: {} },
      { key: "headers", label: "Headers", kind: "keyValue", default: {} },
      {
        key: "sendBody",
        label: "Send body",
        kind: "boolean",
        default: true,
      },
      {
        key: "bodyContentType",
        label: "Body content type",
        kind: "select",
        options: [
          { value: "json", label: "JSON" },
          { value: "form-urlencoded", label: "Form URL-encoded" },
          { value: "raw", label: "Raw" },
        ],
        default: "json",
      },
      {
        key: "body",
        label: "Body",
        kind: "json",
        description: 'JSON body, or the string "item" to send the current item',
        default: null,
      },
      {
        key: "responseFormat",
        label: "Response format",
        kind: "select",
        options: [
          { value: "json", label: "JSON" },
          { value: "text", label: "Text" },
          { value: "binary", label: "Binary (base64)" },
        ],
        default: "json",
      },
      {
        key: "fullResponse",
        label: "Include full response (status/headers)",
        kind: "boolean",
        default: true,
      },
      {
        key: "timeoutMs",
        label: "Timeout (ms)",
        kind: "number",
        default: 30000,
      },
      {
        key: "paginationMode",
        label: "Pagination",
        kind: "select",
        options: [
          { value: "off", label: "Off" },
          { value: "nextUrl", label: "Next URL (body/Link header)" },
          { value: "offset", label: "Offset / limit" },
        ],
        default: "off",
      },
      {
        key: "paginationMaxPages",
        label: "Max pages",
        kind: "number",
        default: 10,
        when: { key: "paginationMode", equals: ["nextUrl", "offset"] },
      },
      {
        key: "nextUrlPath",
        label: "Next URL field",
        kind: "string",
        default: "next",
        when: { key: "paginationMode", equals: "nextUrl" },
      },
      {
        key: "paginationLimit",
        label: "Page size",
        kind: "number",
        default: 100,
        when: { key: "paginationMode", equals: "offset" },
      },
      {
        key: "offsetParam",
        label: "Offset query param",
        kind: "string",
        default: "offset",
        when: { key: "paginationMode", equals: "offset" },
      },
      {
        key: "limitParam",
        label: "Limit query param",
        kind: "string",
        default: "limit",
        when: { key: "paginationMode", equals: "offset" },
      },
    ],
  },
  webhookRespond: {
    description:
      "Sends the HTTP response for a webhook-triggered run. Passes through upstream HTTP status when the input is a full response ({ statusCode, body }).",
    fields: [
      {
        key: "statusCode",
        label: "Status code",
        kind: "number",
        description:
          "HTTP status. Use an expression, or leave 200 — fullResponse inputs (statusCode + body) override automatically.",
        default: 200,
      },
      {
        key: "responseData",
        label: "Respond with",
        kind: "select",
        options: [
          { value: "firstEntryJson", label: "First entry JSON" },
          { value: "allEntries", label: "All entries" },
          { value: "firstEntryBinary", label: "First entry binary" },
          { value: "noData", label: "No data" },
        ],
        default: "firstEntryJson",
      },
      { key: "body", label: "Custom body (JSON)", kind: "json", default: null },
      {
        key: "responseHeaders",
        label: "Response headers",
        kind: "keyValue",
        default: {},
      },
    ],
  },
  apiRespond: {
    description:
      "Sends the HTTP response for an API project endpoint run. Passes through upstream HTTP status when the input is a full response ({ statusCode, body }). Owns response status and OpenAPI response schema.",
    fields: [
      {
        key: "statusCode",
        label: "Status code",
        kind: "number",
        description:
          "HTTP status. Use an expression, or leave 200 — fullResponse inputs (statusCode + body) override automatically. Non-2xx responses are logged.",
        default: 200,
      },
      {
        key: "responseSchema",
        label: "Response",
        kind: "jsonSchema",
        description:
          "Expected response fields (or raw JSON / $ref) for OpenAPI docs.",
        default: { type: "object", properties: {}, required: [] },
      },
      {
        key: "responseData",
        label: "Respond with",
        kind: "select",
        options: [
          { value: "firstEntryJson", label: "First entry JSON" },
          { value: "allEntries", label: "All entries" },
          { value: "noData", label: "No data" },
        ],
        default: "firstEntryJson",
      },
      { key: "body", label: "Custom body (JSON)", kind: "json", default: null },
      {
        key: "responseHeaders",
        label: "Response headers",
        kind: "keyValue",
        default: {},
      },
    ],
  },
  graphql: {
    description: "Executes a GraphQL query against an endpoint.",
    fields: [
      { key: "url", label: "Endpoint URL", kind: "string", default: "" },
      { key: "query", label: "Query", kind: "code", default: "{ __typename }" },
      { key: "variables", label: "Variables (JSON)", kind: "json", default: {} },
      { key: "accessToken", label: "Access token", kind: "string", default: "" },
    ],
  },
  jwt: {
    description: "Signs or verifies an HS256 JWT.",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "sign", label: "Sign" },
          { value: "verify", label: "Verify" },
        ],
        default: "sign",
      },
      { key: "secret", label: "Secret", kind: "string", default: "" },
      {
        key: "payload",
        label: "Payload (JSON)",
        kind: "json",
        default: {},
        when: { key: "operation", equals: "sign" },
      },
      {
        key: "token",
        label: "Token",
        kind: "text",
        default: "",
        when: { key: "operation", equals: "verify" },
      },
    ],
  },
  n8nApi: {
    description: "Calls an n8n instance REST API (N8N_BASE_URL + N8N_API_KEY).",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "getWorkflows", label: "Get workflows" },
          { value: "getWorkflow", label: "Get workflow" },
          { value: "getExecutions", label: "Get executions" },
          { value: "getExecution", label: "Get execution" },
        ],
        default: "getWorkflows",
      },
      { key: "baseUrl", label: "n8n base URL", kind: "string", default: "" },
      { key: "apiKey", label: "API key", kind: "string", default: "" },
      {
        key: "workflowId",
        label: "Workflow ID",
        kind: "string",
        default: "",
        when: { key: "operation", equals: "getWorkflow" },
      },
      {
        key: "executionId",
        label: "Execution ID",
        kind: "string",
        default: "",
        when: { key: "operation", equals: "getExecution" },
      },
    ],
  },
  form: {
    description: "Represents form field data in the flow.",
    fields: [
      { key: "formId", label: "Form ID", kind: "string", default: "form" },
      { key: "fields", label: "Fields (JSON)", kind: "json", default: {} },
    ],
  },
  rssRead: {
    description: "Reads titles from an RSS/Atom feed URL.",
    fields: [
      { key: "url", label: "Feed URL", kind: "string", default: "" },
    ],
  },
  sendEmail: {
    description: "Sends email via SMTP (SMTP_URL or SMTP_HOST/USER/PASSWORD).",
    fields: [
      {
        key: "credentialId",
        label: "SMTP credential",
        kind: "string",
        default: "",
        credentialType: "smtp",
      },
      { key: "from", label: "From", kind: "string", default: "" },
      { key: "to", label: "To", kind: "string", default: "" },
      { key: "cc", label: "CC", kind: "string", default: "" },
      { key: "bcc", label: "BCC", kind: "string", default: "" },
      { key: "subject", label: "Subject", kind: "string", default: "DataFlow" },
      { key: "text", label: "Text body", kind: "text", default: "" },
      { key: "html", label: "HTML body", kind: "text", default: "" },
      { key: "smtpUrl", label: "SMTP URL override", kind: "string", default: "" },
      { key: "host", label: "SMTP host", kind: "string", default: "" },
      { key: "port", label: "SMTP port", kind: "number", default: 587 },
      { key: "user", label: "SMTP user", kind: "string", default: "" },
      { key: "password", label: "SMTP password", kind: "string", default: "" },
      { key: "secure", label: "TLS/SSL", kind: "boolean", default: false },
    ],
  },
  ftp: {
    description: "FTP/FTPS list, download, or upload (FTP_HOST / FTP_USER / FTP_PASSWORD).",
    fields: [
      {
        key: "credentialId",
        label: "FTP credential",
        kind: "string",
        default: "",
        credentialType: "ftp",
      },
      { key: "host", label: "Host", kind: "string", default: "" },
      { key: "port", label: "Port", kind: "number", default: 21 },
      { key: "user", label: "User", kind: "string", default: "" },
      { key: "password", label: "Password", kind: "string", default: "" },
      { key: "secure", label: "FTPS", kind: "boolean", default: false },
      { key: "path", label: "Remote path", kind: "string", default: "/" },
      { key: "localPath", label: "Local path (upload)", kind: "string", default: "" },
      { key: "content", label: "Upload content", kind: "text", default: "" },
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "list", label: "List" },
          { value: "download", label: "Download" },
          { value: "upload", label: "Upload" },
        ],
        default: "list",
      },
    ],
  },
  ssh: {
    description: "Runs a command over SSH (SSH_HOST / SSH_USER + password or key).",
    fields: [
      {
        key: "credentialId",
        label: "SSH credential",
        kind: "string",
        default: "",
        credentialType: "ssh",
      },
      { key: "host", label: "Host", kind: "string", default: "" },
      { key: "port", label: "Port", kind: "number", default: 22 },
      { key: "username", label: "Username", kind: "string", default: "" },
      { key: "password", label: "Password", kind: "string", default: "" },
      { key: "privateKey", label: "Private key", kind: "text", default: "" },
      { key: "command", label: "Command", kind: "string", default: "uname -a" },
    ],
  },
  git: {
    description: "Runs a git command in a working directory.",
    fields: [
      { key: "command", label: "Command", kind: "string", default: "git status --short" },
      { key: "cwd", label: "Working directory", kind: "string", default: "" },
    ],
  },
  ldap: {
    description: "LDAP search or compare (LDAP_URL / LDAP_BIND_DN / LDAP_PASSWORD).",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "search", label: "Search" },
          { value: "compare", label: "Compare" },
        ],
        default: "search",
      },
      { key: "host", label: "Host / URL", kind: "string", default: "ldap://localhost" },
      { key: "bindDN", label: "Bind DN", kind: "string", default: "" },
      { key: "bindPassword", label: "Bind password", kind: "string", default: "" },
      { key: "baseDn", label: "Base DN", kind: "string", default: "" },
      { key: "filter", label: "Search filter", kind: "string", default: "(objectClass=*)" },
      {
        key: "dn",
        label: "Entry DN",
        kind: "string",
        default: "",
        when: { key: "operation", equals: "compare" },
      },
      {
        key: "attribute",
        label: "Attribute",
        kind: "string",
        default: "",
        when: { key: "operation", equals: "compare" },
      },
      {
        key: "value",
        label: "Value",
        kind: "string",
        default: "",
        when: { key: "operation", equals: "compare" },
      },
    ],
  },

  // —— Microsoft ——
  mssql: {
    description: "Runs a SQL query against MSSQL.",
    fields: [
      {
        key: "credentialId",
        label: "MSSQL credential",
        kind: "string",
        default: "",
        credentialType: "microsoftSql",
      },
      {
        key: "connectionString",
        label: "Connection string",
        kind: "text",
        description:
          "Uses the Windows user running the DataFlow server (Trusted_Connection). Falls back to MSSQL_CONNECTION_STRING. Type $ to insert a variable.",
        placeholder: "$vars.WealthWebsite",
        default: "$vars.WealthWebsite",
      },
      {
        key: "query",
        label: "Query",
        kind: "code",
        language: "sql",
        placeholder: "Type $ to insert a variable",
        default: "SELECT 1 AS ok",
      },
    ],
  },
  microsoftGraph: {
    description: "Calls Microsoft Graph API.",
    fields: [
      {
        key: "credentialId",
        label: "Microsoft credential",
        kind: "string",
        default: "",
        credentialType: "microsoftGraphOAuth2Api",
      },
      { key: "path", label: "Graph path", kind: "string", default: "/me" },
      {
        key: "method",
        label: "Method",
        kind: "select",
        options: METHOD_OPTIONS,
        default: "GET",
      },
      { key: "accessToken", label: "Access token", kind: "string", default: "" },
      { key: "body", label: "Body (JSON)", kind: "json", default: null },
    ],
  },
  outlook: {
    description: "Lists or sends Outlook mail via Graph.",
    fields: [
      {
        key: "credentialId",
        label: "Microsoft credential",
        kind: "string",
        default: "",
        credentialType: "microsoftOutlookOAuth2Api",
      },
      {
        key: "resource",
        label: "Resource",
        kind: "select",
        options: [
          { value: "message", label: "Message" },
          { value: "draft", label: "Draft" },
          { value: "folder", label: "Folder" },
        ],
        default: "message",
      },
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "getAll", label: "Get many" },
          { value: "get", label: "Get" },
          { value: "send", label: "Send" },
          { value: "create", label: "Create" },
        ],
        default: "getAll",
      },
      {
        key: "action",
        label: "Action (legacy)",
        kind: "select",
        options: [
          { value: "listMessages", label: "List messages" },
          { value: "sendMail", label: "Send mail" },
        ],
        default: "listMessages",
      },
      {
        key: "path",
        label: "Messages path",
        kind: "string",
        default: "/me/messages?$top=10",
        when: { key: "action", equals: "listMessages" },
      },
      {
        key: "messageId",
        label: "Message ID",
        kind: "string",
        default: "",
        when: { key: "operation", equals: "get" },
      },
      {
        key: "to",
        label: "To",
        kind: "string",
        default: "",
      },
      {
        key: "subject",
        label: "Subject",
        kind: "string",
        default: "DataFlow",
      },
      {
        key: "body",
        label: "Body",
        kind: "text",
        default: "",
      },
      { key: "accessToken", label: "Access token", kind: "string", default: "" },
    ],
  },
  excelOnline: {
    description: "Accesses OneDrive / Excel Online via Graph.",
    fields: [
      {
        key: "credentialId",
        label: "Microsoft credential",
        kind: "string",
        default: "",
        credentialType: "microsoftExcelOAuth2Api",
      },
      {
        key: "resource",
        label: "Resource",
        kind: "select",
        options: [
          { value: "workbook", label: "Workbook" },
          { value: "worksheet", label: "Worksheet" },
          { value: "range", label: "Range" },
        ],
        default: "workbook",
      },
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "get", label: "Get" },
          { value: "list", label: "List children" },
          { value: "getRange", label: "Get range" },
        ],
        default: "list",
      },
      {
        key: "workbookId",
        label: "Workbook / Drive item ID",
        kind: "string",
        default: "",
      },
      {
        key: "worksheetName",
        label: "Worksheet name",
        kind: "string",
        default: "Sheet1",
      },
      {
        key: "range",
        label: "Range (A1)",
        kind: "string",
        default: "A1:C10",
      },
      {
        key: "path",
        label: "Graph path (override)",
        kind: "string",
        default: "/me/drive/root/children",
      },
      { key: "accessToken", label: "Access token", kind: "string", default: "" },
    ],
  },
  sharepoint: {
    description: "Lists SharePoint sites or drive children.",
    fields: [
      {
        key: "credentialId",
        label: "Microsoft credential",
        kind: "string",
        default: "",
        credentialType: "microsoftSharePointOAuth2Api",
      },
      {
        key: "resource",
        label: "Resource",
        kind: "select",
        options: [
          { value: "site", label: "Site" },
          { value: "list", label: "List" },
          { value: "item", label: "List item" },
          { value: "file", label: "File" },
        ],
        default: "site",
      },
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "getAll", label: "Get many" },
          { value: "get", label: "Get" },
          { value: "getChildren", label: "Get children" },
        ],
        default: "getAll",
      },
      { key: "siteId", label: "Site ID", kind: "string", default: "" },
      { key: "listId", label: "List ID", kind: "string", default: "" },
      {
        key: "path",
        label: "Fallback path",
        kind: "string",
        default: "/sites?search=*",
      },
      { key: "accessToken", label: "Access token", kind: "string", default: "" },
    ],
  },
  dynamics: {
    description:
      "Dataverse Web API (FetchXML / OData GET, create, update, delete, option sets) — same ops as n8n-nodes-dataverse-auth (+ PR #4 DELETE).",
    fields: [
      {
        key: "credentialId",
        label: "Dataverse credential",
        kind: "string",
        default: "",
        credentialType: "dataverseAuth",
      },
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "GET", label: "GET" },
          { value: "PATCH", label: "PATCH (update)" },
          { value: "POST", label: "POST (create)" },
          { value: "DELETE", label: "DELETE (record)" },
          { value: "OPTIONSET", label: "Option set" },
          { value: "GLOBALOPTIONSET", label: "Global option set" },
          { value: "ENTITY", label: "Entity lookup" },
        ],
        default: "GET",
      },
      {
        key: "type",
        label: "Query type",
        kind: "select",
        options: [
          { value: "FETCHXML", label: "FetchXML" },
          { value: "ODATA", label: "OData" },
        ],
        default: "FETCHXML",
        when: { key: "operation", equals: "GET" },
      },
      {
        key: "getQuery",
        label: "Query",
        kind: "code",
        language: "text",
        description:
          "FetchXML document, or OData path (e.g. accounts?$top=10)",
        default: "",
        when: { key: "operation", equals: "GET" },
      },
      {
        key: "entityName",
        label: "Entity logical name",
        kind: "string",
        placeholder: "account",
        description:
          "Logical name (account → accounts). Used by PATCH/POST/DELETE/ENTITY.",
        default: "",
        when: {
          key: "operation",
          equals: ["PATCH", "POST", "DELETE", "ENTITY"],
        },
      },
      {
        key: "recordId",
        label: "Record Id",
        kind: "string",
        default: "",
        when: { key: "operation", equals: ["PATCH", "DELETE"] },
      },
      {
        key: "columns",
        label: "Columns",
        kind: "keyValue",
        description: "Column logical name → value",
        default: {},
        when: { key: "operation", equals: ["PATCH", "POST"] },
      },
      {
        key: "body",
        label: "Body (JSON)",
        kind: "json",
        default: null,
        when: { key: "operation", equals: ["PATCH", "POST"] },
      },
      {
        key: "optionsetEntityName",
        label: "Entity logical name",
        kind: "string",
        placeholder: "account",
        default: "",
        when: { key: "operation", equals: "OPTIONSET" },
      },
      {
        key: "optionsetAttributeName",
        label: "Attribute name",
        kind: "string",
        default: "",
        when: { key: "operation", equals: "OPTIONSET" },
      },
      {
        key: "globalAttributeName",
        label: "Global option set name",
        kind: "string",
        default: "",
        when: { key: "operation", equals: "GLOBALOPTIONSET" },
      },
      {
        key: "entityIdColumn",
        label: "Id column",
        kind: "string",
        placeholder: "accountid",
        default: "",
        when: { key: "operation", equals: "ENTITY" },
      },
      {
        key: "entityNameColumn",
        label: "Name column",
        kind: "string",
        placeholder: "name",
        default: "",
        when: { key: "operation", equals: "ENTITY" },
      },
      {
        key: "baseUrl",
        label: "Scope / org URL override",
        kind: "string",
        placeholder: "https://org.crm4.dynamics.com",
        default: "",
      },
      {
        key: "accessToken",
        label: "Access token override",
        kind: "string",
        default: "",
      },
    ],
  },

  // —— Files ——
  readFile: {
    description: "Reads a file from disk into an item.",
    fields: [
      { key: "path", label: "File path", kind: "string", default: "" },
      {
        key: "encoding",
        label: "Encoding",
        kind: "select",
        options: [
          { value: "utf8", label: "UTF-8" },
          { value: "base64", label: "Base64" },
          { value: "ascii", label: "ASCII" },
        ],
        default: "utf8",
      },
    ],
  },
  writeFile: {
    description: "Writes content or current items to a file.",
    fields: [
      { key: "path", label: "File path", kind: "string", default: "" },
      {
        key: "content",
        label: "Content",
        kind: "text",
        description: "Leave empty to write JSON of current items",
        default: "",
      },
    ],
  },
  listFiles: {
    description: "Lists files in a directory.",
    fields: [
      { key: "path", label: "Directory", kind: "string", default: "." },
    ],
  },
  moveFile: {
    description: "Moves/renames a file.",
    fields: [
      { key: "from", label: "From", kind: "string", default: "" },
      { key: "to", label: "To", kind: "string", default: "" },
    ],
  },
  copyFile: {
    description: "Copies a file.",
    fields: [
      { key: "from", label: "From", kind: "string", default: "" },
      { key: "to", label: "To", kind: "string", default: "" },
    ],
  },
  deleteFile: {
    description: "Deletes a file or directory.",
    fields: [
      { key: "path", label: "Path", kind: "string", default: "" },
      { key: "recursive", label: "Recursive", kind: "boolean", default: false },
    ],
  },
  csv: {
    description: "Parses CSV text into items or serializes items to CSV.",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "parse", label: "Parse" },
          { value: "serialize", label: "Serialize" },
        ],
        default: "parse",
      },
      {
        key: "content",
        label: "CSV content",
        kind: "text",
        default: "",
        when: { key: "operation", equals: "parse" },
      },
    ],
  },
  json: {
    description: "Parses JSON text or stringifies items.",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "parse", label: "Parse" },
          { value: "stringify", label: "Stringify" },
        ],
        default: "parse",
      },
      {
        key: "content",
        label: "JSON content",
        kind: "text",
        default: "[]",
        when: { key: "operation", equals: "parse" },
      },
    ],
  },
  xml: {
    description: "Builds XML from items or parses XML into JSON.",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "wrap", label: "Build / wrap" },
          { value: "parse", label: "Parse" },
        ],
        default: "wrap",
      },
      {
        key: "root",
        label: "Root element",
        kind: "string",
        default: "items",
        when: { key: "operation", equals: ["wrap", "build"] },
      },
      {
        key: "content",
        label: "XML content",
        kind: "text",
        default: "",
        when: { key: "operation", equals: "parse" },
      },
    ],
  },
  excel: {
    description: "Reads or writes an Excel (.xlsx) file.",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "read", label: "Read" },
          { value: "write", label: "Write" },
        ],
        default: "read",
      },
      { key: "path", label: "File path", kind: "string", default: "output.xlsx" },
    ],
  },
  zip: {
    description: "Create or extract a ZIP archive (or gzip a single file).",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "zip", label: "Zip (create archive)" },
          { value: "unzip", label: "Unzip (extract archive)" },
          { value: "gzip", label: "Gzip file" },
          { value: "gunzip", label: "Gunzip file" },
        ],
        default: "zip",
      },
      { key: "from", label: "Source path", kind: "string", default: "" },
      { key: "to", label: "Destination path", kind: "string", default: "" },
    ],
  },
  compression: {
    description: "Gzip/deflate compress or decompress binary or text fields.",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "compress", label: "Compress" },
          { value: "decompress", label: "Decompress" },
        ],
        default: "compress",
      },
      {
        key: "format",
        label: "Format",
        kind: "select",
        options: [
          { value: "gzip", label: "Gzip" },
          { value: "deflate", label: "Deflate" },
        ],
        default: "gzip",
      },
      {
        key: "binaryPropertyName",
        label: "Binary property",
        kind: "string",
        default: "data",
      },
      {
        key: "field",
        label: "Text field (if no binary)",
        kind: "string",
        default: "content",
      },
    ],
  },
  convertToFile: {
    description: "Writes items (or content) to a file path.",
    fields: [
      { key: "fileName", label: "File name", kind: "string", default: "output.json" },
      { key: "path", label: "Path", kind: "string", default: "" },
      { key: "content", label: "Content override", kind: "text", default: "" },
      {
        key: "encoding",
        label: "Encoding",
        kind: "select",
        options: [
          { value: "utf8", label: "UTF-8" },
          { value: "base64", label: "Base64" },
        ],
        default: "utf8",
      },
    ],
  },
  extractFromFile: {
    description: "Reads a file and parses JSON when possible.",
    fields: [
      { key: "path", label: "File path", kind: "string", default: "" },
      {
        key: "encoding",
        label: "Encoding",
        kind: "select",
        options: [
          { value: "utf8", label: "UTF-8" },
          { value: "base64", label: "Base64" },
        ],
        default: "utf8",
      },
    ],
  },
  readPdf: {
    description: "Extracts text from a PDF file or binary input.",
    fields: [
      { key: "path", label: "PDF path", kind: "string", default: "" },
    ],
  },
  moveBinaryData: {
    description: "Converts between JSON items and base64 binary.",
    fields: [
      {
        key: "mode",
        label: "Mode",
        kind: "select",
        options: [
          { value: "jsonToBinary", label: "JSON → Binary" },
          { value: "binaryToJson", label: "Binary → JSON" },
        ],
        default: "jsonToBinary",
      },
    ],
  },
  editImage: {
    description: "Resize, crop, or reformat image binary with sharp.",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "noop", label: "No-op" },
          { value: "resize", label: "Resize" },
          { value: "crop", label: "Crop" },
          { value: "format", label: "Format" },
        ],
        default: "noop",
      },
      {
        key: "binaryPropertyName",
        label: "Binary property",
        kind: "string",
        default: "data",
      },
      { key: "width", label: "Width", kind: "number", default: 256 },
      { key: "height", label: "Height", kind: "number", default: 256 },
      { key: "left", label: "Crop left", kind: "number", default: 0 },
      { key: "top", label: "Crop top", kind: "number", default: 0 },
      {
        key: "format",
        label: "Output format",
        kind: "select",
        options: [
          { value: "png", label: "PNG" },
          { value: "jpeg", label: "JPEG" },
          { value: "webp", label: "WebP" },
        ],
        default: "png",
        when: { key: "operation", equals: "format" },
      },
    ],
  },
  icalendar: {
    description: "Builds a simple VEVENT iCalendar payload.",
    fields: [
      { key: "title", label: "Title", kind: "string", default: "DataFlow Event" },
      { key: "start", label: "Start (ISO)", kind: "string", default: "" },
    ],
  },

  // —— Data ——
  set: {
    description: "Edit Fields (Set) — set values with include all/none/selected/except.",
    fields: [
      {
        key: "mode",
        label: "Mode",
        kind: "select",
        options: [
          { value: "manual", label: "Manual mapping" },
          { value: "raw", label: "Raw JSON" },
        ],
        default: "manual",
      },
      {
        key: "include",
        label: "Include other fields",
        kind: "select",
        options: [
          { value: "all", label: "All" },
          { value: "none", label: "None" },
          { value: "selected", label: "Selected" },
          { value: "except", label: "All except" },
        ],
        default: "all",
      },
      {
        key: "includeFields",
        label: "Fields to include",
        kind: "stringList",
        default: [],
        when: { key: "include", equals: "selected" },
      },
      {
        key: "excludeFields",
        label: "Fields to exclude",
        kind: "stringList",
        default: [],
        when: { key: "include", equals: "except" },
      },
      {
        key: "values",
        label: "Values",
        kind: "assignments",
        default: [],
        when: { key: "mode", equals: "manual" },
      },
      {
        key: "jsonOutput",
        label: "JSON output",
        kind: "code",
        language: "json",
        default: "{}",
        when: { key: "mode", equals: "raw" },
      },
      {
        key: "dotNotation",
        label: "Dot notation",
        kind: "boolean",
        default: true,
      },
      {
        key: "stripBinary",
        label: "Strip binary",
        kind: "boolean",
        default: false,
      },
    ],
  },
  editFields: {
    description: "Renames/moves fields via a from→to mapping.",
    fields: [
      { key: "mapping", label: "Mapping (from → to)", kind: "keyValue", default: {} },
    ],
  },
  filter: {
    description: "Keeps or discards items (outputs: kept / discarded).",
    fields: [
      {
        key: "conditions",
        label: "Conditions",
        kind: "conditions",
        default: { combinator: "AND", conditions: [] },
      },
      {
        key: "ignoreCase",
        label: "Ignore case",
        kind: "boolean",
        default: false,
      },
      { key: "field", label: "Field path (legacy)", kind: "string", default: "" },
      {
        key: "operator",
        label: "Operator (legacy)",
        kind: "select",
        options: FILTER_OPS,
        default: "equals",
      },
      { key: "value", label: "Value (legacy)", kind: "string", default: "" },
    ],
  },
  sort: {
    description: "Sorts items by one or more fields.",
    fields: [
      {
        key: "type",
        label: "Type",
        kind: "select",
        options: [
          { value: "simple", label: "Simple" },
          { value: "random", label: "Random" },
          { value: "code", label: "Code" },
        ],
        default: "simple",
      },
      {
        key: "field",
        label: "Field path",
        kind: "string",
        default: "",
        when: { key: "type", equals: "simple" },
      },
      {
        key: "direction",
        label: "Direction",
        kind: "select",
        options: [
          { value: "asc", label: "Ascending" },
          { value: "desc", label: "Descending" },
        ],
        default: "asc",
        when: { key: "type", equals: "simple" },
      },
      {
        key: "sortFields",
        label: "Multi-field sort (JSON)",
        kind: "json",
        description: '[{ "field": "name", "direction": "asc" }]',
        default: [],
        when: { key: "type", equals: "simple" },
      },
      {
        key: "code",
        label: "Comparator code",
        kind: "code",
        language: "js",
        default:
          "return a.json.value < b.json.value ? -1 : a.json.value > b.json.value ? 1 : 0;",
        when: { key: "type", equals: "code" },
      },
    ],
  },
  limit: {
    description: "Limits to first or last N items.",
    fields: [
      { key: "max", label: "Max items", kind: "number", default: 10 },
      {
        key: "keep",
        label: "Keep",
        kind: "select",
        options: [
          { value: "first", label: "First items" },
          { value: "last", label: "Last items" },
        ],
        default: "first",
      },
    ],
  },
  splitInBatches: {
    description: "Loop Over Items — outputs loop / done with batch cursor.",
    fields: [
      { key: "batchSize", label: "Batch size", kind: "number", default: 10 },
    ],
  },
  splitOut: {
    description: "Expands an array field into separate items.",
    fields: [
      {
        key: "field",
        label: "Fields to split out",
        kind: "string",
        default: "data",
        description: "Comma-separated; use $binary for binary keys",
      },
      {
        key: "include",
        label: "Include other fields",
        kind: "select",
        options: [
          { value: "noOtherFields", label: "No other fields" },
          { value: "allOtherFields", label: "All other fields" },
          { value: "selectedOtherFields", label: "Selected other fields" },
        ],
        default: "noOtherFields",
      },
      {
        key: "selectedOtherFields",
        label: "Selected other fields",
        kind: "stringList",
        default: [],
        when: { key: "include", equals: "selectedOtherFields" },
      },
      {
        key: "destinationFieldName",
        label: "Destination field",
        kind: "string",
        default: "",
        description: "Optional rename (comma-separated for multi)",
      },
      {
        key: "includeBinary",
        label: "Include binary",
        kind: "boolean",
        default: true,
      },
    ],
  },
  aggregate: {
    description: "Collect fields into arrays, or wrap all item data in one list.",
    fields: [
      {
        key: "mode",
        label: "Mode",
        kind: "select",
        options: [
          { value: "individualFields", label: "Individual fields → arrays" },
          { value: "allItemData", label: "All item data → list" },
        ],
        default: "individualFields",
      },
      {
        key: "field",
        label: "Field",
        kind: "string",
        default: "value",
        when: { key: "mode", equals: "individualFields" },
      },
      {
        key: "destinationFieldName",
        label: "Destination field",
        kind: "string",
        default: "data",
        when: { key: "mode", equals: "allItemData" },
      },
      {
        key: "operation",
        label: "Legacy numeric op",
        kind: "select",
        options: [
          { value: "count", label: "Count" },
          { value: "sum", label: "Sum" },
          { value: "avg", label: "Average" },
          { value: "min", label: "Min" },
          { value: "max", label: "Max" },
        ],
        default: "count",
      },
    ],
  },
  summarize: {
    description: "Sum/avg/min/max/count with optional group-by.",
    fields: [
      { key: "field", label: "Primary field", kind: "string", default: "value" },
      {
        key: "fields",
        label: "Fields",
        kind: "stringList",
        description: "One field per line",
        default: ["value"],
      },
      {
        key: "groupBy",
        label: "Group by",
        kind: "string",
        default: "",
        description: "Optional field path",
      },
      {
        key: "aggregations",
        label: "Aggregations (JSON)",
        kind: "json",
        description:
          '[{ "field": "amount", "aggregation": "sum", "outputFieldName": "total" }]',
        default: [],
      },
    ],
  },
  itemLists: {
    description: "Flattens an array field or wraps items as a list.",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "flatten", label: "Flatten" },
          { value: "wrap", label: "Wrap" },
        ],
        default: "flatten",
      },
      {
        key: "field",
        label: "Array field",
        kind: "string",
        default: "batch",
        when: { key: "operation", equals: "flatten" },
      },
    ],
  },
  renameKeys: {
    description: "Renames object keys using a mapping.",
    fields: [
      { key: "mapping", label: "Mapping (from → to)", kind: "keyValue", default: {} },
    ],
  },
  removeDuplicates: {
    description: "Same-run dedupe (outputs: kept / discarded).",
    fields: [
      {
        key: "compare",
        label: "Compare",
        kind: "select",
        options: [
          { value: "selectedFields", label: "Selected fields" },
          { value: "allFields", label: "All fields" },
          { value: "allFieldsExcept", label: "All fields except" },
        ],
        default: "selectedFields",
      },
      {
        key: "fields",
        label: "Fields",
        kind: "stringList",
        default: [],
        when: { key: "compare", equals: "selectedFields" },
      },
      {
        key: "exceptFields",
        label: "Except fields",
        kind: "stringList",
        default: [],
        when: { key: "compare", equals: "allFieldsExcept" },
      },
      {
        key: "removeOtherFields",
        label: "Remove other fields",
        kind: "boolean",
        default: false,
      },
    ],
  },
  compareDatasets: {
    description: "Compares two inputs (handles 0/1) by a key field.",
    fields: [
      { key: "field", label: "Key field", kind: "string", default: "id" },
    ],
  },
  code: {
    description: "JavaScript transform (no require/import). Modes: all items or each item.",
    fields: [
      {
        key: "mode",
        label: "Mode",
        kind: "select",
        options: [
          { value: "runOnceForAllItems", label: "Run once for all items" },
          { value: "runOnceForEachItem", label: "Run once for each item" },
        ],
        default: "runOnceForAllItems",
      },
      {
        key: "code",
        label: "Code",
        kind: "code",
        language: "js",
        default:
          "return items.map(i => ({ json: { ...i.json, ok: true } }));",
      },
    ],
  },
  aiTransform: {
    description:
      "Transforms each item via an OpenAI-compatible chat API (OPENAI_API_KEY).",
    fields: [
      { key: "prompt", label: "Prompt", kind: "text", default: "transform" },
      { key: "model", label: "Model", kind: "string", default: "gpt-4o-mini" },
      { key: "baseUrl", label: "API base URL", kind: "string", default: "" },
      { key: "apiKey", label: "API key override", kind: "string", default: "" },
      { key: "temperature", label: "Temperature", kind: "number", default: 0 },
    ],
  },
  dateTime: {
    description: "Emits current time or normalizes a date field.",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "now", label: "Now" },
          { value: "normalize", label: "Normalize field" },
        ],
        default: "now",
      },
      {
        key: "field",
        label: "Date field",
        kind: "string",
        default: "date",
        when: { key: "operation", equals: "normalize" },
      },
    ],
  },
  crypto: {
    description: "Hashes a field or generates a random token.",
    fields: [
      {
        key: "operation",
        label: "Operation",
        kind: "select",
        options: [
          { value: "hash", label: "Hash" },
          { value: "random", label: "Random" },
        ],
        default: "hash",
      },
      {
        key: "algorithm",
        label: "Algorithm",
        kind: "select",
        options: [
          { value: "sha256", label: "SHA-256" },
          { value: "sha1", label: "SHA-1" },
          { value: "md5", label: "MD5" },
        ],
        default: "sha256",
        when: { key: "operation", equals: "hash" },
      },
      {
        key: "field",
        label: "Field",
        kind: "string",
        default: "content",
        when: { key: "operation", equals: "hash" },
      },
      {
        key: "bytes",
        label: "Bytes",
        kind: "number",
        default: 16,
        when: { key: "operation", equals: "random" },
      },
    ],
  },
  html: {
    description: "Renders an HTML template with {{field}} placeholders.",
    fields: [
      {
        key: "template",
        label: "Template",
        kind: "code",
        default: "<pre>{{json}}</pre>",
      },
    ],
  },
  htmlExtract: {
    description: "Extracts a tag’s text from HTML content.",
    fields: [
      {
        key: "selector",
        label: "Tag name",
        kind: "string",
        default: "title",
        placeholder: "title",
      },
    ],
  },
  markdown: {
    description: "Converts markdown text to HTML (basic).",
    fields: [
      { key: "field", label: "Text field", kind: "string", default: "text" },
      {
        key: "mode",
        label: "Mode",
        kind: "select",
        options: [
          { value: "toHtml", label: "To HTML" },
          { value: "passthrough", label: "Passthrough" },
        ],
        default: "toHtml",
      },
    ],
  },
  totp: {
    description: "Generates an RFC 6238 TOTP (secret or TOTP_SECRET).",
    fields: [
      { key: "secret", label: "Secret (Base32 or UTF-8)", kind: "string", default: "" },
      { key: "digits", label: "Digits", kind: "number", default: 6 },
      { key: "period", label: "Period (seconds)", kind: "number", default: 30 },
      {
        key: "algorithm",
        label: "Algorithm",
        kind: "select",
        options: [
          { value: "SHA1", label: "SHA1" },
          { value: "SHA256", label: "SHA256" },
          { value: "SHA512", label: "SHA512" },
        ],
        default: "SHA1",
      },
    ],
  },
  timeSaved: {
    description: "Tracks minutes saved for ROI / Insights.",
    fields: [
      { key: "minutesSaved", label: "Minutes saved", kind: "number", default: 5 },
      {
        key: "mode",
        label: "Mode",
        kind: "select",
        options: [
          { value: "once", label: "Once per run" },
          { value: "perItem", label: "Per item" },
        ],
        default: "once",
      },
    ],
  },
  dataTable: {
    description: "Projects items into a columnar table shape.",
    fields: [
      {
        key: "columns",
        label: "Columns",
        kind: "stringList",
        description: "One column per line (empty = keys of first item)",
        default: [],
      },
    ],
  },
  executionData: {
    description: "Emits run input metadata and item count.",
    fields: [],
  },
  simulate: {
    description: "Returns fixed simulated output items.",
    fields: [
      {
        key: "output",
        label: "Output items (JSON array)",
        kind: "json",
        default: [{ simulated: true }],
      },
    ],
  },
  debugHelper: {
    description: "Throws errors or generates random sample data for debugging.",
    fields: [
      {
        key: "category",
        label: "Category",
        kind: "select",
        options: [
          { value: "doNothing", label: "Do nothing" },
          { value: "throwError", label: "Throw error" },
          { value: "randomData", label: "Generate random data" },
        ],
        default: "throwError",
      },
      {
        key: "message",
        label: "Error message",
        kind: "string",
        default: "Node has thrown an error",
        when: { key: "category", equals: "throwError" },
      },
      {
        key: "count",
        label: "Item count",
        kind: "number",
        default: 5,
        when: { key: "category", equals: "randomData" },
      },
    ],
  },
  evaluation: {
    description: "Scores or records evaluation metrics for a test run.",
    fields: [
      {
        key: "metric",
        label: "Metric name",
        kind: "string",
        default: "accuracy",
      },
      {
        key: "value",
        label: "Metric value",
        kind: "number",
        default: 1,
      },
      {
        key: "expected",
        label: "Expected (JSON)",
        kind: "json",
        default: null,
      },
      {
        key: "actual",
        label: "Actual (JSON)",
        kind: "json",
        default: null,
      },
    ],
  },
};

const FALLBACK: Omit<NodeSchema, "type"> = {
  description: "Configure this node’s parameters.",
  fields: [],
};

export function getNodeSchema(type: NodeType | string): NodeSchema {
  const base = SCHEMAS[type as NodeType] ?? FALLBACK;
  return { type: type as NodeType, ...base };
}

export function defaultParameters(type: NodeType | string): Record<string, unknown> {
  const schema = getNodeSchema(type);
  const out: Record<string, unknown> = {};
  for (const field of schema.fields) {
    if (field.default !== undefined) {
      out[field.key] =
        field.default === null
          ? undefined
          : Array.isArray(field.default)
            ? [...field.default]
            : typeof field.default === "object"
              ? { ...(field.default as Record<string, unknown>) }
              : field.default;
    }
  }
  // Drop undefined keys from null defaults
  for (const key of Object.keys(out)) {
    if (out[key] === undefined) delete out[key];
  }
  return out;
}

export function fieldVisible(
  field: FieldDef,
  values: Record<string, unknown>,
): boolean {
  if (!field.when) return true;
  const actual = String(values[field.when.key] ?? "");
  const expected = field.when.equals;
  return Array.isArray(expected)
    ? expected.includes(actual)
    : actual === expected;
}
