use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use botflow_security::SecurityError;
use serde_json::json;

use crate::routes::HttpState;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthUser {
    pub sub: String,
    pub role: String,
}

pub struct AuthError(pub StatusCode, pub String);

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl FromRequestParts<HttpState> for AuthUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &HttpState,
    ) -> Result<Self, Self::Rejection> {
        if state.auth_disabled {
            return Ok(AuthUser {
                sub: "local".into(),
                role: "admin".into(),
            });
        }
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AuthError(StatusCode::UNAUTHORIZED, "missing bearer token".into()))?;
        let claims = state.jwt.verify(token).map_err(|e| match e {
            SecurityError::Unauthorized => {
                AuthError(StatusCode::UNAUTHORIZED, "invalid token".into())
            }
            other => AuthError(StatusCode::UNAUTHORIZED, other.to_string()),
        })?;
        Ok(AuthUser {
            sub: claims.sub,
            role: claims.role,
        })
    }
}
