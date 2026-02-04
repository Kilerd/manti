use crate::{
    auth::{AuthContext, JwtConfig},
    db::DatabaseService,
    models::{
        api_key::{ApiKeyInfo, ApiKeyResponse, CreateApiKey, CreateApiKeyRequest},
        usage::Usage,
        user::{CreateUser, CreateUserRequest, LoginRequest, LoginResponse, UserInfo},
    },
};
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Response,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use gotcha::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

// User handlers

/// Register a new user
pub async fn register(
    State(db): State<Arc<DatabaseService>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<UserInfo>, StatusCode> {
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

    Ok(Json(saved_user.into()))
}

/// Login with email and password
pub async fn login(
    State(db): State<Arc<DatabaseService>>,
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
pub async fn get_current_user(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Arc<DatabaseService>>,
) -> Result<Json<UserInfo>, StatusCode> {
    let user = db
        .find_user_by_id(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(user.into()))
}

// API Key handlers

/// Create a new API key
pub async fn create_api_key(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Arc<DatabaseService>>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<ApiKeyResponse>, StatusCode> {
    // Calculate expires_at from expires_in_days
    let expires_at = req.expires_in_days.map(|days| Utc::now() + Duration::days(days));

    // Generate new API key using CreateApiKey
    let (create_api_key, raw_key) = CreateApiKey::generate(
        auth.user_id,
        req.name,
        expires_at,
        req.rate_limit_rpm,
        req.allowed_models,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Save to database
    let saved_key = db
        .create_api_key(create_api_key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiKeyResponse {
        id: saved_key.id,
        name: saved_key.name,
        key: raw_key,  // Only returned on creation
        prefix: saved_key.prefix,
        created_at: saved_key.created_at,
        expires_at: saved_key.expires_at,
    }))
}

/// List user's API keys
pub async fn list_api_keys(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Arc<DatabaseService>>,
) -> Result<Json<Vec<ApiKeyInfo>>, StatusCode> {
    let keys = db
        .list_user_api_keys(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let key_infos: Vec<ApiKeyInfo> = keys
        .into_iter()
        .map(|k| {
            let allowed_models = k.get_allowed_models();
            ApiKeyInfo {
                id: k.id,
                name: k.name,
                prefix: k.prefix,
                is_active: k.is_active,
                last_used: k.last_used,
                expires_at: k.expires_at,
                created_at: k.created_at,
                rate_limit_rpm: k.rate_limit_rpm,
                allowed_models,
            }
        })
        .collect();

    Ok(Json(key_infos))
}

/// Revoke an API key
pub async fn revoke_api_key(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Arc<DatabaseService>>,
    Path(key_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    db.revoke_api_key(key_id, auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

// Usage handlers

#[derive(Deserialize)]
pub struct UsageQuery {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

/// Get usage summary for the authenticated user
pub async fn get_usage(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Arc<DatabaseService>>,
    Query(query): Query<UsageQuery>,
) -> Result<Json<Vec<Usage>>, StatusCode> {
    // Default to last 30 days if not specified
    let end = query.end.unwrap_or_else(Utc::now);
    let start = query.start.unwrap_or_else(|| end - Duration::days(30));

    let usage = db
        .get_usage_summary(auth.user_id, start, end)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(usage))
}

/// Health check endpoint (no auth required)
pub async fn health_check() -> Response {
    Json(json!({
        "status": "healthy",
        "service": "manti-llm-gateway",
        "timestamp": Utc::now(),
    }))
    .into_response()
}