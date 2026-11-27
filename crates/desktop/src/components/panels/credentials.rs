use dioxus::prelude::*;

use crate::components::icons::X;
use crate::components::panels::{json_bool, json_id, json_string};
use crate::components::ui::{
    Alert, AlertDescription, AlertVariant, Button, ButtonSize, ButtonVariant, ConfirmDeleteDialog,
    DataTable, DataTableBody, DataTableCell, DataTableHead, DataTableHeader, DataTableRow,
    DataTableWrapper, Empty, EmptyDescription, Input, Label, PageLoader, Select, SelectOption,
    Switch, Textarea,
};

#[derive(Clone, Copy)]
struct CredFieldOption {
    value: &'static str,
    label: &'static str,
}

#[derive(Clone, Copy)]
struct CredFieldDef {
    key: &'static str,
    label: &'static str,
    kind: &'static str,
    required: bool,
    placeholder: &'static str,
    description: &'static str,
    /// String form of default (`"true"`, `"587"`, `"windows"`, …).
    default: Option<&'static str>,
    options: &'static [CredFieldOption],
    when_key: Option<&'static str>,
    when_equals: &'static [&'static str],
}

#[derive(Clone, Copy)]
struct CredTypeDef {
    type_name: &'static str,
    display_name: &'static str,
    category: &'static str,
    description: &'static str,
    fields: &'static [CredFieldDef],
}

const MS_CLIENT_CREDS: &[CredFieldDef] = &[
    CredFieldDef {
        key: "tenantId",
        label: "Tenant ID",
        kind: "string",
        required: true,
        placeholder: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
        description: "",
        default: None,
        options: &[],
        when_key: None,
        when_equals: &[],
    },
    CredFieldDef {
        key: "clientId",
        label: "Client ID",
        kind: "string",
        required: true,
        placeholder: "Application (client) ID",
        description: "",
        default: None,
        options: &[],
        when_key: None,
        when_equals: &[],
    },
    CredFieldDef {
        key: "clientSecret",
        label: "Client Secret",
        kind: "password",
        required: true,
        placeholder: "",
        description: "",
        default: None,
        options: &[],
        when_key: None,
        when_equals: &[],
    },
    CredFieldDef {
        key: "scope",
        label: "Scope",
        kind: "string",
        required: false,
        placeholder: "",
        description: "OAuth scope (include /.default for client credentials when needed)",
        default: Some("https://graph.microsoft.com/.default"),
        options: &[],
        when_key: None,
        when_equals: &[],
    },
];

const DV_DEV_APP_ID: &str = "51f81489-12ee-4a9e-aaae-a2591f45987d";

const DATAVERSE_FIELDS: &[CredFieldDef] = &[
    CredFieldDef {
        key: "authMethod",
        label: "Connection method",
        kind: "select",
        required: true,
        placeholder: "",
        description: "Same methods as XrmToolBox → New connection",
        default: Some("clientSecret"),
        options: &[
            CredFieldOption {
                value: "clientSecret",
                label: "Client Id / Secret",
            },
            CredFieldOption {
                value: "connectionString",
                label: "Connection String",
            },
            CredFieldOption {
                value: "oauthMfa",
                label: "OAuth / MFA",
            },
            CredFieldOption {
                value: "sdkLogin",
                label: "Microsoft Login Control",
            },
            CredFieldOption {
                value: "certificate",
                label: "Certificate",
            },
        ],
        when_key: None,
        when_equals: &[],
    },
    CredFieldDef {
        key: "scope",
        label: "Environment URL",
        kind: "string",
        required: true,
        placeholder: "https://org.crm4.dynamics.com",
        description: "Dataverse / Dynamics org URL",
        default: Some("https://<your-org-name>.crm4.dynamics.com"),
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["clientSecret", "oauthMfa", "sdkLogin", "certificate"],
    },
    CredFieldDef {
        key: "tenantId",
        label: "Tenant ID",
        kind: "string",
        required: true,
        placeholder: "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
        description: "",
        default: None,
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["clientSecret", "certificate"],
    },
    CredFieldDef {
        key: "clientId",
        label: "Client ID",
        kind: "string",
        required: true,
        placeholder: "Azure AD application (client) ID",
        description: "",
        default: None,
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["clientSecret", "certificate"],
    },
    CredFieldDef {
        key: "clientSecret",
        label: "Client Secret",
        kind: "password",
        required: true,
        placeholder: "",
        description: "",
        default: None,
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["clientSecret"],
    },
    CredFieldDef {
        key: "connectionString",
        label: "Connection string",
        kind: "textarea",
        required: true,
        placeholder: "AuthType=ClientSecret;Url=https://org.crm.dynamics.com;ClientId=…;ClientSecret=…",
        description: "XRM Tooling connection string (AuthType=ClientSecret | OAuth | Certificate)",
        default: None,
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["connectionString"],
    },
    CredFieldDef {
        key: "oauthClientId",
        label: "Application (Client) ID",
        kind: "string",
        required: true,
        placeholder: "",
        description: "Defaults to Microsoft sample app — same as XrmToolBox “Use development Azure AD app”",
        default: Some(DV_DEV_APP_ID),
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["oauthMfa", "sdkLogin"],
    },
    CredFieldDef {
        key: "redirectUri",
        label: "Redirect URI",
        kind: "string",
        required: true,
        placeholder: "http://localhost",
        description: "",
        default: Some("http://localhost"),
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["oauthMfa", "sdkLogin"],
    },
    CredFieldDef {
        key: "username",
        label: "User email",
        kind: "string",
        required: false,
        placeholder: "user@contoso.onmicrosoft.com",
        description: "Account used for interactive / MFA sign-in (hint for token cache)",
        default: None,
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["oauthMfa", "sdkLogin"],
    },
    CredFieldDef {
        key: "tenantIdOAuth",
        label: "Tenant ID",
        kind: "string",
        required: false,
        placeholder: "common  ·  or your tenant GUID",
        description: "",
        default: Some("common"),
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["oauthMfa", "sdkLogin"],
    },
    CredFieldDef {
        key: "refreshToken",
        label: "Refresh token",
        kind: "password",
        required: false,
        placeholder: "",
        description: "Paste after interactive login. Used to renew access tokens.",
        default: None,
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["oauthMfa", "sdkLogin"],
    },
    CredFieldDef {
        key: "accessToken",
        label: "Access token (optional)",
        kind: "password",
        required: false,
        placeholder: "",
        description: "Bearer token if you already have one; otherwise refresh token is used",
        default: None,
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["oauthMfa", "sdkLogin"],
    },
    CredFieldDef {
        key: "certificatePem",
        label: "Certificate private key (PEM)",
        kind: "textarea",
        required: true,
        placeholder: "-----BEGIN PRIVATE KEY-----\n…\n-----END PRIVATE KEY-----",
        description: "PEM for client assertion (application user + certificate auth)",
        default: None,
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["certificate"],
    },
    CredFieldDef {
        key: "certificatePassword",
        label: "Certificate password",
        kind: "password",
        required: false,
        placeholder: "",
        description: "Optional — if the PEM / PFX is encrypted",
        default: None,
        options: &[],
        when_key: Some("authMethod"),
        when_equals: &["certificate"],
    },
];

const MSSQL_FIELDS: &[CredFieldDef] = &[
    CredFieldDef {
        key: "authentication",
        label: "Authentication",
        kind: "select",
        required: true,
        placeholder: "",
        description: "",
        default: Some("windows"),
        options: &[
            CredFieldOption {
                value: "windows",
                label: "Windows Authentication",
            },
            CredFieldOption {
                value: "sql",
                label: "SQL Server Authentication",
            },
            CredFieldOption {
                value: "connectionString",
                label: "Connection String",
            },
        ],
        when_key: None,
        when_equals: &[],
    },
    CredFieldDef {
        key: "server",
        label: "Server name",
        kind: "string",
        required: true,
        placeholder: "localhost  ·  .\\SQLEXPRESS  ·  host,1433",
        description: "Same as SSMS “Server name”",
        default: Some("localhost"),
        options: &[],
        when_key: Some("authentication"),
        when_equals: &["windows", "sql"],
    },
    CredFieldDef {
        key: "user",
        label: "Login",
        kind: "string",
        required: true,
        placeholder: "sa",
        description: "",
        default: None,
        options: &[],
        when_key: Some("authentication"),
        when_equals: &["sql"],
    },
    CredFieldDef {
        key: "password",
        label: "Password",
        kind: "password",
        required: true,
        placeholder: "",
        description: "",
        default: None,
        options: &[],
        when_key: Some("authentication"),
        when_equals: &["sql"],
    },
    CredFieldDef {
        key: "database",
        label: "Connect to database",
        kind: "string",
        required: false,
        placeholder: "<default>",
        description: "Type or select the name of the database for the connection",
        default: None,
        options: &[],
        when_key: Some("authentication"),
        when_equals: &["windows", "sql"],
    },
    CredFieldDef {
        key: "networkProtocol",
        label: "Network protocol",
        kind: "select",
        required: false,
        placeholder: "",
        description: "",
        default: Some("default"),
        options: &[
            CredFieldOption {
                value: "default",
                label: "<default>",
            },
            CredFieldOption {
                value: "tcp",
                label: "TCP/IP",
            },
            CredFieldOption {
                value: "np",
                label: "Named Pipes",
            },
            CredFieldOption {
                value: "lpc",
                label: "Shared Memory",
            },
        ],
        when_key: Some("authentication"),
        when_equals: &["windows", "sql"],
    },
    CredFieldDef {
        key: "packetSize",
        label: "Network packet size (bytes)",
        kind: "number",
        required: false,
        placeholder: "",
        description: "SSMS default is 4096",
        default: Some("4096"),
        options: &[],
        when_key: Some("authentication"),
        when_equals: &["windows", "sql"],
    },
    CredFieldDef {
        key: "connectionTimeout",
        label: "Connection time-out (seconds)",
        kind: "number",
        required: false,
        placeholder: "",
        description: "",
        default: Some("30"),
        options: &[],
        when_key: Some("authentication"),
        when_equals: &["windows", "sql"],
    },
    CredFieldDef {
        key: "requestTimeout",
        label: "Execution time-out (seconds)",
        kind: "number",
        required: false,
        placeholder: "",
        description: "0 = wait indefinitely (SSMS default)",
        default: Some("0"),
        options: &[],
        when_key: Some("authentication"),
        when_equals: &["windows", "sql"],
    },
    CredFieldDef {
        key: "encrypt",
        label: "Encrypt connection",
        kind: "boolean",
        required: false,
        placeholder: "",
        description: "",
        default: Some("true"),
        options: &[],
        when_key: Some("authentication"),
        when_equals: &["windows", "sql"],
    },
    CredFieldDef {
        key: "trustServerCertificate",
        label: "Trust server certificate",
        kind: "boolean",
        required: false,
        placeholder: "",
        description: "Typical for local / self-signed SQL Server TLS",
        default: Some("true"),
        options: &[],
        when_key: Some("authentication"),
        when_equals: &["windows", "sql"],
    },
    CredFieldDef {
        key: "applicationIntent",
        label: "Application intent",
        kind: "select",
        required: false,
        placeholder: "",
        description: "",
        default: Some("ReadWrite"),
        options: &[
            CredFieldOption {
                value: "ReadWrite",
                label: "ReadWrite",
            },
            CredFieldOption {
                value: "ReadOnly",
                label: "ReadOnly",
            },
        ],
        when_key: Some("authentication"),
        when_equals: &["windows", "sql"],
    },
    CredFieldDef {
        key: "multiSubnetFailover",
        label: "MultiSubnetFailover",
        kind: "boolean",
        required: false,
        placeholder: "",
        description: "",
        default: Some("false"),
        options: &[],
        when_key: Some("authentication"),
        when_equals: &["windows", "sql"],
    },
    CredFieldDef {
        key: "additionalParameters",
        label: "Additional connection parameters",
        kind: "textarea",
        required: false,
        placeholder: "Application Name=DataFlow;Workstation ID=…",
        description: "Extra key=value; pairs appended to the connection string",
        default: None,
        options: &[],
        when_key: Some("authentication"),
        when_equals: &["windows", "sql"],
    },
    CredFieldDef {
        key: "connectionString",
        label: "Connection string",
        kind: "textarea",
        required: true,
        placeholder:
            "Server=localhost;Database=master;Trusted_Connection=True;Encrypt=True;TrustServerCertificate=True;",
        description: "Full ADO.NET / tedious connection string",
        default: None,
        options: &[],
        when_key: Some("authentication"),
        when_equals: &["connectionString"],
    },
];

const CREDENTIAL_TYPES: &[CredTypeDef] = &[
    CredTypeDef {
        type_name: "dataverseAuth",
        display_name: "Dataverse Auth",
        category: "Microsoft",
        description: "Dataverse / Dynamics 365 — XrmToolBox-style connection methods",
        fields: DATAVERSE_FIELDS,
    },
    CredTypeDef {
        type_name: "microsoftDynamicsOAuth2Api",
        display_name: "Microsoft Dynamics OAuth2 API",
        category: "Microsoft",
        description: "Same Dataverse connection methods as Dataverse Auth",
        fields: DATAVERSE_FIELDS,
    },
    CredTypeDef {
        type_name: "microsoftGraphOAuth2Api",
        display_name: "Microsoft Graph OAuth2 API",
        category: "Microsoft",
        description: "",
        fields: MS_CLIENT_CREDS,
    },
    CredTypeDef {
        type_name: "microsoftOutlookOAuth2Api",
        display_name: "Microsoft Outlook OAuth2 API",
        category: "Microsoft",
        description: "",
        fields: MS_CLIENT_CREDS,
    },
    CredTypeDef {
        type_name: "microsoftTeamsOAuth2Api",
        display_name: "Microsoft Teams OAuth2 API",
        category: "Microsoft",
        description: "",
        fields: MS_CLIENT_CREDS,
    },
    CredTypeDef {
        type_name: "microsoftExcelOAuth2Api",
        display_name: "Microsoft Excel OAuth2 API",
        category: "Microsoft",
        description: "",
        fields: MS_CLIENT_CREDS,
    },
    CredTypeDef {
        type_name: "microsoftSharePointOAuth2Api",
        display_name: "Microsoft SharePoint OAuth2 API",
        category: "Microsoft",
        description: "",
        fields: MS_CLIENT_CREDS,
    },
    CredTypeDef {
        type_name: "microsoftSql",
        display_name: "Microsoft SQL",
        category: "Microsoft",
        description: "Connect like SSMS — Login + Connection Properties",
        fields: MSSQL_FIELDS,
    },
    CredTypeDef {
        type_name: "smtp",
        display_name: "SMTP",
        category: "Email",
        description: "",
        fields: &[
            CredFieldDef {
                key: "host",
                label: "Host",
                kind: "string",
                required: false,
                placeholder: "smtp.example.com",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "port",
                label: "Port",
                kind: "number",
                required: false,
                placeholder: "",
                description: "",
                default: Some("587"),
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "user",
                label: "User",
                kind: "string",
                required: false,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "password",
                label: "Password",
                kind: "password",
                required: false,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "from",
                label: "From",
                kind: "string",
                required: false,
                placeholder: "noreply@example.com",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "smtpUrl",
                label: "SMTP URL (optional)",
                kind: "string",
                required: false,
                placeholder: "smtps://user:pass@host:465",
                description: "If set, used instead of host/user/password",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
        ],
    },
    CredTypeDef {
        type_name: "imap",
        display_name: "IMAP",
        category: "Email",
        description: "",
        fields: &[
            CredFieldDef {
                key: "host",
                label: "Host",
                kind: "string",
                required: true,
                placeholder: "imap.example.com",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "port",
                label: "Port",
                kind: "number",
                required: false,
                placeholder: "",
                description: "",
                default: Some("993"),
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "user",
                label: "User",
                kind: "string",
                required: true,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "password",
                label: "Password",
                kind: "password",
                required: true,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "tls",
                label: "TLS",
                kind: "boolean",
                required: false,
                placeholder: "",
                description: "",
                default: Some("true"),
                options: &[],
                when_key: None,
                when_equals: &[],
            },
        ],
    },
    CredTypeDef {
        type_name: "ftp",
        display_name: "FTP",
        category: "Files",
        description: "",
        fields: &[
            CredFieldDef {
                key: "host",
                label: "Host",
                kind: "string",
                required: true,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "port",
                label: "Port",
                kind: "number",
                required: false,
                placeholder: "",
                description: "",
                default: Some("21"),
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "user",
                label: "User",
                kind: "string",
                required: false,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "password",
                label: "Password",
                kind: "password",
                required: false,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "secure",
                label: "FTPS",
                kind: "boolean",
                required: false,
                placeholder: "",
                description: "",
                default: Some("false"),
                options: &[],
                when_key: None,
                when_equals: &[],
            },
        ],
    },
    CredTypeDef {
        type_name: "ssh",
        display_name: "SSH Password",
        category: "Files",
        description: "",
        fields: &[
            CredFieldDef {
                key: "host",
                label: "Host",
                kind: "string",
                required: true,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "port",
                label: "Port",
                kind: "number",
                required: false,
                placeholder: "",
                description: "",
                default: Some("22"),
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "username",
                label: "Username",
                kind: "string",
                required: true,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "password",
                label: "Password",
                kind: "password",
                required: false,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "privateKey",
                label: "Private key (optional)",
                kind: "textarea",
                required: false,
                placeholder: "",
                description: "PEM private key; used instead of password when set",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
        ],
    },
    CredTypeDef {
        type_name: "httpHeaderAuth",
        display_name: "Header Auth",
        category: "HTTP",
        description: "",
        fields: &[
            CredFieldDef {
                key: "name",
                label: "Header name",
                kind: "string",
                required: true,
                placeholder: "Authorization",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "value",
                label: "Header value",
                kind: "password",
                required: true,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
        ],
    },
    CredTypeDef {
        type_name: "httpBasicAuth",
        display_name: "Basic Auth",
        category: "HTTP",
        description: "",
        fields: &[
            CredFieldDef {
                key: "user",
                label: "User",
                kind: "string",
                required: true,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "password",
                label: "Password",
                kind: "password",
                required: true,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
        ],
    },
    CredTypeDef {
        type_name: "httpBearerAuth",
        display_name: "Bearer Auth",
        category: "HTTP",
        description: "",
        fields: &[CredFieldDef {
            key: "token",
            label: "Token",
            kind: "password",
            required: true,
            placeholder: "",
            description: "",
            default: None,
            options: &[],
            when_key: None,
            when_equals: &[],
        }],
    },
    CredTypeDef {
        type_name: "ldap",
        display_name: "LDAP",
        category: "Other",
        description: "",
        fields: &[
            CredFieldDef {
                key: "url",
                label: "LDAP URL",
                kind: "string",
                required: true,
                placeholder: "ldap://host:389",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "bindDN",
                label: "Bind DN",
                kind: "string",
                required: false,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "bindPassword",
                label: "Bind password",
                kind: "password",
                required: false,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
            CredFieldDef {
                key: "baseDN",
                label: "Base DN",
                kind: "string",
                required: false,
                placeholder: "",
                description: "",
                default: None,
                options: &[],
                when_key: None,
                when_equals: &[],
            },
        ],
    },
    CredTypeDef {
        type_name: "generic",
        display_name: "Generic (JSON)",
        category: "Other",
        description: "Arbitrary key/value secrets",
        fields: &[CredFieldDef {
            key: "_json",
            label: "Secret data (JSON)",
            kind: "textarea",
            required: true,
            placeholder: "{\n  \"apiKey\": \"…\"\n}",
            description: "",
            default: None,
            options: &[],
            when_key: None,
            when_equals: &[],
        }],
    },
];

fn field_is_visible(field: &CredFieldDef, values: &serde_json::Map<String, serde_json::Value>) -> bool {
    let Some(key) = field.when_key else {
        return true;
    };
    let current = values.get(key).map(json_string).unwrap_or_default();
    field.when_equals.iter().any(|eq| *eq == current)
}

fn default_field_values(def: &CredTypeDef) -> serde_json::Map<String, serde_json::Value> {
    let mut out = serde_json::Map::new();
    for f in def.fields {
        let value = if let Some(d) = f.default {
            match f.kind {
                "boolean" => serde_json::Value::Bool(d == "true"),
                "number" => serde_json::Value::Number(
                    d.parse::<i64>()
                        .ok()
                        .map(serde_json::Number::from)
                        .or_else(|| {
                            d.parse::<f64>()
                                .ok()
                                .and_then(serde_json::Number::from_f64)
                        })
                        .unwrap_or_else(|| serde_json::Number::from(0)),
                ),
                _ => serde_json::Value::String(d.to_string()),
            }
        } else if f.kind == "boolean" {
            serde_json::Value::Bool(false)
        } else if f.kind == "number" {
            serde_json::Value::Number(0.into())
        } else {
            serde_json::Value::String(String::new())
        };
        out.insert(f.key.to_string(), value);
    }
    out
}

fn data_to_field_values(
    def: &CredTypeDef,
    data: &serde_json::Value,
) -> serde_json::Map<String, serde_json::Value> {
    let mut out = default_field_values(def);
    let Some(obj) = data.as_object() else {
        return out;
    };
    if def.type_name == "generic" {
        out.insert(
            "_json".into(),
            serde_json::Value::String(
                serde_json::to_string_pretty(data).unwrap_or_else(|_| "{}".into()),
            ),
        );
        return out;
    }
    for f in def.fields {
        let Some(v) = obj.get(f.key) else {
            continue;
        };
        match f.kind {
            "boolean" => {
                out.insert(f.key.to_string(), serde_json::Value::Bool(v.as_bool().unwrap_or(false)));
            }
            "number" => {
                let n = v
                    .as_i64()
                    .map(serde_json::Number::from)
                    .or_else(|| v.as_f64().and_then(serde_json::Number::from_f64))
                    .or_else(|| {
                        v.as_str()
                            .and_then(|s| s.parse::<i64>().ok())
                            .map(serde_json::Number::from)
                    });
                if let Some(n) = n {
                    out.insert(f.key.to_string(), serde_json::Value::Number(n));
                }
            }
            _ => {
                out.insert(
                    f.key.to_string(),
                    serde_json::Value::String(json_string(v)),
                );
            }
        }
    }
    out
}

fn map_str(values: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    values.get(key).map(json_string).unwrap_or_default()
}

fn map_truthy(values: &serde_json::Map<String, serde_json::Value>, key: &str, default: bool) -> bool {
    match values.get(key) {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::String(s)) => {
            let s = s.to_lowercase();
            if s == "false" || s == "0" || s.is_empty() {
                false
            } else {
                true
            }
        }
        Some(serde_json::Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
        Some(_) => true,
        None => default,
    }
}

fn mask_mssql_connection_string(cs: &str) -> String {
    cs.split(';')
        .map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return String::new();
            }
            let Some((k, _)) = part.split_once('=') else {
                return part.to_string();
            };
            if k.trim().eq_ignore_ascii_case("Password") || k.trim().eq_ignore_ascii_case("PWD") {
                format!("{}=••••••••", k.trim())
            } else {
                part.to_string()
            }
        })
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(";")
}

fn build_mssql_connection_string(values: &serde_json::Map<String, serde_json::Value>) -> String {
    let auth = map_str(values, "authentication").to_lowercase();
    let plain = map_str(values, "connectionString").trim().to_string();
    if auth == "connectionstring" || (!plain.is_empty() && map_str(values, "server").trim().is_empty())
    {
        return plain;
    }

    let server = {
        let s = map_str(values, "server");
        let t = s.trim();
        if t.is_empty() {
            "localhost".to_string()
        } else {
            t.to_string()
        }
    };
    let database = map_str(values, "database").trim().to_string();
    let encrypt = map_truthy(values, "encrypt", true);
    let trust = map_truthy(values, "trustServerCertificate", true);

    let mut parts: Vec<String> = vec![format!("Server={server}")];
    if !database.is_empty() {
        parts.push(format!("Database={database}"));
    }

    if auth == "sql" || auth == "sqlserver" {
        let user = {
            let u = map_str(values, "user");
            if u.trim().is_empty() {
                map_str(values, "username")
            } else {
                u
            }
        };
        let password = map_str(values, "password");
        if !user.trim().is_empty() {
            parts.push(format!("User Id={}", user.trim()));
        }
        if !password.is_empty() {
            parts.push(format!("Password={password}"));
        }
    } else {
        parts.push("Trusted_Connection=True".into());
    }

    parts.push(format!("Encrypt={}", if encrypt { "True" } else { "False" }));
    parts.push(format!(
        "TrustServerCertificate={}",
        if trust { "True" } else { "False" }
    ));

    let protocol = map_str(values, "networkProtocol").to_lowercase();
    if protocol == "tcp" {
        parts.push("Network Library=dbmssocn".into());
    } else if protocol == "np" {
        parts.push("Network Library=dbnmpntw".into());
    } else if protocol == "lpc" {
        parts.push("Network Library=dbmslpcn".into());
    }

    if let Ok(packet) = map_str(values, "packetSize").parse::<i64>() {
        if packet > 0 {
            parts.push(format!("Packet Size={packet}"));
        }
    }

    if let Ok(timeout) = map_str(values, "connectionTimeout").parse::<i64>() {
        parts.push(format!("Connection Timeout={timeout}"));
    } else {
        parts.push("Connection Timeout=30".into());
    }

    if let Ok(timeout) = map_str(values, "requestTimeout").parse::<i64>() {
        parts.push(format!("Command Timeout={timeout}"));
    } else {
        parts.push("Command Timeout=0".into());
    }

    let intent = map_str(values, "applicationIntent").trim().to_string();
    if !intent.is_empty() && intent != "ReadWrite" {
        parts.push(format!("ApplicationIntent={intent}"));
    }

    if map_truthy(values, "multiSubnetFailover", false) {
        parts.push("MultiSubnetFailover=True".into());
    }

    let extra = map_str(values, "additionalParameters").trim().to_string();
    if !extra.is_empty() {
        let cleaned = extra.trim_matches(';');
        if !cleaned.is_empty() {
            parts.push(cleaned.to_string());
        }
    }

    parts.join(";")
}

fn values_to_data(
    def: CredTypeDef,
    values: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    if def.type_name == "generic" {
        if let Some(raw) = values.get("_json").and_then(|v| v.as_str()) {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return serde_json::json!({});
            }
            return serde_json::from_str(trimmed)
                .unwrap_or_else(|_| serde_json::json!({ "raw": trimmed }));
        }
        return serde_json::json!({});
    }

    let mut data = serde_json::Map::new();
    for f in def.fields {
        if !field_is_visible(f, values) {
            continue;
        }
        let Some(v) = values.get(f.key) else {
            continue;
        };
        match f.kind {
            "number" => {
                let n = match v {
                    serde_json::Value::Number(n) => Some(n.clone()),
                    serde_json::Value::String(s) if !s.is_empty() => s
                        .parse::<i64>()
                        .ok()
                        .map(serde_json::Number::from)
                        .or_else(|| {
                            s.parse::<f64>()
                                .ok()
                                .and_then(serde_json::Number::from_f64)
                        }),
                    _ => None,
                };
                if let Some(n) = n {
                    data.insert(f.key.to_string(), serde_json::Value::Number(n));
                }
            }
            "boolean" => {
                data.insert(
                    f.key.to_string(),
                    serde_json::Value::Bool(json_bool(
                        &serde_json::Value::Object(values.clone()),
                        f.key,
                        false,
                    )),
                );
            }
            _ => {
                let s = json_string(v);
                if !s.is_empty() {
                    data.insert(f.key.to_string(), serde_json::Value::String(s));
                }
            }
        }
    }

    if def.type_name == "microsoftSql" {
        data.insert(
            "connectionString".into(),
            serde_json::Value::String(build_mssql_connection_string(values)),
        );
    }

    serde_json::Value::Object(data)
}

pub(crate) fn credential_display_name(ty: &str) -> String {
    CREDENTIAL_TYPES
        .iter()
        .find(|t| t.type_name == ty)
        .map(|t| t.display_name.to_string())
        .unwrap_or_else(|| ty.to_string())
}

fn find_credential_type_index(type_name: &str) -> Option<usize> {
    let lower = type_name.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }
    CREDENTIAL_TYPES
        .iter()
        .position(|t| t.type_name == type_name)
        .or_else(|| {
            CREDENTIAL_TYPES.iter().position(|t| {
                let ty = t.type_name.to_lowercase();
                ty.contains(&lower) || lower.contains(&ty)
            })
        })
}

fn find_credential_types(query: &str) -> Vec<&'static CredTypeDef> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return CREDENTIAL_TYPES.iter().collect();
    }
    CREDENTIAL_TYPES
        .iter()
        .filter(|t| {
            t.type_name.to_lowercase().contains(&q)
                || t.display_name.to_lowercase().contains(&q)
                || t.category.to_lowercase().contains(&q)
                || t.description.to_lowercase().contains(&q)
        })
        .collect()
}

fn validate_required(
    def: &CredTypeDef,
    values: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
    for f in def.fields {
        if !f.required || f.kind == "boolean" {
            continue;
        }
        if !field_is_visible(f, values) {
            continue;
        }
        let v = values.get(f.key).map(json_string).unwrap_or_default();
        if v.is_empty() {
            return Some(format!("{} is required", f.label));
        }
    }
    None
}

#[component]
pub fn CredentialDialog(
    open: bool,
    on_open_change: EventHandler<bool>,
    on_created: EventHandler<serde_json::Value>,
    #[props(default)] preferred_type: Option<String>,
    #[props(default)] edit_id: Option<String>,
    #[props(default)] edit_name: Option<String>,
    #[props(default)] edit_type: Option<String>,
) -> Element {
    // Init from props so a fresh mount (Edit / Add) lands on the right step immediately.
    // Parent should remount this dialog when opening so these inits re-run.
    let boot_edit = edit_id
        .as_ref()
        .filter(|s| !s.is_empty())
        .cloned();
    let boot_type = edit_type
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| preferred_type.clone().filter(|s| !s.is_empty()));
    let boot_idx = boot_type
        .as_ref()
        .and_then(|t| find_credential_type_index(t));
    let is_edit = boot_edit.is_some();

    let mut step = use_signal(|| {
        if boot_idx.is_some() || boot_edit.is_some() {
            "form".to_string()
        } else {
            "select".to_string()
        }
    });
    let mut type_query = use_signal(String::new);
    let mut selected_idx = use_signal(|| boot_idx);
    let mut name = use_signal(|| {
        if let Some(n) = edit_name.clone().filter(|s| !s.is_empty()) {
            n
        } else if let Some(idx) = boot_idx {
            format!("{} account", CREDENTIAL_TYPES[idx].display_name)
        } else {
            String::new()
        }
    });
    let mut field_values = use_signal(|| {
        boot_idx
            .map(|idx| default_field_values(&CREDENTIAL_TYPES[idx]))
            .unwrap_or_default()
    });
    let mut sidebar = use_signal(|| "connection".to_string());
    let mut busy = use_signal(|| false);
    let mut form_err = use_signal(|| None::<String>);
    let mut editing_id = use_signal(|| boot_edit.clone().unwrap_or_default());
    let mut test_busy = use_signal(|| false);
    let mut test_ok = use_signal(|| None::<bool>);
    let mut test_msg = use_signal(|| None::<String>);
    let mut mssql_databases = use_signal(Vec::<String>::new);

    // Load decrypted fields when editing (dialog is remounted per open).
    use_effect(move || {
        let Some(eid) = boot_edit.clone() else {
            return;
        };
        let path = format!("/api/credentials/{eid}");
        spawn(async move {
            match crate::api::get_json(&path).await {
                Ok(v) => {
                    let cred = v.get("credential").cloned().unwrap_or(v);
                    let loaded_name = cred.get("name").map(json_string).unwrap_or_default();
                    let loaded_ty = cred.get("type").map(json_string).unwrap_or_default();
                    if !loaded_name.is_empty() {
                        name.set(loaded_name);
                    }
                    if let Some(idx) = find_credential_type_index(&loaded_ty) {
                        selected_idx.set(Some(idx));
                        let def = &CREDENTIAL_TYPES[idx];
                        let data = cred.get("data").cloned().unwrap_or(serde_json::json!({}));
                        field_values.set(data_to_field_values(def, &data));
                        step.set("form".into());
                    }
                    editing_id.set(eid);
                }
                Err(e) => form_err.set(Some(e)),
            }
        });
    });

    let close = move || on_open_change.call(false);

    if !open {
        return rsx! {};
    }

    let mut pick_type = move |idx: usize| {
        if let Some(def) = CREDENTIAL_TYPES.get(idx) {
            selected_idx.set(Some(idx));
            name.set(format!("{} account", def.display_name));
            field_values.set(default_field_values(def));
            step.set("form".to_string());
            sidebar.set("connection".to_string());
            form_err.set(None);
            test_busy.set(false);
            test_ok.set(None);
            test_msg.set(None);
            mssql_databases.set(Vec::new());
        }
    };

    let on_save = EventHandler::new(move |_| {
        let trimmed = name().trim().to_string();
        if trimmed.is_empty() {
            form_err.set(Some("Name is required".into()));
            return;
        }
        let Some(idx) = selected_idx() else {
            return;
        };
        let Some(def) = CREDENTIAL_TYPES.get(idx) else {
            return;
        };
        if let Some(msg) = validate_required(def, &field_values()) {
            form_err.set(Some(msg));
            return;
        }
        form_err.set(None);
        busy.set(true);
        let data = values_to_data(*def, &field_values());
        let id = editing_id();
        let body = if id.is_empty() {
            serde_json::json!({
                "name": trimmed,
                "type": def.type_name,
                "data": data,
            })
        } else {
            serde_json::json!({
                "name": trimmed,
                "data": data,
            })
        };
        spawn(async move {
            let res = if id.is_empty() {
                crate::api::post_json("/api/credentials", body).await
            } else {
                let path = format!("/api/credentials/{id}");
                crate::api::put_json(&path, body).await
            };
            super::credentials_picker::invalidate_credentials_cache();
            busy.set(false);
            match res {
                Ok(v) => {
                    let cred = v.get("credential").cloned().unwrap_or(v);
                    on_created.call(cred);
                    on_open_change.call(false);
                }
                Err(e) => form_err.set(Some(e)),
            }
        });
    });

    let on_test_mssql = EventHandler::new(move |_| {
        let Some(idx) = selected_idx() else {
            return;
        };
        let Some(def) = CREDENTIAL_TYPES.get(idx) else {
            return;
        };
        if def.type_name != "microsoftSql" {
            return;
        }
        if let Some(msg) = validate_required(def, &field_values()) {
            test_ok.set(Some(false));
            test_msg.set(Some(msg));
            return;
        }
        test_busy.set(true);
        test_ok.set(None);
        test_msg.set(Some("Testing connection…".into()));
        let data = values_to_data(*def, &field_values());
        let body = serde_json::json!({
            "type": def.type_name,
            "data": data,
        });
        spawn(async move {
            let res = crate::api::post_json("/api/credentials/test", body).await;
            test_busy.set(false);
            match res {
                Ok(v) => {
                    let ok = v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false);
                    let message = v
                        .get("message")
                        .map(json_string)
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| {
                            if ok {
                                "Connection succeeded".into()
                            } else {
                                "Connection failed".into()
                            }
                        });
                    let dbs = v
                        .get("databases")
                        .and_then(|x| x.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    if !dbs.is_empty() {
                        mssql_databases.set(dbs);
                    }
                    test_ok.set(Some(ok));
                    test_msg.set(Some(message));
                }
                Err(e) => {
                    test_ok.set(Some(false));
                    test_msg.set(Some(e));
                }
            }
        });
    });

    rsx! {
        div {
            class: "fixed inset-0 z-[200] flex items-center justify-center bg-black/50 p-4 backdrop-blur-[1px]",
            onclick: move |_| close(),
            div {
                class: if step() == "select" {
                    "flex max-h-[min(90vh,720px)] w-full max-w-md flex-col overflow-hidden rounded-xl border bg-card shadow-2xl"
                } else {
                    "flex max-h-[min(90vh,720px)] w-full max-w-3xl flex-col overflow-hidden rounded-xl border bg-card shadow-2xl"
                },
                onclick: move |e| e.stop_propagation(),
                role: "dialog",
                "aria-modal": "true",
                if step() == "select" {
                    div { class: "border-b px-5 py-4 text-left",
                        h2 { class: "text-lg font-semibold", "Add new credential" }
                        p { class: "text-sm text-muted-foreground",
                            "Select an app or service to connect to"
                        }
                    }
                    div { class: "flex min-h-0 flex-1 flex-col gap-3 p-5",
                        Input {
                            placeholder: Some("Search apps and services…".into()),
                            value: type_query(),
                            oninput: move |e: FormEvent| type_query.set(e.value()),
                        }
                        div { class: "max-h-[min(40vh,320px)] overflow-y-auto rounded-md border",
                            {
                                let types = find_credential_types(&type_query());
                                rsx! {
                                    for (idx, t) in types.iter().enumerate() {
                                        {
                                            let real_idx = CREDENTIAL_TYPES
                                                .iter()
                                                .position(|x| x.type_name == t.type_name)
                                                .unwrap_or(idx);
                                            rsx! {
                                                Button {
                                                    key: "{t.type_name}",
                                                    variant: ButtonVariant::Ghost,
                                                    class: Some("h-auto w-full flex-col items-start gap-0.5 rounded-none border-b px-3 py-2.5 text-left last:border-b-0".into()),
                                                    onclick: move |_| pick_type(real_idx),
                                                    span { class: "text-sm font-medium", "{t.display_name}" }
                                                    span { class: "text-xs text-muted-foreground",
                                                        "{t.category}"
                                                        if !t.description.is_empty() {
                                                            " · {t.description}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if types.is_empty() {
                                        Empty { class: Some("border-0 py-8".into()),
                                            EmptyDescription { "No matching credential types" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "flex justify-end gap-2 border-t px-5 py-3",
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| close(),
                            "Cancel"
                        }
                        Button {
                            disabled: selected_idx().is_none(),
                            onclick: move |_| {
                                if let Some(i) = selected_idx() {
                                    pick_type(i);
                                }
                            },
                            "Continue"
                        }
                    }
                } else if let Some(def) = selected_idx().and_then(|i| CREDENTIAL_TYPES.get(i)) {
                    div { class: "flex items-start justify-between gap-3 border-b px-5 py-4",
                        div { class: "min-w-0 flex-1",
                            h2 { class: "truncate text-lg font-semibold",
                                {name().trim().to_string()}
                            }
                            p { class: "text-sm text-muted-foreground",
                                if is_edit {
                                    "Edit credential · "
                                }
                                "{def.display_name}"
                            }
                        }
                        div { class: "flex shrink-0 gap-2",
                            if !is_edit && preferred_type.is_none() {
                                Button {
                                    size: ButtonSize::Sm,
                                    variant: ButtonVariant::Ghost,
                                    onclick: move |_| step.set("select".to_string()),
                                    "Change type"
                                }
                            }
                            Button {
                                size: ButtonSize::Sm,
                                disabled: busy(),
                                onclick: move |_| on_save.call(()),
                                "Save"
                            }
                            Button {
                                size: ButtonSize::IconSm,
                                variant: ButtonVariant::Ghost,
                                title: Some("Close".into()),
                                onclick: move |_| close(),
                                X { class: Some("h-4 w-4".into()) }
                            }
                        }
                    }
                    div { class: "grid min-h-0 flex-1 grid-cols-1 md:grid-cols-[170px_1fr]",
                        aside { class: "flex flex-row gap-1 border-b p-3 md:flex-col md:border-b-0 md:border-r",
                            for (id, label) in [("connection", "Connection"), ("details", "Details")] {
                                Button {
                                    key: "{id}",
                                    size: ButtonSize::Sm,
                                    variant: if sidebar() == id {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Ghost
                                    },
                                    class: Some("justify-start".into()),
                                    onclick: move |_| sidebar.set(id.to_string()),
                                    "{label}"
                                }
                            }
                        }
                        div { class: "max-h-[min(55vh,480px)] overflow-y-auto p-5",
                            if let Some(msg) = form_err() {
                                Alert {
                                    variant: AlertVariant::Destructive,
                                    class: Some("mb-3".into()),
                                    AlertDescription {
                                        class: Some("text-destructive".into()),
                                        "{msg}"
                                    }
                                }
                            }
                            div { class: "flex max-w-lg flex-col gap-4",
                                div { class: "flex flex-col gap-2",
                                    Label { class: Some("text-xs text-muted-foreground".into()), "Credential name" }
                                    Input {
                                        value: name(),
                                        placeholder: Some(format!("{} account", def.display_name)),
                                        oninput: move |e: FormEvent| name.set(e.value()),
                                    }
                                }
                                if sidebar() == "details" {
                                    p { class: "text-sm text-muted-foreground",
                                        "Type: "
                                        code { class: "text-foreground", "{def.type_name}" }
                                    }
                                } else {
                                    for f in def.fields {
                                        {
                                            if !field_is_visible(f, &field_values()) {
                                                rsx! {}
                                            } else {
                                                let key = f.key.to_string();
                                                let fkey = f.key;
                                                let current = field_values()
                                                    .get(fkey)
                                                    .map(json_string)
                                                    .unwrap_or_default();
                                                let db_list = mssql_databases();
                                                let use_db_datalist = def.type_name == "microsoftSql"
                                                    && fkey == "database"
                                                    && !db_list.is_empty();
                                                rsx! {
                                                    div { class: "flex flex-col gap-2",
                                                        key: "{fkey}",
                                                        Label {
                                                            class: Some("text-xs text-muted-foreground".into()),
                                                            "{f.label}"
                                                            if f.required { " *" }
                                                        }
                                                        if !f.description.is_empty() {
                                                            p { class: "text-xs text-muted-foreground", "{f.description}" }
                                                        }
                                                        if f.kind == "boolean" {
                                                            Switch {
                                                                checked: json_bool(
                                                                    &serde_json::Value::Object(field_values()),
                                                                    fkey,
                                                                    false,
                                                                ),
                                                                on_checked_change: move |checked| {
                                                                    field_values.with_mut(|m| {
                                                                        m.insert(
                                                                            key.clone(),
                                                                            serde_json::Value::Bool(checked),
                                                                        );
                                                                    });
                                                                },
                                                            }
                                                        } else if f.kind == "select" {
                                                            Select {
                                                                value: current.clone(),
                                                                onchange: move |e: FormEvent| {
                                                                    field_values.with_mut(|m| {
                                                                        m.insert(
                                                                            key.clone(),
                                                                            serde_json::Value::String(e.value()),
                                                                        );
                                                                    });
                                                                },
                                                                for opt in f.options {
                                                                    SelectOption {
                                                                        key: "{opt.value}",
                                                                        value: opt.value.to_string(),
                                                                        selected: current == opt.value,
                                                                        "{opt.label}"
                                                                    }
                                                                }
                                                            }
                                                        } else if f.kind == "textarea" {
                                                            Textarea {
                                                                class: Some("min-h-[88px] font-mono text-xs md:text-xs".into()),
                                                                placeholder: Some(f.placeholder.to_string()),
                                                                value: current.clone(),
                                                                oninput: move |e: FormEvent| {
                                                                    field_values.with_mut(|m| {
                                                                        m.insert(
                                                                            key.clone(),
                                                                            serde_json::Value::String(e.value()),
                                                                        );
                                                                    });
                                                                },
                                                            }
                                                        } else {
                                                            {
                                                                let input_ty = if f.kind == "password" {
                                                                    "password"
                                                                } else if f.kind == "number" {
                                                                    "number"
                                                                } else {
                                                                    "text"
                                                                };
                                                                let list_id = if use_db_datalist {
                                                                    "mssql-db-list"
                                                                } else {
                                                                    ""
                                                                };
                                                                rsx! {
                                                                    if use_db_datalist {
                                                                        input {
                                                                            class: "flex h-9 w-full rounded-md border border-input bg-transparent px-3 text-sm",
                                                                            r#type: "{input_ty}",
                                                                            placeholder: "{f.placeholder}",
                                                                            value: "{current}",
                                                                            list: "{list_id}",
                                                                            oninput: move |e: Event<FormData>| {
                                                                                let val = e.value();
                                                                                field_values.with_mut(|m| {
                                                                                    m.insert(
                                                                                        key.clone(),
                                                                                        serde_json::Value::String(val),
                                                                                    );
                                                                                });
                                                                            },
                                                                        }
                                                                    } else {
                                                                        Input {
                                                                            r#type: Some(input_ty.into()),
                                                                            placeholder: Some(f.placeholder.into()),
                                                                            value: current,
                                                                            oninput: move |e: FormEvent| {
                                                                                let val = e.value();
                                                                                field_values.with_mut(|m| {
                                                                                    m.insert(
                                                                                        key.clone(),
                                                                                        serde_json::Value::String(val),
                                                                                    );
                                                                                });
                                                                            },
                                                                        }
                                                                    }
                                                                    if use_db_datalist {
                                                                        datalist { id: "mssql-db-list",
                                                                            for db in db_list.iter() {
                                                                                option { key: "{db}", value: "{db}" }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if def.type_name == "microsoftSql" {
                                        {
                                            let live_cs = build_mssql_connection_string(&field_values());
                                            let masked = mask_mssql_connection_string(&live_cs);
                                            rsx! {
                                                div { class: "mt-2 flex flex-col gap-3 rounded-md border bg-muted/30 p-3",
                                                    div { class: "flex flex-col gap-1.5",
                                                        Label { "Connection string" }
                                                        p { class: "text-xs text-muted-foreground",
                                                            "Updates as you change the fields above (password masked)"
                                                        }
                                                        pre { class: "max-h-28 overflow-auto whitespace-pre-wrap break-all rounded-md border bg-background px-3 py-2 font-mono text-[11px] leading-relaxed text-foreground",
                                                            "{masked}"
                                                        }
                                                    }
                                                    div { class: "flex flex-wrap items-center gap-2",
                                                        Button {
                                                            size: ButtonSize::Sm,
                                                            variant: ButtonVariant::Secondary,
                                                            disabled: test_busy() || busy(),
                                                            onclick: move |_| on_test_mssql.call(()),
                                                            if test_busy() { "Testing…" } else { "Test connection" }
                                                        }
                                                        if let Some(ok) = test_ok() {
                                                            span {
                                                                class: if ok {
                                                                    "text-xs font-medium text-emerald-600 dark:text-emerald-400"
                                                                } else {
                                                                    "text-xs font-medium text-destructive"
                                                                },
                                                                if ok { "Connected" } else { "Failed" }
                                                            }
                                                        }
                                                    }
                                                    if let Some(msg) = test_msg() {
                                                        p {
                                                            class: if test_ok() == Some(true) {
                                                                "text-xs text-muted-foreground"
                                                            } else if test_ok() == Some(false) {
                                                                "text-xs text-destructive"
                                                            } else {
                                                                "text-xs text-muted-foreground"
                                                            },
                                                            "{msg}"
                                                        }
                                                    }
                                                    if !mssql_databases().is_empty() {
                                                        p { class: "text-xs text-muted-foreground",
                                                            "Database list loaded — use the Connect to database field suggestions above."
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "flex justify-end gap-2 border-t px-5 py-3",
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| close(),
                            "Cancel"
                        }
                        Button {
                            disabled: busy(),
                            onclick: move |_| on_save.call(()),
                            "Save"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn CredentialsPanel() -> Element {
    let mut items = use_signal(Vec::<serde_json::Value>::new);
    let mut query = use_signal(String::new);
    let mut err = use_signal(|| None::<String>);
    let mut dialog_open = use_signal(|| false);
    let mut edit_id = use_signal(|| None::<String>);
    let mut edit_name = use_signal(|| None::<String>);
    let mut edit_type = use_signal(|| None::<String>);
    let mut loading = use_signal(|| true);
    let mut confirm_delete = use_signal(|| None::<(String, String)>);
    let mut delete_busy = use_signal(|| false);

    let reload = move || {
        spawn(async move {
            match crate::api::get_json("/api/credentials").await {
                Ok(v) => {
                    items.set(
                        v.get("credentials")
                            .and_then(|x| x.as_array())
                            .cloned()
                            .unwrap_or_default(),
                    );
                    err.set(None);
                }
                Err(e) => err.set(Some(e)),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        reload();
    });

    let q = query().trim().to_lowercase();
    let filtered: Vec<serde_json::Value> = items()
        .into_iter()
        .filter(|c| {
            if q.is_empty() {
                return true;
            }
            let name = c.get("name").map(json_string).unwrap_or_default();
            let ty = c.get("type").map(json_string).unwrap_or_default();
            let display = credential_display_name(&ty);
            format!("{name} {ty} {display}")
                .to_lowercase()
                .contains(&q)
        })
        .collect();

    rsx! {
        div { class: "flex h-full flex-col",
            div { class: "flex flex-wrap items-end justify-between gap-3 border-b px-6 py-5",
                div {
                    h1 { class: "text-2xl font-semibold tracking-tight", "Credentials" }
                    p { class: "mt-1 text-sm text-muted-foreground",
                        "Secrets for nodes (Dataverse, SMTP, IMAP, …). Values are stored encrypted-at-rest and never shown in the list."
                    }
                }
                div { class: "flex flex-wrap items-center gap-2",
                    Input {
                        class: Some("w-56".into()),
                        placeholder: Some("Search credentials...".into()),
                        value: query(),
                        oninput: move |e: FormEvent| query.set(e.value()),
                    }
                    Button {
                        onclick: move |_| {
                            edit_id.set(None);
                            edit_name.set(None);
                            edit_type.set(None);
                            dialog_open.set(true);
                        },
                        "Add credential"
                    }
                }
            }

            if let Some(message) = err() {
                Alert {
                    variant: AlertVariant::Destructive,
                    class: Some("mx-6 my-2".into()),
                    AlertDescription {
                        class: Some("text-destructive".into()),
                        "{message}"
                    }
                }
            }

            if loading() {
                PageLoader { label: "Loading credentials…" }
            } else {
            div { class: "min-h-0 flex-1 overflow-y-auto",
                div { class: "p-6",
                    DataTableWrapper { class: Some("overflow-hidden rounded-lg bg-card max-h-none".into()),
                        DataTable { class: Some("max-w-none".into()),
                            DataTableHeader { class: Some("bg-muted/40 text-xs uppercase tracking-wide".into()),
                                DataTableRow {
                                    DataTableHead { class: Some("px-4 py-2".into()), "Name" }
                                    DataTableHead { class: Some("px-4 py-2".into()), "Type" }
                                    DataTableHead { class: Some("px-4 py-2 text-right".into()), "Actions" }
                                }
                            }
                            DataTableBody {
                                for item in filtered.iter().cloned() {
                                    {
                                        let id = json_id(&item);
                                        let name = item.get("name").map(json_string).unwrap_or_default();
                                        let ty = item.get("type").map(json_string).unwrap_or_default();
                                        let type_label = credential_display_name(&ty);
                                        let del_id = id.clone();
                                        let edit_item_id = id.clone();
                                        let edit_item_name = name.clone();
                                        let edit_item_type = ty.clone();
                                        rsx! {
                                            DataTableRow {
                                                key: "{id}",
                                                DataTableCell { class: Some("truncate px-4 py-3 font-medium".into()), "{name}" }
                                                DataTableCell { class: Some("truncate px-4 py-3 text-muted-foreground".into()), "{type_label}" }
                                                DataTableCell { class: Some("px-4 py-3".into()),
                                                    div { class: "flex justify-end gap-1",
                                                        Button {
                                                            size: ButtonSize::Sm,
                                                            variant: ButtonVariant::Ghost,
                                                            onclick: move |_| {
                                                                edit_id.set(Some(edit_item_id.clone()));
                                                                edit_name.set(Some(edit_item_name.clone()));
                                                                edit_type.set(Some(edit_item_type.clone()));
                                                                dialog_open.set(true);
                                                            },
                                                            "Edit"
                                                        }
                                                        Button {
                                                            size: ButtonSize::Sm,
                                                            variant: ButtonVariant::Ghost,
                                                            onclick: move |_| {
                                                                confirm_delete.set(Some((del_id.clone(), name.clone())));
                                                            },
                                                            "Delete"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if filtered.is_empty() {
                                    DataTableRow {
                                        DataTableCell {
                                            colspan: Some(3),
                                            class: Some("p-0".into()),
                                            Empty { class: Some("border-0 py-10".into()),
                                                EmptyDescription {
                                                    "No credentials yet. Click Add credential to create one."
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            }

            if let Some((del_id, del_name)) = confirm_delete() {
                {
                    let del_id_confirm = del_id.clone();
                    let description = format!(
                        "This will permanently remove credential \"{del_name}\". This action cannot be undone."
                    );
                    rsx! {
                        ConfirmDeleteDialog {
                            title: "Delete credential?".to_string(),
                            description,
                            busy: delete_busy(),
                            on_cancel: move |_| confirm_delete.set(None),
                            on_confirm: move |_| {
                                let path = format!("/api/credentials/{del_id_confirm}");
                                delete_busy.set(true);
                                spawn(async move {
                                    if crate::api::delete_json(&path).await.is_ok() {
                                        super::credentials_picker::invalidate_credentials_cache();
                                        confirm_delete.set(None);
                                        reload();
                                    }
                                    delete_busy.set(false);
                                });
                            },
                        }
                    }
                }
            }

            // Remount on each open so edit/create init from props (use_signal runs once).
            if dialog_open() {
                CredentialDialog {
                    key: "{edit_id().as_deref().unwrap_or(\"new\")}",
                    open: true,
                    on_open_change: move |open| {
                        dialog_open.set(open);
                        if !open {
                            edit_id.set(None);
                            edit_name.set(None);
                            edit_type.set(None);
                        }
                    },
                    edit_id: edit_id(),
                    edit_name: edit_name(),
                    edit_type: edit_type(),
                    on_created: move |_cred| {
                        super::credentials_picker::invalidate_credentials_cache();
                        reload();
                    },
                }
            }
        }
    }
}
