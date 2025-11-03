use std::sync::Arc;

use async_openai::error::OpenAIError;
use simsimd::SpatialSimilarity as _;

use crate::{
    domain::{messages::Message, tools::Tool},
    openai::embedding_model::EmbeddingModel,
};

pub struct ToolsRag {
    embedding_model: Arc<EmbeddingModel>,
    tools: Vec<Tool>,
    tool_embeddings: Vec<Vec<simsimd::f16>>,
}

impl ToolsRag {
    pub async fn new(
        embedding_model: Arc<EmbeddingModel>,
        tools: Vec<Tool>,
    ) -> Result<Self, OpenAIError> {
        let tool_texts = tools
            .iter()
            .map(|tool| format!("{}\n{}", tool.name, tool.description))
            .collect();

        let tool_embeddings = embedding_model
            .get_embeddings(tool_texts)
            .await?
            .into_iter()
            .map(|emb| emb.into_iter().map(simsimd::f16::from_f32).collect())
            .collect();

        Ok(Self {
            embedding_model,
            tools,
            tool_embeddings,
        })
    }

    pub async fn tools(
        &self,
        messages: &[Message],
        top_n: usize,
    ) -> Result<Vec<Tool>, OpenAIError> {
        let query = into_query(messages);

        let query_embedding = self
            .embedding_model
            .get_embedding(query)
            .await?
            .into_iter()
            .map(simsimd::f16::from_f32)
            .collect::<Vec<_>>();

        let mut distancies_with_tools = self
            .tool_embeddings
            .iter()
            .map(|tool_embedding| simsimd::f16::cos(&query_embedding, tool_embedding).unwrap())
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

fn into_query(messages: &[Message]) -> String {
    let mut messages = messages
        .iter()
        .rev()
        .take(3)
        .take_while(|m| !m.is_tool())
        .filter(|m| m.is_user() || m.is_assistant_without_tool_call())
        .map(|m| match m {
            Message::User(user_message) => format!("User: {}", user_message.content),
            Message::Assistant(assistant_message) => {
                format!("Assistant: {}", assistant_message.content)
            }
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    messages.reverse();

    format!(
        "<query>{}</query><context>{}</context>",
        messages.last().unwrap(),
        messages.join("\n")
    )
}
