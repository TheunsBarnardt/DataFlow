#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use tracing_subscriber::EnvFilter;

mod api;
mod app;
mod components;
mod roles;
mod format_code;
mod codemirror_assets;
mod styles;
mod theme;

fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,botflow_desktop=info")),
        )
        .try_init();

    if std::env::var_os("HOST").is_none() {
        unsafe { std::env::set_var("HOST", "127.0.0.1") };
    }
    if std::env::var_os("PORT").is_none() {
        unsafe { std::env::set_var("PORT", "8788") };
    }
    if std::env::var_os("BOTFLOW_AUTH_DISABLED").is_none() {
        unsafe { std::env::set_var("BOTFLOW_AUTH_DISABLED", "1") };
    }
    if std::env::var_os("HTTP_ALLOW_PRIVATE").is_none() {
        unsafe { std::env::set_var("HTTP_ALLOW_PRIVATE", "true") };
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    // UI futures hop onto this handle for reqwest (Dioxus runtime ≠ engine runtime).
    api::set_runtime_handle(rt.handle().clone());

    let (addr, _handle) = rt.block_on(async {
        let (addr, handle) = botflow_app::spawn_server().await?;
        Ok::<_, anyhow::Error>((addr, handle))
    })?;

    let url = format!("http://{addr}");
    api::set_base_url(&url);
    tracing::info!(%url, "BotFlow engine ready — launching Dioxus studio");

    for _ in 0..50 {
        if std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(100))
            .is_ok()
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Keep the Tokio runtime alive for the engine while Dioxus runs the UI.
    std::mem::forget(rt);

    let desktop_cfg = dioxus::desktop::Config::new()
        .with_window(
            dioxus::desktop::WindowBuilder::new()
                .with_title("BotFlow")
                .with_inner_size(dioxus::desktop::LogicalSize::new(1440.0, 900.0))
                .with_min_inner_size(dioxus::desktop::LogicalSize::new(1100.0, 700.0)),
        )
        .with_disable_context_menu(true);
    let desktop_cfg = codemirror_assets::with_config(desktop_cfg);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(desktop_cfg)
        .launch(app::App);

    Ok(())
}
