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

/// Result of probing a Microsoft SQL credential (connection + optional DB list).
#[derive(Debug, Clone)]
pub struct MssqlTestResult {
    pub ok: bool,
    pub message: String,
    pub databases: Vec<String>,
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

fn resolve_from_maps(
    parameters: &serde_json::Value,
    cred: &serde_json::Value,
    cred_id: Option<&str>,
) -> Result<MssqlConfig, String> {
    let connection_string = cred_str(cred, "connectionString").or_else(|| {
        parameters
            .get("connectionString")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    });

    let mut server = parameters
        .get("server")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| cred_str(cred, "server"))
        .or_else(|| {
            std::env::var("MSSQL_SERVER")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
        .unwrap_or_default();

    let mut database = parameters
        .get("database")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| cred_str(cred, "database"))
        .or_else(|| {
            std::env::var("MSSQL_DATABASE")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
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
        return Err(format!("mssql server not configured ({hint})"));
    }

    let auth_mode = cred_str(cred, "authentication")
        .unwrap_or_else(|| "windows".into())
        .to_lowercase();

    let auth = if auth_mode == "sql" || auth_mode == "sqlserver" {
        let user = cred_str(cred, "user")
            .or_else(|| cred_str(cred, "username"))
            .unwrap_or_default();
        let password = cred_str(cred, "password").unwrap_or_default();
        if user.is_empty() {
            return Err("mssql SQL auth requires Login (user) on the credential".into());
        }
        MssqlAuth::Sql { user, password }
    } else if auth_mode == "connectionstring" {
        let cs = connection_string.as_deref().unwrap_or("");
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
    } else if let Some(ref cs) = connection_string {
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

fn resolve_config(ctx: &ExecutionContext) -> Result<MssqlConfig, NodeError> {
    let cred_id = param_str(ctx, "credentialId");
    let cred = cred_id
        .as_ref()
        .and_then(|id| ctx.credentials.get(id))
        .cloned()
        .unwrap_or(json!({}));
    resolve_from_maps(&ctx.node.parameters, &cred, cred_id.as_deref())
        .map_err(NodeError::Config)
}

async fn run_sqlcmd(
    cfg: &MssqlConfig,
    query: &str,
    extra_args: &[&str],
) -> Result<(String, String), String> {
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
    cmd.args(["-Q", query]);
    cmd.args(extra_args);

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("sqlcmd failed to start: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        let detail = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(format!(
            "sqlcmd exited {}: {detail}",
            output
                .status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown".into())
        ));
    }
    Ok((stdout, stderr))
}

fn parse_database_names(stdout: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in stdout.lines() {
        let name = line.trim().trim_matches(',').trim();
        if name.is_empty() {
            continue;
        }
        let lower = name.to_lowercase();
        if lower.starts_with('(') && lower.contains("rows affected") {
            continue;
        }
        if lower == "name" {
            continue;
        }
        names.push(name.to_string());
    }
    names.sort();
    names.dedup();
    names
}

/// Probe a Microsoft SQL credential payload (form / stored `data` object).
/// On success, also returns database names the login can see.
pub async fn test_mssql_connection(data: &serde_json::Value) -> MssqlTestResult {
    let cfg = match resolve_from_maps(&json!({}), data, None) {
        Ok(c) => c,
        Err(message) => {
            return MssqlTestResult {
                ok: false,
                message,
                databases: vec![],
            };
        }
    };

    // Connect to master when possible so we can enumerate DBs even if a target DB is wrong.
    let list_cfg = MssqlConfig {
        server: cfg.server.clone(),
        database: "master".into(),
        auth: match &cfg.auth {
            MssqlAuth::Windows => MssqlAuth::Windows,
            MssqlAuth::Sql { user, password } => MssqlAuth::Sql {
                user: user.clone(),
                password: password.clone(),
            },
        },
    };

    if let Err(message) = run_sqlcmd(&cfg, "SET NOCOUNT ON; SELECT 1;", &["-h", "-1", "-W"]).await
    {
        // If the chosen DB fails, still try master so Windows/SQL auth can be verified.
        if cfg.database != "master" {
            if let Err(master_err) =
                run_sqlcmd(&list_cfg, "SET NOCOUNT ON; SELECT 1;", &["-h", "-1", "-W"]).await
            {
                return MssqlTestResult {
                    ok: false,
                    message: format!("{message} (master: {master_err})"),
                    databases: vec![],
                };
            }
        } else {
            return MssqlTestResult {
                ok: false,
                message,
                databases: vec![],
            };
        }
    }

    let databases = match run_sqlcmd(
        &list_cfg,
        "SET NOCOUNT ON; SELECT name FROM sys.databases ORDER BY name;",
        &["-h", "-1", "-W"],
    )
    .await
    {
        Ok((stdout, _)) => parse_database_names(&stdout),
        Err(_) => Vec::new(),
    };

    MssqlTestResult {
        ok: true,
        message: format!(
            "Connected to {} / {} ({} databases visible)",
            cfg.server,
            if cfg.database.trim().is_empty() {
                "master"
            } else {
                cfg.database.as_str()
            },
            databases.len()
        ),
        databases,
    }
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

        let (stdout, _) = run_sqlcmd(&cfg, query, &["-s", ",", "-W"])
            .await
            .map_err(NodeError::msg)?;

        Ok(NodeResult::single(vec![json!({
            "rowsText": stdout,
            "query": query,
        })]))
    }
}
