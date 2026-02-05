use crate::models::anthropic::AnthropicStreamEvent;
use crate::models::streaming::ChatCompletionChunk;

/// Accumulates token counts from streaming responses
#[derive(Debug, Default)]
pub struct StreamUsageAggregator {
    /// Input/prompt tokens (set from first chunk or message_start)
    pub prompt_tokens: i64,
    /// Output/completion tokens (accumulated or from final event)
    pub completion_tokens: i64,
    /// Accumulated content length for estimation fallback
    pub completion_chars: usize,
    /// Model name captured from stream
    pub model: String,
}

impl StreamUsageAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process an OpenAI chat completion chunk and extract usage if present
    pub fn process_openai_chunk(&mut self, chunk: &ChatCompletionChunk) {
        // Capture model name
        if self.model.is_empty() && !chunk.model.is_empty() {
            self.model = chunk.model.clone();
        }

        // Final chunk may contain usage (when stream_options.include_usage is true)
        if let Some(ref usage) = chunk.usage {
            self.prompt_tokens = usage.prompt_tokens;
            self.completion_tokens = usage.completion_tokens;
        }

        // Accumulate content length for estimation fallback
        for choice in &chunk.choices {
            if let Some(ref content) = choice.delta.content {
                self.completion_chars += content.len();
            }
        }
    }

    /// Process an Anthropic streaming event
    pub fn process_anthropic_event(&mut self, event: &AnthropicStreamEvent) {
        match event {
            AnthropicStreamEvent::MessageStart { message } => {
                self.prompt_tokens = message.usage.input_tokens;
                if self.model.is_empty() {
                    self.model = message.model.clone();
                }
            }
            AnthropicStreamEvent::MessageDelta { usage, .. } => {
                if let Some(usage) = usage {
                    self.completion_tokens = usage.output_tokens;
                }
            }
            AnthropicStreamEvent::ContentBlockDelta { delta, .. } => {
                // Accumulate for estimation fallback
                if let crate::models::anthropic::StreamDelta::TextDelta { text } = delta {
                    self.completion_chars += text.len();
                }
            }
            _ => {}
        }
    }

    /// Estimate completion tokens if not provided by API
    /// Rough estimate: ~4 chars per token for English text
    pub fn estimate_completion_tokens(&self) -> i64 {
        if self.completion_tokens > 0 {
            self.completion_tokens
        } else {
            (self.completion_chars as i64 / 4).max(1)
        }
    }

    /// Get final completion tokens (actual or estimated)
    pub fn final_completion_tokens(&self) -> i64 {
        self.estimate_completion_tokens()
    }

    /// Get total tokens (prompt + completion)
    pub fn total_tokens(&self) -> i64 {
        self.prompt_tokens + self.final_completion_tokens()
    }

    /// Check if we have actual usage data or just estimates
    pub fn has_actual_usage(&self) -> bool {
        self.completion_tokens > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimation_fallback() {
        let mut agg = StreamUsageAggregator::new();
        agg.completion_chars = 400; // ~100 tokens

        assert_eq!(agg.estimate_completion_tokens(), 100);
    }

    #[test]
    fn test_actual_usage_preferred() {
        let mut agg = StreamUsageAggregator::new();
        agg.completion_chars = 400;
        agg.completion_tokens = 50; // Actual value from API

        assert_eq!(agg.estimate_completion_tokens(), 50);
    }
}
