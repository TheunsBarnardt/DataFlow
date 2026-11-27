mod auth;
mod insights;
mod platform_auth_routes;
mod public_apis;
mod routes;
mod ui;

use anyhow::Result;
use axum::Router;
use axum::http::{header, HeaderValue, Method};
use botflow_application_services::AppState;
use botflow_security::JwtAuth;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing::info;

#[derive(Clone)]
pub struct HttpConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub jwt: JwtAuth,
    pub auth_disabled: bool,
}

pub fn build_router(state: AppState, cfg: &HttpConfig) -> Router {
    let cors = if cfg.cors_origins.iter().any(|o| o == "*") {
        CorsLayer::new()
            .allow_origin(AllowOrigin::any())
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                header::ACCEPT,
                header::HeaderName::from_static("x-api-key"),
            ])
    } else {
        let origins: Vec<HeaderValue> = cfg
            .cors_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                header::ACCEPT,
                header::HeaderName::from_static("x-api-key"),
            ])
            .allow_credentials(true)
    };

    let http_state = routes::HttpState {
        app: state.clone(),
        jwt: cfg.jwt.clone(),
        auth_disabled: cfg.auth_disabled,
    };
    let mut app = Router::new()
        .merge(routes::router(state.clone(), cfg.clone()))
        .merge(public_apis::router().with_state(http_state.clone()))
        .merge(ui::router().with_state(http_state));

    // Local editor assets (desktop Query/SQL highlighting) — avoids blocked CDN in webview.
    if let Some(monaco_dir) = monaco_static_dir() {
        info!(path = %monaco_dir.display(), "serving local Monaco at /vendor/monaco");
        app = app.nest_service("/vendor/monaco", ServeDir::new(monaco_dir));
    }
    if let Some(cm_dir) = codemirror_static_dir() {
        info!(path = %cm_dir.display(), "serving local CodeMirror at /vendor/codemirror");
        app = app.nest_service("/vendor/codemirror", ServeDir::new(cm_dir));
    }

    // Optional static fallback (WEB_DIST). Primary studio is the Dioxus desktop app.
    let dist = std::env::var("WEB_DIST").unwrap_or_else(|_| "web/dist".into());
    let dist_path = PathBuf::from(&dist);
    let index = dist_path.join("index.html");
    if index.is_file() {
        info!(path = %dist_path.display(), "serving static WEB_DIST fallback");
        let spa = ServeDir::new(&dist_path).not_found_service(ServeFile::new(index));
        app = app.fallback_service(spa);
    }

    app.layer(cors).layer(TraceLayer::new_for_http())
}

fn monaco_static_dir() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("assets/monaco"),
        PathBuf::from("crates/desktop/assets/monaco"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../desktop/assets/monaco"),
    ];
    candidates.into_iter().find(|p| p.join("vs/loader.js").is_file())
}

fn codemirror_static_dir() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from("assets/codemirror"),
        PathBuf::from("crates/desktop/assets/codemirror"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../desktop/assets/codemirror"),
    ];
    candidates
        .into_iter()
        .find(|p| p.join("bf-codemirror.js").is_file())
}

pub async fn serve(state: AppState, cfg: HttpConfig) -> Result<()> {
    let app = build_router(state, &cfg);
    let addr: SocketAddr = format!("{}:{}", cfg.host, cfg.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    info!(%bound, "botflow-server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Bind and return the listener so callers (desktop shell) can read the real port first.
pub async fn bind(
    state: AppState,
    cfg: HttpConfig,
) -> Result<(SocketAddr, tokio::net::TcpListener, Router)> {
    let app = build_router(state, &cfg);
    let addr: SocketAddr = format!("{}:{}", cfg.host, cfg.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    info!(%bound, "botflow-server listening");
    Ok((bound, listener, app))
}

pub async fn serve_listener(listener: tokio::net::TcpListener, app: Router) -> Result<()> {
    axum::serve(listener, app).await?;
    Ok(())
}
