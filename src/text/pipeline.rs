use std::rc::Rc;

use uuid::Uuid;

use crate::{
    context::context_service::ContextService,
    entities::messages::{AssistantMessage, Message, ToolMessage, UserMessage},
    mcp::tools_repo::McpToolsRepo,
    text::{
        events::{Event, EventProcessor},
        openai_llm::{OpenAILargeLanguageModel, StreamItem},
    },
};

pub struct TextPipeline {
    pub llm: OpenAILargeLanguageModel,
    pub context_service: ContextService,
    pub tools_repo: McpToolsRepo,
    pub event_processor: Rc<dyn EventProcessor>,
}

impl TextPipeline {
    pub async fn answer_prompt(
        &self,
        chat_id: Uuid,
        user_prompt: String,
    ) -> anyhow::Result<AssistantMessage> {
        self.context_service
            .add_message(
                chat_id,
                &Message::User(UserMessage {
                    content: user_prompt,
                }),
            )
            .await?;

        loop {
            let assistant_message = self.generate_new_assistant_message(chat_id).await?;
            if assistant_message.tool_calls.is_empty() {
                return Ok(assistant_message);
            }
        }
    }

    pub async fn generate_new_assistant_message(
        &self,
        chat_id: Uuid,
    ) -> anyhow::Result<AssistantMessage> {
        let tools_with_errors = self.tools_repo.tools().await;
        for error in tools_with_errors.errors {
            self.event_processor
                .process(Event::Error(error.into()))
                .await;
        }

        let tools = tools_with_errors.value;
        let history = self.context_service.history(chat_id).await?;

        let mut stream = self.llm.stream(history, tools).await?;

        let mut assistant_message = AssistantMessage {
            content: String::new(),
            tool_calls: vec![],
        };
        let mut tool_messages = vec![];
        while let Some(message) = stream.next().await {
            match message {
                StreamItem::Content(response) => {
                    assistant_message.content = response;
                }
                StreamItem::ToolCall(tool_call) => {
                    assistant_message.tool_calls.push(tool_call.clone());
                    self.event_processor.process(Event::ToolCall(tool_call.clone())).await;
                    match self.tools_repo.call_tool(&tool_call).await {
                        Ok(tool_call_result) => {
                            let tool_message = ToolMessage::from_call_tool_result(
                                tool_call.id.clone(),
                                tool_call_result,
                            );
                            tool_messages.push(tool_message.clone());
                            self.event_processor.process(Event::ToolCallResult(tool_message)).await;
                        }
                        Err(e) => {
                            self.event_processor.process(Event::Error(e.into())).await;
                        }
                    }
                }
            }
        }

        self.context_service
            .add_message(chat_id, &Message::Assistant(assistant_message.clone()))
            .await?;
        for tool_message in tool_messages {
            self.context_service
                .add_message(chat_id, &Message::Tool(tool_message))
                .await?;
        }

        self.event_processor.process(Event::AssistantResponded).await;
        Ok(assistant_message)
    }
}
