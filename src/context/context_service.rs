use std::rc::Rc;

use simsimd::{f16, SpatialSimilarity};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    context::openai_embedding_model::OpenAIEmbeddingModel,
    db::chat_repo::ChatRepo,
    entities::{
        messages::{self, Message, UserMessage},
        tools::Tool,
    },
    mcp::tools_repo::McpToolsRepo,
};

pub struct ContextService {
    pub chat_repo: Rc<ChatRepo>,
    pub tools_repo: Rc<McpToolsRepo>,
    pub embedding_model: Rc<OpenAIEmbeddingModel>,
    pub system_prompt: messages::SystemMessage,
}

impl ContextService {
    pub async fn add_message(&self, chat_id: Uuid, message: &Message) -> Result<(), Error> {
        self.chat_repo.add_message(chat_id, message).await?;
        Ok(())
    }

    pub async fn history(&self, chat_id: Uuid) -> Result<Vec<Message>, Error> {
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
            .filter(|m| !Message::is_tool(&m) && !Message::is_assistant_with_tool_call(&m))
            .chain(after_latest_tool)
            .collect()
    }

    pub async fn most_relevant_tools(
        &self,
        message: &UserMessage,
        tools: &[Tool],
        top_n: usize,
    ) -> anyhow::Result<Vec<Tool>> {
        let mut texts = vec![message.content.clone()];
        let tool_texts = tools
            .iter()
            .map(|tool| format!("{}\n{}", tool.name, tool.description));
        texts.extend(tool_texts);

        let mut embeddings = self
            .embedding_model
            .get_embeddings(texts)
            .await?
            .into_iter()
            .map(|embedding| embedding.into_iter().map(|x| f16::from_f32(x)).collect());

        let message_embedding = embeddings.next().unwrap();
        let tool_embeddings = embeddings.collect::<Vec<Vec<f16>>>();

        let mut distancies_with_tools = tool_embeddings
            .into_iter()
            .map(|tool_embedding| f16::cos(&message_embedding, &tool_embedding).unwrap())
            .zip(tools)
            .collect::<Vec<_>>();

        distancies_with_tools
            .sort_unstable_by(|(distance, _), (other, _)| distance.total_cmp(other));
        // println!("{:?}", distancies_with_tools.iter().map(|(d, t)| (d, t.name.clone())).collect::<Vec<_>>());

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
    SQL(#[from] libsql::Error),
}
