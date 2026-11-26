use async_trait::async_trait;
use bb8::{Pool, RunError};
use botflow_node_api::{
    ExecutionContext, NodeError, NodeExecutor, NodeMetadata, NodeResult, PortDef,
};
use serde_json::json;
use std::collections::HashMap;
use std::future::Future;
use std::sync::LazyLock;
use std::time::Duration;
use tiberius::{AuthMethod, Client, Config, EncryptionLevel, Row, SqlBrowser};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

/// MSSQL node — Tiberius client with pooled TCP connections (no per-query sqlcmd spawn).
pub struct MssqlNode;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(4);
const CHECKOUT_TIMEOUT: Duration = Duration::from_secs(5);
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(2);

/// Remember which transport works for a given server/db/auth so we don't
/// re-pay Tiberius TCP timeouts on every call when only sqlcmd works.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TransportPref {
    Unknown,
    Tiberius,
    Sqlcmd,
}

static TRANSPORT_PREF: LazyLock<Mutex<HashMap<String, TransportPref>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

async fn get_transport_pref(cfg: &MssqlConfig) -> TransportPref {
    let key = pool_key(cfg);
    let map = TRANSPORT_PREF.lock().await;
    map.get(&key).copied().unwrap_or(TransportPref::Unknown)
}

async fn set_transport_pref(cfg: &MssqlConfig, pref: TransportPref) {
    let key = pool_key(cfg);
    let mut map = TRANSPORT_PREF.lock().await;
    map.insert(key, pref);
}

#[derive(Clone)]
enum MssqlAuth {
    Windows,
    Sql { user: String, password: String },
}

#[derive(Clone)]
struct ParsedServer {
    host: String,
    port: u16,
    instance: Option<String>,
}

#[derive(Clone)]
struct MssqlConfig {
    server: String,
    database: String,
    auth: MssqlAuth,
    encrypt: bool,
    trust_cert: bool,
    parsed: ParsedServer,
}

/// Result of probing a Microsoft SQL credential (connection + optional DB list).
#[derive(Debug, Clone)]
pub struct MssqlTestResult {
    pub ok: bool,
    pub message: String,
    pub databases: Vec<String>,
}

type TiberiusClient = Client<Compat<TcpStream>>;

#[derive(Debug)]
struct MssqlPoolError(String);

impl std::fmt::Display for MssqlPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MssqlPoolError {}

struct TiberiusManager {
    recipe: MssqlConfig,
}

impl bb8::ManageConnection for TiberiusManager {
    type Connection = TiberiusClient;
    type Error = MssqlPoolError;

    fn connect(&self) -> impl Future<Output = Result<Self::Connection, Self::Error>> + Send {
        let recipe = self.recipe.clone();
        async move {
            // Hard timeout inside connect so hung TCP/TLS/browser attempts release pool slots.
            match tokio::time::timeout(CONNECT_TIMEOUT, connect_with_fallbacks(&recipe)).await {
                Ok(Ok(client)) => Ok(client),
                Ok(Err(e)) => Err(MssqlPoolError(e)),
                Err(_) => Err(MssqlPoolError(format!(
                    "connect timed out after {}s to {} (host={} port={} instance={:?}). Enable TCP/IP for the instance, or set Server to host,port",
                    CONNECT_TIMEOUT.as_secs(),
                    recipe.server,
                    recipe.parsed.host,
                    recipe.parsed.port,
                    recipe.parsed.instance
                ))),
            }
        }
    }

    fn is_valid(
        &self,
        conn: &mut Self::Connection,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            conn.simple_query("SELECT 1")
                .await
                .map_err(|e| MssqlPoolError(e.to_string()))?
                .into_row()
                .await
                .map_err(|e| MssqlPoolError(e.to_string()))?;
            Ok(())
        }
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

static POOLS: LazyLock<Mutex<HashMap<String, Pool<TiberiusManager>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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

fn cred_bool(cred: &serde_json::Value, key: &str, default: bool) -> bool {
    match cred.get(key) {
        Some(serde_json::Value::Bool(b)) => *b,
        Some(serde_json::Value::String(s)) => {
            let s = s.trim().to_lowercase();
            matches!(s.as_str(), "1" | "true" | "yes" | "y")
        }
        Some(serde_json::Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        _ => default,
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

fn parse_conn_bool(cs: &str, keys: &[&str], default: bool) -> bool {
    parse_conn_part(cs, keys)
        .map(|v| {
            let v = v.to_lowercase();
            matches!(v.as_str(), "true" | "yes" | "sspi" | "1")
        })
        .unwrap_or(default)
}

/// Parse SSMS-style server names: `host`, `host,1433`, `host\INSTANCE`, `.\SQLEXPRESS`.
fn parse_server(server: &str) -> ParsedServer {
    let mut s = server.trim();
    if let Some(rest) = s.strip_prefix("tcp:") {
        s = rest.trim();
    }

    let mut host_part = s.to_string();
    let mut instance = None;

    if let Some((host, inst)) = s.split_once('\\') {
        host_part = host.trim().to_string();
        let inst = inst.trim();
        if !inst.is_empty() {
            instance = Some(inst.to_string());
        }
    }

    if host_part.is_empty()
        || host_part == "."
        || host_part.eq_ignore_ascii_case("(local)")
        || host_part.eq_ignore_ascii_case("local")
    {
        host_part = "localhost".into();
    }

    let mut port = 1433u16;
    if instance.is_none() {
        if let Some((host, port_str)) = host_part.rsplit_once(',') {
            if let Ok(p) = port_str.trim().parse::<u16>() {
                host_part = host.trim().to_string();
                port = p;
            }
        } else if let Some((host, port_str)) = host_part.rsplit_once(':') {
            // Avoid treating IPv6 as host:port; only plain hostname:port.
            if !host_part.starts_with('[') && host.parse::<std::net::Ipv6Addr>().is_err() {
                if let Ok(p) = port_str.trim().parse::<u16>() {
                    host_part = host.trim().to_string();
                    port = p;
                }
            }
        }
    }

    ParsedServer {
        host: host_part,
        port,
        instance,
    }
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
        let has_trusted = parse_conn_bool(cs, &["Trusted_Connection", "Integrated Security"], false);
        let user = parse_conn_part(cs, &["User Id", "UID", "User"]).unwrap_or_default();
        let password = parse_conn_part(cs, &["Password", "PWD"]).unwrap_or_default();
        if !has_trusted && !user.is_empty() {
            MssqlAuth::Sql { user, password }
        } else {
            MssqlAuth::Windows
        }
    } else if let Some(ref cs) = connection_string {
        let has_trusted = parse_conn_bool(cs, &["Trusted_Connection", "Integrated Security"], false);
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

    let mut encrypt = cred_bool(cred, "encrypt", true);
    let mut trust_cert = cred_bool(cred, "trustServerCertificate", true);
    if let Some(ref cs) = connection_string {
        encrypt = parse_conn_bool(cs, &["Encrypt"], encrypt);
        trust_cert = parse_conn_bool(cs, &["TrustServerCertificate"], trust_cert);
    }

    let parsed = parse_server(&server);
    Ok(MssqlConfig {
        server,
        database,
        auth,
        encrypt,
        trust_cert,
        parsed,
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

fn build_tiberius_config(cfg: &MssqlConfig, encrypt: bool, with_instance: bool) -> Config {
    let mut config = Config::new();
    config.host(&cfg.parsed.host);
    config.port(cfg.parsed.port);
    if with_instance {
        if let Some(ref instance) = cfg.parsed.instance {
            config.instance_name(instance);
        }
    }
    config.database(&cfg.database);
    match &cfg.auth {
        MssqlAuth::Windows => {
            config.authentication(AuthMethod::Integrated);
        }
        MssqlAuth::Sql { user, password } => {
            config.authentication(AuthMethod::sql_server(user, password));
        }
    }
    if encrypt {
        // Required + trust_cert matches sqlcmd -C / TrustServerCertificate.
        config.encryption(EncryptionLevel::Required);
    } else {
        // Many local Express installs accept login-only / no TLS.
        config.encryption(EncryptionLevel::Off);
    }
    if cfg.trust_cert || !encrypt {
        config.trust_cert();
    }
    config
}

async fn tcp_connect(
    config: &Config,
    use_browser: bool,
) -> Result<TcpStream, tiberius::error::Error> {
    if use_browser {
        TcpStream::connect_named(config).await
    } else {
        TcpStream::connect(config.get_addr())
            .await
            .map_err(tiberius::error::Error::from)
    }
}

async fn connect_one(
    cfg: &MssqlConfig,
    encrypt: bool,
    use_browser: bool,
) -> Result<TiberiusClient, String> {
    let config = build_tiberius_config(cfg, encrypt, use_browser);
    let tcp = tcp_connect(&config, use_browser)
        .await
        .map_err(|e| format!("tcp {e}"))?;
    tcp.set_nodelay(true).map_err(|e| format!("nodelay {e}"))?;
    Client::connect(config, tcp.compat_write())
        .await
        .map_err(|e| format!("handshake {e}"))
}

/// Try configured settings, then common local-SQL fallbacks (encrypt off / no SQL Browser).
async fn connect_with_fallbacks(cfg: &MssqlConfig) -> Result<TiberiusClient, String> {
    let want_browser = cfg.parsed.instance.is_some();
    let mut errors = Vec::new();

    // Keep this short — failed TCP must not dominate request latency before sqlcmd fallback.
    let attempts: &[(bool, bool, &str)] = if want_browser {
        &[
            (cfg.encrypt, true, "configured+browser"),
            (false, false, "encrypt=off+port"),
        ]
    } else {
        &[
            (cfg.encrypt, false, "configured"),
            (false, false, "encrypt=off"),
        ]
    };

    for &(encrypt, browser, label) in attempts {
        // Per-attempt budget so we can try fallbacks inside CONNECT_TIMEOUT.
        match tokio::time::timeout(ATTEMPT_TIMEOUT, connect_one(cfg, encrypt, browser)).await {
            Ok(Ok(client)) => {
                if label != "configured" && label != "configured+browser" {
                    tracing::info!(
                        server = %cfg.server,
                        strategy = label,
                        "mssql connected via fallback"
                    );
                }
                return Ok(client);
            }
            Ok(Err(e)) => errors.push(format!("{label}: {e}")),
            Err(_) => errors.push(format!("{label}: timed out")),
        }
    }

    Err(format!(
        "all connect strategies failed for {} ({})",
        cfg.server,
        errors.join(" | ")
    ))
}

fn pool_key(cfg: &MssqlConfig) -> String {
    let auth = match &cfg.auth {
        MssqlAuth::Windows => "windows".to_string(),
        MssqlAuth::Sql { user, password } => format!("sql:{user}:{password}"),
    };
    format!(
        "{}|{}|{}|enc={}|trust={}",
        cfg.server, cfg.database, auth, cfg.encrypt, cfg.trust_cert
    )
}

async fn evict_pool(cfg: &MssqlConfig) {
    let key = pool_key(cfg);
    let mut pools = POOLS.lock().await;
    pools.remove(&key);
}

async fn get_pool(cfg: &MssqlConfig) -> Result<Pool<TiberiusManager>, String> {
    let key = pool_key(cfg);
    {
        let pools = POOLS.lock().await;
        if let Some(pool) = pools.get(&key) {
            return Ok(pool.clone());
        }
    }

    let manager = TiberiusManager {
        recipe: cfg.clone(),
    };
    // Don't wait for min connections during build — first get() creates them.
    let pool = Pool::builder()
        .max_size(8)
        .connection_timeout(CHECKOUT_TIMEOUT)
        .test_on_check_out(false)
        .build_unchecked(manager);

    let mut pools = POOLS.lock().await;
    Ok(pools.entry(key).or_insert_with(|| pool).clone())
}

async fn map_checkout_err(cfg: &MssqlConfig, err: RunError<MssqlPoolError>) -> String {
    evict_pool(cfg).await;
    match err {
        RunError::TimedOut => format!(
            "mssql checkout timed out after {}s connecting to {} (host={} port={} instance={:?}). Ensure TCP/IP is enabled; for named instances use host,port or start SQL Browser",
            CHECKOUT_TIMEOUT.as_secs(),
            cfg.server,
            cfg.parsed.host,
            cfg.parsed.port,
            cfg.parsed.instance
        ),
        RunError::User(e) => format!("mssql connect failed: {e}"),
    }
}

fn column_to_json(row: &Row, idx: usize) -> serde_json::Value {
    // Probe concrete FromSql impls. Ok(None) means SQL NULL (any matching null TDS variant).
    if let Ok(v) = row.try_get::<bool, _>(idx) {
        return match v {
            Some(x) => json!(x),
            None => serde_json::Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<i16, _>(idx) {
        return match v {
            Some(x) => json!(x),
            None => serde_json::Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<i32, _>(idx) {
        return match v {
            Some(x) => json!(x),
            None => serde_json::Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<i64, _>(idx) {
        return match v {
            Some(x) => json!(x),
            None => serde_json::Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<f32, _>(idx) {
        return match v {
            Some(x) => json!(x),
            None => serde_json::Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<f64, _>(idx) {
        return match v {
            Some(x) => json!(x),
            None => serde_json::Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<&str, _>(idx) {
        return match v {
            Some(x) => json!(x),
            None => serde_json::Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<chrono::NaiveDateTime, _>(idx) {
        return match v {
            Some(x) => json!(x.format("%Y-%m-%dT%H:%M:%S%.f").to_string()),
            None => serde_json::Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<chrono::NaiveDate, _>(idx) {
        return match v {
            Some(x) => json!(x.format("%Y-%m-%d").to_string()),
            None => serde_json::Value::Null,
        };
    }
    if let Ok(v) = row.try_get::<&[u8], _>(idx) {
        return match v {
            Some(_) => json!("[binary]"),
            None => serde_json::Value::Null,
        };
    }
    serde_json::Value::Null
}

fn row_to_json(row: &Row) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        obj.insert(col.name().to_string(), column_to_json(row, i));
    }
    serde_json::Value::Object(obj)
}

async fn run_sqlcmd(
    cfg: &MssqlConfig,
    query: &str,
    extra_args: &[&str],
    timeout: Duration,
) -> Result<String, String> {
    use std::process::Stdio;

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
    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let child = cmd
        .spawn()
        .map_err(|e| format!("sqlcmd failed to start: {e}"))?;
    let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return Err(format!("sqlcmd failed: {e}")),
        Err(_) => {
            return Err(format!(
                "sqlcmd timed out after {}s",
                timeout.as_secs()
            ));
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        let detail = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(format!("sqlcmd exited: {detail}"));
    }
    Ok(stdout)
}

fn is_separator_line(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty() && t.chars().all(|c| c == '-' || c == '\t' || c == ' ')
}

fn is_rows_affected_line(line: &str) -> bool {
    let lower = line.trim().to_lowercase();
    lower.starts_with('(') && lower.contains("rows affected")
}

fn parse_tsv_rows(stdout: &str) -> Vec<serde_json::Value> {
    let mut lines = stdout.lines().filter_map(|line| {
        let t = line.trim_end();
        if t.is_empty() || is_separator_line(t) || is_rows_affected_line(t) {
            None
        } else {
            Some(t)
        }
    });
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let cols: Vec<String> = header.split('\t').map(|c| c.trim().to_string()).collect();
    if cols.is_empty() || cols.iter().all(|c| c.is_empty()) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for line in lines {
        let vals: Vec<&str> = line.split('\t').collect();
        let mut obj = serde_json::Map::new();
        for (i, col) in cols.iter().enumerate() {
            if col.is_empty() {
                continue;
            }
            let raw = vals.get(i).map(|s| s.trim()).unwrap_or("");
            let value = if raw.is_empty() || raw.eq_ignore_ascii_case("NULL") {
                serde_json::Value::Null
            } else if let Ok(n) = raw.parse::<i64>() {
                json!(n)
            } else if let Ok(n) = raw.parse::<f64>() {
                json!(n)
            } else if raw.eq_ignore_ascii_case("true") {
                json!(true)
            } else if raw.eq_ignore_ascii_case("false") {
                json!(false)
            } else {
                json!(raw)
            };
            obj.insert(col.clone(), value);
        }
        out.push(serde_json::Value::Object(obj));
    }
    out
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

async fn run_query_tiberius(cfg: &MssqlConfig, query: &str) -> Result<Vec<serde_json::Value>, String> {
    let sql = query.trim().trim_end_matches(';').trim();
    let sql = format!("SET NOCOUNT ON; {sql};");
    let pool = get_pool(cfg).await?;
    let mut client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return Err(map_checkout_err(cfg, e).await),
    };
    let stream = client
        .simple_query(sql)
        .await
        .map_err(|e| format!("mssql query failed: {e}"))?;
    let results = stream
        .into_results()
        .await
        .map_err(|e| format!("mssql result failed: {e}"))?;
    for set in results {
        if !set.is_empty() {
            return Ok(set.iter().map(row_to_json).collect());
        }
    }
    Ok(Vec::new())
}

async fn run_query_sqlcmd(cfg: &MssqlConfig, query: &str) -> Result<Vec<serde_json::Value>, String> {
    let sql = query.trim().trim_end_matches(';').trim();
    let sql = format!("SET NOCOUNT ON; {sql};");
    let stdout = run_sqlcmd(cfg, &sql, &["-s", "\t", "-y", "8000"], Duration::from_secs(60)).await?;
    Ok(parse_tsv_rows(&stdout))
}

/// Prefer pooled Tiberius when it works; otherwise use sqlcmd and remember that choice.
async fn run_query(cfg: &MssqlConfig, query: &str) -> Result<Vec<serde_json::Value>, String> {
    match get_transport_pref(cfg).await {
        TransportPref::Sqlcmd => return run_query_sqlcmd(cfg, query).await,
        TransportPref::Tiberius => {
            return match run_query_tiberius(cfg, query).await {
                Ok(rows) => Ok(rows),
                Err(e) => {
                    set_transport_pref(cfg, TransportPref::Unknown).await;
                    Err(e)
                }
            };
        }
        TransportPref::Unknown => {}
    }

    match run_query_tiberius(cfg, query).await {
        Ok(rows) => {
            set_transport_pref(cfg, TransportPref::Tiberius).await;
            Ok(rows)
        }
        Err(tiberius_err) => {
            tracing::warn!(
                error = %tiberius_err,
                "mssql Tiberius unavailable; using sqlcmd for this server"
            );
            set_transport_pref(cfg, TransportPref::Sqlcmd).await;
            run_query_sqlcmd(cfg, query)
                .await
                .map_err(|sqlcmd_err| format!("{tiberius_err} | sqlcmd fallback: {sqlcmd_err}"))
        }
    }
}

async fn run_non_query(cfg: &MssqlConfig, query: &str) -> Result<(), String> {
    let sql = query.trim().trim_end_matches(';').trim();
    let sql = format!("SET NOCOUNT ON; {sql};");

    let try_tiberius = async {
        let pool = get_pool(cfg).await?;
        let mut client = match pool.get().await {
            Ok(c) => c,
            Err(e) => return Err(map_checkout_err(cfg, e).await),
        };
        let stream = client
            .simple_query(sql.clone())
            .await
            .map_err(|e| format!("mssql query failed: {e}"))?;
        let _ = stream
            .into_results()
            .await
            .map_err(|e| format!("mssql result failed: {e}"))?;
        Ok::<(), String>(())
    };

    match get_transport_pref(cfg).await {
        TransportPref::Sqlcmd => {
            return run_sqlcmd(cfg, &sql, &[], Duration::from_secs(60))
                .await
                .map(|_| ());
        }
        TransportPref::Tiberius => {
            return try_tiberius.await;
        }
        TransportPref::Unknown => {}
    }

    match try_tiberius.await {
        Ok(()) => {
            set_transport_pref(cfg, TransportPref::Tiberius).await;
            Ok(())
        }
        Err(tiberius_err) => {
            tracing::warn!(
                error = %tiberius_err,
                "mssql Tiberius unavailable; using sqlcmd for this server"
            );
            set_transport_pref(cfg, TransportPref::Sqlcmd).await;
            run_sqlcmd(cfg, &sql, &[], Duration::from_secs(60))
                .await
                .map(|_| ())
                .map_err(|sqlcmd_err| format!("{tiberius_err} | sqlcmd fallback: {sqlcmd_err}"))
        }
    }
}

/// Probe a Microsoft SQL credential payload (form / stored `data` object).
/// Tries Tiberius (TCP) first, then sqlcmd (shared memory / named pipes friendly).
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

    let list_cfg = MssqlConfig {
        server: cfg.server.clone(),
        database: "master".into(),
        auth: cfg.auth.clone(),
        encrypt: cfg.encrypt,
        trust_cert: cfg.trust_cert,
        parsed: cfg.parsed.clone(),
    };

    // Fast Tiberius probe (fails quickly when TCP isn't available).
    let tiberius_ok = matches!(
        tokio::time::timeout(CONNECT_TIMEOUT, async {
            let mut client = connect_with_fallbacks(&cfg).await?;
            let _ = client
                .simple_query("SELECT 1")
                .await
                .map_err(|e| e.to_string())?
                .into_row()
                .await
                .map_err(|e| e.to_string())?;
            Ok::<(), String>(())
        })
        .await,
        Ok(Ok(()))
    );

    // sqlcmd probe — works with local shared memory / named pipes.
    let sqlcmd_timeout = Duration::from_secs(20);
    let sqlcmd_err = match run_sqlcmd(
        &cfg,
        "SET NOCOUNT ON; SELECT 1;",
        &["-h", "-1", "-W"],
        sqlcmd_timeout,
    )
    .await
    {
        Ok(_) => None,
        Err(e) => {
            if cfg.database != "master" {
                match run_sqlcmd(
                    &list_cfg,
                    "SET NOCOUNT ON; SELECT 1;",
                    &["-h", "-1", "-W"],
                    sqlcmd_timeout,
                )
                .await
                {
                    Ok(_) => None,
                    Err(master_err) => Some(format!("{e} (master: {master_err})")),
                }
            } else {
                Some(e)
            }
        }
    };

    if sqlcmd_err.is_some() && !tiberius_ok {
        return MssqlTestResult {
            ok: false,
            message: sqlcmd_err.unwrap_or_else(|| "mssql connect failed".into()),
            databases: vec![],
        };
    }

    let databases = match run_sqlcmd(
        &list_cfg,
        "SET NOCOUNT ON; SELECT name FROM sys.databases ORDER BY name;",
        &["-h", "-1", "-W"],
        sqlcmd_timeout,
    )
    .await
    {
        Ok(stdout) => parse_database_names(&stdout),
        Err(_) => Vec::new(),
    };

    if tiberius_ok {
        set_transport_pref(&cfg, TransportPref::Tiberius).await;
    } else {
        set_transport_pref(&cfg, TransportPref::Sqlcmd).await;
    }

    let via = if tiberius_ok { "tiberius" } else { "sqlcmd" };
    MssqlTestResult {
        ok: true,
        message: format!(
            "Connected to {} / {} via {via} ({} databases visible)",
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

        let lower = query.trim().to_lowercase();
        let is_result_set = lower.starts_with("select") || lower.starts_with("with");

        let work = async {
            if is_result_set {
                let mut out = run_query(&cfg, query).await?;
                if out.is_empty() {
                    out.push(json!({ "rowCount": 0 }));
                }
                Ok::<_, String>(NodeResult::single(out))
            } else {
                run_non_query(&cfg, query).await?;
                Ok(NodeResult::single(vec![json!({
                    "ok": true,
                    "output": serde_json::Value::Null,
                })]))
            }
        };

        match tokio::time::timeout(ctx.timeout, work).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(NodeError::msg(e)),
            Err(_) => Err(NodeError::msg(format!(
                "mssql timed out after {}s (server unreachable or query too slow)",
                ctx.timeout.as_secs()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_server_host_port() {
        let p = parse_server("db.example.com,1433");
        assert_eq!(p.host, "db.example.com");
        assert_eq!(p.port, 1433);
        assert!(p.instance.is_none());
    }

    #[test]
    fn parse_server_named_instance() {
        let p = parse_server(".\\SQLEXPRESS");
        assert_eq!(p.host, "localhost");
        assert_eq!(p.instance.as_deref(), Some("SQLEXPRESS"));
    }

    #[test]
    fn parse_server_localhost() {
        let p = parse_server("localhost");
        assert_eq!(p.host, "localhost");
        assert_eq!(p.port, 1433);
    }

    #[test]
    fn resolve_sql_auth_from_cred() {
        let cred = json!({
            "authentication": "sql",
            "server": "localhost",
            "database": "AppDb",
            "user": "sa",
            "password": "secret",
            "encrypt": true,
            "trustServerCertificate": true
        });
        let cfg = resolve_from_maps(&json!({}), &cred, None).unwrap();
        assert_eq!(cfg.database, "AppDb");
        match cfg.auth {
            MssqlAuth::Sql { user, password } => {
                assert_eq!(user, "sa");
                assert_eq!(password, "secret");
            }
            _ => panic!("expected sql auth"),
        }
    }
}
