use async_trait::async_trait;

use crate::entities::{messages::{ToolCallMessage, ToolMessage}, tools::Tool};

pub enum Event {
    Error(anyhow::Error),
    ResponseTextChunk(String),
    ToolCall(ToolCallMessage),
    ToolCallResult(ToolMessage),
    RequestedAssistant,
    AssistantResponded,
    StartLoadingTools,
    FinishLoadingTools,
    FilteredTools(Vec<Tool>)
}

#[async_trait]
pub trait EventEmitter {
    async fn emit(&self, event: Event);
}
