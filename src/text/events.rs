use async_trait::async_trait;

use crate::entities::{messages::ToolMessage, tools::ToolCall};

pub enum Event {
    Error(anyhow::Error),
    ResponseTextChunk(String),
    ToolCall(ToolCall),
    ToolCallResult(ToolMessage),
    AssistantResponded,
}

#[async_trait]
pub trait EventProcessor {
    async fn process(&self, event: Event);
}
