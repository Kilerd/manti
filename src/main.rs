use async_stream::stream;
use axum::{
    response::{IntoResponse, Response, sse::{Event, KeepAlive, Sse}},
    routing::{get, post},
    Json, Router,
};
use futures::stream::Stream;
use futures::StreamExt;
use manti::{
    api::routes::create_router,
    config::Settings,
    db::DatabaseService,
    models::{
        chat::ChatCompletionRequest,
        response::{ChatCompletionResponse, ModelListResponse},
        streaming::ChatCompletionChunk,
    },
    reload_providers, MODEL_REGISTRY,
};
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber;

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
    match model_instance.chat_completions(request.clone()).await {
        Ok(response) => {
            match response {
                ChatCompletionResponse::NonStream(completion) => {
                    let usage_data = &completion.usage;
                    let cost = model_instance.calculate_cost(
                        usage_data.prompt_tokens,
                        usage_data.completion_tokens,
                    );

                    info!(
                        "Completed request - model: {}, tokens: {}/{}/{}, cost: ${:.4}",
                        completion.model,
                        usage_data.prompt_tokens,
                        usage_data.completion_tokens,
                        usage_data.total_tokens,
                        cost
                    );

                    // TODO: Record usage when authentication is integrated

                    Json(json!(completion)).into_response()
                }
                ChatCompletionResponse::Stream(stream) => {
                    // For streaming, we can't track usage accurately yet
                    // TODO: Implement token counting for streams

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

    // Initialize database service
    info!("🗄️  Initializing database...");
    let db_service = Arc::new(DatabaseService::new(&settings.database.url)?);

    // Run migrations
    info!("📝 Running database migrations...");
    db_service.migrate().await?;

    // Load providers from database
    info!("📦 Loading providers from database...");
    let loaded = reload_providers(&db_service, &settings.auth.jwt_secret).await?;
    info!("  ✓ Loaded {} provider(s)", loaded);

    // Log available models
    let available_models = MODEL_REGISTRY.list_model_names();
    info!("📊 Available models: {}", available_models.join(", "));

    // API routes are ready but need integration with gotcha's handler system
    // For now, the authentication and database services are initialized

    // Start server
    let addr = format!("{}:{}", settings.server.host, settings.server.port);
    info!("🌐 Starting server on http://{}", addr);
    info!("📖 Endpoints:");
    info!("   POST /v1/chat/completions - Chat completions (OpenAI-compatible)");
    info!("   GET  /v1/models           - List available models");
    info!("   GET  /health              - Health check");
    info!("   POST /auth/register       - Register new user");
    info!("   POST /auth/login          - Login");
    info!("   GET  /user/me             - Get current user (auth required)");
    info!("   POST /api-keys            - Create API key (auth required)");
    info!("   GET  /api-keys            - List API keys (auth required)");
    info!("   GET  /usage               - Get usage stats (auth required)");

    // Create API routes with database state
    let api_router = create_router(db_service);

    // Create LLM routes (no state needed, uses global MODEL_REGISTRY)
    let llm_router = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models));

    // Merge all routes
    let app = api_router.merge(llm_router);

    // Start server
    let listener = TcpListener::bind(&addr).await?;
    info!("Server listening on http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
