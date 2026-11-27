//! Vendored CodeMirror 6 IIFE for the desktop webview.

use std::path::{Path, PathBuf};

use dioxus::desktop::{use_asset_handler, wry, Config};
use dioxus::prelude::*;
use wry::http::{header, StatusCode};

fn assets_root() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"),
        PathBuf::from("crates/desktop/assets"),
        PathBuf::from("assets"),
    ];
    candidates
        .into_iter()
        .find(|p| p.join("codemirror/bf-codemirror.js").is_file())
}

fn bundle_path() -> Option<PathBuf> {
    assets_root().map(|r| r.join("codemirror/bf-codemirror.js"))
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or_default() {
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn respond(status: StatusCode, mime: &str, body: Vec<u8>) -> wry::http::Response<Vec<u8>> {
    wry::http::Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "public, max-age=86400")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .body(body)
        .unwrap_or_else(|_| wry::http::Response::new(Vec::new()))
}

pub fn with_config(cfg: Config) -> Config {
    if let Some(p) = bundle_path() {
        tracing::info!(path = %p.display(), "CodeMirror bundle available");
    } else {
        tracing::warn!("CodeMirror bundle missing (assets/codemirror/bf-codemirror.js)");
    }
    cfg
}

/// HTML5 drag bridge: Dioxus desktop stubs DataTransfer.set_data, so we set the
/// real browser payload from `[data-bf-expr]` elements during dragstart.
const EXPR_DND_JS: &str = r#"
(function(){
  if (window.__bfExprDnD && window.__bfExprDnD.bound) return true;
  window.__bfExprDnD = {
    mime: "application/x-dataflow-expr",
    payload: null,
    bound: false,
    normalize: function(expr) {
      var t = String(expr || "").trim();
      if (!t) return "";
      if (t.indexOf("={{") === 0) return t;
      if (t.indexOf("{{") === 0) return "=" + t;
      return "={{ " + t + " }}";
    },
    consume: function() {
      var v = this.payload;
      this.payload = null;
      return v;
    },
    bind: function() {
      if (this.bound) return;
      this.bound = true;
      var self = this;
      document.addEventListener("dragstart", function(e) {
        var el = e.target && e.target.closest && e.target.closest("[data-bf-expr]");
        if (!el || !e.dataTransfer) return;
        var expr = el.getAttribute("data-bf-expr") || "";
        if (!expr) return;
        self.payload = expr;
        try {
          e.dataTransfer.setData(self.mime, expr);
          e.dataTransfer.setData("text/plain", expr);
          e.dataTransfer.effectAllowed = "copy";
        } catch (_) {}
      }, true);
    }
  };
  window.__bfExprDnD.bind();
  return true;
})()
"#;

/// Load `window.bfCodeMirror` once by evaluating the vendored IIFE in the webview.
pub fn preload() {
    use_effect(|| {
        spawn(async move {
            let _ = document::eval(EXPR_DND_JS).await;

            if let Ok(v) =
                document::eval("return !!(window.bfCodeMirror && window.bfCodeMirror.create)").await
            {
                if v.as_bool() == Some(true) {
                    tracing::info!("CodeMirror already loaded");
                    return;
                }
            }

            let Some(path) = bundle_path().filter(|p| p.is_file()) else {
                tracing::warn!("CodeMirror bundle not found on disk");
                return;
            };
            let Ok(js) = std::fs::read_to_string(&path) else {
                tracing::warn!(path = %path.display(), "cannot read CodeMirror bundle");
                return;
            };

            let wrapped = format!(
                concat!(
                    "try {{\n",
                    "{js}\n",
                    "return !!(window.bfCodeMirror && window.bfCodeMirror.create);\n",
                    "}} catch (e) {{\n",
                    "  console.error('CodeMirror load failed', e);\n",
                    "  return 'ERR:' + (e && e.message ? e.message : String(e));\n",
                    "}}"
                ),
                js = js
            );

            match document::eval(&wrapped).await {
                Ok(v) if v.as_bool() == Some(true) => {
                    tracing::info!("CodeMirror loaded");
                }
                Ok(v) => tracing::warn!(result = ?v, "CodeMirror load returned unexpected result"),
                Err(e) => tracing::warn!(?e, "CodeMirror load eval failed"),
            }
        });
    });
}

/// Optional same-origin `/codemirror/*` fallback (engine also serves `/vendor/codemirror`).
pub fn use_asset_routes() {
    let root = assets_root();
    use_asset_handler("codemirror", move |request, responder| {
        let Some(root) = root.clone() else {
            responder.respond(respond(
                StatusCode::NOT_FOUND,
                "text/plain",
                b"missing".to_vec(),
            ));
            return;
        };
        let rel = request
            .uri()
            .path()
            .strip_prefix("/codemirror/")
            .unwrap_or("")
            .trim_start_matches('/');
        if rel.is_empty() || rel.contains("..") {
            responder.respond(respond(
                StatusCode::FORBIDDEN,
                "text/plain",
                b"forbidden".to_vec(),
            ));
            return;
        }
        let Ok(root) = root.canonicalize() else {
            responder.respond(respond(
                StatusCode::NOT_FOUND,
                "text/plain",
                b"bad root".to_vec(),
            ));
            return;
        };
        let file = root.join("codemirror").join(rel);
        let Ok(file) = file.canonicalize() else {
            responder.respond(respond(
                StatusCode::NOT_FOUND,
                "text/plain",
                b"not found".to_vec(),
            ));
            return;
        };
        if !file.starts_with(&root) || !file.is_file() {
            responder.respond(respond(
                StatusCode::NOT_FOUND,
                "text/plain",
                b"not found".to_vec(),
            ));
            return;
        }
        match std::fs::read(&file) {
            Ok(bytes) => responder.respond(respond(StatusCode::OK, mime_for(&file), bytes)),
            Err(_) => responder.respond(respond(
                StatusCode::INTERNAL_SERVER_ERROR,
                "text/plain",
                b"read error".to_vec(),
            )),
        }
    });
}
