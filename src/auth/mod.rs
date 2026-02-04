use crate::db::DatabaseService;
use crate::models::{api_key::ApiKey, user::User};
use axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,  // User ID
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub exp: i64,  // Expiration time
    pub iat: i64,  // Issued at
}

/// Authentication context
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub is_admin: bool,
    pub rate_limit_rpm: Option<i32>,
}

/// JWT configuration
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

impl JwtConfig {
    pub fn from_env() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key-here".to_string()),
            expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
        }
    }

    /// Generate a JWT token for a user
    pub fn generate_token(&self, user: &User) -> crate::Result<String> {
        let now = chrono::Utc::now();
        let exp = now + chrono::Duration::hours(self.expiration_hours);

        let claims = Claims {
            sub: user.id,
            email: user.email.clone(),
            username: user.username.clone(),
            is_admin: user.is_admin,
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| crate::MantiError::Auth(format!("Failed to generate token: {}", e)))
    }

    /// Verify and decode a JWT token
    pub fn verify_token(&self, token: &str) -> crate::Result<Claims> {
        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|e| crate::MantiError::Auth(format!("Invalid token: {}", e)))
    }
}

/// Authentication middleware
pub async fn auth_middleware(
    State(db): State<Arc<DatabaseService>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        // Check if it's a Bearer token (JWT) or API key
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            // Try API key first
            if token.starts_with("sk-manti-") {
                // Extract prefix for faster lookup
                let prefix = token.chars().take(8).collect::<String>();

                // Find API key by prefix first (faster)
                if let Ok(Some(api_key)) = db.find_api_key_by_prefix(&prefix).await {
                    // Verify the full key
                    if api_key.verify(token) && api_key.is_valid() {
                        // Update last used timestamp
                        let _ = db.update_api_key_last_used(api_key.id).await;

                        // Create auth context
                        let auth_context = AuthContext {
                            user_id: api_key.user_id,
                            api_key_id: Some(api_key.id),
                            is_admin: false,  // API keys don't have admin access
                            rate_limit_rpm: api_key.rate_limit_rpm,
                        };

                        // Insert auth context into request extensions
                        request.extensions_mut().insert(auth_context);
                        request.extensions_mut().insert(api_key);

                        return Ok(next.run(request).await);
                    }
                }
            } else {
                // Try JWT token
                let jwt_config = JwtConfig::from_env();

                if let Ok(claims) = jwt_config.verify_token(token) {
                    // Create auth context
                    let auth_context = AuthContext {
                        user_id: claims.sub,
                        api_key_id: None,
                        is_admin: claims.is_admin,
                        rate_limit_rpm: None,  // JWT users use default rate limits
                    };

                    // Insert auth context into request extensions
                    request.extensions_mut().insert(auth_context);

                    return Ok(next.run(request).await);
                }
            }
        }
    }

    // No valid authentication found
    Err(StatusCode::UNAUTHORIZED)
}

/// Optional authentication middleware (allows unauthenticated requests)
pub async fn optional_auth_middleware(
    State(db): State<Arc<DatabaseService>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract authorization header
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    if let Some(auth_header) = auth_header {
        // Check if it's a Bearer token (JWT) or API key
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            // Try API key first
            if token.starts_with("sk-manti-") {
                // Extract prefix for faster lookup
                let prefix = token.chars().take(8).collect::<String>();

                // Find API key by prefix first (faster)
                if let Ok(Some(api_key)) = db.find_api_key_by_prefix(&prefix).await {
                    // Verify the full key
                    if api_key.verify(token) && api_key.is_valid() {
                        // Update last used timestamp
                        let _ = db.update_api_key_last_used(api_key.id).await;

                        // Create auth context
                        let auth_context = AuthContext {
                            user_id: api_key.user_id,
                            api_key_id: Some(api_key.id),
                            is_admin: false,
                            rate_limit_rpm: api_key.rate_limit_rpm,
                        };

                        // Insert auth context into request extensions
                        request.extensions_mut().insert(auth_context);
                        request.extensions_mut().insert(api_key);
                    }
                }
            } else {
                // Try JWT token
                let jwt_config = JwtConfig::from_env();

                if let Ok(claims) = jwt_config.verify_token(token) {
                    // Create auth context
                    let auth_context = AuthContext {
                        user_id: claims.sub,
                        api_key_id: None,
                        is_admin: claims.is_admin,
                        rate_limit_rpm: None,
                    };

                    // Insert auth context into request extensions
                    request.extensions_mut().insert(auth_context);
                }
            }
        }
    }

    // Continue even without authentication
    next.run(request).await
}

/// Extract auth context from request (for use in handlers)
pub fn get_auth_context(request: &Request) -> Option<AuthContext> {
    request.extensions().get::<AuthContext>().cloned()
}

/// Extract API key from request (for use in handlers when checking model permissions)
pub fn get_api_key(request: &Request) -> Option<ApiKey> {
    request.extensions().get::<ApiKey>().cloned()
}