//! Format conversion utilities between OpenAI and Anthropic API formats

use super::anthropic::{
    AnthropicContent, AnthropicMessage, AnthropicRequest, AnthropicResponse, AnthropicStreamEvent,
    AnthropicTool, AnthropicToolChoice, AnthropicUsage, ContentBlock, ContentBlockStartData,
    MessageDeltaData, MessageStartData, OutputConfig, OutputFormat, StreamDelta, StreamUsage,
    SystemContent, ToolChoiceSpecific,
};
use super::chat::{
    ChatChoice, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ContentPart, ImageUrl,
    MessageContent, ResponseFormat, Tool, ToolCall, ToolCallFunction, ToolChoice,
    ToolChoiceFunction, ToolChoiceObject, ToolFunction, Usage,
};
use super::streaming::{ChatCompletionChunk, ChunkChoice, Delta, DeltaToolCall, DeltaToolCallFunction};
use serde_json::{json, Value};

// ============================================================================
// Anthropic -> OpenAI Conversion
// ============================================================================

/// Convert Anthropic request to OpenAI format
pub fn anthropic_to_openai(request: AnthropicRequest) -> ChatCompletionRequest {
    let mut messages = Vec::new();

    // Convert system content to system message
    if let Some(system) = request.system {
        let system_text = match system {
            SystemContent::Text(text) => text,
            SystemContent::Blocks(blocks) => blocks
                .into_iter()
                .map(|b| b.text)
                .collect::<Vec<_>>()
                .join("\n"),
        };
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: MessageContent::Text(system_text),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // Convert messages
    for msg in request.messages {
        messages.push(anthropic_message_to_openai(msg));
    }

    // Convert tools
    let tools = request.tools.map(|tools| {
        tools
            .into_iter()
            .filter_map(anthropic_tool_to_openai)
            .collect()
    });

    // Convert tool_choice
    let tool_choice = request.tool_choice.map(anthropic_tool_choice_to_openai);

    // Convert response_format from output_config
    let response_format = request.output_config.and_then(|config| match config.format {
        OutputFormat::JsonSchema { schema } => Some(ResponseFormat {
            r#type: "json_schema".to_string(),
            json_schema: Some(schema),
        }),
        OutputFormat::Text => None,
    });

    ChatCompletionRequest {
        model: request.model,
        messages,
        temperature: request.temperature,
        top_p: request.top_p,
        n: None,
        stream: request.stream,
        stop: request.stop_sequences,
        max_tokens: Some(request.max_tokens),
        max_completion_tokens: None,
        presence_penalty: None,
        frequency_penalty: None,
        logit_bias: None,
        user: request.metadata.and_then(|m| m.user_id),
        seed: None,
        tools,
        tool_choice,
        response_format,
    }
}

fn anthropic_message_to_openai(msg: AnthropicMessage) -> ChatMessage {
    let (content, tool_calls, tool_call_id) = match msg.content {
        AnthropicContent::Text(text) => (MessageContent::Text(text), None, None),
        AnthropicContent::Blocks(blocks) => {
            let mut text_parts = Vec::new();
            let mut tool_calls_list = Vec::new();
            let mut tool_result_id = None;

            for block in blocks {
                match block {
                    ContentBlock::Text { text, .. } => {
                        text_parts.push(ContentPart::Text { text });
                    }
                    ContentBlock::Image { source } => {
                        if let Some(data) = source.data {
                            let url = format!("data:{};base64,{}", source.media_type, data);
                            text_parts.push(ContentPart::ImageUrl {
                                image_url: ImageUrl { url, detail: None },
                            });
                        } else if let Some(url) = source.url {
                            text_parts.push(ContentPart::ImageUrl {
                                image_url: ImageUrl { url, detail: None },
                            });
                        }
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        tool_calls_list.push(ToolCall {
                            id,
                            r#type: "function".to_string(),
                            function: ToolCallFunction {
                                name,
                                arguments: input.to_string(),
                            },
                        });
                    }
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } => {
                        tool_result_id = Some(tool_use_id);
                        let result_text = match content {
                            super::anthropic::ToolResultContent::Empty => String::new(),
                            super::anthropic::ToolResultContent::Text(t) => t,
                            super::anthropic::ToolResultContent::Blocks(blocks) => blocks
                                .into_iter()
                                .filter_map(|b| match b {
                                    super::anthropic::ToolResultBlock::Text { text } => Some(text),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join("\n"),
                        };
                        text_parts.push(ContentPart::Text { text: result_text });
                    }
                    ContentBlock::Document { .. } => {
                        // Documents not supported in OpenAI, skip
                    }
                    ContentBlock::Thinking { .. } => {
                        // Thinking blocks not supported in OpenAI, skip
                    }
                }
            }

            let content = if text_parts.len() == 1 {
                match text_parts.into_iter().next().unwrap() {
                    ContentPart::Text { text } => MessageContent::Text(text),
                    other => MessageContent::Parts(vec![other]),
                }
            } else if text_parts.is_empty() {
                MessageContent::Text(String::new())
            } else {
                MessageContent::Parts(text_parts)
            };

            let tool_calls = if tool_calls_list.is_empty() {
                None
            } else {
                Some(tool_calls_list)
            };

            (content, tool_calls, tool_result_id)
        }
    };

    ChatMessage {
        role: msg.role,
        content,
        name: None,
        tool_calls,
        tool_call_id,
    }
}

fn anthropic_tool_to_openai(tool: AnthropicTool) -> Option<Tool> {
    match tool {
        AnthropicTool::ClientTool {
            name,
            description,
            input_schema,
        } => Some(Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name,
                description: Some(description),
                parameters: Some(input_schema),
            },
        }),
        AnthropicTool::ServerTool(_) => {
            // Server tools (like web_search) are Anthropic-specific
            None
        }
    }
}

fn anthropic_tool_choice_to_openai(choice: AnthropicToolChoice) -> ToolChoice {
    match choice {
        AnthropicToolChoice::Auto(s) => ToolChoice::String(s),
        AnthropicToolChoice::Specific(spec) => {
            if let Some(name) = spec.name {
                ToolChoice::Object(ToolChoiceObject {
                    r#type: "function".to_string(),
                    function: ToolChoiceFunction { name },
                })
            } else {
                ToolChoice::String(spec.r#type)
            }
        }
    }
}

// ============================================================================
// OpenAI -> Anthropic Conversion
// ============================================================================

/// Convert OpenAI request to Anthropic format
pub fn openai_to_anthropic(request: ChatCompletionRequest) -> AnthropicRequest {
    let mut system: Option<SystemContent> = None;
    let mut messages = Vec::new();

    for msg in request.messages {
        if msg.role == "system" {
            let text = match msg.content {
                MessageContent::Text(t) => t,
                MessageContent::Parts(parts) => parts
                    .into_iter()
                    .filter_map(|p| match p {
                        ContentPart::Text { text } => Some(text),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            };
            system = Some(SystemContent::Text(text));
        } else {
            messages.push(openai_message_to_anthropic(msg));
        }
    }

    let tools = request.tools.map(|tools| {
        tools
            .into_iter()
            .map(openai_tool_to_anthropic)
            .collect()
    });

    let tool_choice = request.tool_choice.map(openai_tool_choice_to_anthropic);

    let output_config = request.response_format.and_then(|rf| {
        if rf.r#type == "json_schema" {
            rf.json_schema.map(|schema| OutputConfig {
                format: OutputFormat::JsonSchema { schema },
            })
        } else {
            None
        }
    });

    AnthropicRequest {
        model: request.model,
        max_tokens: request.max_tokens.or(request.max_completion_tokens).unwrap_or(4096),
        messages,
        system,
        temperature: request.temperature,
        top_p: request.top_p,
        top_k: None,
        stop_sequences: request.stop,
        stream: request.stream,
        metadata: request.user.map(|u| super::anthropic::Metadata { user_id: Some(u) }),
        service_tier: None,
        thinking: None,
        output_config,
        tools,
        tool_choice,
    }
}

fn openai_message_to_anthropic(msg: ChatMessage) -> AnthropicMessage {
    let mut blocks = Vec::new();

    // Handle content
    match msg.content {
        MessageContent::Text(text) => {
            if !text.is_empty() {
                blocks.push(ContentBlock::Text {
                    text,
                    cache_control: None,
                    citations: None,
                });
            }
        }
        MessageContent::Parts(parts) => {
            for part in parts {
                match part {
                    ContentPart::Text { text } => {
                        blocks.push(ContentBlock::Text {
                            text,
                            cache_control: None,
                            citations: None,
                        });
                    }
                    ContentPart::ImageUrl { image_url } => {
                        let source = if image_url.url.starts_with("data:") {
                            // Parse data URL
                            let parts: Vec<&str> = image_url.url.splitn(2, ',').collect();
                            if parts.len() == 2 {
                                let media_type = parts[0]
                                    .strip_prefix("data:")
                                    .and_then(|s| s.strip_suffix(";base64"))
                                    .unwrap_or("image/jpeg")
                                    .to_string();
                                super::anthropic::ImageSource {
                                    r#type: "base64".to_string(),
                                    media_type,
                                    data: Some(parts[1].to_string()),
                                    url: None,
                                }
                            } else {
                                continue;
                            }
                        } else {
                            super::anthropic::ImageSource {
                                r#type: "url".to_string(),
                                media_type: "image/jpeg".to_string(),
                                data: None,
                                url: Some(image_url.url),
                            }
                        };
                        blocks.push(ContentBlock::Image { source });
                    }
                }
            }
        }
    }

    // Handle tool calls (assistant message)
    if let Some(tool_calls) = msg.tool_calls {
        for tc in tool_calls {
            let input: Value = serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));
            blocks.push(ContentBlock::ToolUse {
                id: tc.id,
                name: tc.function.name,
                input,
            });
        }
    }

    // Handle tool result (tool message)
    if let Some(tool_call_id) = msg.tool_call_id {
        let content_text = match &blocks.first() {
            Some(ContentBlock::Text { text, .. }) => text.clone(),
            _ => String::new(),
        };
        blocks.clear();
        blocks.push(ContentBlock::ToolResult {
            tool_use_id: tool_call_id,
            content: super::anthropic::ToolResultContent::Text(content_text),
            is_error: None,
        });
    }

    let content = if blocks.len() == 1 {
        if let ContentBlock::Text { text, .. } = &blocks[0] {
            AnthropicContent::Text(text.clone())
        } else {
            AnthropicContent::Blocks(blocks)
        }
    } else {
        AnthropicContent::Blocks(blocks)
    };

    AnthropicMessage {
        role: if msg.role == "tool" {
            "user".to_string()
        } else {
            msg.role
        },
        content,
    }
}

fn openai_tool_to_anthropic(tool: Tool) -> AnthropicTool {
    AnthropicTool::ClientTool {
        name: tool.function.name,
        description: tool.function.description.unwrap_or_default(),
        input_schema: tool.function.parameters.unwrap_or(json!({"type": "object"})),
    }
}

fn openai_tool_choice_to_anthropic(choice: ToolChoice) -> AnthropicToolChoice {
    match choice {
        ToolChoice::String(s) => AnthropicToolChoice::Auto(s),
        ToolChoice::Object(obj) => AnthropicToolChoice::Specific(ToolChoiceSpecific {
            r#type: "tool".to_string(),
            name: Some(obj.function.name),
            disable_parallel_tool_use: None,
        }),
    }
}

// ============================================================================
// Response Conversion
// ============================================================================

/// Convert OpenAI response to Anthropic format
pub fn anthropic_response_from_openai(response: ChatCompletionResponse) -> AnthropicResponse {
    let choice = response.choices.into_iter().next();
    let (content, stop_reason) = if let Some(choice) = choice {
        let mut blocks = Vec::new();

        // Convert message content
        match choice.message.content {
            MessageContent::Text(text) => {
                if !text.is_empty() {
                    blocks.push(ContentBlock::Text {
                        text,
                        cache_control: None,
                        citations: None,
                    });
                }
            }
            MessageContent::Parts(parts) => {
                for part in parts {
                    if let ContentPart::Text { text } = part {
                        blocks.push(ContentBlock::Text {
                            text,
                            cache_control: None,
                            citations: None,
                        });
                    }
                }
            }
        }

        // Convert tool calls
        if let Some(tool_calls) = choice.message.tool_calls {
            for tc in tool_calls {
                let input: Value = serde_json::from_str(&tc.function.arguments).unwrap_or(json!({}));
                blocks.push(ContentBlock::ToolUse {
                    id: tc.id,
                    name: tc.function.name,
                    input,
                });
            }
        }

        let stop_reason = choice.finish_reason.map(|r| match r.as_str() {
            "stop" => "end_turn".to_string(),
            "length" => "max_tokens".to_string(),
            "tool_calls" => "tool_use".to_string(),
            other => other.to_string(),
        });

        (blocks, stop_reason)
    } else {
        (Vec::new(), None)
    };

    AnthropicResponse {
        id: response.id,
        r#type: "message".to_string(),
        role: "assistant".to_string(),
        content,
        model: response.model,
        stop_reason,
        stop_sequence: None,
        usage: AnthropicUsage {
            input_tokens: response.usage.prompt_tokens,
            output_tokens: response.usage.completion_tokens,
            cache_creation_input_tokens: None,
            cache_read_input_tokens: None,
            service_tier: None,
        },
    }
}

/// Convert Anthropic response to OpenAI format
pub fn openai_response_from_anthropic(response: AnthropicResponse) -> ChatCompletionResponse {
    let mut text_content = String::new();
    let mut tool_calls = Vec::new();

    for block in response.content {
        match block {
            ContentBlock::Text { text, .. } => {
                text_content.push_str(&text);
            }
            ContentBlock::ToolUse { id, name, input } => {
                tool_calls.push(ToolCall {
                    id,
                    r#type: "function".to_string(),
                    function: ToolCallFunction {
                        name,
                        arguments: input.to_string(),
                    },
                });
            }
            _ => {}
        }
    }

    let finish_reason = response.stop_reason.map(|r| match r.as_str() {
        "end_turn" => "stop".to_string(),
        "max_tokens" => "length".to_string(),
        "tool_use" => "tool_calls".to_string(),
        other => other.to_string(),
    });

    ChatCompletionResponse {
        id: response.id,
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp(),
        model: response.model,
        choices: vec![ChatChoice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: MessageContent::Text(text_content),
                name: None,
                tool_calls: if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls)
                },
                tool_call_id: None,
            },
            finish_reason,
            logprobs: None,
        }],
        usage: Usage {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
        },
        system_fingerprint: None,
    }
}

// ============================================================================
// Streaming Conversion
// ============================================================================

/// State for converting OpenAI stream to Anthropic format
#[derive(Default)]
pub struct StreamConversionState {
    pub message_id: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub content_index: i32,
    pub sent_message_start: bool,
    pub sent_content_start: bool,
    pub current_tool_index: Option<i32>,
}

/// Convert OpenAI chunk to Anthropic stream events
pub fn openai_chunk_to_anthropic_events(
    chunk: ChatCompletionChunk,
    state: &mut StreamConversionState,
) -> Vec<AnthropicStreamEvent> {
    let mut events = Vec::new();

    // Initialize state from chunk
    if state.message_id.is_empty() {
        state.message_id = chunk.id.clone();
    }
    if state.model.is_empty() {
        state.model = chunk.model.clone();
    }

    // Emit message_start if not sent
    if !state.sent_message_start {
        state.sent_message_start = true;
        events.push(AnthropicStreamEvent::MessageStart {
            message: MessageStartData {
                id: state.message_id.clone(),
                r#type: "message".to_string(),
                role: "assistant".to_string(),
                content: vec![],
                model: state.model.clone(),
                stop_reason: None,
                stop_sequence: None,
                usage: StreamUsage {
                    input_tokens: state.input_tokens,
                    output_tokens: 0,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                },
            },
        });
    }

    for choice in chunk.choices {
        // Handle text content
        if let Some(content) = choice.delta.content {
            if !state.sent_content_start {
                state.sent_content_start = true;
                events.push(AnthropicStreamEvent::ContentBlockStart {
                    index: state.content_index,
                    content_block: ContentBlockStartData::Text {
                        text: String::new(),
                    },
                });
            }
            events.push(AnthropicStreamEvent::ContentBlockDelta {
                index: state.content_index,
                delta: StreamDelta::TextDelta { text: content },
            });
        }

        // Handle tool calls
        if let Some(tool_calls) = choice.delta.tool_calls {
            for tc in tool_calls {
                let tool_index = tc.index;

                // Check if this is a new tool call
                if state.current_tool_index != Some(tool_index) {
                    // Close previous content block if needed
                    if state.sent_content_start && state.current_tool_index.is_none() {
                        events.push(AnthropicStreamEvent::ContentBlockStop {
                            index: state.content_index,
                        });
                        state.content_index += 1;
                    } else if let Some(prev_idx) = state.current_tool_index {
                        events.push(AnthropicStreamEvent::ContentBlockStop {
                            index: prev_idx,
                        });
                        state.content_index += 1;
                    }

                    state.current_tool_index = Some(tool_index);

                    // Start new tool_use block
                    if let (Some(id), Some(name)) = (tc.id, tc.function.as_ref().and_then(|f| f.name.clone())) {
                        events.push(AnthropicStreamEvent::ContentBlockStart {
                            index: state.content_index,
                            content_block: ContentBlockStartData::ToolUse {
                                id,
                                name,
                                input: json!({}),
                            },
                        });
                    }
                }

                // Emit argument delta
                if let Some(func) = tc.function {
                    if let Some(args) = func.arguments {
                        events.push(AnthropicStreamEvent::ContentBlockDelta {
                            index: state.content_index,
                            delta: StreamDelta::InputJsonDelta { partial_json: args },
                        });
                    }
                }
            }
        }

        // Handle finish
        if let Some(finish_reason) = choice.finish_reason {
            // Close any open content block
            if state.sent_content_start || state.current_tool_index.is_some() {
                events.push(AnthropicStreamEvent::ContentBlockStop {
                    index: state.content_index,
                });
            }

            let stop_reason = match finish_reason.as_str() {
                "stop" => "end_turn".to_string(),
                "length" => "max_tokens".to_string(),
                "tool_calls" => "tool_use".to_string(),
                other => other.to_string(),
            };

            // Update usage from chunk if available
            if let Some(usage) = &chunk.usage {
                state.input_tokens = usage.prompt_tokens;
                state.output_tokens = usage.completion_tokens;
            }

            events.push(AnthropicStreamEvent::MessageDelta {
                delta: MessageDeltaData {
                    stop_reason: Some(stop_reason),
                    stop_sequence: None,
                },
                usage: Some(StreamUsage {
                    input_tokens: state.input_tokens,
                    output_tokens: state.output_tokens,
                    cache_creation_input_tokens: None,
                    cache_read_input_tokens: None,
                }),
            });

            events.push(AnthropicStreamEvent::MessageStop);
        }
    }

    events
}

/// Convert Anthropic stream event to OpenAI chunk
pub fn anthropic_event_to_openai_chunk(
    event: AnthropicStreamEvent,
    state: &mut StreamConversionState,
) -> Option<ChatCompletionChunk> {
    let now = chrono::Utc::now().timestamp();

    match event {
        AnthropicStreamEvent::MessageStart { message } => {
            state.message_id = message.id;
            state.model = message.model;
            state.input_tokens = message.usage.input_tokens;

            Some(ChatCompletionChunk {
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
            })
        }

        AnthropicStreamEvent::ContentBlockStart { index, content_block } => {
            match content_block {
                ContentBlockStartData::Text { text } if !text.is_empty() => {
                    Some(ChatCompletionChunk {
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
                    })
                }
                ContentBlockStartData::ToolUse { id, name, .. } => {
                    Some(ChatCompletionChunk {
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
                    })
                }
                _ => None,
            }
        }

        AnthropicStreamEvent::ContentBlockDelta { index, delta } => {
            match delta {
                StreamDelta::TextDelta { text } => Some(ChatCompletionChunk {
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
                }),
                StreamDelta::InputJsonDelta { partial_json } => Some(ChatCompletionChunk {
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
                }),
                _ => None,
            }
        }

        AnthropicStreamEvent::MessageDelta { delta, usage } => {
            if let Some(usage) = usage {
                state.output_tokens = usage.output_tokens;
            }

            delta.stop_reason.map(|stop_reason| {
                let finish_reason = match stop_reason.as_str() {
                    "end_turn" => "stop",
                    "max_tokens" => "length",
                    "tool_use" => "tool_calls",
                    other => other,
                };

                ChatCompletionChunk {
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
                }
            })
        }

        _ => None,
    }
}
