use crate::{
    auth::AuthContext,
    models::{
        model::{CreateModel, CreateModelRequest, ModelInfo, UpdateModelRequest},
        provider_config::{
            CreateProviderConfig, CreateProviderConfigRequest, ProviderConfig, ProviderConfigInfo,
            UpdateProviderConfigRequest, UsageStats,
        },
        user::{UpdateUserGroupsRequest, UserInfo},
    },
    reload_providers, Db,
};
use chrono::{DateTime, Duration, Utc};
use gotcha::axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
};
use gotcha::{Json, Schematic};
use serde::Deserialize;
use uuid::Uuid;

fn require_admin(auth: &AuthContext) -> Result<(), StatusCode> {
    auth.require_admin()?;
    Ok(())
}

// Provider configuration handlers (admin only, global providers)

/// List all provider configurations (admin only)
#[gotcha::api(group = "Admin - Providers")]
pub async fn list_all_providers(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<ProviderConfigInfo>>, StatusCode> {
    require_admin(&auth)?;

    let configs = db
        .list_all_provider_configs()
        .await
        .map_err(|e| e.to_status_code())?;

    Ok(Json(configs.into_iter().map(|c| c.into()).collect()))
}

/// Create a new provider configuration (admin only)
#[gotcha::api(group = "Admin - Providers")]
pub async fn create_provider(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Json(req): Json<CreateProviderConfigRequest>,
) -> Result<Json<ProviderConfigInfo>, StatusCode> {
    require_admin(&auth)?;

    // Validate provider type
    if !ProviderConfig::is_valid_provider_type(&req.provider_type) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Encrypt the API key
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret".to_string());
    let encrypted_key = ProviderConfig::encrypt_api_key(&req.api_key, &jwt_secret);

    let create_config = CreateProviderConfig {
        provider_type: req.provider_type,
        name: req.name,
        api_key_encrypted: encrypted_key,
        base_url: req.base_url,
        priority: req.priority.unwrap_or(0),
        is_active: true,
        rate_limit: req.rate_limit,
        monthly_quota: req.monthly_quota,
        used_quota: 0.0,
        allowed_groups: req.allowed_groups.unwrap_or_default(),
    };

    let created = db
        .create_provider_config(create_config)
        .await
        .map_err(|e| e.to_status_code())?;

    // Reload providers after create
    let _ = reload_providers(&db, &jwt_secret).await;

    Ok(Json(created.into()))
}

/// Update a provider configuration (admin only)
#[gotcha::api(group = "Admin - Providers")]
pub async fn update_provider(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateProviderConfigRequest>,
) -> Result<Json<ProviderConfigInfo>, StatusCode> {
    require_admin(&auth)?;

    // Verify the provider exists
    db.get_provider_config(id)
        .await
        .map_err(|e| e.to_status_code())?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Encrypt new API key if provided
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret".to_string());
    let encrypted_key = req
        .api_key
        .as_ref()
        .map(|api_key| ProviderConfig::encrypt_api_key(api_key, &jwt_secret));

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
            req.allowed_groups,
        )
        .await
        .map_err(|e| e.to_status_code())?;

    // Reload providers after update
    let _ = reload_providers(&db, &jwt_secret).await;

    Ok(Json(updated.into()))
}

/// Delete a provider configuration (admin only)
#[gotcha::api(group = "Admin - Providers")]
pub async fn delete_provider(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(id): Path<Uuid>,
) -> Result<Json<()>, StatusCode> {
    require_admin(&auth)?;

    // Verify the provider exists
    db.get_provider_config(id)
        .await
        .map_err(|e| e.to_status_code())?
        .ok_or(StatusCode::NOT_FOUND)?;

    db.delete_provider_config(id)
        .await
        .map_err(|e| e.to_status_code())?;

    // Reload providers after delete
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret".to_string());
    let _ = reload_providers(&db, &jwt_secret).await;

    Ok(Json(()))
}

// Model handlers (admin only)

/// List models for a provider (admin only)
#[gotcha::api(group = "Admin - Models")]
pub async fn list_provider_models(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(provider_id): Path<Uuid>,
) -> Result<Json<Vec<ModelInfo>>, StatusCode> {
    require_admin(&auth)?;

    // Verify the provider exists
    db.get_provider_config(provider_id)
        .await
        .map_err(|e| e.to_status_code())?
        .ok_or(StatusCode::NOT_FOUND)?;

    let models = db
        .list_models_for_provider(provider_id)
        .await
        .map_err(|e| e.to_status_code())?;

    Ok(Json(models.into_iter().map(|m| m.into()).collect()))
}

/// Create a model for a provider (admin only)
#[gotcha::api(group = "Admin - Models")]
pub async fn create_model(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(provider_id): Path<Uuid>,
    Json(req): Json<CreateModelRequest>,
) -> Result<Json<ModelInfo>, StatusCode> {
    require_admin(&auth)?;

    // Verify the provider exists
    db.get_provider_config(provider_id)
        .await
        .map_err(|e| e.to_status_code())?
        .ok_or(StatusCode::NOT_FOUND)?;

    let create_model = CreateModel {
        provider_config_id: provider_id,
        model_id: req.model_id,
        display_name: req.display_name,
        input_cost_per_1k: req.input_cost_per_1k,
        output_cost_per_1k: req.output_cost_per_1k,
        max_context: req.max_context,
        supports_tools: req.supports_tools.unwrap_or(false),
        supports_vision: req.supports_vision.unwrap_or(false),
        is_active: true,
    };

    let created = db
        .create_model(create_model)
        .await
        .map_err(|e| e.to_status_code())?;

    // Reload providers after model change
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret".to_string());
    let _ = reload_providers(&db, &jwt_secret).await;

    Ok(Json(created.into()))
}

/// Update a model (admin only)
#[gotcha::api(group = "Admin - Models")]
pub async fn update_model(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(model_id): Path<Uuid>,
    Json(req): Json<UpdateModelRequest>,
) -> Result<Json<ModelInfo>, StatusCode> {
    require_admin(&auth)?;

    // Verify the model exists
    db.get_model(model_id)
        .await
        .map_err(|e| e.to_status_code())?
        .ok_or(StatusCode::NOT_FOUND)?;

    let updated = db
        .update_model(
            model_id,
            req.display_name.map(Some),
            req.input_cost_per_1k.map(Some),
            req.output_cost_per_1k.map(Some),
            req.max_context.map(Some),
            req.supports_tools,
            req.supports_vision,
            req.is_active,
        )
        .await
        .map_err(|e| e.to_status_code())?;

    // Reload providers after model change
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret".to_string());
    let _ = reload_providers(&db, &jwt_secret).await;

    Ok(Json(updated.into()))
}

/// Delete a model (admin only)
#[gotcha::api(group = "Admin - Models")]
pub async fn delete_model(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(model_id): Path<Uuid>,
) -> Result<Json<()>, StatusCode> {
    require_admin(&auth)?;

    // Verify the model exists
    db.get_model(model_id)
        .await
        .map_err(|e| e.to_status_code())?
        .ok_or(StatusCode::NOT_FOUND)?;

    db.delete_model(model_id)
        .await
        .map_err(|e| e.to_status_code())?;

    // Reload providers after model change
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "default-secret".to_string());
    let _ = reload_providers(&db, &jwt_secret).await;

    Ok(Json(()))
}

// User management handlers

/// Get all users (admin only)
#[gotcha::api(group = "Admin - Users")]
pub async fn list_users(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
) -> Result<Json<Vec<UserInfo>>, StatusCode> {
    require_admin(&auth)?;

    let users = db.list_all_users().await.map_err(|e| e.to_status_code())?;

    Ok(Json(users.into_iter().map(|u| u.into()).collect()))
}

/// Update user groups (admin only)
#[gotcha::api(group = "Admin - Users")]
pub async fn update_user_groups(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path(user_id): Path<Uuid>,
    Json(req): Json<UpdateUserGroupsRequest>,
) -> Result<Json<UserInfo>, StatusCode> {
    require_admin(&auth)?;

    // Verify the user exists
    db.find_user_by_id(user_id)
        .await
        .map_err(|e| e.to_status_code())?
        .ok_or(StatusCode::NOT_FOUND)?;

    let updated = db
        .update_user_groups(user_id, req.user_groups)
        .await
        .map_err(|e| e.to_status_code())?;

    Ok(Json(updated.into()))
}

// Usage statistics handlers

#[derive(Debug, Deserialize, gotcha::Schematic)]
pub struct UsageQuery {
    pub start: Option<String>, // ISO 8601 datetime
    pub end: Option<String>,   // ISO 8601 datetime
}

/// Get usage statistics for a user
#[gotcha::api(group = "Admin - Usage")]
pub async fn get_usage_stats(
    Extension(auth): Extension<AuthContext>,
    State(db): State<Db>,
    Path((user_id,)): Path<(Uuid,)>,
    Query(query): Query<UsageQuery>,
) -> Result<Json<UsageStats>, StatusCode> {
    // Only admin or the user themselves can view their usage
    if auth.user_id() != Some(user_id) {
        require_admin(&auth)?;
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
