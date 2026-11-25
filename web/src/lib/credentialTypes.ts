/** Credential type catalog used by Add-credential dialog (n8n-aligned). */

export type CredentialFieldKind =
  | "string"
  | "password"
  | "number"
  | "boolean"
  | "textarea"
  | "select";

export type CredentialFieldDef = {
  key: string;
  label: string;
  kind: CredentialFieldKind;
  placeholder?: string;
  description?: string;
  default?: string | number | boolean;
  required?: boolean;
  options?: { value: string; label: string }[];
  /** Show only when another field matches */
  when?: { key: string; equals: string | string[] };
};

export type CredentialTypeDef = {
  /** Stored `type` on the credential document */
  type: string;
  /** n8n-style display name */
  displayName: string;
  description?: string;
  /** Group for the picker list */
  category: string;
  fields: CredentialFieldDef[];
};

const MS_CLIENT_CREDS: CredentialFieldDef[] = [
  {
    key: "tenantId",
    label: "Tenant ID",
    kind: "string",
    placeholder: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
    required: true,
  },
  {
    key: "clientId",
    label: "Client ID",
    kind: "string",
    placeholder: "Application (client) ID",
    required: true,
  },
  {
    key: "clientSecret",
    label: "Client Secret",
    kind: "password",
    required: true,
  },
  {
    key: "scope",
    label: "Scope",
    kind: "string",
    description: "OAuth scope (include /.default for client credentials when needed)",
    default: "https://graph.microsoft.com/.default",
  },
];

/** Microsoft public sample app (same as XrmToolBox “Use development Azure AD app”). */
const DV_DEV_APP_ID = "51f81489-12ee-4a9e-aaae-a2591f45987d";
const DV_DEV_REDIRECT = "http://localhost";

const DATAVERSE_FIELDS: CredentialFieldDef[] = [
  {
    key: "authMethod",
    label: "Connection method",
    kind: "select",
    description: "Same methods as XrmToolBox → New connection",
    options: [
      { value: "clientSecret", label: "Client Id / Secret" },
      { value: "connectionString", label: "Connection String" },
      { value: "oauthMfa", label: "OAuth / MFA" },
      { value: "sdkLogin", label: "Microsoft Login Control" },
      { value: "certificate", label: "Certificate" },
    ],
    default: "clientSecret",
    required: true,
  },
  {
    key: "scope",
    label: "Environment URL",
    kind: "string",
    placeholder: "https://org.crm4.dynamics.com",
    description: "Dataverse / Dynamics org URL",
    default: "https://<your-org-name>.crm4.dynamics.com",
    required: true,
    when: {
      key: "authMethod",
      equals: ["clientSecret", "oauthMfa", "sdkLogin", "certificate"],
    },
  },
  // —— Client Id / Secret ——
  {
    key: "tenantId",
    label: "Tenant ID",
    kind: "string",
    placeholder: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
    required: true,
    when: { key: "authMethod", equals: ["clientSecret", "certificate"] },
  },
  {
    key: "clientId",
    label: "Client ID",
    kind: "string",
    placeholder: "Azure AD application (client) ID",
    required: true,
    when: { key: "authMethod", equals: ["clientSecret", "certificate"] },
  },
  {
    key: "clientSecret",
    label: "Client Secret",
    kind: "password",
    required: true,
    when: { key: "authMethod", equals: "clientSecret" },
  },
  // —— Connection String ——
  {
    key: "connectionString",
    label: "Connection string",
    kind: "textarea",
    placeholder:
      "AuthType=ClientSecret;Url=https://org.crm.dynamics.com;ClientId=…;ClientSecret=…",
    description:
      "XRM Tooling connection string (AuthType=ClientSecret | OAuth | Certificate)",
    required: true,
    when: { key: "authMethod", equals: "connectionString" },
  },
  // —— OAuth / MFA ——
  {
    key: "oauthClientId",
    label: "Application (Client) ID",
    kind: "string",
    default: DV_DEV_APP_ID,
    description: `Defaults to Microsoft sample app (${DV_DEV_APP_ID}) — same as XrmToolBox “Use development Azure AD app”`,
    required: true,
    when: { key: "authMethod", equals: ["oauthMfa", "sdkLogin"] },
  },
  {
    key: "redirectUri",
    label: "Redirect URI",
    kind: "string",
    default: DV_DEV_REDIRECT,
    placeholder: "http://localhost",
    required: true,
    when: { key: "authMethod", equals: ["oauthMfa", "sdkLogin"] },
  },
  {
    key: "username",
    label: "User email",
    kind: "string",
    placeholder: "user@contoso.onmicrosoft.com",
    description: "Account used for interactive / MFA sign-in (hint for token cache)",
    when: { key: "authMethod", equals: ["oauthMfa", "sdkLogin"] },
  },
  {
    key: "tenantIdOAuth",
    label: "Tenant ID",
    kind: "string",
    placeholder: "common  ·  or your tenant GUID",
    default: "common",
    when: { key: "authMethod", equals: ["oauthMfa", "sdkLogin"] },
  },
  {
    key: "refreshToken",
    label: "Refresh token",
    kind: "password",
    description:
      "Paste after interactive login (XrmToolBox stores this in a token cache). Used to renew access tokens.",
    when: { key: "authMethod", equals: ["oauthMfa", "sdkLogin"] },
  },
  {
    key: "accessToken",
    label: "Access token (optional)",
    kind: "password",
    description: "Bearer token if you already have one; otherwise refresh token is used",
    when: { key: "authMethod", equals: ["oauthMfa", "sdkLogin"] },
  },
  // —— Certificate ——
  {
    key: "certificatePem",
    label: "Certificate private key (PEM)",
    kind: "textarea",
    placeholder: "-----BEGIN PRIVATE KEY-----\n…\n-----END PRIVATE KEY-----",
    description: "PEM for client assertion (application user + certificate auth)",
    required: true,
    when: { key: "authMethod", equals: "certificate" },
  },
  {
    key: "certificatePassword",
    label: "Certificate password",
    kind: "password",
    description: "Optional — if the PEM / PFX is encrypted",
    when: { key: "authMethod", equals: "certificate" },
  },
];

export const CREDENTIAL_TYPES: CredentialTypeDef[] = [
  {
    type: "dataverseAuth",
    displayName: "Dataverse Auth",
    description:
      "Dataverse / Dynamics 365 — XrmToolBox-style connection methods",
    category: "Microsoft",
    fields: DATAVERSE_FIELDS,
  },
  {
    type: "microsoftDynamicsOAuth2Api",
    displayName: "Microsoft Dynamics OAuth2 API",
    description: "Same Dataverse connection methods as Dataverse Auth",
    category: "Microsoft",
    fields: DATAVERSE_FIELDS,
  },
  {
    type: "microsoftGraphOAuth2Api",
    displayName: "Microsoft Graph OAuth2 API",
    category: "Microsoft",
    fields: MS_CLIENT_CREDS,
  },
  {
    type: "microsoftOutlookOAuth2Api",
    displayName: "Microsoft Outlook OAuth2 API",
    category: "Microsoft",
    fields: MS_CLIENT_CREDS,
  },
  {
    type: "microsoftTeamsOAuth2Api",
    displayName: "Microsoft Teams OAuth2 API",
    category: "Microsoft",
    fields: MS_CLIENT_CREDS,
  },
  {
    type: "microsoftExcelOAuth2Api",
    displayName: "Microsoft Excel OAuth2 API",
    category: "Microsoft",
    fields: MS_CLIENT_CREDS,
  },
  {
    type: "microsoftSharePointOAuth2Api",
    displayName: "Microsoft SharePoint OAuth2 API",
    category: "Microsoft",
    fields: MS_CLIENT_CREDS,
  },
  {
    type: "microsoftSql",
    displayName: "Microsoft SQL",
    description: "Connect like SSMS — Login + Connection Properties",
    category: "Microsoft",
    fields: [
      {
        key: "authentication",
        label: "Authentication",
        kind: "select",
        options: [
          { value: "windows", label: "Windows Authentication" },
          { value: "sql", label: "SQL Server Authentication" },
          { value: "connectionString", label: "Connection String" },
        ],
        default: "windows",
        required: true,
      },
      // —— Login (SSMS) ——
      {
        key: "server",
        label: "Server name",
        kind: "string",
        placeholder: "localhost  ·  .\\SQLEXPRESS  ·  host,1433",
        description: "Same as SSMS “Server name”",
        default: "localhost",
        required: true,
        when: { key: "authentication", equals: ["windows", "sql"] },
      },
      {
        key: "user",
        label: "Login",
        kind: "string",
        placeholder: "sa",
        required: true,
        when: { key: "authentication", equals: "sql" },
      },
      {
        key: "password",
        label: "Password",
        kind: "password",
        required: true,
        when: { key: "authentication", equals: "sql" },
      },
      // —— Connection Properties ——
      {
        key: "database",
        label: "Connect to database",
        kind: "string",
        placeholder: "<default>",
        description: "Type or select the name of the database for the connection",
        when: { key: "authentication", equals: ["windows", "sql"] },
      },
      {
        key: "networkProtocol",
        label: "Network protocol",
        kind: "select",
        options: [
          { value: "default", label: "<default>" },
          { value: "tcp", label: "TCP/IP" },
          { value: "np", label: "Named Pipes" },
          { value: "lpc", label: "Shared Memory" },
        ],
        default: "default",
        when: { key: "authentication", equals: ["windows", "sql"] },
      },
      {
        key: "packetSize",
        label: "Network packet size (bytes)",
        kind: "number",
        default: 4096,
        description: "SSMS default is 4096",
        when: { key: "authentication", equals: ["windows", "sql"] },
      },
      {
        key: "connectionTimeout",
        label: "Connection time-out (seconds)",
        kind: "number",
        default: 30,
        when: { key: "authentication", equals: ["windows", "sql"] },
      },
      {
        key: "requestTimeout",
        label: "Execution time-out (seconds)",
        kind: "number",
        default: 0,
        description: "0 = wait indefinitely (SSMS default)",
        when: { key: "authentication", equals: ["windows", "sql"] },
      },
      {
        key: "encrypt",
        label: "Encrypt connection",
        kind: "boolean",
        default: true,
        when: { key: "authentication", equals: ["windows", "sql"] },
      },
      {
        key: "trustServerCertificate",
        label: "Trust server certificate",
        kind: "boolean",
        default: true,
        description: "Typical for local / self-signed SQL Server TLS",
        when: { key: "authentication", equals: ["windows", "sql"] },
      },
      {
        key: "applicationIntent",
        label: "Application intent",
        kind: "select",
        options: [
          { value: "ReadWrite", label: "ReadWrite" },
          { value: "ReadOnly", label: "ReadOnly" },
        ],
        default: "ReadWrite",
        when: { key: "authentication", equals: ["windows", "sql"] },
      },
      {
        key: "multiSubnetFailover",
        label: "MultiSubnetFailover",
        kind: "boolean",
        default: false,
        when: { key: "authentication", equals: ["windows", "sql"] },
      },
      {
        key: "additionalParameters",
        label: "Additional connection parameters",
        kind: "textarea",
        placeholder: "Application Name=DataFlow;Workstation ID=…",
        description:
          "Extra key=value; pairs appended to the connection string (SSMS “Additional Connection Parameters”)",
        when: { key: "authentication", equals: ["windows", "sql"] },
      },
      // —— Plain connection string ——
      {
        key: "connectionString",
        label: "Connection string",
        kind: "textarea",
        placeholder:
          "Server=localhost;Database=master;Trusted_Connection=True;Encrypt=True;TrustServerCertificate=True;",
        description: "Full ADO.NET / tedious connection string",
        required: true,
        when: { key: "authentication", equals: "connectionString" },
      },
    ],
  },
  {
    type: "smtp",
    displayName: "SMTP",
    category: "Email",
    fields: [
      { key: "host", label: "Host", kind: "string", placeholder: "smtp.example.com" },
      { key: "port", label: "Port", kind: "number", default: 587 },
      { key: "user", label: "User", kind: "string" },
      { key: "password", label: "Password", kind: "password" },
      { key: "from", label: "From", kind: "string", placeholder: "noreply@example.com" },
      {
        key: "smtpUrl",
        label: "SMTP URL (optional)",
        kind: "string",
        placeholder: "smtps://user:pass@host:465",
        description: "If set, used instead of host/user/password",
      },
    ],
  },
  {
    type: "imap",
    displayName: "IMAP",
    category: "Email",
    fields: [
      { key: "host", label: "Host", kind: "string", placeholder: "imap.example.com", required: true },
      { key: "port", label: "Port", kind: "number", default: 993 },
      { key: "user", label: "User", kind: "string", required: true },
      { key: "password", label: "Password", kind: "password", required: true },
      { key: "tls", label: "TLS", kind: "boolean", default: true },
    ],
  },
  {
    type: "ftp",
    displayName: "FTP",
    category: "Files",
    fields: [
      { key: "host", label: "Host", kind: "string", required: true },
      { key: "port", label: "Port", kind: "number", default: 21 },
      { key: "user", label: "User", kind: "string" },
      { key: "password", label: "Password", kind: "password" },
      { key: "secure", label: "FTPS", kind: "boolean", default: false },
    ],
  },
  {
    type: "ssh",
    displayName: "SSH Password",
    category: "Files",
    fields: [
      { key: "host", label: "Host", kind: "string", required: true },
      { key: "port", label: "Port", kind: "number", default: 22 },
      { key: "username", label: "Username", kind: "string", required: true },
      { key: "password", label: "Password", kind: "password" },
      {
        key: "privateKey",
        label: "Private key (optional)",
        kind: "textarea",
        description: "PEM private key; used instead of password when set",
      },
    ],
  },
  {
    type: "httpHeaderAuth",
    displayName: "Header Auth",
    category: "HTTP",
    fields: [
      { key: "name", label: "Header name", kind: "string", placeholder: "Authorization", required: true },
      { key: "value", label: "Header value", kind: "password", required: true },
    ],
  },
  {
    type: "httpBasicAuth",
    displayName: "Basic Auth",
    category: "HTTP",
    fields: [
      { key: "user", label: "User", kind: "string", required: true },
      { key: "password", label: "Password", kind: "password", required: true },
    ],
  },
  {
    type: "httpBearerAuth",
    displayName: "Bearer Auth",
    category: "HTTP",
    fields: [
      { key: "token", label: "Token", kind: "password", required: true },
    ],
  },
  {
    type: "ldap",
    displayName: "LDAP",
    category: "Other",
    fields: [
      { key: "url", label: "LDAP URL", kind: "string", placeholder: "ldap://host:389", required: true },
      { key: "bindDN", label: "Bind DN", kind: "string" },
      { key: "bindPassword", label: "Bind password", kind: "password" },
      { key: "baseDN", label: "Base DN", kind: "string" },
    ],
  },
  {
    type: "generic",
    displayName: "Generic (JSON)",
    description: "Arbitrary key/value secrets",
    category: "Other",
    fields: [
      {
        key: "_json",
        label: "Secret data (JSON)",
        kind: "textarea",
        placeholder: '{\n  "apiKey": "…"\n}',
        required: true,
      },
    ],
  },
];

export function getCredentialType(type: string): CredentialTypeDef | undefined {
  return CREDENTIAL_TYPES.find((t) => t.type === type);
}

export function findCredentialTypes(query: string): CredentialTypeDef[] {
  const q = query.trim().toLowerCase();
  if (!q) return CREDENTIAL_TYPES;
  return CREDENTIAL_TYPES.filter(
    (t) =>
      t.type.toLowerCase().includes(q) ||
      t.displayName.toLowerCase().includes(q) ||
      t.category.toLowerCase().includes(q) ||
      (t.description ?? "").toLowerCase().includes(q),
  );
}

export function fieldIsVisible(
  field: CredentialFieldDef,
  values: Record<string, string | number | boolean>,
): boolean {
  if (!field.when) return true;
  const current = String(values[field.when.key] ?? "");
  const expected = field.when.equals;
  return Array.isArray(expected)
    ? expected.includes(current)
    : current === expected;
}

export function defaultFieldValues(def: CredentialTypeDef): Record<string, string | number | boolean> {
  const out: Record<string, string | number | boolean> = {};
  for (const f of def.fields) {
    if (f.default !== undefined) out[f.key] = f.default;
    else if (f.kind === "boolean") out[f.key] = false;
    else if (f.kind === "number") out[f.key] = 0;
    else out[f.key] = "";
  }
  return out;
}

/** Build Tedious/ODBC-friendly connection string from SSMS-style fields. */
export function buildMssqlConnectionString(
  values: Record<string, unknown>,
): string {
  const auth = String(values.authentication ?? "windows").toLowerCase();
  const plain = String(values.connectionString ?? "").trim();
  if (auth === "connectionstring" || (plain && !String(values.server ?? "").trim())) {
    return plain;
  }
  if (auth === "connectionstring") return plain;

  const server = String(values.server ?? "").trim() || "localhost";
  const database = String(values.database ?? "").trim();
  const encrypt = values.encrypt !== false && values.encrypt !== "false";
  const trust =
    values.trustServerCertificate !== false &&
    values.trustServerCertificate !== "false";

  const parts: string[] = [`Server=${server}`];
  if (database) parts.push(`Database=${database}`);

  if (auth === "sql" || auth === "sqlserver") {
    const user = String(values.user ?? values.username ?? "").trim();
    const password = String(values.password ?? "");
    if (user) parts.push(`User Id=${user}`);
    if (password) parts.push(`Password=${password}`);
  } else {
    parts.push("Trusted_Connection=True");
  }

  parts.push(`Encrypt=${encrypt ? "True" : "False"}`);
  parts.push(`TrustServerCertificate=${trust ? "True" : "False"}`);

  const protocol = String(values.networkProtocol ?? "default").toLowerCase();
  if (protocol === "tcp") parts.push("Network Library=dbmssocn");
  else if (protocol === "np") parts.push("Network Library=dbnmpntw");
  else if (protocol === "lpc") parts.push("Network Library=dbmslpcn");

  const packetSize = Number(values.packetSize ?? 0);
  if (packetSize > 0) parts.push(`Packet Size=${packetSize}`);

  const connectionTimeout = Number(values.connectionTimeout ?? 30);
  if (Number.isFinite(connectionTimeout)) {
    parts.push(`Connection Timeout=${connectionTimeout}`);
  }

  // 0 = infinite in SSMS; tedious uses 0 similarly for requestTimeout
  const requestTimeout = Number(values.requestTimeout ?? 0);
  if (Number.isFinite(requestTimeout)) {
    parts.push(`Command Timeout=${requestTimeout}`);
  }

  const intent = String(values.applicationIntent ?? "").trim();
  if (intent && intent !== "ReadWrite") {
    parts.push(`ApplicationIntent=${intent}`);
  }

  if (
    values.multiSubnetFailover === true ||
    values.multiSubnetFailover === "true"
  ) {
    parts.push("MultiSubnetFailover=True");
  }

  const extra = String(values.additionalParameters ?? "").trim();
  if (extra) {
    const cleaned = extra.replace(/^;+|;+$/g, "");
    if (cleaned) parts.push(cleaned);
  }

  return parts.join(";");
}

/** Build API `data` object from form values for a credential type. */
export function buildCredentialData(
  def: CredentialTypeDef,
  values: Record<string, string | number | boolean>,
): Record<string, unknown> {
  if (def.type === "generic") {
    const raw = String(values._json ?? "").trim();
    if (!raw) return {};
    return JSON.parse(raw) as Record<string, unknown>;
  }
  const data: Record<string, unknown> = {};
  for (const f of def.fields) {
    if (!fieldIsVisible(f, values)) continue;
    const v = values[f.key];
    if (v === undefined || v === "") continue;
    data[f.key] = f.kind === "number" ? Number(v) : v;
  }
  if (def.type === "microsoftSql") {
    data.connectionString = buildMssqlConnectionString({ ...values, ...data });
  }
  return data;
}
