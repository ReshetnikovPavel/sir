use async_openai::types::{ChatCompletionTool, FunctionObject};
use rmcp::{self, model::CallToolRequestParam};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{json::JsonObject, messages::ToolCallMessage};

#[derive(Clone, Debug, Serialize, Hash, PartialEq, Eq)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: JsonObject,
    pub server_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: JsonObject,
    pub server_name: String,
}

impl Tool {
    pub fn from_rmcp_and_server_name(tool: rmcp::model::Tool, server_name: String) -> Self {
        Self {
            name: tool.name.to_string(),
            description: tool.description.to_string(),
            parameters: (*tool.input_schema).clone(),
            server_name,
        }
    }
}

impl From<Tool> for ChatCompletionTool {
    fn from(tool: Tool) -> Self {
        Self {
            r#type: async_openai::types::ChatCompletionToolType::Function,
            function: FunctionObject {
                name: tool.name,
                description: Some(tool.description),
                parameters: Some(Value::Object(tool.parameters)),
                strict: None,
            },
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
