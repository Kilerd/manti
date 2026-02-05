use super::*;
use crate::models::chat::{
    ChatChoice, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, MessageContent, Usage,
};
use crate::models::streaming::{
    ChatCompletionChunk, ChunkChoice, Delta, DeltaToolCall, DeltaToolCallFunction,
};
use async_trait::async_trait;
use futures::stream::{self, BoxStream, StreamExt};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};

// ============================================================================
// Anthropic Streaming Event Types
// ============================================================================

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageStartData },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: i32,
        content_block: ContentBlockStartData,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: i32, delta: ContentDelta },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: i32 },
    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDeltaData,
        usage: Option<StreamUsage>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error { error: AnthropicStreamError },
}

#[derive(Deserialize, Debug)]
struct MessageStartData {
    id: String,
    model: String,
    #[allow(dead_code)]
    role: Option<String>,
    usage: Option<StreamUsage>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ContentBlockStartData {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String },
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
}

#[derive(Deserialize, Debug)]
struct MessageDeltaData {
    stop_reason: Option<String>,
    #[allow(dead_code)]
    stop_sequence: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct StreamUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
}

#[derive(Deserialize, Debug)]
struct AnthropicStreamError {
    r#type: String,
    message: String,
}

/// Tracks state during streaming
#[derive(Default)]
struct StreamState {
    message_id: String,
    model: String,
    input_tokens: i64,
    output_tokens: i64,
    tool_call_index: i32,
}

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

    fn convert_to_anthropic_format(&self, request: ChatCompletionRequest) -> Value {
        // Extract system message if present
        let system_message = request
            .messages
            .iter()
            .find(|msg| msg.role == "system")
            .map(|msg| match &msg.content {
                MessageContent::Text(text) => text.clone(),
                MessageContent::Parts(_) => "".to_string(),
            });

        // Filter out system messages and convert to Anthropic format
        let messages: Vec<Value> = request
            .messages
            .into_iter()
            .filter(|msg| msg.role != "system")
            .map(|msg| {
                let content = match msg.content {
                    MessageContent::Text(text) => json!(text),
                    MessageContent::Parts(parts) => {
                        // Convert parts to Anthropic format
                        let anthropic_parts: Vec<Value> =
                            parts.into_iter().map(|part| json!(part)).collect();
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
            let anthropic_tools: Vec<Value> = tools
                .into_iter()
                .map(|tool| {
                    json!({
                        "name": tool.function.name,
                        "description": tool.function.description,
                        "input_schema": tool.function.parameters
                    })
                })
                .collect();
            body["tools"] = json!(anthropic_tools);
        }

        body
    }

    /// Process a single Anthropic stream event and convert to OpenAI-compatible chunks
    fn process_stream_event(
        event: AnthropicStreamEvent,
        state: &mut StreamState,
        first_chunk: &bool,
    ) -> Option<Vec<crate::Result<ChatCompletionChunk>>> {
        let now = chrono::Utc::now().timestamp();

        match event {
            AnthropicStreamEvent::MessageStart { message } => {
                state.message_id = message.id;
                state.model = message.model;
                if let Some(usage) = message.usage {
                    state.input_tokens = usage.input_tokens.unwrap_or(0);
                }

                // Emit initial chunk with role
                if *first_chunk {
                    let chunk = ChatCompletionChunk {
                        id: state.message_id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: now,
                        model: state.model.clone(),
                        choices: vec![ChunkChoice {
                            index: 0,
                            delta: Delta {
                                role: Some("assistant".to_string()),
                                content: None,
                                tool_calls: None,
                            },
                            finish_reason: None,
                            logprobs: None,
                        }],
                        system_fingerprint: None,
                        usage: None,
                    };
                    return Some(vec![Ok(chunk)]);
                }
                None
            }

            AnthropicStreamEvent::ContentBlockStart {
                index,
                content_block,
            } => match content_block {
                ContentBlockStartData::Text { text } => {
                    if !text.is_empty() {
                        let chunk = ChatCompletionChunk {
                            id: state.message_id.clone(),
                            object: "chat.completion.chunk".to_string(),
                            created: now,
                            model: state.model.clone(),
                            choices: vec![ChunkChoice {
                                index: 0,
                                delta: Delta {
                                    role: None,
                                    content: Some(text),
                                    tool_calls: None,
                                },
                                finish_reason: None,
                                logprobs: None,
                            }],
                            system_fingerprint: None,
                            usage: None,
                        };
                        return Some(vec![Ok(chunk)]);
                    }
                    None
                }
                ContentBlockStartData::ToolUse { id, name } => {
                    state.tool_call_index = index;
                    let chunk = ChatCompletionChunk {
                        id: state.message_id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: now,
                        model: state.model.clone(),
                        choices: vec![ChunkChoice {
                            index: 0,
                            delta: Delta {
                                role: None,
                                content: None,
                                tool_calls: Some(vec![DeltaToolCall {
                                    index,
                                    id: Some(id),
                                    r#type: Some("function".to_string()),
                                    function: Some(DeltaToolCallFunction {
                                        name: Some(name),
                                        arguments: None,
                                    }),
                                }]),
                            },
                            finish_reason: None,
                            logprobs: None,
                        }],
                        system_fingerprint: None,
                        usage: None,
                    };
                    Some(vec![Ok(chunk)])
                }
            },

            AnthropicStreamEvent::ContentBlockDelta {
                index,
                delta: content_delta,
            } => match content_delta {
                ContentDelta::TextDelta { text } => {
                    let chunk = ChatCompletionChunk {
                        id: state.message_id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: now,
                        model: state.model.clone(),
                        choices: vec![ChunkChoice {
                            index: 0,
                            delta: Delta {
                                role: None,
                                content: Some(text),
                                tool_calls: None,
                            },
                            finish_reason: None,
                            logprobs: None,
                        }],
                        system_fingerprint: None,
                        usage: None,
                    };
                    Some(vec![Ok(chunk)])
                }
                ContentDelta::InputJsonDelta { partial_json } => {
                    let chunk = ChatCompletionChunk {
                        id: state.message_id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: now,
                        model: state.model.clone(),
                        choices: vec![ChunkChoice {
                            index: 0,
                            delta: Delta {
                                role: None,
                                content: None,
                                tool_calls: Some(vec![DeltaToolCall {
                                    index,
                                    id: None,
                                    r#type: None,
                                    function: Some(DeltaToolCallFunction {
                                        name: None,
                                        arguments: Some(partial_json),
                                    }),
                                }]),
                            },
                            finish_reason: None,
                            logprobs: None,
                        }],
                        system_fingerprint: None,
                        usage: None,
                    };
                    Some(vec![Ok(chunk)])
                }
            },

            AnthropicStreamEvent::ContentBlockStop { .. } => None,

            AnthropicStreamEvent::MessageDelta { delta, usage } => {
                if let Some(usage) = usage {
                    state.output_tokens = usage.output_tokens.unwrap_or(0);
                }

                if let Some(stop_reason) = delta.stop_reason {
                    let finish_reason = match stop_reason.as_str() {
                        "end_turn" => "stop",
                        "tool_use" => "tool_calls",
                        "max_tokens" => "length",
                        "stop_sequence" => "stop",
                        other => other,
                    };

                    let chunk = ChatCompletionChunk {
                        id: state.message_id.clone(),
                        object: "chat.completion.chunk".to_string(),
                        created: now,
                        model: state.model.clone(),
                        choices: vec![ChunkChoice {
                            index: 0,
                            delta: Delta::default(),
                            finish_reason: Some(finish_reason.to_string()),
                            logprobs: None,
                        }],
                        system_fingerprint: None,
                        usage: Some(Usage {
                            prompt_tokens: state.input_tokens,
                            completion_tokens: state.output_tokens,
                            total_tokens: state.input_tokens + state.output_tokens,
                        }),
                    };
                    return Some(vec![Ok(chunk)]);
                }
                None
            }

            AnthropicStreamEvent::MessageStop => None,
            AnthropicStreamEvent::Ping => None,
            AnthropicStreamEvent::Error { error } => {
                Some(vec![Err(crate::MantiError::Provider(format!(
                    "Anthropic stream error: {} - {}",
                    error.r#type, error.message
                )))])
            }
        }
    }

    fn convert_from_anthropic_response(
        &self,
        response: Value,
        model: String,
    ) -> ChatCompletionResponse {
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

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
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
                "Anthropic API error: {}",
                error_text
            )));
        }

        let anthropic_response: Value = response
            .json()
            .await
            .map_err(|e| crate::MantiError::Provider(e.to_string()))?;

        Ok(self.convert_from_anthropic_response(anthropic_response, model))
    }

    async fn chat_completions_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> crate::Result<BoxStream<'static, crate::Result<ChatCompletionChunk>>> {
        let url = format!("{}/messages", self.get_base_url());
        let mut body = self.convert_to_anthropic_format(request);
        body["stream"] = json!(true);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
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
                "Anthropic API error: {}",
                error_text
            )));
        }

        // Parse SSE stream and convert to OpenAI-compatible format
        let stream = response
            .bytes_stream()
            .scan(
                (StreamState::default(), String::new(), true),
                |state, chunk| {
                    let (stream_state, buffer, first_chunk) = state;

                    let chunk = match chunk {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            return std::future::ready(Some(vec![Err(
                                crate::MantiError::Provider(e.to_string()),
                            )]));
                        }
                    };

                    // Append to buffer
                    let text = String::from_utf8_lossy(&chunk);
                    buffer.push_str(&text);

                    let mut results: Vec<crate::Result<ChatCompletionChunk>> = Vec::new();

                    // Parse complete SSE events from buffer
                    while let Some(event_end) = buffer.find("\n\n") {
                        let event_text = buffer[..event_end].to_string();
                        *buffer = buffer[event_end + 2..].to_string();

                        // Parse "data: " line
                        for line in event_text.lines() {
                            if let Some(data) = line.strip_prefix("data: ") {
                                if let Ok(event) =
                                    serde_json::from_str::<AnthropicStreamEvent>(data)
                                {
                                    if let Some(chunks) =
                                        Self::process_stream_event(event, stream_state, first_chunk)
                                    {
                                        *first_chunk = false;
                                        results.extend(chunks);
                                    }
                                }
                            }
                        }
                    }

                    std::future::ready(Some(results))
                },
            )
            .flat_map(stream::iter);

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
        self.models.iter().any(|m| m.id == model) || model.starts_with("claude-")
    }

    fn get_models(&self) -> Vec<ModelConfig> {
        self.models.clone()
    }

    /// Native Anthropic Messages API - direct passthrough without format conversion
    async fn anthropic_messages(
        &self,
        request: crate::models::anthropic::AnthropicRequest,
    ) -> crate::Result<crate::models::anthropic::AnthropicResponse> {
        let url = format!("{}/messages", self.get_base_url());

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| crate::MantiError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(crate::MantiError::Provider(format!(
                "Anthropic API error: {}",
                error_text
            )));
        }

        let anthropic_response: crate::models::anthropic::AnthropicResponse = response
            .json()
            .await
            .map_err(|e| crate::MantiError::Provider(e.to_string()))?;

        Ok(anthropic_response)
    }

    /// Native Anthropic Messages API streaming - direct passthrough
    async fn anthropic_messages_stream(
        &self,
        mut request: crate::models::anthropic::AnthropicRequest,
    ) -> crate::Result<BoxStream<'static, crate::Result<crate::models::anthropic::AnthropicStreamEvent>>> {
        let url = format!("{}/messages", self.get_base_url());
        request.stream = Some(true);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| crate::MantiError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(crate::MantiError::Provider(format!(
                "Anthropic API error: {}",
                error_text
            )));
        }

        // Parse SSE stream and return native Anthropic events
        let stream = response
            .bytes_stream()
            .scan(String::new(), |buffer, chunk| {
                let chunk = match chunk {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        return std::future::ready(Some(vec![Err(
                            crate::MantiError::Provider(e.to_string()),
                        )]));
                    }
                };

                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);

                let mut results: Vec<crate::Result<crate::models::anthropic::AnthropicStreamEvent>> = Vec::new();

                while let Some(event_end) = buffer.find("\n\n") {
                    let event_text = buffer[..event_end].to_string();
                    *buffer = buffer[event_end + 2..].to_string();

                    for line in event_text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if let Ok(event) = serde_json::from_str::<crate::models::anthropic::AnthropicStreamEvent>(data) {
                                results.push(Ok(event));
                            }
                        }
                    }
                }

                std::future::ready(Some(results))
            })
            .flat_map(stream::iter);

        Ok(Box::pin(stream))
    }
}
