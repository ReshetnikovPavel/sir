use rmcp::model::{CallToolResult, Content};
use serde::{Deserialize, Serialize};

use crate::domain::json::JsonObject;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "role")]
#[serde(rename_all = "lowercase")]
pub enum Message {
    System(SystemMessage),
    User(UserMessage),
    Assistant(AssistantMessage),
    Tool(ToolMessage),
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
    pub tool_calls: Vec<ToolCallMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolMessage {
    pub content: Vec<Content>,
    pub tool_call_id: String,
    pub is_error: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCallMessage {
    pub id: String,
    pub name: String,
    pub arguments: JsonObject,
}

impl Message {
    pub fn is_system(&self) -> bool {
        matches!(self, Message::System(_))
    }

    pub fn is_user(&self) -> bool {
        matches!(self, Message::User(_))
    }

    pub fn is_assistant(&self) -> bool {
        matches!(self, Message::Assistant(_))
    }

    pub fn is_tool(&self) -> bool {
        matches!(self, Message::Tool(_))
    }

    pub fn is_assistant_with_tool_call(&self) -> bool {
        match self {
            Message::Assistant(assistant) => !assistant.tool_calls.is_empty(),
            _ => false,
        }
    }

    pub fn is_assistant_without_tool_call(&self) -> bool {
        match self {
            Message::Assistant(assistant) => assistant.tool_calls.is_empty(),
            _ => false,
        }
    }
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
