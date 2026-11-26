use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum SecurityError {
    #[error("{0}")]
    Message(String),
    #[error("ssrf blocked: {0}")]
    Ssrf(String),
    #[error("unauthorized")]
    Unauthorized,
}

#[derive(Clone)]
pub struct SecretBox {
    key: [u8; 32],
}

impl SecretBox {
    pub fn from_key_b64(key_b64: &str) -> Result<Self> {
        let bytes = B64.decode(key_b64).context("invalid CREDENTIALS_KEY base64")?;
        if bytes.len() != 32 {
            return Err(anyhow!("CREDENTIALS_KEY must decode to 32 bytes"));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(Self { key })
    }

    pub fn from_passphrase(pass: &str) -> Self {
        let hash = Sha256::digest(pass.as_bytes());
        let mut key = [0u8; 32];
        key.copy_from_slice(&hash);
        Self { key }
    }

    pub fn encrypt_json(&self, value: &Value) -> Result<String> {
        let plaintext = serde_json::to_vec(value)?;
        let cipher = Aes256Gcm::new_from_slice(&self.key)?;
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| anyhow!("encrypt failed: {e}"))?;
        let mut out = nonce_bytes.to_vec();
        out.extend(ciphertext);
        Ok(B64.encode(out))
    }

    pub fn decrypt_json(&self, blob_b64: &str) -> Result<Value> {
        let raw = B64.decode(blob_b64)?;
        if raw.len() < 13 {
            return Err(anyhow!("ciphertext too short"));
        }
        let (nonce_bytes, ct) = raw.split_at(12);
        let cipher = Aes256Gcm::new_from_slice(&self.key)?;
        let nonce = Nonce::from_slice(nonce_bytes);
        let plaintext = cipher
            .decrypt(nonce, ct)
            .map_err(|e| anyhow!("decrypt failed: {e}"))?;
        Ok(serde_json::from_slice(&plaintext)?)
    }
}

pub fn redact_value(value: &Value) -> Value {
    const SENSITIVE: &[&str] = &[
        "password",
        "secret",
        "token",
        "apiKey",
        "api_key",
        "authorization",
        "accessToken",
        "refreshToken",
        "privateKey",
    ];
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if SENSITIVE.iter().any(|s| k.eq_ignore_ascii_case(s)) {
                    out.insert(k.clone(), Value::String("***".into()));
                } else {
                    out.insert(k.clone(), redact_value(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(redact_value).collect()),
        other => other.clone(),
    }
}

#[derive(Clone, Default)]
pub struct SsrfGuard {
    pub allow_private: bool,
    pub blocked_hosts: HashSet<String>,
}

impl SsrfGuard {
    pub fn validate_url(&self, raw: &str) -> Result<(), SecurityError> {
        let url = Url::parse(raw).map_err(|e| SecurityError::Message(e.to_string()))?;
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(SecurityError::Ssrf(format!("scheme not allowed: {scheme}")));
        }
        let host = url.host_str().unwrap_or("").to_lowercase();
        if self.blocked_hosts.contains(&host) {
            return Err(SecurityError::Ssrf(format!("host blocked: {host}")));
        }
        if !self.allow_private {
            if host == "localhost"
                || host.ends_with(".localhost")
                || host == "metadata.google.internal"
                || host.starts_with("127.")
                || host.starts_with("10.")
                || host.starts_with("192.168.")
                || host.starts_with("169.254.")
                || host == "::1"
            {
                return Err(SecurityError::Ssrf(format!(
                    "private/link-local host blocked: {host}"
                )));
            }
            if let Some(rest) = host.strip_prefix("172.") {
                if let Some(second) = rest.split('.').next() {
                    if let Ok(n) = second.parse::<u8>() {
                        if (16..=31).contains(&n) {
                            return Err(SecurityError::Ssrf(format!(
                                "private host blocked: {host}"
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Clone)]
pub struct JwtAuth {
    encoding: EncodingKey,
    decoding: DecodingKey,
    #[allow(dead_code)]
    ttl_hours: i64,
}

impl JwtAuth {
    pub fn new(secret: &str, ttl_hours: i64) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret.as_bytes()),
            decoding: DecodingKey::from_secret(secret.as_bytes()),
            ttl_hours,
        }
    }

    pub fn issue(&self, sub: &str, role: &str) -> Result<String> {
        let now = Utc::now();
        let claims = Claims {
            sub: sub.into(),
            role: role.into(),
            iat: now.timestamp(),
            exp: (now + Duration::hours(self.ttl_hours)).timestamp(),
        };
        Ok(encode(&Header::default(), &claims, &self.encoding)?)
    }

    pub fn verify(&self, token: &str) -> Result<Claims, SecurityError> {
        decode::<Claims>(token, &self.decoding, &Validation::default())
            .map(|d| d.claims)
            .map_err(|_| SecurityError::Unauthorized)
    }
}

/// Verify an HS256 JWT with an arbitrary secret and return the payload object.
/// Used by public `/apis/{slug}` project auth (claims shape varies by issuer).
pub fn verify_hs256_jwt(token: &str, secret: &str) -> Result<Value, SecurityError> {
    let mut validation = Validation::default();
    validation.validate_exp = true;
    validation.required_spec_claims.clear();
    decode::<Value>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|d| d.claims)
    .map_err(|_| SecurityError::Unauthorized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn encrypt_roundtrip() {
        let box_ = SecretBox::from_passphrase("test-secret");
        let enc = box_.encrypt_json(&json!({"password":"x"})).unwrap();
        let dec = box_.decrypt_json(&enc).unwrap();
        assert_eq!(dec["password"], "x");
    }

    #[test]
    fn redact_secrets() {
        let v = redact_value(&json!({"token":"abc","ok":1}));
        assert_eq!(v["token"], "***");
        assert_eq!(v["ok"], 1);
    }

    #[test]
    fn blocks_localhost() {
        let g = SsrfGuard::default();
        assert!(g.validate_url("http://127.0.0.1/x").is_err());
        assert!(g.validate_url("https://example.com").is_ok());
    }
}
