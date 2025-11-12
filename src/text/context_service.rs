use std::sync::Arc;

use async_openai::error::OpenAIError;
use serde::Deserialize;
use thiserror::Error;

use crate::{
    db::{chat_repo::ChatRepo, id::Id},
    domain::{events::EventEmitter, messages::Message, tools::Tool},
    rag::tools_rag::ToolsRag,
};

pub struct ContextService {
    pub tools_rag: Arc<ToolsRag>,
    pub chat_repo: Arc<ChatRepo>,
    pub event_emitter: Arc<EventEmitter>,
    pub system_prompt: Message,
    pub options: ContextOptions,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContextOptions {
    pub window: usize,
    pub tools: usize,
}

impl ContextService {
    pub async fn context(&self, chat_id: Id) -> Result<(Vec<Message>, Vec<Tool>), Error> {
        let history = self.history(chat_id).await?;
        let tools = self.tools(&history).await?;

        Ok((history, tools))
    }

    pub async fn history(&self, chat_id: Id) -> Result<Vec<Message>, Error> {
        let mut chat = self.chat_repo.get_messages(chat_id).await?;
        chat = remove_old_tool_messages(chat)
            .into_iter()
            .rev()
            .take(self.options.window)
            .collect::<Vec<_>>();
        chat.reverse();

        let mut messages = vec![self.system_prompt.clone()];
        messages.extend(chat);
        Ok(messages)
    }

    async fn tools(&self, messages: &[Message]) -> Result<Vec<Tool>, Error> {
        let tools = self.tools_rag.tools(messages, self.options.tools).await?;
        Ok(tools)
    }
}

fn remove_old_tool_messages(mut messages: Vec<Message>) -> Vec<Message> {
    let latest_tool = messages.iter().rposition(Message::is_tool).unwrap_or(0);

    let user = messages[..latest_tool]
        .iter()
        .rposition(Message::is_user)
        .unwrap_or(0);

    let after_latest_tool = messages.split_off(user);
    messages
        .into_iter()
        .filter(|m| !Message::is_tool(m) && !Message::is_assistant_with_tool_call(m))
        .chain(after_latest_tool)
        .collect()
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Sql(#[from] libsql::Error),
    #[error(transparent)]
    Mcp(#[from] rmcp::ServiceError),
    #[error(transparent)]
    OpenAI(#[from] OpenAIError),
}
