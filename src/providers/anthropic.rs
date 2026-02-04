use super::*;
use async_trait::async_trait;
use crate::models::chat::{ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ChatChoice, Usage, MessageContent};
use crate::models::streaming::ChatCompletionChunk;
use futures::stream::BoxStream;
use reqwest::Client;
use serde_json::{json, Value};

pub struct AnthropicProvider {
    config: ProviderConfig,
    client: Client,
    models: Vec<ModelConfig>,
}

impl AnthropicProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let models = vec![
            ModelConfig {
                id: "claude-3-5-sonnet-20241022".to_string(),
                provider: "anthropic".to_string(),
                input_cost_per_1k: 0.003,
                output_cost_per_1k: 0.015,
                max_context: 200000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelConfig {
                id: "claude-3-5-haiku-20241022".to_string(),
                provider: "anthropic".to_string(),
                input_cost_per_1k: 0.001,
                output_cost_per_1k: 0.005,
                max_context: 200000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelConfig {
                id: "claude-3-opus-20240229".to_string(),
                provider: "anthropic".to_string(),
                input_cost_per_1k: 0.015,
                output_cost_per_1k: 0.075,
                max_context: 200000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelConfig {
                id: "claude-3-sonnet-20240229".to_string(),
                provider: "anthropic".to_string(),
                input_cost_per_1k: 0.003,
                output_cost_per_1k: 0.015,
                max_context: 200000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelConfig {
                id: "claude-3-haiku-20240307".to_string(),
                provider: "anthropic".to_string(),
                input_cost_per_1k: 0.00025,
                output_cost_per_1k: 0.00125,
                max_context: 200000,
                supports_tools: true,
                supports_vision: true,
            },
        ];

        Self {
            config,
            client: Client::new(),
            models,
        }
    }

    fn get_base_url(&self) -> String {
        self.config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com/v1".to_string())
    }

    fn convert_to_anthropic_format(&self, mut request: ChatCompletionRequest) -> Value {
        // Extract system message if present
        let system_message = request.messages
            .iter()
            .find(|msg| msg.role == "system")
            .map(|msg| match &msg.content {
                MessageContent::Text(text) => text.clone(),
                MessageContent::Parts(_) => "".to_string(),
            });

        // Filter out system messages and convert to Anthropic format
        let messages: Vec<Value> = request.messages
            .into_iter()
            .filter(|msg| msg.role != "system")
            .map(|msg| {
                let content = match msg.content {
                    MessageContent::Text(text) => json!(text),
                    MessageContent::Parts(parts) => {
                        // Convert parts to Anthropic format
                        let anthropic_parts: Vec<Value> = parts.into_iter().map(|part| {
                            json!(part)
                        }).collect();
                        json!(anthropic_parts)
                    }
                };

                json!({
                    "role": if msg.role == "assistant" { "assistant" } else { "user" },
                    "content": content
                })
            })
            .collect();

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.or(request.max_completion_tokens).unwrap_or(4096),
        });

        if let Some(system) = system_message {
            body["system"] = json!(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }

        if let Some(stop) = request.stop {
            body["stop_sequences"] = json!(stop);
        }

        if let Some(tools) = request.tools {
            // Convert OpenAI tools format to Anthropic format
            let anthropic_tools: Vec<Value> = tools.into_iter().map(|tool| {
                json!({
                    "name": tool.function.name,
                    "description": tool.function.description,
                    "input_schema": tool.function.parameters
                })
            }).collect();
            body["tools"] = json!(anthropic_tools);
        }

        body
    }

    fn convert_from_anthropic_response(&self, response: Value, model: String) -> ChatCompletionResponse {
        let id = response["id"].as_str().unwrap_or("").to_string();
        let created = chrono::Utc::now().timestamp();

        // Convert Anthropic content to our format
        let content = if let Some(content_array) = response["content"].as_array() {
            if let Some(first) = content_array.first() {
                first["text"].as_str().unwrap_or("").to_string()
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };

        ChatCompletionResponse {
            id,
            object: "chat.completion".to_string(),
            created,
            model,
            choices: vec![ChatChoice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: MessageContent::Text(content),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: response["stop_reason"].as_str().map(|s| s.to_string()),
                logprobs: None,
            }],
            usage: Usage {
                prompt_tokens: response["usage"]["input_tokens"].as_i64().unwrap_or(0),
                completion_tokens: response["usage"]["output_tokens"].as_i64().unwrap_or(0),
                total_tokens: response["usage"]["input_tokens"].as_i64().unwrap_or(0)
                    + response["usage"]["output_tokens"].as_i64().unwrap_or(0),
            },
            system_fingerprint: None,
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
    }

    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat_completions(
        &self,
        request: ChatCompletionRequest,
    ) -> crate::Result<ChatCompletionResponse> {
        let url = format!("{}/messages", self.get_base_url());
        let model = request.model.clone();
        let body = self.convert_to_anthropic_format(request);

        let response = self.client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::MantiError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(crate::MantiError::Provider(format!("Anthropic API error: {}", error_text)));
        }

        let anthropic_response: Value = response.json().await
            .map_err(|e| crate::MantiError::Provider(e.to_string()))?;

        Ok(self.convert_from_anthropic_response(anthropic_response, model))
    }

    async fn chat_completions_stream(
        &self,
        _request: ChatCompletionRequest,
    ) -> crate::Result<BoxStream<'static, crate::Result<ChatCompletionChunk>>> {
        // Streaming implementation would require SSE parsing similar to OpenAI
        // For now, returning an error
        Err(crate::MantiError::Provider(
            "Streaming not yet implemented for Anthropic".to_string()
        ))
    }

    fn calculate_cost(&self, model: &str, input_tokens: i64, output_tokens: i64) -> f64 {
        let model_config = self.models
            .iter()
            .find(|m| m.id == model)
            .unwrap_or(&self.models[0]);

        let input_cost = (input_tokens as f64 / 1000.0) * model_config.input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * model_config.output_cost_per_1k;

        input_cost + output_cost
    }

    fn supports_model(&self, model: &str) -> bool {
        self.models.iter().any(|m| m.id == model) || model.starts_with("claude-")
    }

    fn get_models(&self) -> Vec<ModelConfig> {
        self.models.clone()
    }
}