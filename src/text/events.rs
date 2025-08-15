use async_trait::async_trait;

use crate::entities::messages::{ToolCallMessage, ToolMessage};

pub enum Event {
    Error(anyhow::Error),
    ResponseTextChunk(String),
    ToolCall(ToolCallMessage),
    ToolCallResult(ToolMessage),
    AssistantResponded,
}

#[async_trait]
pub trait EventProcessor {
    async fn process(&self, event: Event);
}
