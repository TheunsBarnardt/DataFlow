//! n8n-style binary payload browser (preview + download).

use dioxus::prelude::*;
use dioxus::document;
use serde_json::Value;

#[derive(Clone, PartialEq)]
pub struct BinaryPayload {
    pub key: String,
    pub data: String,
    pub mime_type: String,
    pub file_name: Option<String>,
}

fn as_record(v: &Value) -> Option<&serde_json::Map<String, Value>> {
    v.as_object()
}

fn is_binary_shape(v: &Value) -> bool {
    let Some(obj) = as_record(v) else {
        return false;
    };
    obj.get("data")
        .and_then(|d| d.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

pub fn extract_binary_payloads(value: &Value) -> Vec<BinaryPayload> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    fn push(
        out: &mut Vec<BinaryPayload>,
        seen: &mut std::collections::HashSet<String>,
        key: String,
        data: String,
        mime_type: Option<String>,
        file_name: Option<String>,
    ) {
        let id = format!("{}:{}", key, &data[..data.len().min(32)]);
        if seen.contains(&id) {
            return;
        }
        seen.insert(id);
        out.push(BinaryPayload {
            key,
            data,
            mime_type: mime_type.unwrap_or_else(|| "application/octet-stream".into()),
            file_name,
        });
    }

    fn walk(
        node: &Value,
        path: &str,
        out: &mut Vec<BinaryPayload>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        if node.is_null() {
            return;
        }
        if let Some(arr) = node.as_array() {
            for (i, item) in arr.iter().enumerate() {
                walk(item, &format!("{path}[{i}]"), out, seen);
            }
            return;
        }
        let Some(obj) = as_record(node) else {
            return;
        };
        let path_lc = path.to_lowercase();
        if is_binary_shape(node)
            && (obj.contains_key("mimeType") || path_lc.contains("binary"))
        {
            let data = obj.get("data").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let mime = obj.get("mimeType").and_then(|v| v.as_str()).map(str::to_string);
            let file_name = obj.get("fileName").and_then(|v| v.as_str()).map(str::to_string);
            push(out, seen, path.to_string(), data, mime, file_name);
        }
        if let Some(binary_bag) = obj.get("binary").and_then(as_record) {
            for (k, v) in binary_bag {
                if is_binary_shape(v) {
                    let data = v.get("data").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let mime = v.get("mimeType").and_then(|x| x.as_str()).map(str::to_string);
                    let file_name = v.get("fileName").and_then(|x| x.as_str()).map(str::to_string);
                    push(out, seen, k.clone(), data, mime, file_name);
                } else if let Some(s) = v.as_str() {
                    push(out, seen, k.clone(), s.to_string(), None, None);
                } else {
                    walk(v, &format!("{path}.binary.{k}"), out, seen);
                }
            }
        }
        for (k, v) in obj {
            if k == "binary" {
                continue;
            }
            if k.to_lowercase().contains("binary") && is_binary_shape(v) {
                let data = v.get("data").and_then(|x| x.as_str()).unwrap_or("").to_string();
                let mime = v.get("mimeType").and_then(|x| x.as_str()).map(str::to_string);
                push(out, seen, k.clone(), data, mime, None);
            } else if v.is_object() || v.is_array() {
                walk(v, &format!("{path}.{k}"), out, seen);
            }
        }
    }

    walk(value, "data", &mut out, &mut seen);
    out
}

pub fn has_binary_data(value: &Value) -> bool {
    !extract_binary_payloads(value).is_empty()
}

fn looks_like_base64(s: &str) -> bool {
    if s.len() < 8 {
        return false;
    }
    let sample = s.chars().take(200).collect::<String>();
    sample
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=' | '\r' | '\n'))
}

fn preview_text(payload: &BinaryPayload) -> Option<String> {
    let mime = payload.mime_type.to_lowercase();
    let textish = mime.starts_with("text/")
        || mime.contains("json")
        || mime.contains("xml")
        || mime.contains("javascript")
        || mime == "application/octet-stream";
    if !textish && !looks_like_base64(&payload.data) {
        return None;
    }
    let mut decoded = payload.data.clone();
    if looks_like_base64(&payload.data) {
        decoded = match base64_decode(&payload.data) {
            Ok(s) => s,
            Err(_) => payload.data.clone(),
        };
    }
    if decoded.len() > 4000 {
        return Some(format!("{}…", &decoded[..4000]));
    }
    let head: String = decoded.chars().take(200).collect();
    if head.chars().any(|c| matches!(c, '\0'..='\x08' | '\x0e'..='\x1f')) {
        return None;
    }
    Some(decoded)
}

fn base64_decode(raw: &str) -> Result<String, ()> {
    let compact: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64_decode_bytes(&compact)?;
    String::from_utf8(bytes).map_err(|_| ())
}

fn base64_decode_bytes(s: &str) -> Result<Vec<u8>, ()> {
    const PAD: u8 = 255;
    let mut decode = [PAD; 256];
    for (i, c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        .iter()
        .enumerate()
    {
        decode[*c as usize] = i as u8;
    }
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf = [0u8; 4];
    let mut n = 0usize;
    for &b in s.as_bytes() {
        if b == b'=' {
            break;
        }
        let v = decode[b as usize];
        if v == PAD {
            return Err(());
        }
        buf[n] = v;
        n += 1;
        if n == 4 {
            out.push((buf[0] << 2) | (buf[1] >> 4));
            out.push((buf[1] << 4) | (buf[2] >> 2));
            out.push((buf[2] << 6) | buf[3]);
            n = 0;
        }
    }
    if n >= 2 {
        out.push((buf[0] << 2) | (buf[1] >> 4));
    }
    if n >= 3 {
        out.push((buf[1] << 4) | (buf[2] >> 2));
    }
    Ok(out)
}

fn download_filename(payload: &BinaryPayload) -> String {
    if let Some(ref name) = payload.file_name {
        if !name.is_empty() {
            return name.clone();
        }
    }
    let ext = if payload.mime_type.contains("json") {
        "json"
    } else if payload.mime_type.contains("pdf") {
        "pdf"
    } else if payload.mime_type.starts_with("text/") {
        "txt"
    } else {
        "bin"
    };
    let stem = payload
        .key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
            c
        } else {
            '_'
        })
        .collect::<String>();
    let stem = if stem.is_empty() { "binary" } else { stem.as_str() };
    format!("{stem}.{ext}")
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn trigger_download(payload: &BinaryPayload) {
    let b64 = if looks_like_base64(&payload.data) {
        payload.data.chars().filter(|c| !c.is_whitespace()).collect()
    } else {
        encode_base64(payload.data.as_bytes())
    };
    let filename = download_filename(payload);
    let mime =
        serde_json::to_string(&payload.mime_type).unwrap_or_else(|_| "\"application/octet-stream\"".into());
    let filename_js = serde_json::to_string(&filename).unwrap_or_else(|_| "\"binary.bin\"".into());
    let b64_js = serde_json::to_string(&b64).unwrap_or_else(|_| "\"\"".into());
    let _ = document::eval(&format!(
        r#"(function(){{
  const raw = atob({b64_js}.replace(/\s/g,''));
  const bytes = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
  const blob = new Blob([bytes], {{ type: {mime} }});
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = {filename_js};
  a.click();
  URL.revokeObjectURL(url);
}})()"#,
    ));
}

#[component]
pub fn BinaryViewer(#[props(default)] data: Value) -> Element {
    let payloads = extract_binary_payloads(&data);
    if payloads.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "flex flex-col gap-2 rounded-md border border-amber-500/30 bg-amber-500/5 p-2",
            div { class: "text-[10px] font-semibold uppercase tracking-wide text-muted-foreground",
                "Binary data"
            }
            for p in payloads {
                {
                    let preview = preview_text(&p);
                    let p_dl = p.clone();
                    rsx! {
                        div {
                            key: "{p.key}-{p.mime_type}",
                            class: "flex flex-col gap-1.5 rounded border bg-background/80 p-2",
                            div { class: "flex items-center gap-2",
                                code { class: "truncate text-[11px] font-medium", "{p.key}" }
                                span { class: "text-[10px] text-muted-foreground", "{p.mime_type}" }
                                button {
                                    r#type: "button",
                                    class: "ml-auto inline-flex h-6 items-center rounded-md border px-1.5 text-[10px] hover:bg-muted",
                                    onclick: move |_| trigger_download(&p_dl),
                                    "Download"
                                }
                            }
                            if let Some(text) = preview {
                                pre {
                                    class: "max-h-32 overflow-auto rounded bg-muted/60 p-1.5 text-[10px] leading-snug break-all whitespace-pre-wrap",
                                    "{text}"
                                }
                            } else {
                                p { class: "text-[10px] text-muted-foreground",
                                    "Binary payload — use Download to save."
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}