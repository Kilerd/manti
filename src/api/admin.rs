use crate::{
    auth::AuthContext,
    db::DatabaseService,
    models::{
        provider_config::{
            CreateProviderConfigRequest, ProviderConfig, ProviderConfigInfo,
            UpdateProviderConfigRequest, UsageStats,
        },
    },
};
use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use gotcha::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

// Admin middleware - check if user is admin
pub async fn require_admin(auth: &AuthContext, db: &DatabaseService) -> Result<(), StatusCode> {
    let user = db
        .find_user_by_id(auth.user_id)
        .await
        .map_err(|e| e.to_status_code())?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(())
}

// Provider configuration handlers

/// List all provider configurations (admin only)
pub async fn list_all_providers(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Arc<DatabaseService>>,
) -> Result<Json<Vec<ProviderConfigInfo>>, StatusCode> {
    require_admin(&auth, &db).await?;

    let configs = db
        .list_all_provider_configs()
        .await
        .map_err(|e| e.to_status_code())?;

    Ok(Json(configs.into_iter().map(|c| c.into()).collect()))
}

/// List provider configurations for a specific user
pub async fn list_user_providers(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Arc<DatabaseService>>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<Vec<ProviderConfigInfo>>, StatusCode> {
    // Only admin or the user themselves can list their providers
    if auth.user_id != user_id {
        require_admin(&auth, &db).await?;
    }

    let configs = db
        .list_provider_configs(user_id)
        .await
        .map_err(|e| e.to_status_code())?;

    Ok(Json(configs.into_iter().map(|c| c.into()).collect()))
}

/// Create a new provider configuration
pub async fn create_provider(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Arc<DatabaseService>>,
    Json(req): Json<CreateProviderConfigRequest>,
) -> Result<Json<ProviderConfigInfo>, StatusCode> {
    // Validate provider type
    if !ProviderConfig::is_valid_provider_type(&req.provider_type) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Determine the user_id
    let user_id = if let Some(requested_user_id) = req.user_id {
        // If requesting to create for another user, must be admin
        if requested_user_id != auth.user_id {
            require_admin(&auth, &db).await?;
        }
        requested_user_id
    } else {
        auth.user_id
    };

    // Encrypt the API key
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret".to_string());
    let encrypted_key = ProviderConfig::encrypt_api_key(&req.api_key, &jwt_secret);

    let mut config = ProviderConfig::new(
        user_id,
        req.provider_type,
        req.name,
        encrypted_key,
    );

    config.base_url = req.base_url;
    config.priority = req.priority.unwrap_or(0);
    config.rate_limit = req.rate_limit;
    config.monthly_quota = req.monthly_quota;

    let created = db
        .create_provider_config(&config)
        .await
        .map_err(|e| e.to_status_code())?;

    Ok(Json(created.into()))
}

/// Update a provider configuration
pub async fn update_provider(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Arc<DatabaseService>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProviderConfigRequest>,
) -> Result<Json<ProviderConfigInfo>, StatusCode> {
    // Get the existing config to check ownership
    let existing = db
        .get_provider_config(id)
        .await
        .map_err(|e| e.to_status_code())?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Only admin or the owner can update
    if existing.user_id != auth.user_id {
        require_admin(&auth, &db).await?;
    }

    // Encrypt new API key if provided
    let encrypted_key = if let Some(ref api_key) = req.api_key {
        let jwt_secret =
            std::env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret".to_string());
        Some(ProviderConfig::encrypt_api_key(api_key, &jwt_secret))
    } else {
        None
    };

    let updated = db
        .update_provider_config(
            id,
            req.name,
            encrypted_key,
            req.base_url.map(Some),
            req.priority,
            req.is_active,
            req.rate_limit.map(Some),
            req.monthly_quota.map(Some),
        )
        .await
        .map_err(|e| e.to_status_code())?;

    Ok(Json(updated.into()))
}

/// Delete a provider configuration
pub async fn delete_provider(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Arc<DatabaseService>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // Get the existing config to check ownership
    let existing = db
        .get_provider_config(id)
        .await
        .map_err(|e| e.to_status_code())?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Only admin or the owner can delete
    if existing.user_id != auth.user_id {
        require_admin(&auth, &db).await?;
    }

    db.delete_provider_config(id, existing.user_id)
        .await
        .map_err(|e| e.to_status_code())?;

    Ok(StatusCode::NO_CONTENT)
}

// Usage statistics handlers

#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    pub start: Option<String>, // ISO 8601 datetime
    pub end: Option<String>,   // ISO 8601 datetime
}

/// Get usage statistics for a user
pub async fn get_usage_stats(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Arc<DatabaseService>>,
    Path(user_id): Path<Uuid>,
    Query(query): Query<UsageQuery>,
) -> Result<Json<UsageStats>, StatusCode> {
    // Only admin or the user themselves can view their usage
    if auth.user_id != user_id {
        require_admin(&auth, &db).await?;
    }

    // Parse dates or use defaults (last 30 days)
    let end = if let Some(end_str) = query.end {
        DateTime::parse_from_rfc3339(&end_str)
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .with_timezone(&Utc)
    } else {
        Utc::now()
    };

    let start = if let Some(start_str) = query.start {
        DateTime::parse_from_rfc3339(&start_str)
            .map_err(|_| StatusCode::BAD_REQUEST)?
            .with_timezone(&Utc)
    } else {
        end - Duration::days(30)
    };

    let stats = db
        .get_usage_stats(user_id, start, end)
        .await
        .map_err(|e| e.to_status_code())?;

    Ok(Json(stats))
}

/// Get all users (admin only)
pub async fn list_users(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Arc<DatabaseService>>,
) -> Result<Json<Vec<crate::models::user::UserInfo>>, StatusCode> {
    require_admin(&auth, &db).await?;

    // TODO: Implement list all users in DatabaseService
    // For now, return empty array
    Ok(Json(vec![]))
}
