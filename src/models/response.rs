use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};

use super::chat::ChatCompletionResponse as ChatCompletion;
use super::streaming::ChatCompletionChunk;

/// Unified response type for chat completions
/// Supports both streaming and non-streaming responses
pub enum ChatCompletionResponse {
    Stream(BoxStream<'static, crate::Result<ChatCompletionChunk>>),
    NonStream(ChatCompletion),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    #[serde(rename = "owned_by")]
    pub owned_by: String,
    pub created: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListResponse {
    pub object: String,
    pub data: Vec<ModelInfo>,
}