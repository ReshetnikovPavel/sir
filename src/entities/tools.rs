use async_openai::types::{ChatCompletionMessageToolCall, ChatCompletionTool, FunctionCall, FunctionObject};
use rmcp::{self, model::CallToolRequestParam};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::entities::json::JsonObject;

#[derive(Clone, Debug)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: JsonObject,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: JsonObject,
}

impl From<rmcp::model::Tool> for Tool {
    fn from(tool: rmcp::model::Tool) -> Self {
        Self {
            name: tool.name.to_string(),
            description: tool.description.to_string(),
            parameters: (*tool.input_schema).clone(),
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
                strict: None
            },
        }
    }
}

impl From<ToolCall> for CallToolRequestParam  {
    fn from(tool_call: ToolCall) -> Self {
        Self {
            name: tool_call.name.into(),
            arguments: tool_call.arguments.into(),
        }
    }
}

impl TryFrom<ChatCompletionMessageToolCall> for ToolCall {
    type Error = serde_json::Error;

    fn try_from(tool_call: ChatCompletionMessageToolCall) -> Result<Self, Self::Error> {
        Ok(Self {
            id: tool_call.id,
            name: tool_call.function.name,
            arguments: serde_json::from_str(&tool_call.function.arguments)?,
        })
    }
}

impl From<ToolCall> for ChatCompletionMessageToolCall {
    fn from(tool_call: ToolCall) -> Self {
        Self {
            id: tool_call.id,
            r#type: async_openai::types::ChatCompletionToolType::Function,
            function: FunctionCall {
                name: tool_call.name,
                arguments: Value::Object(tool_call.arguments).to_string(),
            }
        }
    }
}
