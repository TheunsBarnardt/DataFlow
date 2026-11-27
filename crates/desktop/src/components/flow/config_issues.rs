//! Canvas config checks — parity with `web/src/lib/nodeConfigIssues.ts`.

use serde_json::Value;

use crate::components::node_schemas::{field_visible, get_schema};

fn is_blank(value: Option<&Value>) -> bool {
    match value {
        None => true,
        Some(Value::Null) => true,
        Some(Value::String(s)) => s.trim().is_empty(),
        _ => false,
    }
}

pub fn get_node_config_issues(
    ty: &str,
    parameters: &Value,
    disabled: bool,
) -> Vec<String> {
    if disabled || ty == "stickyNote" {
        return Vec::new();
    }

    let mut issues = Vec::new();
    let push_empty = |issues: &mut Vec<String>, key: &str, label: &str| {
        if is_blank(parameters.get(key)) {
            issues.push(format!("{label} is required"));
        }
    };

    match ty {
        "httpRequest" | "graphql" | "rssRead" | "rssFeedTrigger" => {
            push_empty(&mut issues, "url", "URL");
        }
        "code" => push_empty(&mut issues, "code", "Code"),
        "localFileTrigger" => push_empty(&mut issues, "path", "Watch path"),
        "webhook" => {
            let auth = parameters
                .get("authentication")
                .and_then(|v| v.as_str())
                .unwrap_or("none");
            if matches!(auth, "headerBearer" | "basic") && is_blank(parameters.get("secret")) {
                issues.push("Secret is required for authentication".into());
            }
        }
        "sendEmail" => push_empty(&mut issues, "to", "To"),
        "ftp" => push_empty(&mut issues, "host", "Host"),
        "mssql" => {
            if is_blank(parameters.get("query")) {
                push_empty(&mut issues, "query", "Query");
            }
        }
        "dynamics" => {
            let op = parameters
                .get("operation")
                .and_then(|v| v.as_str())
                .unwrap_or("GET");
            if op.eq_ignore_ascii_case("GET") {
                let qtype = parameters
                    .get("type")
                    .or_else(|| parameters.get("queryType"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("FETCHXML");
                let query = parameters
                    .get("getQuery")
                    .or_else(|| parameters.get("query"));
                if qtype.eq_ignore_ascii_case("FETCHXML") && is_blank(query) {
                    issues.push("FetchXML Query is required".into());
                } else if qtype.eq_ignore_ascii_case("ODATA") && is_blank(query) {
                    // Legacy entity-only OData is OK; only flag when neither query nor entity.
                    if is_blank(parameters.get("entityName"))
                        && is_blank(parameters.get("entity"))
                    {
                        issues.push("OData Query is required".into());
                    }
                }
            }
        }
        _ => {}
    }

    if let Some(schema) = get_schema(ty) {
        for field in &schema.fields {
            if field.key != "credentialId" {
                continue;
            }
            if !field_visible(field, parameters) {
                continue;
            }
            if is_blank(parameters.get("credentialId")) {
                // Legacy seeded Wealth flows may still carry connectionString only.
                if ty == "mssql" && !is_blank(parameters.get("connectionString")) {
                    continue;
                }
                // Graph/Outlook-style nodes often accept a pasted accessToken instead.
                if matches!(
                    ty,
                    "microsoftGraph" | "outlook" | "teams" | "excel" | "sharepoint" | "dynamics"
                ) && !is_blank(parameters.get("accessToken"))
                {
                    continue;
                }
                let label = if field.label.is_empty() {
                    "Credential"
                } else {
                    field.label.as_str()
                };
                issues.push(format!("{label} is required"));
            }
        }
    }

    if ty == "httpRequest"
        && parameters
            .get("authentication")
            .and_then(|v| v.as_str())
            == Some("predefinedCredentialType")
        && is_blank(parameters.get("credentialId"))
        && !issues.iter().any(|i| i.contains("Credential"))
    {
        issues.push("Credential is required".into());
    }

    issues.sort();
    issues.dedup();
    issues
}
