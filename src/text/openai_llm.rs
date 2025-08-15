use std::rc::Rc;

use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatChoiceStream, ChatCompletionMessageToolCall, ChatCompletionResponseStream,
        ChatCompletionToolChoiceOption, CreateChatCompletionRequest, FinishReason, FunctionCall,
    },
    Client,
};
use futures::StreamExt as _;

use crate::{
    entities::{
        messages::Message,
        tools::{Tool, ToolCall},
    },
    text::events::{Event, EventProcessor},
};

pub struct OpenAILargeLanguageModel {
    pub client: Client<OpenAIConfig>,
    pub model: String,
    pub event_processor: Rc<dyn EventProcessor>,
}

impl OpenAILargeLanguageModel {
    pub async fn stream(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<Tool>>,
    ) -> Result<Stream, OpenAIError> {
        let messages = messages.into_iter().map(|m| m.into()).collect();
        let tools = tools.map(|tools| tools.into_iter().map(|t| t.into()).collect());

        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages,
            stream: Some(true),
            tools,
            tool_choice: Some(ChatCompletionToolChoiceOption::Auto),
            ..Default::default()
        };
        println!("{:?}", request);

        let stream = self.client.chat().create_stream(request).await?;
        Ok(Stream::new(stream, self.event_processor.clone()))
    }
}

pub struct Stream {
    stream: ChatCompletionResponseStream,
    event_processor: Rc<dyn EventProcessor>,
    content: String,
    tool_collector: ToolStreamCollector,
    is_done: bool,
}

pub enum StreamItem {
    Content(String),
    ToolCall(ToolCall),
}

impl Stream {
    fn new(stream: ChatCompletionResponseStream, event_processor: Rc<dyn EventProcessor>) -> Self {
        Self {
            stream,
            event_processor,
            content: String::new(),
            tool_collector: ToolStreamCollector::new(),
            is_done: false,
        }
    }

    pub async fn next(&mut self) -> Option<StreamItem> {
        if self.is_done {
            return None;
        }

        while let Some(response) = self.stream.next().await {
            match response {
                Ok(response) => {
                    let choice = &response.choices[0];
                    if choice.delta.tool_calls.is_some() {
                        let result = self.get_full_tool_call(choice).await;
                        if result.is_some() {
                            return result;
                        }
                    } else if let Some(content_chunk) = &choice.delta.content {
                        self.event_processor
                            .process(Event::ResponseTextChunk(content_chunk.clone()))
                            .await;
                        self.content.push_str(content_chunk);
                    }
                }
                Err(e) => {
                    self.event_processor.process(Event::Error(e.into())).await;
                }
            }
        }
        self.is_done = true;

        (!self.content.is_empty()).then_some(StreamItem::Content(self.content.clone()))
    }

    async fn get_full_tool_call(&mut self, choice: &ChatChoiceStream) -> Option<StreamItem> {
        if let Some(tool_call) = self.tool_collector.add_data(choice) {
            match tool_call.clone().try_into() {
                Ok(tool_call) => return Some(StreamItem::ToolCall(tool_call)),
                Err(e) => {
                    self.event_processor.process(Event::Error(e.into())).await;
                }
            }
        }
        None
    }
}

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
