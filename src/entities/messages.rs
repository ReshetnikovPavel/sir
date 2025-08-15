use async_openai::types::{
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestToolMessage,
    ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent, CreateChatCompletionResponse,
};
use rmcp::model::{CallToolResult, Content, RawContent, ResourceContents};
use serde::{Deserialize, Serialize};

use crate::entities::tools::ToolCall;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "role")]
#[serde(rename_all = "lowercase")]
pub enum Message {
    System(SystemMessage),
    User(UserMessage),
    Assistant(AssistantMessage),
    Tool(ToolMessage),
}

impl Message {
    pub fn is_system(&self) -> bool {
        match self {
            Message::System(_) => true,
            _ => false,
        }
    }

    pub fn is_user(&self) -> bool {
        match self {
            Message::User(_) => true,
            _ => false,
        }
    }

    pub fn is_assistant(&self) -> bool {
        match self {
            Message::Assistant(_) => true,
            _ => false,
        }
    }

    pub fn is_tool(&self) -> bool {
        match self {
            Message::Tool(_) => true,
            _ => false,
        }
    }

    pub fn is_assistant_with_tool_call(&self) -> bool {
        match self {
            Message::Assistant(assistant) => !assistant.tool_calls.is_empty(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemMessage {
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolMessage {
    pub tool_call_id: String,
    pub content: Vec<Content>,
    pub is_error: bool,
}

impl ToolMessage {
    pub fn from_call_tool_result(id: String, result: CallToolResult) -> Self {
        Self {
            tool_call_id: id,
            content: result.content,
            is_error: result.is_error.is_some_and(|b| b),
        }
    }
}

impl From<Message> for ChatCompletionRequestMessage {
    fn from(message: Message) -> Self {
        match message {
            Message::System(system) => ChatCompletionRequestMessage::System(system.into()),
            Message::User(user) => ChatCompletionRequestMessage::User(user.into()),
            Message::Assistant(assistant) => {
                ChatCompletionRequestMessage::Assistant(assistant.into())
            }
            Message::Tool(tool) => ChatCompletionRequestMessage::Tool(tool.into()),
        }
    }
}

impl From<SystemMessage> for ChatCompletionRequestSystemMessage {
    fn from(message: SystemMessage) -> Self {
        Self {
            content: ChatCompletionRequestSystemMessageContent::Text(message.content),
            name: None,
        }
    }
}

impl From<UserMessage> for ChatCompletionRequestUserMessage {
    fn from(message: UserMessage) -> Self {
        Self {
            content: ChatCompletionRequestUserMessageContent::Text(message.content),
            name: None,
        }
    }
}

impl From<AssistantMessage> for ChatCompletionRequestAssistantMessage {
    fn from(message: AssistantMessage) -> Self {
        Self {
            content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                message.content,
            )),
            tool_calls: Some(
                message
                    .tool_calls
                    .into_iter()
                    .map(|call| call.into())
                    .collect(),
            ),
            ..Default::default()
        }
    }
}

impl From<CreateChatCompletionResponse> for AssistantMessage {
    fn from(message: CreateChatCompletionResponse) -> Self {
        let message = message.choices[0].message.clone();
        Self {
            content: message.content.unwrap_or_default(),
            tool_calls: message
                .tool_calls
                .unwrap_or_default()
                .into_iter()
                .map(|tool_call| tool_call.try_into().unwrap())
                .collect(),
        }
    }
}

impl From<ToolMessage> for ChatCompletionRequestToolMessage {
    fn from(message: ToolMessage) -> Self {
        Self {
            content: ChatCompletionRequestToolMessageContent::Text(
                message
                    .content
                    .into_iter()
                    .map(|c| match c.raw {
                        RawContent::Text(raw_text_content) => raw_text_content.text,
                        RawContent::Image(raw_image_content) => raw_image_content.data,
                        RawContent::Resource(raw_embedded_resource) => {
                            match raw_embedded_resource.resource {
                                ResourceContents::TextResourceContents {
                                    uri: _,
                                    mime_type: _,
                                    text,
                                } => text,
                                ResourceContents::BlobResourceContents {
                                    uri: _,
                                    mime_type: _,
                                    blob,
                                } => blob,
                            }
                        }
                    })
                    .collect::<String>(),
            ),
            tool_call_id: message.tool_call_id,
        }
    }
}
