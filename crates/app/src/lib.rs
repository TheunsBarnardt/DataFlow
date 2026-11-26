use anyhow::Result;
use botflow_application_services::AppState;
use botflow_http_server::{bind, serve, serve_listener, HttpConfig};
use botflow_integrations::register_integration_nodes;
use botflow_node_api::NodeRegistry;
use botflow_nodes_core::register_all_nodes;
use botflow_runtime_core::RuntimeLimits;
use botflow_security::{JwtAuth, SecretBox};
use botflow_storage::Db;
use std::net::SocketAddr;
use tokio::task::JoinHandle;

pub struct BootConfig {
    pub host: String,
    pub port: u16,
    pub mongo_uri: String,
    pub cors_origins: String,
    pub jwt_secret: String,
    pub auth_disabled: bool,
    pub credentials_key: Option<String>,
}

impl BootConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8788),
            mongo_uri: std::env::var("MONGODB_URI")
                .unwrap_or_else(|_| "mongodb://127.0.0.1:27017/botflow".into()),
            cors_origins: std::env::var("CORS_ORIGINS").unwrap_or_else(|_| {
                "http://127.0.0.1:8788,http://localhost:8788,tauri://localhost,http://tauri.localhost,https://tauri.localhost".into()
            }),
            jwt_secret: std::env::var("BOTFLOW_JWT_SECRET")
                .unwrap_or_else(|_| "botflow-dev-jwt-secret".into()),
            auth_disabled: std::env::var("BOTFLOW_AUTH_DISABLED")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
            credentials_key: std::env::var("CREDENTIALS_KEY").ok(),
        }
    }
}

async fn build_state(boot: &BootConfig) -> Result<(AppState, HttpConfig, usize)> {
    let secrets = match &boot.credentials_key {
        Some(k) => SecretBox::from_key_b64(k).unwrap_or_else(|_| SecretBox::from_passphrase(k)),
        None => SecretBox::from_passphrase("botflow-dev-credentials-key"),
    };

    let db = match Db::connect(&boot.mongo_uri).await {
        Ok(db) => db,
        Err(e) => {
            tracing::error!(error = %e, "MongoDB unavailable — start the MongoDB service and retry");
            return Err(e);
        }
    };

    let mut registry = NodeRegistry::new();
    register_all_nodes(&mut registry);
    register_integration_nodes(&mut registry);

    let limits = RuntimeLimits {
        max_concurrent_nodes: std::env::var("MAX_CONCURRENT_NODES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8),
        ..RuntimeLimits::default()
    };

    let node_count = registry.list_metadata().len();
    let state = AppState::new(db, registry, secrets, limits);
    let cfg = HttpConfig {
        host: boot.host.clone(),
        port: boot.port,
        cors_origins: boot
            .cors_origins
            .split(',')
            .map(|s| s.trim().to_string())
            .collect(),
        jwt: JwtAuth::new(&boot.jwt_secret, 24),
        auth_disabled: boot.auth_disabled,
    };
    Ok((state, cfg, node_count))
}

/// Headless API/server process (no desktop window).
pub async fn run_server() -> Result<()> {
    let boot = BootConfig::from_env();
    let (state, cfg, node_count) = build_state(&boot).await?;
    tracing::info!(
        nodes = node_count,
        auth_disabled = boot.auth_disabled,
        "botflow-server ready"
    );
    serve(state, cfg).await
}

/// Bind loopback server and spawn it on the current Tokio runtime (desktop shell).
pub async fn spawn_server() -> Result<(SocketAddr, JoinHandle<Result<()>>)> {
    let boot = BootConfig::from_env();
    let (state, cfg, node_count) = build_state(&boot).await?;
    tracing::info!(
        nodes = node_count,
        auth_disabled = boot.auth_disabled,
        "botflow engine ready (desktop)"
    );
    let (addr, listener, app) = bind(state, cfg).await?;
    let handle = tokio::spawn(async move { serve_listener(listener, app).await });
    Ok((addr, handle))
}

/// In-process engine for the native Rust desktop UI (optional loopback API for webhooks).
pub async fn boot_desktop_engine() -> Result<(AppState, usize, SocketAddr, JoinHandle<Result<()>>)> {
    let boot = BootConfig::from_env();
    let (state, cfg, node_count) = build_state(&boot).await?;
    tracing::info!(
        nodes = node_count,
        auth_disabled = boot.auth_disabled,
        "botflow engine ready (native desktop)"
    );
    let api_state = state.clone();
    let (addr, listener, app) = bind(api_state, cfg).await?;
    let handle = tokio::spawn(async move { serve_listener(listener, app).await });
    Ok((state, node_count, addr, handle))
}
