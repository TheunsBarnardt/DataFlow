import { fieldVisible, getNodeSchema } from "@/lib/nodeSchemas";

function isBlank(value: unknown): boolean {
  if (value == null) return true;
  if (typeof value === "string") return value.trim() === "";
  return false;
}

/**
 * Lightweight canvas config checks (n8n-style issue triangle).
 * Heuristics only — not a full validation engine.
 */
export function getNodeConfigIssues(
  type: string,
  parameters: Record<string, unknown> | undefined,
  opts?: { disabled?: boolean },
): string[] {
  if (opts?.disabled || type === "stickyNote") return [];

  const params = parameters ?? {};
  const issues: string[] = [];
  const schema = getNodeSchema(type);

  const pushEmpty = (key: string, label: string) => {
    if (isBlank(params[key])) issues.push(`${label} is required`);
  };

  switch (type) {
    case "httpRequest":
    case "graphql":
    case "rssRead":
    case "rssFeedTrigger":
      pushEmpty("url", "URL");
      break;
    case "code":
      pushEmpty("code", "Code");
      break;
    case "localFileTrigger":
      pushEmpty("path", "Watch path");
      break;
    case "webhook": {
      const auth = String(params.authentication ?? "none");
      if (
        (auth === "headerBearer" || auth === "basic") &&
        isBlank(params.secret)
      ) {
        issues.push("Secret is required for authentication");
      }
      break;
    }
    case "sendEmail":
      pushEmpty("to", "To");
      break;
    case "ftp":
      pushEmpty("host", "Host");
      break;
    case "mssql":
      if (isBlank(params.connectionString) && isBlank(params.query)) {
        // Prefer connection string / query when present in schema params
        if ("connectionString" in params || "query" in params) {
          if (isBlank(params.connectionString)) {
            pushEmpty("connectionString", "Connection string");
          }
        }
      }
      break;
    default:
      break;
  }

  // Credential fields that are visible for the current parameter state
  for (const field of schema.fields) {
    if (field.key !== "credentialId") continue;
    if (!fieldVisible(field, params)) continue;
    if (isBlank(params.credentialId)) {
      issues.push(`${field.label || "Credential"} is required`);
    }
  }

  // Auth modes that pick a credential on httpRequest
  if (
    type === "httpRequest" &&
    String(params.authentication ?? "none") === "predefinedCredentialType" &&
    isBlank(params.credentialId)
  ) {
    if (!issues.some((i) => i.includes("Credential"))) {
      issues.push("Credential is required");
    }
  }

  return [...new Set(issues)];
}

export function nodeHasConfigIssues(
  type: string,
  parameters: Record<string, unknown> | undefined,
  opts?: { disabled?: boolean },
): boolean {
  return getNodeConfigIssues(type, parameters, opts).length > 0;
}
