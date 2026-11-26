use async_trait::async_trait;
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeResult, PortDef,
};
use serde_json::json;

/// MSSQL node — executes via `sqlcmd` (Trusted Connection friendly on Windows).
pub struct MssqlNode;

enum MssqlAuth {
    Windows,
    Sql { user: String, password: String },
}

struct MssqlConfig {
    server: String,
    database: String,
    auth: MssqlAuth,
}

fn param_str(ctx: &ExecutionContext, key: &str) -> Option<String> {
    ctx.node
        .parameters
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn cred_str(cred: &serde_json::Value, key: &str) -> Option<String> {
    let raw = match cred.get(key)? {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => return None,
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_conn_part(cs: &str, keys: &[&str]) -> Option<String> {
    for part in cs.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        let k = k.trim();
        if keys.iter().any(|want| k.eq_ignore_ascii_case(want)) {
            let v = v.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn resolve_config(ctx: &ExecutionContext) -> Result<MssqlConfig, NodeError> {
    let cred_id = param_str(ctx, "credentialId");
    let cred = cred_id
        .as_ref()
        .and_then(|id| ctx.credentials.get(id))
        .cloned()
        .unwrap_or(json!({}));

    let connection_string = cred_str(&cred, "connectionString")
        .or_else(|| param_str(ctx, "connectionString"));

    let mut server = param_str(ctx, "server")
        .or_else(|| cred_str(&cred, "server"))
        .or_else(|| std::env::var("MSSQL_SERVER").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_default();

    let mut database = param_str(ctx, "database")
        .or_else(|| cred_str(&cred, "database"))
        .or_else(|| std::env::var("MSSQL_DATABASE").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| "master".into());

    if server.is_empty() {
        if let Some(ref cs) = connection_string {
            if let Some(s) = parse_conn_part(
                cs,
                &[
                    "Server",
                    "Data Source",
                    "Address",
                    "Addr",
                    "Network Address",
                ],
            ) {
                server = s;
            }
            if database == "master" {
                if let Some(d) = parse_conn_part(cs, &["Database", "Initial Catalog"]) {
                    database = d;
                }
            }
        }
    }

    if server.is_empty() {
        let hint = if cred_id.is_some() {
            "open the Microsoft SQL credential and set Server name (or MSSQL_SERVER)"
        } else {
            "pick a Microsoft SQL credential, or set parameters.server / MSSQL_SERVER"
        };
        return Err(NodeError::Config(format!(
            "mssql server not configured ({hint})"
        )));
    }

    let auth_mode = cred_str(&cred, "authentication")
        .unwrap_or_else(|| "windows".into())
        .to_lowercase();

    let auth = if auth_mode == "sql" || auth_mode == "sqlserver" {
        let user = cred_str(&cred, "user")
            .or_else(|| cred_str(&cred, "username"))
            .unwrap_or_default();
        let password = cred_str(&cred, "password").unwrap_or_default();
        if user.is_empty() {
            return Err(NodeError::Config(
                "mssql SQL auth requires Login (user) on the credential".into(),
            ));
        }
        MssqlAuth::Sql { user, password }
    } else if let Some(ref cs) = connection_string {
        // Connection-string mode: detect SQL auth vs trusted from the string.
        let has_trusted = parse_conn_part(cs, &["Trusted_Connection", "Integrated Security"])
            .map(|v| {
                let v = v.to_lowercase();
                v == "true" || v == "yes" || v == "sspi"
            })
            .unwrap_or(false);
        let user = parse_conn_part(cs, &["User Id", "UID", "User"]).unwrap_or_default();
        let password = parse_conn_part(cs, &["Password", "PWD"]).unwrap_or_default();
        if !has_trusted && !user.is_empty() {
            MssqlAuth::Sql { user, password }
        } else {
            MssqlAuth::Windows
        }
    } else {
        MssqlAuth::Windows
    };

    Ok(MssqlConfig {
        server,
        database,
        auth,
    })
}

#[async_trait]
impl NodeExecutor for MssqlNode {
    fn metadata(&self) -> NodeMetadata {
        NodeMetadata {
            type_name: "mssql".into(),
            display_name: "MSSQL".into(),
            category: "database".into(),
            description: "Execute business SQL against Microsoft SQL Server".into(),
            version: 1,
            is_trigger: false,
            inputs: vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
            outputs: vec![PortDef {
                id: "main".into(),
                name: "Main".into(),
            }],
        }
    }

    async fn execute(&self, ctx: ExecutionContext) -> Result<NodeResult, NodeError> {
        let query = ctx
            .node
            .parameters
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| NodeError::Config("query is required".into()))?;
        let cfg = resolve_config(&ctx)?;

        let mut cmd = tokio::process::Command::new("sqlcmd");
        cmd.args(["-S", &cfg.server, "-d", &cfg.database, "-C"]);
        match &cfg.auth {
            MssqlAuth::Windows => {
                cmd.arg("-E");
            }
            MssqlAuth::Sql { user, password } => {
                cmd.args(["-U", user, "-P", password]);
            }
        }
        cmd.args(["-Q", query, "-s", ",", "-W"]);

        let output = cmd
            .output()
            .await
            .map_err(|e| NodeError::msg(format!("sqlcmd failed to start: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.status.success() {
            return Err(NodeError::msg(format!(
                "sqlcmd exited {}: {stderr}",
                output.status
            )));
        }
        Ok(NodeResult::single(vec![json!({
            "rowsText": stdout,
            "query": query,
        })]))
    }
}
