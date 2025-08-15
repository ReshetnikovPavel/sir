use std::{collections::HashMap, rc::Rc};

use uuid::Uuid;

use crate::{
    context::context_service::ContextService,
    entities::{
        messages::{AssistantMessage, Message, ToolMessage, UserMessage},
        tools::ToolCall,
    },
    mcp::tools_repo::McpToolsRepo,
    text::{
        events::{Event, EventProcessor},
        openai_llm::OpenAILargeLanguageModel,
    },
};

pub struct TextPipeline {
    pub llm: OpenAILargeLanguageModel,
    pub context_service: ContextService,
    pub tools_repo: Rc<McpToolsRepo>,
    pub event_processor: Rc<dyn EventProcessor>,
}

impl TextPipeline {
    pub async fn answer_prompt(
        &self,
        chat_id: Uuid,
        user_prompt: String,
    ) -> anyhow::Result<AssistantMessage> {
        let user_message = UserMessage {
            content: user_prompt + " /no_think",
        };

        self.context_service
            .add_message(chat_id, &Message::User(user_message.clone()))
            .await?;

        let assistant_message = self.generate_new_assistant_message(chat_id).await?;
        if assistant_message.tool_calls.is_empty() {
            return Ok(assistant_message);
        }
        return self.generate_new_assistant_message(chat_id).await;
    }

    pub async fn generate_new_assistant_message(
        &self,
        chat_id: Uuid,
    ) -> anyhow::Result<AssistantMessage> {
        let (history, tools) = self.context_service.context(chat_id).await?;
        let tools_by_names = tools
            .iter()
            .map(|tool| (tool.name.clone(), tool.clone()))
            .collect::<HashMap<_, _>>();

        let assistant_message = self.llm.chat(history, Some(tools)).await?;
        self.event_processor
            .process(Event::ResponseTextChunk(assistant_message.content.clone()))
            .await;
        self.event_processor
            .process(Event::AssistantResponded)
            .await;

        for tool_call_message in &assistant_message.tool_calls {
            self.event_processor
                .process(Event::ToolCall(tool_call_message.clone()))
                .await;
        }

        let tool_calls = assistant_message
            .tool_calls
            .iter()
            .map(|tool_call_message| {
                let tool = tools_by_names.get(&tool_call_message.name).unwrap();
                ToolCall::from_message_and_server_name(
                    tool_call_message.clone(),
                    tool.server_name.clone(),
                )
            })
            .collect::<Vec<_>>();

        let tool_call_ids = tool_calls
            .iter()
            .map(|tc| tc.id.clone())
            .collect::<Vec<_>>();

        let call_tool_results = self.tools_repo.call_tools(tool_calls).await;

        for (id, res) in tool_call_ids.into_iter().zip(call_tool_results) {
            match res {
                Ok(res) => {
                    let tool_message = ToolMessage::from_call_tool_result(id, res);
                    self.event_processor
                        .process(Event::ToolCallResult(tool_message.clone()))
                        .await;
                    self.context_service
                        .add_message(chat_id, &Message::Tool(tool_message))
                        .await?
                }
                Err(e) => {
                    self.event_processor.process(Event::Error(e.into())).await;
                }
            }
        }
        Ok(assistant_message)
    }
}
