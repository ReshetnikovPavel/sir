use std::{collections::HashMap, rc::Rc};

use crate::{
    db::{chat_repo::ChatRepo, id::Id},
    domain::{
        events::{Event, EventEmitter},
        messages::{AssistantMessage, Message, ToolMessage, UserMessage},
        states::State,
        tools::ToolCall,
    },
    mcp::tools_repo::McpToolsRepo,
    openai::llm::LargeLanguageModel,
    text::context_service::ContextService,
};

pub struct TextPipeline {
    pub llm: LargeLanguageModel,
    pub context_service: ContextService,
    pub chat_repo: Rc<ChatRepo>,
    pub tools_repo: Rc<McpToolsRepo>,
    pub event_emitter: Rc<EventEmitter>,
}

impl TextPipeline {
    pub async fn answer_prompt(
        &self,
        chat_id: i64,
        user_prompt: String,
    ) -> anyhow::Result<Vec<AssistantMessage>> {
        self.chat_repo
            .add_message(
                chat_id,
                Message::User(UserMessage {
                    content: user_prompt,
                }),
            )
            .await?;

        let mut responses: Vec<AssistantMessage> = vec![];
        loop {
            let not_use_tools = responses
                .last()
                .is_some_and(|latest_message| !latest_message.tool_calls.is_empty());

            let (assistant_message, state) = self
                .generate_new_assistant_message(chat_id, !not_use_tools)
                .await?;

            responses.push(assistant_message);
            match state {
                State::Generate => continue,
                State::Stop => return Ok(responses),
            }
        }
    }

    pub async fn generate_new_assistant_message(
        &self,
        chat_id: Id,
        use_tools: bool,
    ) -> anyhow::Result<(AssistantMessage, State)> {
        let (history, tools) = if use_tools {
            self.context_service.context(chat_id).await?
        } else {
            (self.context_service.history(chat_id).await?, vec![])
        };

        let tools_by_names = tools
            .iter()
            .map(|tool| (tool.name.clone(), tool.clone()))
            .collect::<HashMap<_, _>>();

        self.event_emitter.emit(Event::RequestedAssistant).await;

        let assistant_message = self
            .llm
            .chat(history, if tools.is_empty() { None } else { Some(tools) })
            .await?;

        self.event_emitter
            .emit(Event::ResponseTextChunk(assistant_message.content.clone()))
            .await;

        self.chat_repo
            .add_message(chat_id, Message::Assistant(assistant_message.clone()))
            .await?;

        self.event_emitter.emit(Event::AssistantResponded).await;

        for tool_call_message in &assistant_message.tool_calls {
            self.event_emitter
                .emit(Event::ToolCall(tool_call_message.clone()))
                .await;
        }

        let called_tools = assistant_message
            .tool_calls
            .iter()
            .map(|tool_call_message| tools_by_names.get(&tool_call_message.name).unwrap())
            .collect::<Vec<_>>();

        let tool_calls = assistant_message
            .tool_calls
            .iter()
            .zip(&called_tools)
            .map(|(tool_call_message, tool)| {
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
                    self.event_emitter
                        .emit(Event::ToolCallResult(tool_message.clone()))
                        .await;
                    self.chat_repo
                        .add_message(chat_id, Message::Tool(tool_message))
                        .await?;
                }
                Err(e) => {
                    self.event_emitter.emit(Event::Error(e.into())).await;
                }
            }
        }

        let all_stop = called_tools
            .iter()
            .map(|t| t.on_response)
            .all(|state| state == State::Stop);

        let state = if all_stop {
            State::Stop
        } else {
            State::Generate
        };

        Ok((assistant_message, state))
    }
}
