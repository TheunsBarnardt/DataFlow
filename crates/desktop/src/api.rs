use serde_json::Value;
use std::sync::{OnceLock, RwLock};

static BASE_URL: OnceLock<RwLock<String>> = OnceLock::new();
static AUTH_TOKEN: OnceLock<RwLock<Option<String>>> = OnceLock::new();
/// Engine Tokio runtime — UI futures (Dioxus) must hop here for reqwest.
static TOKIO_HANDLE: OnceLock<tokio::runtime::Handle> = OnceLock::new();

fn base() -> String {
    BASE_URL
        .get_or_init(|| RwLock::new("http://127.0.0.1:8788".into()))
        .read()
        .unwrap()
        .clone()
}

pub fn base_url() -> String {
    base()
}

pub fn set_base_url(url: impl Into<String>) {
    *BASE_URL
        .get_or_init(|| RwLock::new(String::new()))
        .write()
        .unwrap() = url.into();
}

pub fn set_auth_token(token: Option<String>) {
    *AUTH_TOKEN
        .get_or_init(|| RwLock::new(None))
        .write()
        .unwrap() = token;
}

pub fn auth_token() -> Option<String> {
    AUTH_TOKEN
        .get()
        .and_then(|t| t.read().unwrap().clone())
}

pub fn set_runtime_handle(handle: tokio::runtime::Handle) {
    let _ = TOKIO_HANDLE.set(handle);
}

fn client_timeout(path: &str) -> std::time::Duration {
    // Credential probes (esp. MSSQL) can take longer than a normal API call.
    if path.contains("/credentials/test") {
        std::time::Duration::from_secs(60)
    } else {
        std::time::Duration::from_secs(15)
    }
}

fn client(path: &str) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(client_timeout(path))
        .build()
        .expect("http client")
}

fn url(path: &str) -> String {
    let base = base();
    if path.starts_with("http") {
        path.to_string()
    } else if path.starts_with('/') {
        format!("{base}{path}")
    } else {
        format!("{base}/{path}")
    }
}

async fn send(method: reqwest::Method, path: &str, body: Option<Value>) -> Result<Value, String> {
    let mut req = client(path).request(method, url(path));
    if let Some(Some(token)) = AUTH_TOKEN.get().map(|t| t.read().unwrap().clone()) {
        req = req.bearer_auth(token);
    }
    req = req.header("content-type", "application/json");
    if let Some(b) = body {
        req = req.json(&b);
    }
    let res = req.send().await.map_err(|e| e.to_string())?;
    let status = res.status();
    if status.as_u16() == 204 {
        return Ok(Value::Null);
    }
    let text = res.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let err = serde_json::from_str::<Value>(&text).ok();
        let msg = err
            .as_ref()
            .and_then(|v| v.get("error").or_else(|| v.get("message")))
            .and_then(|v| v.as_str())
            .unwrap_or(status.as_str());
        return Err(msg.to_string());
    }
    if text.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

async fn on_engine_runtime<F, T>(fut: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>> + Send + 'static,
    T: Send + 'static,
{
    match TOKIO_HANDLE.get() {
        Some(handle) => handle
            .spawn(fut)
            .await
            .map_err(|e| format!("engine task failed: {e}"))?,
        None => fut.await,
    }
}

pub async fn get_json(path: &str) -> Result<Value, String> {
    let path = path.to_string();
    on_engine_runtime(async move { send(reqwest::Method::GET, &path, None).await }).await
}

pub async fn post_json(path: &str, body: Value) -> Result<Value, String> {
    let path = path.to_string();
    on_engine_runtime(async move { send(reqwest::Method::POST, &path, Some(body)).await }).await
}

pub async fn put_json(path: &str, body: Value) -> Result<Value, String> {
    let path = path.to_string();
    on_engine_runtime(async move { send(reqwest::Method::PUT, &path, Some(body)).await }).await
}

pub async fn patch_json(path: &str, body: Value) -> Result<Value, String> {
    let path = path.to_string();
    on_engine_runtime(async move { send(reqwest::Method::PATCH, &path, Some(body)).await }).await
}

pub async fn delete_json(path: &str) -> Result<Value, String> {
    let path = path.to_string();
    on_engine_runtime(async move { send(reqwest::Method::DELETE, &path, None).await }).await
}
