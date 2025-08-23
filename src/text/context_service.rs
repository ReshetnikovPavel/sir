use std::rc::Rc;

use async_openai::error::OpenAIError;
use thiserror::Error;

use crate::{
    db::{chat_repo::ChatRepo, id::Id},
    domain::{
        events::{Event, EventEmitter},
        messages::Message,
        tools::Tool,
    },
    rag::tools_rag::ToolsRag,
};

pub struct ContextService {
    pub tools_rag: ToolsRag,
    pub chat_repo: Rc<ChatRepo>,
    pub event_emitter: Rc<EventEmitter>,
    pub system_prompt: Message,
    pub top_n_tools: usize,
}

impl ContextService {
    pub async fn context(&self, chat_id: Id) -> Result<(Vec<Message>, Vec<Tool>), Error> {
        let history = self.history(chat_id).await?;
        let tools = self.tools(&history).await?;

        Ok((history, tools))
    }

    pub async fn history(&self, chat_id: Id) -> Result<Vec<Message>, Error> {
        let system_message = self.system_prompt.clone();
        let chat = self.chat_repo.get_messages(chat_id).await?;
        let chat = self.without_old_tool_calls(chat);
        let mut messages = vec![system_message];
        messages.extend(chat);
        Ok(messages)
    }

    fn without_old_tool_calls(&self, mut messages: Vec<Message>) -> Vec<Message> {
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

    async fn tools(&self, messages: &[Message]) -> Result<Vec<Tool>, Error> {
        self.event_emitter.emit(Event::StartFliteringTools).await;

        let tools = self.tools_rag.tools(messages, self.top_n_tools).await?;

        self.event_emitter
            .emit(Event::FilteredTools(tools.clone()))
            .await;

        Ok(tools)
    }
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
