mod api;
mod auth;
mod config;
mod db;
mod models;
mod providers;

use std::ops::Deref;
use std::sync::Arc;

use async_stream::stream;
use futures::stream::Stream;
use futures::StreamExt;
use gotcha::axum::extract::FromRef;
use gotcha::axum::response::{
    sse::{Event, KeepAlive, Sse},
    IntoResponse, Response,
};
use gotcha::Gotcha;
use gotcha::Json;
use once_cell::sync::Lazy;
use serde_json::json;
use thiserror::Error;
use tracing::{error, info};
use tracing_subscriber;

use crate::api::{admin, handlers};
use crate::auth::auth_middleware;
use crate::config::Settings;
use crate::db::DatabaseService;
use crate::models::{
    chat::ChatCompletionRequest,
    provider_config::ProviderConfig as DbProviderConfig,
    response::{ChatCompletionResponse, ModelListResponse},
    streaming::ChatCompletionChunk,
};
use crate::providers::{model_registry::ModelRegistry, ModelConfig, ProviderConfig, ProviderType};

// ============================================================================
// Database wrapper
// ============================================================================

/// Database service wrapper for state extraction
#[derive(Clone)]
pub struct Db(pub Arc<DatabaseService>);

impl Deref for Db {
    type Target = DatabaseService;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ============================================================================
// Global registry
// ============================================================================

/// Global model registry
pub static MODEL_REGISTRY: Lazy<Arc<ModelRegistry>> = Lazy::new(|| Arc::new(ModelRegistry::new()));

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
        let api_key =
            match DbProviderConfig::decrypt_api_key(&db_config.api_key_encrypted, jwt_secret) {
                Ok(key) => key,
                Err(e) => {
                    error!(
                        "Failed to decrypt API key for provider '{}': {}",
                        db_config.name, e
                    );
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
            info!(
                "Registered {} provider: {}",
                db_config.provider_type, db_config.name
            );
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
                error!(
                    "Failed to load models for provider '{}': {}",
                    db_config.name, e
                );
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
                input_cost_per_1k: model.input_cost_per_1k
                    .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0))
                    .unwrap_or(0.0),
                output_cost_per_1k: model.output_cost_per_1k
                    .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0))
                    .unwrap_or(0.0),
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
                info!(
                    "Registered model: {} (provider: {})",
                    model.model_id, db_config.name
                );
                model_count += 1;
            }
        }
    }

    info!(
        "Loaded {} provider(s) with {} model(s)",
        provider_count, model_count
    );
    Ok(model_count)
}

// ============================================================================
// Error types
// ============================================================================

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
    pub fn to_status_code(&self) -> gotcha::axum::http::StatusCode {
        use gotcha::axum::http::StatusCode;
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

// ============================================================================
// Application state
// ============================================================================

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
}

impl Default for AppState {
    fn default() -> Self {
        unimplemented!()
    }
}

// Allow extracting Db from GotchaContext
impl FromRef<gotcha::GotchaContext<AppState, Settings>> for Db {
    fn from_ref(ctx: &gotcha::GotchaContext<AppState, Settings>) -> Self {
        ctx.state.db.clone()
    }
}

// ============================================================================
// Route handlers
// ============================================================================

async fn chat_completions(Json(request): Json<ChatCompletionRequest>) -> Response {
    info!(
        "Received chat completion request for model: {}",
        request.model
    );

    // Get the model instance
    let model_instance = match MODEL_REGISTRY.get_model(&request.model) {
        Some(instance) => instance,
        None => {
            error!("Model not found: {}", request.model);
            return Json(json!({
                "error": {
                    "message": format!("Model '{}' not found", request.model),
                    "type": "invalid_request_error",
                    "code": "model_not_found"
                }
            }))
            .into_response();
        }
    };

    // Process the request through the model instance
    match model_instance.chat_completions(request.clone()).await {
        Ok(response) => match response {
            ChatCompletionResponse::NonStream(completion) => {
                let usage_data = &completion.usage;
                let cost = model_instance
                    .calculate_cost(usage_data.prompt_tokens, usage_data.completion_tokens);

                info!(
                    "Completed request - model: {}, tokens: {}/{}/{}, cost: ${:.4}",
                    completion.model,
                    usage_data.prompt_tokens,
                    usage_data.completion_tokens,
                    usage_data.total_tokens,
                    cost
                );

                Json(json!(completion)).into_response()
            }
            ChatCompletionResponse::Stream(stream) => {
                let event_stream = stream_to_sse(stream);
                Sse::new(event_stream)
                    .keep_alive(KeepAlive::default())
                    .into_response()
            }
        },
        Err(e) => {
            error!("Provider error: {}", e);
            Json(json!({
                "error": {
                    "message": format!("Provider error: {}", e),
                    "type": "provider_error",
                    "code": "provider_error"
                }
            }))
            .into_response()
        }
    }
}

fn stream_to_sse(
    mut stream: futures::stream::BoxStream<'static, Result<ChatCompletionChunk>>,
) -> impl Stream<Item = std::result::Result<Event, gotcha::axum::Error>> {
    stream! {
        while let Some(result) = stream.next().await {
            match result {
                Ok(chunk) => {
                    let data = serde_json::to_string(&chunk).unwrap_or_else(|e| {
                        error!("Failed to serialize chunk: {}", e);
                        "{}".to_string()
                    });
                    yield Ok(Event::default().data(data));
                }
                Err(e) => {
                    error!("Error in stream: {}", e);
                    let error_data = json!({
                        "error": {
                            "message": format!("Stream error: {}", e),
                            "type": "stream_error",
                            "code": "stream_error"
                        }
                    });
                    yield Ok(Event::default().data(serde_json::to_string(&error_data).unwrap()));
                    break;
                }
            }
        }
        yield Ok(Event::default().data("[DONE]"));
    }
}

async fn list_models() -> Response {
    let models = MODEL_REGISTRY.get_all_models();

    let response = ModelListResponse {
        object: "list".to_string(),
        data: models,
    };

    Json(response).into_response()
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("Starting Manti LLM Gateway");

    let settings = Settings::new().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    info!("Initializing database...");
    let db_service = Arc::new(DatabaseService::new(&settings.application.database.url)?);

    info!("Running database migrations...");
    db_service.migrate().await?;

    info!("Loading providers from database...");
    let loaded = reload_providers(&db_service, &settings.application.auth.jwt_secret).await?;
    info!("Loaded {} provider(s)", loaded);

    let available_models = MODEL_REGISTRY.list_model_names();
    info!("Available models: {}", available_models.join(", "));

    let db = Db(db_service);
    let app_state = AppState { db: db.clone() };

    let addr = format!("{}:{}", &settings.basic.host, &settings.basic.port);
    info!("Starting server on http://{}", addr);

    Gotcha::with_types::<AppState, Settings>()
        .state(app_state)
        .config(settings)
        // LLM routes
        .post("/v1/chat/completions", chat_completions)
        .get("/v1/models", list_models)
        // Public auth routes
        .get("/health", handlers::health_check)
        .post("/auth/register", handlers::register)
        .post("/auth/login", handlers::login)
        .post("/auth/refresh", handlers::refresh_token)
        // Protected user routes
        .get("/user/me", handlers::get_current_user)
        .get("/user/profile", handlers::get_current_user)
        .put("/user/profile", handlers::update_profile)
        .get("/auth/validate", handlers::validate_token)
        .post("/auth/logout", handlers::logout)
        // Models route
        .get("/models", handlers::list_available_models)
        // API key routes
        .post("/api-keys", handlers::create_api_key)
        .get("/api-keys", handlers::list_api_keys)
        .delete("/api-keys/:id", handlers::revoke_api_key)
        // Usage routes
        .get("/usage", handlers::get_usage)
        .get("/usage/stats", handlers::get_usage_stats)
        // Admin routes
        .get("/admin/providers", admin::list_all_providers)
        .post("/admin/providers", admin::create_provider)
        .put("/admin/providers/:id", admin::update_provider)
        .delete("/admin/providers/:id", admin::delete_provider)
        .get(
            "/admin/providers/:provider_id/models",
            admin::list_provider_models,
        )
        .post("/admin/providers/:provider_id/models", admin::create_model)
        .put("/admin/models/:id", admin::update_model)
        .delete("/admin/models/:id", admin::delete_model)
        .get("/admin/users", admin::list_users)
        .put("/admin/users/:user_id/groups", admin::update_user_groups)
        .get("/admin/users/:user_id/usage", admin::get_usage_stats)
        .layer(gotcha::axum::middleware::from_fn_with_state(
            db,
            auth_middleware,
        ))
        .with_cors()
        .with_openapi()
        .listen(addr)
        .await?;

    Ok(())
}
