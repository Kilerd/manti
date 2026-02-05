use crate::models::user::User;
use crate::Db;
use gotcha::axum::{
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use gotcha::tracing::{debug, error, info, warn};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid, // User ID
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub exp: i64, // Expiration time
    pub iat: i64, // Issued at
}

#[derive(Debug, Clone)]
pub enum AuthContext {
    User(Claims),
    ApiKey { user_id: Uuid, api_key_id: Uuid },
    None,
}

impl AuthContext {
    pub fn user_id(&self) -> Option<Uuid> {
        match self {
            AuthContext::User(claims) => Some(claims.sub),
            AuthContext::ApiKey { user_id, .. } => Some(*user_id),
            AuthContext::None => None,
        }
    }

    pub fn require_user(&self) -> Result<&Claims, StatusCode> {
        match self {
            AuthContext::User(claims) => Ok(claims),
            _ => Err(StatusCode::UNAUTHORIZED),
        }
    }

    pub fn require_auth(&self) -> Result<Uuid, StatusCode> {
        self.user_id().ok_or(StatusCode::UNAUTHORIZED)
    }

    pub fn require_admin(&self) -> Result<&Claims, StatusCode> {
        match self {
            AuthContext::User(claims) if claims.is_admin => Ok(claims),
            _ => Err(StatusCode::FORBIDDEN),
        }
    }
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
/// Supports both OpenAI format (Authorization: Bearer) and Anthropic format (x-api-key)
pub async fn auth_middleware(
    State(db): State<Db>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Try Authorization header first (OpenAI format)
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    // Also check x-api-key header (Anthropic format)
    let x_api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|h| h.to_str().ok());

    info!(
        "Auth headers - Authorization: {:?}, x-api-key: {:?}",
        auth_header.map(|s| &s[..s.len().min(30)]),
        x_api_key.map(|s| &s[..s.len().min(30)])
    );

    // Extract token from either header
    let token = if let Some(auth) = auth_header {
        auth.strip_prefix("Bearer ").map(|s| s.to_string())
    } else {
        None
    }
    .or_else(|| x_api_key.map(|s| s.to_string()));

    let Some(token) = token else {
        request.extensions_mut().insert(AuthContext::None);
        return Ok(next.run(request).await);
    };

    debug!(
        "Processing auth token: {}...",
        &token[..token.len().min(20)]
    );

    let context = if token.starts_with("sk-manti-") {
        debug!("Token identified as API key");
        match db.find_api_key(&token).await {
            Ok(Some(api_key)) => {
                info!("API key authenticated for user: {}", api_key.user_id);
                let _ = db.update_api_key_last_used(api_key.id).await;
                AuthContext::ApiKey {
                    user_id: api_key.user_id,
                    api_key_id: api_key.id,
                }
            }
            Ok(None) => {
                warn!(
                    "API key not found or invalid: {}...",
                    &token[..token.len().min(20)]
                );
                AuthContext::None
            }
            Err(e) => {
                error!("Database error looking up API key: {}", e);
                AuthContext::None
            }
        }
    } else {
        debug!("Token identified as JWT");
        let jwt_config = JwtConfig::from_env();
        match jwt_config.verify_token(&token) {
            Ok(claims) => {
                info!("JWT authenticated for user: {}", claims.sub);
                AuthContext::User(claims)
            }
            Err(e) => {
                warn!("JWT verification failed: {}", e);
                AuthContext::None
            }
        }
    };

    request.extensions_mut().insert(context);
    Ok(next.run(request).await)
}
