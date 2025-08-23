use std::rc::Rc;

use async_openai::error::OpenAIError;
use simsimd::{f16, SpatialSimilarity};
use thiserror::Error;

use crate::{
    db::{chat_repo::ChatRepo, id::Id},
    domain::{
        events::{Event, EventEmitter},
        messages::{self, Message},
        tools::Tool,
    },
    mcp::tools_repo::McpToolsRepo,
    openai::embedding_model::OpenAIEmbeddingModel,
};

pub struct ContextService {
    pub chat_repo: Rc<ChatRepo>,
    pub tools_repo: Rc<McpToolsRepo>,
    pub embedding_model: Rc<OpenAIEmbeddingModel>,
    pub event_emitter: Rc<EventEmitter>,
    pub system_prompt: messages::SystemMessage,
    pub top_n_tools: usize,
}

impl ContextService {
    pub async fn context(&self, chat_id: Id) -> Result<(Vec<Message>, Vec<Tool>), Error> {
        let history = self.history(chat_id).await?;
        let tools = self.tools(&history).await?;

        Ok((history, tools))
    }

    pub async fn history(&self, chat_id: Id) -> Result<Vec<Message>, Error> {
        let system_message = Message::System(self.system_prompt.clone());
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

        let get_tools_result = self.tools_repo.tools().await;
        let tools = get_tools_result.value;

        for error in get_tools_result.errors {
            self.event_emitter.emit(Event::Error(error.into())).await;
        }

        let tools = self
            .most_relevant_tools(messages, &tools, self.top_n_tools)
            .await?;

        self.event_emitter
            .emit(Event::FilteredTools(tools.clone()))
            .await;

        Ok(tools)
    }

    async fn most_relevant_tools(
        &self,
        messages: &[Message],
        tools: &[Tool],
        top_n: usize,
    ) -> Result<Vec<Tool>, Error> {
        let messages_text = messages[messages.len().saturating_sub(3)..]
            .iter()
            .filter(|m| m.is_user() || m.is_assistant_without_tool_call())
            .map(|m| match m {
                Message::User(user_message) => user_message.content.clone(),
                Message::Assistant(assistant_message) => assistant_message.content.clone(),
                _ => unreachable!(),
            } + "\n")
            .collect::<String>();

        let mut texts = vec![messages_text];

        let tool_texts = tools
            .iter()
            .map(|tool| format!("{}\n{}", tool.name, tool.description));
        texts.extend(tool_texts);

        let mut embeddings = self
            .embedding_model
            .get_embeddings(texts)
            .await?
            .into_iter()
            .map(|embedding| embedding.into_iter().map(f16::from_f32).collect());

        let message_embedding = embeddings.next().unwrap();
        let tool_embeddings = embeddings.collect::<Vec<Vec<f16>>>();

        let mut distancies_with_tools = tool_embeddings
            .into_iter()
            .map(|tool_embedding| f16::cos(&message_embedding, &tool_embedding).unwrap())
            .zip(tools)
            .collect::<Vec<_>>();

        distancies_with_tools
            .sort_unstable_by(|(distance, _), (other, _)| distance.total_cmp(other));

        let tools = distancies_with_tools
            .into_iter()
            .map(|(_, tool)| tool.clone())
            .take(top_n)
            .collect::<Vec<_>>();

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
