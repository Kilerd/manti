pub mod openai;
pub mod anthropic;
pub mod registry;
pub mod model_instance;
pub mod model_registry;

use async_trait::async_trait;
use crate::models::chat::{ChatCompletionRequest, ChatCompletionResponse};
use crate::models::streaming::ChatCompletionChunk;
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
    ) -> crate::Result<ChatCompletionResponse>;

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