use rmcp::{self, model::CallToolRequestParam};
use serde::{Deserialize, Serialize};

use crate::domain::{json::JsonObject, messages::ToolCallMessage, states::State};

#[derive(Clone, Debug, Serialize, Hash, PartialEq, Eq)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: JsonObject,
    pub server_name: String,
    pub on_response: State,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: JsonObject,
    pub server_name: String,
}

impl Tool {
    pub fn new(tool: rmcp::model::Tool, server_name: String, on_response: State) -> Self {
        Self {
            name: tool.name.to_string(),
            description: tool.description.to_string(),
            parameters: (*tool.input_schema).clone(),
            server_name,
            on_response,
        }
    }
}

impl ToolCall {
    pub fn from_message_and_server_name(tool_call: ToolCallMessage, server_name: String) -> Self {
        Self {
            id: tool_call.id,
            name: tool_call.name,
            arguments: tool_call.arguments,
            server_name,
        }
    }
}

impl From<ToolCall> for CallToolRequestParam {
    fn from(tool_call: ToolCall) -> Self {
        Self {
            name: tool_call.name.into(),
            arguments: tool_call.arguments.into(),
        }
    }
}
