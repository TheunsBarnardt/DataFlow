#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use std::io::{self, Write};
use tracing_subscriber::{fmt::MakeWriter, EnvFilter};

mod api;
mod app;
mod chart_assets;
mod components;
mod roles;
mod format_code;
mod codemirror_assets;
mod styles;
mod theme;

/// Windows + piped stdio (`dx serve`): a closed stderr turns `eprintln!` into
/// `failed printing to stderr: The pipe is being closed. (os error 232)`.
fn is_broken_pipe(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::BrokenPipe | io::ErrorKind::UnexpectedEof
    ) || matches!(err.raw_os_error(), Some(232 | 109))
}

fn safe_stderr_write(bytes: &[u8]) {
    let mut out = io::stderr().lock();
    if let Err(e) = out.write_all(bytes) {
        if !is_broken_pipe(&e) {
            let _ = writeln!(io::stdout().lock(), "stderr write failed: {e}");
        }
    }
}

fn install_safe_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any>".to_string()
        };
        // Peer closed the log pipe during rebuild/shutdown — not actionable.
        if payload.contains("failed printing to stderr")
            || payload.contains("The pipe is being closed")
            || payload.contains("os error 232")
            || payload.contains("Broken pipe")
        {
            return;
        }
        let loc = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".into());
        safe_stderr_write(format!("panic at {loc}: {payload}\n").as_bytes());
    }));
}

/// tracing MakeWriter that swallows broken-pipe instead of surfacing I/O errors.
#[derive(Clone, Debug)]
struct ResilientStdio;

struct ResilientWriter {
    inner: io::Stderr,
}

impl Write for ResilientWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.inner.write(buf) {
            Err(e) if is_broken_pipe(&e) => Ok(buf.len()),
            other => other,
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.inner.flush() {
            Err(e) if is_broken_pipe(&e) => Ok(()),
            other => other,
        }
    }
}

impl<'a> MakeWriter<'a> for ResilientStdio {
    type Writer = ResilientWriter;

    fn make_writer(&'a self) -> Self::Writer {
        ResilientWriter {
            inner: io::stderr(),
        }
    }
}

fn main() -> Result<()> {
    install_safe_panic_hook();

    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,botflow_desktop=info")),
        )
        .with_writer(ResilientStdio)
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
        .with_disable_context_menu(true)
        // Windows WebView blocks HTML5 drag/drop when the file-drop handler is on.
        .with_disable_drag_drop_handler(true);
    let desktop_cfg = codemirror_assets::with_config(desktop_cfg);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(desktop_cfg)
        .launch(app::App);

    Ok(())
}
