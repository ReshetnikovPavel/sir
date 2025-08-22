use tokio::sync::mpsc::Sender;

use crate::entities::{
    messages::{ToolCallMessage, ToolMessage},
    tools::Tool,
};

pub enum Event {
    Error(anyhow::Error),
    ResponseTextChunk(String),
    ToolCall(ToolCallMessage),
    ToolCallResult(ToolMessage),
    RequestedAssistant,
    AssistantResponded,
    StartLoadingTools,
    FinishLoadingTools,
    FilteredTools(Vec<Tool>),
}

pub struct EventEmitter {
    pub tx: Sender<Event>,
}

impl EventEmitter {
    pub async fn emit(&self, event: Event) {
        if let Err(e) = self.tx.send(event).await {
            log::error!("{:#?}", e)
        }
    }
}
