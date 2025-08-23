use std::rc::Rc;

use async_openai::error::OpenAIError;
use simsimd::{f16, SpatialSimilarity};
use thiserror::Error;

use crate::{
    db::{chat_repo::ChatRepo, id::Id},
    domain::{
        events::{Event, EventEmitter},
        messages::{Message, SystemMessage},
        tools::Tool,
    },
    mcp::tools_repo::McpToolsRepo,
    openai::embedding_model::OpenAIEmbeddingModel,
};

pub struct ContextService {
    chat_repo: Rc<ChatRepo>,
    embedding_model: Rc<OpenAIEmbeddingModel>,
    event_emitter: Rc<EventEmitter>,
    system_prompt: Message,
    top_n_tools: usize,
    tools: Vec<Tool>,
    tool_embeddings: Vec<Vec<simsimd::f16>>,
}

impl ContextService {
    pub async fn new(
        chat_repo: Rc<ChatRepo>,
        tools_repo: Rc<McpToolsRepo>,
        embedding_model: Rc<OpenAIEmbeddingModel>,
        event_emitter: Rc<EventEmitter>,
        system_prompt: String,
        top_n_tools: usize,
    ) -> Result<Self, Error> {
        let result = tools_repo.tools().await;
        let tools = result.value;
        for error in result.errors {
            event_emitter.emit(Event::Error(error.into())).await;
        }

        let tool_texts = tools
            .iter()
            .map(|tool| format!("{}\n{}", tool.name, tool.description))
            .collect();

        let tool_embeddings = embedding_model
            .get_embeddings(tool_texts)
            .await?
            .into_iter()
            .map(|emb| emb.into_iter().map(f16::from_f32).collect())
            .collect();

        Ok(Self {
            chat_repo,
            embedding_model,
            event_emitter,
            system_prompt: Message::System(SystemMessage {
                content: system_prompt,
            }),
            top_n_tools,
            tools,
            tool_embeddings,
        })
    }

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

        let tools = self.most_relevant_tools(messages, self.top_n_tools).await?;

        self.event_emitter
            .emit(Event::FilteredTools(tools.clone()))
            .await;

        Ok(tools)
    }

    async fn most_relevant_tools(
        &self,
        messages: &[Message],
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

        let message_embedding = self
            .embedding_model
            .get_embedding(messages_text)
            .await?
            .into_iter()
            .map(f16::from_f32)
            .collect::<Vec<_>>();

        let mut distancies_with_tools = self
            .tool_embeddings
            .iter()
            .map(|tool_embedding| f16::cos(&message_embedding, tool_embedding).unwrap())
            .zip(self.tools.iter())
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
