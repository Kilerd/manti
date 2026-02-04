use super::{Provider, ModelConfig};
use crate::models::chat::ChatCompletionRequest;
use crate::models::response::ChatCompletionResponse;
use std::sync::Arc;

/// A model instance that binds a specific model configuration to a provider
#[derive(Clone)]
pub struct ModelInstance {
    pub name: String,
    pub model_type: String,
    pub provider: Arc<dyn Provider>,
    pub config: ModelConfig,
}

impl ModelInstance {
    pub fn new(
        name: String,
        model_type: String,
        provider: Arc<dyn Provider>,
        config: ModelConfig,
    ) -> Self {
        Self {
            name,
            model_type,
            provider,
            config,
        }
    }

    /// Process a chat completion request
    pub async fn chat_completions(
        &self,
        mut request: ChatCompletionRequest,
    ) -> crate::Result<ChatCompletionResponse> {
        // Replace the model name with the actual model type for the provider
        request.model = self.model_type.clone();

        // Check if streaming is requested
        if request.stream.unwrap_or(false) {
            let stream = self.provider.chat_completions_stream(request).await?;
            Ok(ChatCompletionResponse::Stream(stream))
        } else {
            let response = self.provider.chat_completions(request).await?;
            Ok(ChatCompletionResponse::NonStream(response))
        }
    }

    /// Calculate the cost for this model
    pub fn calculate_cost(&self, input_tokens: i64, output_tokens: i64) -> f64 {
        self.provider.calculate_cost(&self.model_type, input_tokens, output_tokens)
    }
}