pub mod openai;
pub mod anthropic;
pub mod registry;
pub mod model_instance;
pub mod model_registry;

use async_trait::async_trait;
use crate::models::chat::{ChatCompletionRequest, ChatCompletionResponse as ChatCompletion};
use crate::models::streaming::ChatCompletionChunk;
use crate::models::anthropic::{AnthropicRequest, AnthropicResponse, AnthropicStreamEvent};
use futures::stream::BoxStream;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Google,
    Azure,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,
    pub api_key: String,
    pub base_url: Option<String>,
    pub organization: Option<String>,
    pub extra_params: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub id: String,
    pub provider: String,
    pub input_cost_per_1k: f64,  // in USD
    pub output_cost_per_1k: f64, // in USD
    pub max_context: i32,
    pub supports_tools: bool,
    pub supports_vision: bool,
}

#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider type
    fn provider_type(&self) -> ProviderType;

    /// Get the provider name
    fn name(&self) -> &str;

    /// Process chat completion request
    async fn chat_completions(
        &self,
        request: ChatCompletionRequest,
    ) -> crate::Result<ChatCompletion>;

    /// Process streaming chat completion request
    async fn chat_completions_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> crate::Result<BoxStream<'static, crate::Result<ChatCompletionChunk>>>;

    /// Calculate cost for the request
    fn calculate_cost(&self, model: &str, input_tokens: i64, output_tokens: i64) -> f64;

    /// Check if model is supported
    fn supports_model(&self, model: &str) -> bool;

    /// Get available models
    fn get_models(&self) -> Vec<ModelConfig>;

    /// Process Anthropic Messages API request (native format)
    /// Default implementation converts to OpenAI format and calls chat_completions
    async fn anthropic_messages(
        &self,
        request: AnthropicRequest,
    ) -> crate::Result<AnthropicResponse> {
        use crate::models::conversion::{anthropic_to_openai, anthropic_response_from_openai};
        let mut openai_request = anthropic_to_openai(request);
        openai_request.stream = Some(false);
        let openai_response = self.chat_completions(openai_request).await?;
        Ok(anthropic_response_from_openai(openai_response))
    }

    /// Process Anthropic Messages API streaming request (native format)
    /// Default implementation converts to OpenAI format and wraps the stream
    async fn anthropic_messages_stream(
        &self,
        request: AnthropicRequest,
    ) -> crate::Result<BoxStream<'static, crate::Result<AnthropicStreamEvent>>> {
        use crate::models::conversion::{anthropic_to_openai, openai_chunk_to_anthropic_events, StreamConversionState};
        use futures::StreamExt;

        let openai_request = anthropic_to_openai(request);
        let openai_stream = self.chat_completions_stream(openai_request).await?;

        let converted_stream = openai_stream
            .scan(StreamConversionState::default(), |state, chunk_result| {
                let events = match chunk_result {
                    Ok(chunk) => openai_chunk_to_anthropic_events(chunk, state)
                        .into_iter()
                        .map(Ok)
                        .collect::<Vec<_>>(),
                    Err(e) => vec![Err(e)],
                };
                std::future::ready(Some(events))
            })
            .flat_map(futures::stream::iter);

        Ok(Box::pin(converted_stream))
    }
}

/// Provider factory
pub struct ProviderFactory;

impl ProviderFactory {
    pub fn create(config: ProviderConfig) -> crate::Result<Arc<dyn Provider>> {
        match config.provider_type {
            ProviderType::OpenAI => {
                Ok(Arc::new(openai::OpenAIProvider::new(config)))
            }
            ProviderType::Anthropic => {
                Ok(Arc::new(anthropic::AnthropicProvider::new(config)))
            }
            _ => Err(crate::MantiError::Provider(
                format!("Provider type {:?} not implemented", config.provider_type)
            )),
        }
    }
}