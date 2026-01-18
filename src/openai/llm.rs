use secrecy::ExposeSecret;
use serde::Deserialize;

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
}

impl LargeLanguageModel {
    pub async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<Tool>>,
    ) -> anyhow::Result<AssistantMessage> {
        let url = self.config.api_base.clone() + "/chat/completions";
        let messages = get_messages(&messages);
        let tools = tools.map(|t| get_tools(&t));

        let request = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
        });

        let body = serde_json::to_string(&request)?;
        let response = self
            .client
            .post(url)
            .body(body)
            .bearer_auth(self.config.api_key.expose_secret())
            .send()
            .await?
            .error_for_status()?;

        let response = response.json::<Response>().await?;
        let content = response
            .choices.first()
            .ok_or(anyhow::Error::msg("Chat completion had no choices"))?
            .message
            .content
            .clone();

        let tool_calls = response
            .tool_calls
            .unwrap_or_default()
            .into_iter()
            .filter_map(|tc| {
                Some(ToolCallMessage {
                    id: tc.id,
                    name: tc.function.name,
                    arguments: serde_json::from_str(&tc.function.arguments)
                        .inspect_err(|e| log::error!("{}", e))
                        .ok()?,
                })
            })
            .collect();
        Ok(AssistantMessage {
            content,
            tool_calls,
        })
    }
}

#[derive(Deserialize)]
struct Response {
    choices: Vec<Choice>,
    tool_calls: Option<Vec<ResponseToolCall>>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct ResponseToolCall {
    id: String,
    function: Function,
}

#[derive(Deserialize)]
struct Function {
    name: String,
    arguments: String,
}

fn get_messages(messages: &[Message]) -> serde_json::Value {
    messages
        .iter()
        .map(|m| match m {
            Message::System(s) => serde_json::json!({
                "role": "system",
                "content": s.content,
            }),
            Message::User(u) => serde_json::json!({
                "role": "user",
                "content": u.content,
            }),
            Message::Assistant(a) => serde_json::json!({
                "role": "assistant",
                "content": a.content,
            }),
            Message::Tool(t) => serde_json::json!({
                "role": "tool",
                "content": t.content,
                "tool_call_id": t.tool_call_id,
            }),
        })
        .collect()
}

fn get_tools(tools: &[Tool]) -> serde_json::Value {
    tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "description": t.description,
                    "name": t.name,
                    "parameters": t.parameters
                }
            })
        })
        .collect()
}
