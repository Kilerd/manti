use async_stream::stream;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use futures::StreamExt;
use gotcha::prelude::*;
use manti::{
    config::Settings,
    models::{
        chat::ChatCompletionRequest,
        response::{ChatCompletionResponse, ModelListResponse},
        streaming::ChatCompletionChunk,
    },
    providers::{model_registry::ModelRegistry, ProviderConfig, ProviderType},
};
use serde_json::json;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber;

// Global model registry
static MODEL_REGISTRY: gotcha::Lazy<Arc<ModelRegistry>> =
    gotcha::Lazy::new(|| Arc::new(ModelRegistry::new()));

async fn chat_completions(
    Json(request): Json<ChatCompletionRequest>,
) -> Response {
    info!("Received chat completion request for model: {}", request.model);

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
            })).into_response();
        }
    };

    // Process the request through the model instance
    match model_instance.chat_completions(request).await {
        Ok(response) => {
            match response {
                ChatCompletionResponse::NonStream(completion) => {
                    let usage = &completion.usage;
                    let cost = model_instance.calculate_cost(
                        usage.prompt_tokens,
                        usage.completion_tokens,
                    );

                    info!(
                        "Completed request - model: {}, tokens: {}/{}/{}, cost: ${:.4}",
                        completion.model,
                        usage.prompt_tokens,
                        usage.completion_tokens,
                        usage.total_tokens,
                        cost
                    );

                    Json(json!(completion)).into_response()
                }
                ChatCompletionResponse::Stream(stream) => {
                    // Convert the stream to SSE events
                    let event_stream = stream_to_sse(stream);
                    Sse::new(event_stream)
                        .keep_alive(KeepAlive::default())
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!("Provider error: {}", e);
            Json(json!({
                "error": {
                    "message": format!("Provider error: {}", e),
                    "type": "provider_error",
                    "code": "provider_error"
                }
            })).into_response()
        }
    }
}

// Convert provider stream to SSE events
fn stream_to_sse(
    mut stream: futures::stream::BoxStream<'static, manti::Result<ChatCompletionChunk>>,
) -> impl Stream<Item = Result<Event, axum::Error>> {
    stream! {
        while let Some(result) = stream.next().await {
            match result {
                Ok(chunk) => {
                    // Format the chunk as OpenAI-compatible SSE data
                    let data = serde_json::to_string(&chunk).unwrap_or_else(|e| {
                        error!("Failed to serialize chunk: {}", e);
                        "{}".to_string()
                    });

                    yield Ok(Event::default().data(data));
                }
                Err(e) => {
                    error!("Error in stream: {}", e);
                    // Send error as an SSE event
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

        // Send the final [DONE] message as per OpenAI API convention
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

async fn health_check() -> Response {
    Json(json!({
        "status": "healthy",
        "service": "manti-llm-gateway",
        "models_loaded": MODEL_REGISTRY.list_model_names().len(),
    })).into_response()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("🚀 Starting Manti LLM Gateway");

    // Load configuration
    let settings = Settings::new().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        e
    })?;

    info!("📦 Initializing providers...");

    // Register providers based on configuration
    if let Some(openai_config) = &settings.providers.openai {
        info!("  ✓ Registering OpenAI provider");
        MODEL_REGISTRY.register_provider(
            "openai".to_string(),
            ProviderConfig {
                provider_type: ProviderType::OpenAI,
                api_key: openai_config.api_key.clone(),
                base_url: openai_config.base_url.clone(),
                organization: None,
                extra_params: Default::default(),
            },
        )?;
    }

    if let Some(anthropic_config) = &settings.providers.anthropic {
        info!("  ✓ Registering Anthropic provider");
        MODEL_REGISTRY.register_provider(
            "anthropic".to_string(),
            ProviderConfig {
                provider_type: ProviderType::Anthropic,
                api_key: anthropic_config.api_key.clone(),
                base_url: anthropic_config.base_url.clone(),
                organization: None,
                extra_params: Default::default(),
            },
        )?;
    }

    // Log available models
    let available_models = MODEL_REGISTRY.list_model_names();
    info!("📊 Available models: {}", available_models.join(", "));

    // Start server
    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    info!("🌐 Starting server on http://{}", addr);
    info!("📖 Endpoints:");
    info!("   POST /v1/chat/completions - Chat completions (OpenAI-compatible)");
    info!("   GET  /v1/models           - List available models");
    info!("   GET  /health              - Health check");

    Gotcha::new()
        .post("/v1/chat/completions", chat_completions)
        .get("/v1/models", list_models)
        .get("/health", health_check)
        .listen(&addr)
        .await?;

    Ok(())
}