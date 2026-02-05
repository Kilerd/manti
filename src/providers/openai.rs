use super::*;
use crate::models::chat::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, MessageContent,
};
use crate::models::streaming::ChatCompletionChunk;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde_json::json;

pub struct OpenAIProvider {
    config: ProviderConfig,
    client: Client,
    models: Vec<ModelConfig>,
}

impl OpenAIProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let models = vec![
            ModelConfig {
                id: "gpt-4o".to_string(),
                provider: "openai".to_string(),
                input_cost_per_1k: 0.0025,
                output_cost_per_1k: 0.01,
                max_context: 128000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelConfig {
                id: "gpt-4o-mini".to_string(),
                provider: "openai".to_string(),
                input_cost_per_1k: 0.00015,
                output_cost_per_1k: 0.0006,
                max_context: 128000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelConfig {
                id: "gpt-4-turbo".to_string(),
                provider: "openai".to_string(),
                input_cost_per_1k: 0.01,
                output_cost_per_1k: 0.03,
                max_context: 128000,
                supports_tools: true,
                supports_vision: true,
            },
            ModelConfig {
                id: "gpt-4".to_string(),
                provider: "openai".to_string(),
                input_cost_per_1k: 0.03,
                output_cost_per_1k: 0.06,
                max_context: 8192,
                supports_tools: true,
                supports_vision: false,
            },
            ModelConfig {
                id: "gpt-3.5-turbo".to_string(),
                provider: "openai".to_string(),
                input_cost_per_1k: 0.0005,
                output_cost_per_1k: 0.0015,
                max_context: 16385,
                supports_tools: true,
                supports_vision: false,
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
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
    }

    fn convert_messages(&self, messages: Vec<ChatMessage>) -> Vec<serde_json::Value> {
        messages
            .into_iter()
            .map(|msg| {
                let content = match msg.content {
                    MessageContent::Text(text) => json!(text),
                    MessageContent::Parts(parts) => json!(parts),
                };

                let mut message = json!({
                    "role": msg.role,
                    "content": content,
                });

                if let Some(name) = msg.name {
                    message["name"] = json!(name);
                }

                if let Some(tool_calls) = msg.tool_calls {
                    message["tool_calls"] = json!(tool_calls);
                }

                if let Some(tool_call_id) = msg.tool_call_id {
                    message["tool_call_id"] = json!(tool_call_id);
                }

                message
            })
            .collect()
    }
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAI
    }

    fn name(&self) -> &str {
        "openai"
    }

    async fn chat_completions(
        &self,
        request: ChatCompletionRequest,
    ) -> crate::Result<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.get_base_url());

        let messages = self.convert_messages(request.messages);

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": false,
        });

        // Add optional parameters
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }
        if let Some(n) = request.n {
            body["n"] = json!(n);
        }
        if let Some(stop) = request.stop {
            body["stop"] = json!(stop);
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(max_completion_tokens) = request.max_completion_tokens {
            body["max_completion_tokens"] = json!(max_completion_tokens);
        }
        if let Some(presence_penalty) = request.presence_penalty {
            body["presence_penalty"] = json!(presence_penalty);
        }
        if let Some(frequency_penalty) = request.frequency_penalty {
            body["frequency_penalty"] = json!(frequency_penalty);
        }
        if let Some(logit_bias) = request.logit_bias {
            body["logit_bias"] = json!(logit_bias);
        }
        if let Some(user) = request.user {
            body["user"] = json!(user);
        }
        if let Some(seed) = request.seed {
            body["seed"] = json!(seed);
        }
        if let Some(tools) = request.tools {
            body["tools"] = json!(tools);
        }
        if let Some(tool_choice) = request.tool_choice {
            body["tool_choice"] = json!(tool_choice);
        }
        if let Some(response_format) = request.response_format {
            body["response_format"] = json!(response_format);
        }

        let mut req = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_key),
            )
            .header("Content-Type", "application/json");

        if let Some(org) = &self.config.organization {
            req = req.header("OpenAI-Organization", org);
        }

        let response = req
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::MantiError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(crate::MantiError::Provider(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        let result: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| crate::MantiError::Provider(e.to_string()))?;

        Ok(result)
    }

    async fn chat_completions_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> crate::Result<BoxStream<'static, crate::Result<ChatCompletionChunk>>> {
        let url = format!("{}/chat/completions", self.get_base_url());

        let messages = self.convert_messages(request.messages);

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        // Add optional parameters (same as non-streaming)
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }
        if let Some(n) = request.n {
            body["n"] = json!(n);
        }
        if let Some(stop) = request.stop {
            body["stop"] = json!(stop);
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(max_completion_tokens) = request.max_completion_tokens {
            body["max_completion_tokens"] = json!(max_completion_tokens);
        }
        if let Some(presence_penalty) = request.presence_penalty {
            body["presence_penalty"] = json!(presence_penalty);
        }
        if let Some(frequency_penalty) = request.frequency_penalty {
            body["frequency_penalty"] = json!(frequency_penalty);
        }
        if let Some(user) = request.user {
            body["user"] = json!(user);
        }
        if let Some(seed) = request.seed {
            body["seed"] = json!(seed);
        }
        if let Some(tools) = request.tools {
            body["tools"] = json!(tools);
        }
        if let Some(tool_choice) = request.tool_choice {
            body["tool_choice"] = json!(tool_choice);
        }
        if let Some(response_format) = request.response_format {
            body["response_format"] = json!(response_format);
        }

        let mut req = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_key),
            )
            .header("Content-Type", "application/json");

        if let Some(org) = &self.config.organization {
            req = req.header("OpenAI-Organization", org);
        }

        let response = req
            .json(&body)
            .send()
            .await
            .map_err(|e| crate::MantiError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(crate::MantiError::Provider(format!(
                "OpenAI API error: {}",
                error_text
            )));
        }

        // Parse SSE stream with buffering to handle chunks split across boundaries
        let byte_stream = response.bytes_stream();
        let stream = async_stream::stream! {
            let mut buffer = String::new();
            futures::pin_mut!(byte_stream);

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Process complete lines from buffer
                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer[..newline_pos].trim().to_string();
                            buffer = buffer[newline_pos + 1..].to_string();

                            if line.is_empty() {
                                continue;
                            }

                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if data.trim() == "[DONE]" {
                                    continue;
                                }
                                match serde_json::from_str::<ChatCompletionChunk>(data) {
                                    Ok(chunk) => yield Ok(chunk),
                                    Err(e) => {
                                        if !data.trim().is_empty() {
                                            yield Err(crate::MantiError::Provider(e.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(crate::MantiError::Provider(e.to_string()));
                        break;
                    }
                }
            }

            // Process any remaining data in buffer
            if !buffer.trim().is_empty() {
                for line in buffer.lines() {
                    let line = line.trim();
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data.trim() != "[DONE]" && !data.trim().is_empty() {
                            if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data) {
                                yield Ok(chunk);
                            }
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn calculate_cost(&self, model: &str, input_tokens: i64, output_tokens: i64) -> f64 {
        let model_config = self
            .models
            .iter()
            .find(|m| m.id == model)
            .unwrap_or(&self.models[0]);

        let input_cost = (input_tokens as f64 / 1000.0) * model_config.input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * model_config.output_cost_per_1k;

        input_cost + output_cost
    }

    fn supports_model(&self, model: &str) -> bool {
        self.models.iter().any(|m| m.id == model) || model.starts_with("gpt-")
    }

    fn get_models(&self) -> Vec<ModelConfig> {
        self.models.clone()
    }
}
