mod auth;
mod insights;
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
            ])
            .allow_credentials(true)
    };

    let mut app = Router::new()
        .merge(routes::router(state.clone(), cfg.clone()))
        .merge(ui::router().with_state(routes::HttpState {
            app: state,
            jwt: cfg.jwt.clone(),
            auth_disabled: cfg.auth_disabled,
        }));

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
