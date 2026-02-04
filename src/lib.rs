pub mod config;
pub mod models;
pub mod providers;
pub mod db;
pub mod auth;
pub mod api;

use std::sync::Arc;
use once_cell::sync::Lazy;
use thiserror::Error;
use tracing::{error, info};

use crate::db::DatabaseService;
use crate::models::provider_config::ProviderConfig as DbProviderConfig;
use crate::providers::{model_registry::ModelRegistry, ModelConfig, ProviderConfig, ProviderType};

/// Global model registry
pub static MODEL_REGISTRY: Lazy<Arc<ModelRegistry>> =
    Lazy::new(|| Arc::new(ModelRegistry::new()));

/// Reload all providers and models from database
pub async fn reload_providers(db: &DatabaseService, jwt_secret: &str) -> Result<usize> {
    // Clear existing providers and models
    MODEL_REGISTRY.clear();

    // Load providers from database
    let provider_configs = db.list_all_provider_configs().await?;
    let mut provider_count = 0;

    for db_config in &provider_configs {
        if !db_config.is_active {
            continue;
        }

        // Decrypt API key
        let api_key = match DbProviderConfig::decrypt_api_key(&db_config.api_key_encrypted, jwt_secret) {
            Ok(key) => key,
            Err(e) => {
                error!("Failed to decrypt API key for provider '{}': {}", db_config.name, e);
                continue;
            }
        };

        // Convert provider_type string to ProviderType enum
        let provider_type = match db_config.provider_type.as_str() {
            "openai" => ProviderType::OpenAI,
            "anthropic" => ProviderType::Anthropic,
            "google" => ProviderType::Google,
            other => ProviderType::Custom(other.to_string()),
        };

        if let Err(e) = MODEL_REGISTRY.register_provider(
            db_config.name.clone(),
            ProviderConfig {
                provider_type,
                api_key,
                base_url: db_config.base_url.clone(),
                organization: None,
                extra_params: Default::default(),
            },
            db_config.id,
            db_config.allowed_groups.clone(),
        ) {
            error!("Failed to register provider '{}': {}", db_config.name, e);
        } else {
            info!("Registered {} provider: {}", db_config.provider_type, db_config.name);
            provider_count += 1;
        }
    }

    // Load models from database and register them
    let mut model_count = 0;
    for db_config in &provider_configs {
        if !db_config.is_active {
            continue;
        }

        // Get models for this provider
        let models = match db.list_models_for_provider(db_config.id).await {
            Ok(models) => models,
            Err(e) => {
                error!("Failed to load models for provider '{}': {}", db_config.name, e);
                continue;
            }
        };

        for model in models {
            if !model.is_active {
                continue;
            }

            let model_config = ModelConfig {
                id: model.model_id.clone(),
                provider: db_config.name.clone(),
                input_cost_per_1k: model.input_cost_per_1k.unwrap_or(0.0),
                output_cost_per_1k: model.output_cost_per_1k.unwrap_or(0.0),
                max_context: model.max_context.unwrap_or(4096),
                supports_tools: model.supports_tools,
                supports_vision: model.supports_vision,
            };

            if let Err(e) = MODEL_REGISTRY.register_model(
                &db_config.name,
                model_config,
                db_config.id,
                db_config.allowed_groups.clone(),
            ) {
                error!("Failed to register model '{}': {}", model.model_id, e);
            } else {
                info!("Registered model: {} (provider: {})", model.model_id, db_config.name);
                model_count += 1;
            }
        }
    }

    info!("Loaded {} provider(s) with {} model(s)", provider_count, model_count);
    Ok(model_count)
}

#[derive(Error, Debug)]
pub enum MantiError {
    #[error("Database error: {0}")]
    Database(#[from] conservator::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] conservator::MigrateError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Billing error: {0}")]
    Billing(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Internal server error")]
    Internal,
}

pub type Result<T> = std::result::Result<T, MantiError>;

impl MantiError {
    pub fn to_status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            MantiError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            MantiError::Migration(_) => StatusCode::INTERNAL_SERVER_ERROR,
            MantiError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            MantiError::Auth(_) => StatusCode::UNAUTHORIZED,
            MantiError::Provider(_) => StatusCode::BAD_GATEWAY,
            MantiError::Billing(_) => StatusCode::PAYMENT_REQUIRED,
            MantiError::Validation(_) => StatusCode::BAD_REQUEST,
            MantiError::NotFound(_) => StatusCode::NOT_FOUND,
            MantiError::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            MantiError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}