use async_openai::types::{ChatCompletionToolChoiceOption, CreateChatCompletionRequest};
use secrecy::ExposeSecret;

use crate::{
    domain::{
        messages::{AssistantMessage, Message, ToolCallMessage},
        tools::Tool,
    },
    openai::config::OpenAIConfig,
};

#[derive(Clone)]
pub struct LargeLanguageModel {
    pub config: OpenAIConfig,
    pub client: reqwest::Client,
    pub model: String,
}

impl LargeLanguageModel {
    pub async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<Tool>>,
    ) -> anyhow::Result<AssistantMessage> {
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
        let url = self.config.api_base.clone() + "/chat/completions";
        let json = serde_json::to_string(&request)?;
        let response = self
            .client
            .post(url)
            .body(json)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_key.expose_secret()),
            )
            .send()
            .await?
            .error_for_status()?;

        let response = response.json::<serde_json::Value>().await?;

        Ok(response.try_into()?)
    }
}

impl TryFrom<serde_json::Value> for AssistantMessage {
    type Error = JsonParsingError;

    fn try_from(response: serde_json::Value) -> Result<Self, Self::Error> {
        let choices = response
            .get("choices")
            .ok_or(Self::Error::Missing("choices", response.to_string()))?;

        let choice = choices
            .get(0)
            .ok_or(Self::Error::Missing("0", choices.to_string()))?;

        let message = choice
            .get("message")
            .ok_or(Self::Error::Missing("message", choices.to_string()))?;

        let content = message
            .get("content")
            .ok_or(Self::Error::Missing("content", message.to_string()))?;

        let content = content.as_str().ok_or(Self::Error::WrongType(
            "content",
            "string",
            content.to_string(),
        ))?;

        let tool_calls = match message.get("tool_calls") {
            Some(tool_calls) => {
                let tool_calls = tool_calls.as_array().ok_or(Self::Error::WrongType(
                    "tool_calls",
                    "array",
                    tool_calls.to_string(),
                ))?;
                let mut tool_calls_vec = Vec::with_capacity(tool_calls.len());
                for tool_call in tool_calls {
                    tool_calls_vec.push(tool_call.clone().try_into()?);
                }
                tool_calls_vec
            }
            None => vec![],
        };

        Ok(Self {
            content: content.to_string(),
            tool_calls,
        })
    }
}

impl TryFrom<serde_json::Value> for ToolCallMessage {
    type Error = JsonParsingError;

    fn try_from(tool_call: serde_json::Value) -> Result<Self, Self::Error> {
        let id = tool_call
            .get("id")
            .ok_or(Self::Error::Missing("id", tool_call.to_string()))?;
        let id = id
            .as_str()
            .ok_or(Self::Error::WrongType("id", "string", id.to_string()))?;

        let function = tool_call
            .get("function")
            .ok_or(Self::Error::Missing("function", tool_call.to_string()))?;

        let name = function
            .get("name")
            .ok_or(Self::Error::Missing("name", function.to_string()))?;
        let name =
            name.as_str()
                .ok_or(Self::Error::WrongType("name", "string", name.to_string()))?;

        let arguments = function
            .get("arguments")
            .ok_or(Self::Error::Missing("arguments", function.to_string()))?;
        let arguments = arguments.as_str().ok_or(Self::Error::WrongType(
            "arguments",
            "string",
            arguments.to_string(),
        ))?;
        let arguments = serde_json::from_str(arguments)?;

        Ok(Self {
            id: id.to_owned(),
            name: name.to_owned(),
            arguments,
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum JsonParsingError {
    #[error("Missing required index `{0}` in object `{1}`")]
    Missing(&'static str, String),
    #[error("Wrong type for field `{0}`, expected type `{1}`, found value `{2}`")]
    WrongType(&'static str, &'static str, String),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}
