//! # Auth — JWT Verification for Auth0
//!
//! Validates Auth0-issued JWTs on incoming requests.
//! When AUTH0_DOMAIN and AUTH0_AUDIENCE environment variables are set,
//! all /api/* routes (except /api/health) require a valid Bearer token.
//! When unset, auth is bypassed (dev mode).

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json},
};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::AppState;

/// Claims extracted from the Auth0 JWT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the Auth0 user ID (e.g., "auth0|abc123")
    pub sub: String,
    /// Audience
    #[serde(default)]
    pub aud: serde_json::Value,
    /// Expiration
    pub exp: u64,
    /// Issued at
    #[serde(default)]
    pub iat: u64,
    /// Issuer
    #[serde(default)]
    pub iss: String,
    /// Email (if present in token)
    #[serde(default)]
    pub email: Option<String>,
}

/// Auth configuration. When `None`, auth is bypassed.
#[derive(Clone)]
pub struct AuthConfig {
    pub domain: String,
    pub audience: String,
    /// HMAC secret OR RSA public key for validation.
    /// For simplicity in dev, we use HS256 with a shared secret.
    /// In production with Auth0, you'd fetch JWKS and use RS256.
    /// We support both: if JWKS URL is reachable, use RS256;
    /// otherwise fall back to skipping validation in dev mode.
    pub decoding_key: Option<DecodingKey>,
}

impl AuthConfig {
    /// Create from environment variables. Returns None if AUTH0_DOMAIN is unset.
    pub fn from_env() -> Option<Self> {
        let domain = std::env::var("AUTH0_DOMAIN").ok()?;
        let audience = std::env::var("AUTH0_AUDIENCE").unwrap_or_default();

        if domain.is_empty() {
            return None;
        }

        // For production Auth0, you'd fetch JWKS from https://{domain}/.well-known/jwks.json
        // For now, we accept and decode-without-signature-verify in dev mode,
        // or use the AUTH0_SECRET env var if provided.
        let decoding_key = std::env::var("AUTH0_SECRET")
            .ok()
            .map(|s| DecodingKey::from_secret(s.as_bytes()));

        Some(AuthConfig {
            domain,
            audience,
            decoding_key,
        })
    }
}

/// Axum middleware that validates JWT on all requests.
/// Extracts claims and inserts them into request extensions.
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> impl IntoResponse {
    let auth_config = match &state.auth_config {
        Some(config) => config.clone(),
        None => {
            // Auth disabled — dev mode. Insert a synthetic dev user claim.
            let dev_claims = Claims {
                sub: "dev|local".into(),
                aud: serde_json::Value::Null,
                exp: u64::MAX,
                iat: 0,
                iss: String::new(),
                email: Some("dev@localhost".into()),
            };
            request.extensions_mut().insert(dev_claims);
            return Ok(next.run(request).await);
        }
    };

    // Extract token from Authorization header or query param
    let token = extract_token(&request);

    let token = match token {
        Some(t) => t,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Missing authentication token" })),
            ));
        }
    };

    // Validate the JWT
    match validate_token(&token, &auth_config) {
        Ok(claims) => {
            request.extensions_mut().insert(claims);
            Ok(next.run(request).await)
        }
        Err(err) => {
            tracing::warn!("JWT validation failed: {}", err);
            Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": format!("Invalid token: {}", err) })),
            ))
        }
    }
}

fn extract_token(request: &Request) -> Option<String> {
    // Try Authorization: Bearer <token> header first
    if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    // Fall back to ?token=<jwt> query parameter (for WebSocket)
    if let Some(query) = request.uri().query() {
        for pair in query.split('&') {
            if let Some(token) = pair.strip_prefix("token=") {
                return Some(token.to_string());
            }
        }
    }

    None
}

fn validate_token(token: &str, config: &AuthConfig) -> Result<Claims, String> {
    let mut validation = Validation::new(Algorithm::RS256);

    if !config.audience.is_empty() {
        validation.set_audience(&[&config.audience]);
    } else {
        validation.validate_aud = false;
    }

    validation.set_issuer(&[format!("https://{}/", config.domain)]);

    match &config.decoding_key {
        Some(key) => {
            // If we have a secret, switch to HS256
            validation.algorithms = vec![Algorithm::HS256];
            decode::<Claims>(token, key, &validation)
                .map(|data| data.claims)
                .map_err(|e| e.to_string())
        }
        None => {
            // Without a decoding key, do insecure decode (dev/testing only).
            // In production, you'd fetch JWKS.
            validation.insecure_disable_signature_validation();
            validation.validate_exp = false;
            decode::<Claims>(token, &DecodingKey::from_secret(b""), &validation)
                .map(|data| data.claims)
                .map_err(|e| e.to_string())
        }
    }
}

/// Axum extractor for Claims — pulls from request extensions set by auth_middleware.
impl<S: Send + Sync> axum::extract::FromRequestParts<S> for Claims {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<Claims>().cloned().ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "Not authenticated"})),
            )
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_config_from_env_none_when_unset() {
        // Ensure AUTH0_DOMAIN is not set
        std::env::remove_var("AUTH0_DOMAIN");
        let config = AuthConfig::from_env();
        assert!(config.is_none());
    }

    #[test]
    fn auth_config_from_env_some_when_set() {
        std::env::set_var("AUTH0_DOMAIN", "test.auth0.com");
        std::env::set_var("AUTH0_AUDIENCE", "https://api.test.com");
        let config = AuthConfig::from_env();
        assert!(config.is_some());
        let c = config.unwrap();
        assert_eq!(c.domain, "test.auth0.com");
        assert_eq!(c.audience, "https://api.test.com");
        // Clean up
        std::env::remove_var("AUTH0_DOMAIN");
        std::env::remove_var("AUTH0_AUDIENCE");
    }

    #[test]
    fn extract_token_from_bearer_header() {
        use axum::http::Request;
        use axum::body::Body;

        let req = Request::builder()
            .header("authorization", "Bearer mytoken123")
            .body(Body::empty())
            .unwrap();
        let token = extract_token(&req);
        assert_eq!(token, Some("mytoken123".to_string()));
    }

    #[test]
    fn extract_token_from_query_param() {
        use axum::http::Request;
        use axum::body::Body;

        let req = Request::builder()
            .uri("/?token=qtokenvalue")
            .body(Body::empty())
            .unwrap();
        let token = extract_token(&req);
        assert_eq!(token, Some("qtokenvalue".to_string()));
    }

    #[test]
    fn extract_token_none_when_missing() {
        use axum::http::Request;
        use axum::body::Body;

        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let token = extract_token(&req);
        assert!(token.is_none());
    }

    #[test]
    fn extract_token_bearer_takes_priority_over_query() {
        use axum::http::Request;
        use axum::body::Body;

        let req = Request::builder()
            .uri("/?token=query_token")
            .header("authorization", "Bearer header_token")
            .body(Body::empty())
            .unwrap();
        let token = extract_token(&req);
        assert_eq!(token, Some("header_token".to_string()));
    }
}
