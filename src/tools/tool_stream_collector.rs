use async_openai::types::{ChatChoiceStream, ChatCompletionMessageToolCallChunk, FinishReason};

use crate::tools::tool::JsonObject;

use super::tool::ToolCall;


pub struct ToolStreamCollector {
    name: String,
    arguments: String,
}

impl ToolStreamCollector {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            arguments: String::new(),
        }
    }

    pub fn add_data(&mut self, choice: &ChatChoiceStream) -> Result<Option<ToolCall>, serde_json::Error> {
        if choice.finish_reason == Some(FinishReason::ToolCalls) {
            let arguments = serde_json::from_str::<JsonObject>(&self.arguments)?;
            let result = Ok(Some(ToolCall { name: self.name.clone(), arguments }));
            self.name = String::new();
            self.arguments = String::new();

            return result;
        }

        if let Some(chunks) = &choice.delta.tool_calls {
            for chunk in chunks {
                if let Some(function) = &chunk.function {
                    if let Some(name) = &function.name {
                        self.name.push_str(&name);
                    }
                    if let Some(arguments) = &function.arguments {
                        self.arguments.push_str(&arguments);
                    }
                }
            }
        }
        Ok(None)
    }
}

