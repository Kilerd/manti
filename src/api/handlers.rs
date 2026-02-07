use crate::{
    auth::{AuthContext, JwtConfig},
    models::{
        api_key::{ApiKeyInfo, ApiKeyStats, CreateApiKey, CreateApiKeyRequest},
        billing::{BillingInfo, UserBalanceInfo},
        model::ModelInfo,
        usage::UsageInfo,
        user::{CreateUser, CreateUserRequest, LoginRequest, LoginResponse, UserInfo},
    },
    Db,
};
use chrono::{DateTime, Duration, Utc};
use gotcha::axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
};
use gotcha::prelude::*;
use gotcha::{Json, Schematic};
use serde::Deserialize;
use uuid::Uuid;

// User handlers

/// Register a new user
#[gotcha::api(group = "Auth")]
pub async fn register(
    State(db): State<Db>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    // Validate email format
    if !req.email.contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check if user already exists
    if let Ok(Some(_)) = db.find_user_by_email(&req.email).await {
        return Err(StatusCode::CONFLICT);
    }

    // Create new user
    let create_user = CreateUser::new(req.email, req.username, &req.password, false)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Save to database
    let saved_user = db
        .create_user(create_user)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Generate JWT token for immediate login
    let jwt_config = JwtConfig::from_env();
    let token = jwt_config
        .generate_token(&saved_user)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse {
        token,
        user: saved_user.into(),
    }))
}

/// Login with email and password
#[gotcha::api(group = "Auth")]
pub async fn login(
    State(db): State<Db>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    // Find user by email
    let user = db
        .find_user_by_email(&req.email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Verify password
    if !user.verify_password(&req.password) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Update last login
    let _ = db.update_user_last_login(user.id).await;

    // Generate JWT token
    let jwt_config = JwtConfig::from_env();
    let token = jwt_config
        .generate_token(&user)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse {
        token,
        user: user.into(),
    }))
}

/// Get current user info
#[gotcha::api(group = "User")]
pub async fn get_current_user(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<UserInfo>, StatusCode> {
    let user_id = auth.require_auth()?;
    let user = db
        .find_user_by_id(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(user.into()))
}

/// Validate token response
#[derive(serde::Serialize, Schematic)]
pub struct ValidateTokenResponse {
    pub user: UserInfo,
}

/// Validate token - returns user info if token is valid
#[gotcha::api(group = "Auth")]
pub async fn validate_token(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<ValidateTokenResponse>, StatusCode> {
    let user_id = auth.require_auth()?;
    let user = db
        .find_user_by_id(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ValidateTokenResponse { user: user.into() }))
}

/// Logout - invalidate token (stateless, just returns success)
#[gotcha::api(group = "Auth")]
pub async fn logout() -> Result<Json<serde_json::Value>, StatusCode> {
    // JWT is stateless, so logout is handled client-side by removing the token
    Ok(Json(json!({ "message": "Logged out successfully" })))
}

/// Refresh token request
#[derive(Deserialize, Schematic)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

/// Refresh token - generate new access token
#[gotcha::api(group = "Auth")]
pub async fn refresh_token(
    State(db): State<Db>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Verify and decode the refresh token
    let jwt_config = JwtConfig::from_env();
    let claims = jwt_config
        .verify_token(&req.refresh_token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Find the user
    let user = db
        .find_user_by_id(claims.sub)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Generate new token
    let token = jwt_config
        .generate_token(&user)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "token": token })))
}

/// Update profile request
#[derive(Deserialize, Schematic)]
pub struct UpdateProfileRequest {
    pub username: Option<String>,
}

/// Update user profile
#[gotcha::api(group = "User")]
pub async fn update_profile(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Json(_req): Json<UpdateProfileRequest>,
) -> Result<Json<UserInfo>, StatusCode> {
    let user_id = auth.require_auth()?;
    // For now, just return current user - profile update can be implemented later
    let user = db
        .find_user_by_id(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // TODO: Implement actual profile update when needed

    Ok(Json(user.into()))
}

/// List models available to the current user based on their user_groups
#[gotcha::api(group = "Models")]
pub async fn list_available_models(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<ModelInfo>>, StatusCode> {
    let user_id = auth.require_auth()?;
    let user = db
        .find_user_by_id(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let models = db
        .list_models_for_groups(&user.user_groups)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(models.into_iter().map(|m| m.into()).collect()))
}

// API Key handlers

/// Create a new API key
#[gotcha::api(group = "API Keys")]
pub async fn create_api_key(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyInfo>, StatusCode> {
    let user_id = auth.require_auth()?;
    let expires_at = req
        .expires_in_days
        .map(|days| Utc::now() + Duration::days(days));

    let create_api_key = CreateApiKey::generate(
        user_id,
        req.name,
        expires_at,
        req.rate_limit_rpm,
        req.allowed_models,
    );

    let saved_key = db
        .create_api_key(create_api_key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(saved_key.into()))
}

/// List user's API keys
#[gotcha::api(group = "API Keys")]
pub async fn list_api_keys(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<ApiKeyInfo>>, StatusCode> {
    let user_id = auth.require_auth()?;
    let keys = db
        .list_user_api_keys(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(keys.into_iter().map(|k| k.into()).collect()))
}

/// Revoke an API key
#[gotcha::api(group = "API Keys")]
pub async fn revoke_api_key(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(key_id): Path<Uuid>,
) -> Result<Json<()>, StatusCode> {
    let user_id = auth.require_auth()?;
    db.delete_api_key(key_id, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(()))
}

// Usage handlers

#[derive(Deserialize, Schematic)]
pub struct UsageQuery {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

/// Get usage summary for the authenticated user
#[gotcha::api(group = "Usage")]
pub async fn get_usage(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Query(query): Query<UsageQuery>,
) -> Result<Json<Vec<UsageInfo>>, StatusCode> {
    let user_id = auth.require_auth()?;
    // Default to last 30 days if not specified
    let end = query.end.unwrap_or_else(Utc::now);
    let start = query.start.unwrap_or_else(|| end - Duration::days(30));

    let usage = db
        .get_usage_summary(user_id, start, end)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(usage.into_iter().map(|u| u.into()).collect()))
}

/// Dashboard stats response
#[derive(serde::Serialize, Schematic)]
pub struct DashboardStats {
    pub total_requests: i64,
    pub total_tokens: i64,
    pub total_cost: f64,
    pub active_keys: i64,
}

/// Get dashboard stats for the authenticated user
#[gotcha::api(group = "Usage")]
pub async fn get_usage_stats(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<DashboardStats>, StatusCode> {
    let user_id = auth.require_auth()?;
    // Get usage stats for last 30 days
    let end = Utc::now();
    let start = end - Duration::days(30);

    let usage_stats = db
        .get_usage_stats(user_id, start, end)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Count active API keys
    let api_keys = db
        .list_user_api_keys(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let active_keys = api_keys
        .iter()
        .filter(|k| k.is_active && k.is_valid())
        .count() as i64;

    Ok(Json(DashboardStats {
        total_requests: usage_stats.total_requests,
        total_tokens: usage_stats.total_tokens,
        total_cost: usage_stats.total_cost,
        active_keys,
    }))
}

/// Health check response
#[derive(serde::Serialize, Schematic)]
pub struct HealthCheckResponse {
    pub status: String,
    pub service: String,
    pub timestamp: DateTime<Utc>,
}

/// Health check endpoint (no auth required)
#[gotcha::api(group = "Health")]
pub async fn health_check() -> Json<HealthCheckResponse> {
    Json(HealthCheckResponse {
        status: "healthy".to_string(),
        service: "manti-llm-gateway".to_string(),
        timestamp: Utc::now(),
    })
}

// API Key stats

/// Get usage statistics for all API keys of the authenticated user
#[gotcha::api(group = "API Keys")]
pub async fn get_api_key_stats(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<ApiKeyStats>>, StatusCode> {
    let user_id = auth.require_auth()?;

    let stats = db
        .get_api_key_usage_stats(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(stats))
}

// User balance handlers

/// Get current user's balance
#[gotcha::api(group = "User")]
pub async fn get_my_balance(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<UserBalanceInfo>, StatusCode> {
    let user_id = auth.require_auth()?;

    let balance = db
        .get_user_balance(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(balance.into()))
}

// Billing handlers

/// List all billing records for the authenticated user
#[gotcha::api(group = "Billing")]
pub async fn list_billings(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<BillingInfo>>, StatusCode> {
    let user_id = auth.require_auth()?;

    let billings = db
        .list_user_billings(user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(billings.into_iter().map(|b| b.into()).collect()))
}

/// Get a specific billing record
#[gotcha::api(group = "Billing")]
pub async fn get_billing(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(billing_id): Path<Uuid>,
) -> Result<Json<BillingInfo>, StatusCode> {
    let user_id = auth.require_auth()?;

    let billing = db
        .get_billing(billing_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify ownership
    if billing.user_id != user_id {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(Json(billing.into()))
}
