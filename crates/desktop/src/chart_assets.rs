//! Vendored ApexCharts + Rust/UI chart_init for desktop Insights.

use std::path::PathBuf;

use dioxus::prelude::*;

fn charts_dir() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/charts"),
        PathBuf::from("crates/desktop/assets/charts"),
        PathBuf::from("assets/charts"),
    ];
    candidates
        .into_iter()
        .find(|p| p.join("apexcharts.min.js").is_file())
}

fn read_asset(name: &str) -> Option<String> {
    let path = charts_dir()?.join(name);
    std::fs::read_to_string(path).ok()
}

/// Ensure ApexCharts + chart_init are loaded in the webview (idempotent).
pub async fn ensure_charts_loaded() {
    if let Ok(v) = document::eval(
        "return !!(window.ApexCharts && window.__chartInitDone && window.__bfRefreshCharts)",
    )
    .await
    {
        if v.as_bool() == Some(true) {
            let _ = document::eval(
                "if (window.__bfRefreshCharts) window.__bfRefreshCharts(); return true;",
            )
            .await;
            return;
        }
    }

    let Some(apex) = read_asset("apexcharts.min.js") else {
        tracing::warn!("ApexCharts asset missing (assets/charts/apexcharts.min.js)");
        return;
    };
    let Some(init) = read_asset("chart_init.js") else {
        tracing::warn!("chart_init.js missing (assets/charts/chart_init.js)");
        return;
    };

    if let Ok(v) =
        document::eval("return !!(window.ApexCharts)").await
    {
        if v.as_bool() != Some(true) {
            let wrapped = format!(
                concat!(
                    "try {{\n",
                    "{js}\n",
                    "return !!(window.ApexCharts);\n",
                    "}} catch (e) {{\n",
                    "  console.error('ApexCharts load failed', e);\n",
                    "  return 'ERR:' + (e && e.message ? e.message : String(e));\n",
                    "}}"
                ),
                js = apex
            );
            match document::eval(&wrapped).await {
                Ok(v) if v.as_bool() == Some(true) => {}
                Ok(v) => {
                    tracing::warn!(result = ?v, "ApexCharts load returned unexpected result");
                    return;
                }
                Err(e) => {
                    tracing::warn!(?e, "ApexCharts load eval failed");
                    return;
                }
            }
        }
    }

    let wrapped = format!(
        concat!(
            "try {{\n",
            "{js}\n",
            "if (window.__bfRefreshCharts) window.__bfRefreshCharts();\n",
            "return !!(window.__chartInitDone);\n",
            "}} catch (e) {{\n",
            "  console.error('chart_init load failed', e);\n",
            "  return 'ERR:' + (e && e.message ? e.message : String(e));\n",
            "}}"
        ),
        js = init
    );
    match document::eval(&wrapped).await {
        Ok(v) if v.as_bool() == Some(true) => {
            tracing::info!("Rust/UI charts runtime ready");
        }
        Ok(v) => tracing::warn!(result = ?v, "chart_init load returned unexpected result"),
        Err(e) => tracing::warn!(?e, "chart_init load eval failed"),
    }
}
