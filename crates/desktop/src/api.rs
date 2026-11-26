use serde_json::Value;
use std::sync::{OnceLock, RwLock};

static BASE_URL: OnceLock<RwLock<String>> = OnceLock::new();
static AUTH_TOKEN: OnceLock<RwLock<Option<String>>> = OnceLock::new();

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

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
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
    let mut req = client().request(method, url(path));
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

pub async fn get_json(path: &str) -> Result<Value, String> {
    send(reqwest::Method::GET, path, None).await
}

pub async fn post_json(path: &str, body: Value) -> Result<Value, String> {
    send(reqwest::Method::POST, path, Some(body)).await
}

pub async fn put_json(path: &str, body: Value) -> Result<Value, String> {
    send(reqwest::Method::PUT, path, Some(body)).await
}

pub async fn patch_json(path: &str, body: Value) -> Result<Value, String> {
    send(reqwest::Method::PATCH, path, Some(body)).await
}

pub async fn delete_json(path: &str) -> Result<Value, String> {
    send(reqwest::Method::DELETE, path, None).await
}
