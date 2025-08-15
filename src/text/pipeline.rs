use std::rc::Rc;

use uuid::Uuid;

use crate::{
    context::context_service::ContextService,
    entities::{
        messages::{AssistantMessage, Message, ToolMessage, UserMessage},
        tools::Tool,
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
    pub top_n_tools: usize,
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

        let tools_with_errors = self.tools_repo.tools().await;
        for error in tools_with_errors.errors {
            self.event_processor
                .process(Event::Error(error.into()))
                .await;
        }

        let tools = tools_with_errors.value;
        let most_relevant_tools = self
            .context_service
            .most_relevant_tools(&user_message, &tools, self.top_n_tools)
            .await
            .unwrap();
        println!();
        println!(
            "TOP::: {:?}",
            most_relevant_tools
                .iter()
                .map(|t| t.name.clone())
                .collect::<Vec<_>>()
        );

        let assistant_message = self
            .generate_new_assistant_message(chat_id, Some(most_relevant_tools.clone()))
            .await?;
        if assistant_message.tool_calls.is_empty() {
            return Ok(assistant_message);
        }
        return self
            .generate_new_assistant_message(chat_id, Some(most_relevant_tools))
            .await;
    }

    pub async fn generate_new_assistant_message(
        &self,
        chat_id: Uuid,
        tools: Option<Vec<Tool>>,
    ) -> anyhow::Result<AssistantMessage> {
        let history = self.context_service.history(chat_id).await?;

        let assistant_message = self.llm.chat(history, tools).await?;
        self.event_processor.process(Event::ResponseTextChunk(assistant_message.content.clone())).await;
        self.event_processor.process(Event::AssistantResponded).await;

        for tool_call in &assistant_message.tool_calls {
            self.event_processor.process(Event::ToolCall(tool_call.clone())).await;
        }

        for tool_call in &assistant_message.tool_calls {
            match self.tools_repo.call_tool(&tool_call).await {
                Ok(tool_call_result) => {
                    let tool_message =
                        ToolMessage::from_call_tool_result(tool_call.id.clone(), tool_call_result);

                    self.event_processor.process(Event::ToolCallResult(tool_message.clone())).await;

                    self.context_service
                        .add_message(chat_id, &Message::Tool(tool_message))
                        .await?;
                }
                Err(e) => {
                    self.event_processor.process(Event::Error(e.into())).await;
                }
            }
        }

        Ok(assistant_message)
    }
}
