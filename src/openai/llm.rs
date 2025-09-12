use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{ChatCompletionToolChoiceOption, CreateChatCompletionRequest},
    Client,
};

use crate::domain::{
    messages::{AssistantMessage, Message},
    tools::Tool,
};

pub struct LargeLanguageModel {
    pub client: Client<OpenAIConfig>,
    pub model: String,
}

impl LargeLanguageModel {
    pub async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<Tool>>,
    ) -> Result<AssistantMessage, OpenAIError> {
        let messages = messages.into_iter().map(|m| m.into()).collect();
        let tools = tools.map(|tools| tools.into_iter().map(|t| t.into()).collect());

        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages,
            stream: None,
            tools,
            tool_choice: Some(ChatCompletionToolChoiceOption::Auto),
            ..Default::default()
        };

        Ok(self.client.chat().create(request).await?.into())
    }
}
