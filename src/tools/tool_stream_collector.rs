use async_openai::types::{
    ChatChoiceStream, ChatCompletionMessageToolCall, FinishReason, FunctionCall,
};

pub struct ToolStreamCollector {
    id: String,
    name: String,
    arguments: String,
}

impl ToolStreamCollector {
    pub fn new() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            arguments: String::new(),
        }
    }

    pub fn add_data(&mut self, choice: &ChatChoiceStream) -> Option<ChatCompletionMessageToolCall> {
        if choice.finish_reason == Some(FinishReason::ToolCalls) {
            let result = Some(ChatCompletionMessageToolCall {
                id: self.id.clone(),
                r#type: async_openai::types::ChatCompletionToolType::Function,
                function: FunctionCall {
                    name: self.name.clone(),
                    arguments: self.arguments.clone(),
                },
            });
            self.name = String::new();
            self.arguments = String::new();

            return result;
        }

        if let Some(chunks) = &choice.delta.tool_calls {
            for chunk in chunks {
                if let Some(id) = &chunk.id {
                    self.id.push_str(id);
                }
                if let Some(function) = &chunk.function {
                    if let Some(name) = &function.name {
                        self.name.push_str(name);
                    }
                    if let Some(arguments) = &function.arguments {
                        self.arguments.push_str(arguments);
                    }
                }
            }
        }
        None
    }
}
