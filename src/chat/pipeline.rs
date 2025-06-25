use rmcp::model::CallToolResult;
use std::sync::Arc;
use tokio_stream::StreamExt;

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessage,
        ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
        ChatCompletionRequestMessageContentPartText, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestSystemMessageContent, ChatCompletionRequestToolMessage,
        ChatCompletionRequestToolMessageContent, ChatCompletionRequestToolMessageContentPart,
        ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
        ChatCompletionTool, CreateChatCompletionRequest,
    },
    Client,
};

use crate::{
    history::{self, history_repo::HistoryRepo},
    tools::{tool_stream_collector::ToolStreamCollector, tools_repo::ToolsRepo},
};

use super::displayer::Displayer;

pub struct Pipeline {
    client: Client<OpenAIConfig>,
    model: String,
    history_repo: Arc<dyn HistoryRepo>,
    tools_repo: Arc<ToolsRepo>,
}

impl Pipeline {
    pub async fn new(
        api_base: &str,
        api_key: &str,
        model: &str,
        system_prompt: &str,
        history_repo: Arc<dyn HistoryRepo>,
        tools_repo: Arc<ToolsRepo>,
    ) -> Result<Self, history::error::Error> {
        let config = OpenAIConfig::new()
            .with_api_base(api_base)
            .with_api_key(api_key);
        let client = Client::with_config(config);
        let system_message = ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(system_prompt.to_owned()),
            name: None,
        };
        history_repo.set_system_message(system_message).await?;

        Ok(Self {
            client,
            model: model.to_owned(),
            history_repo,
            tools_repo,
        })
    }

    pub async fn call(
        &mut self,
        prompt: &str,
        displayer: Arc<dyn Displayer>,
    ) -> Result<(), ()> {
        self.history_repo.add(&user_message(prompt)).await.unwrap();

        let history = self.history_repo.history().await.map_err(|_| ())?;

        let tools = self
            .tools_repo
            .tools()
            .await
            .unwrap()
            .iter()
            .map(|t| t.clone().into())
            .collect::<Vec<ChatCompletionTool>>();

        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages: history,
            stream: Some(true),
            tools: Some(tools),
            ..Default::default()
        };

        let mut stream = self.client.chat().create_stream(request).await.unwrap();

        let mut tool_stream_collector = ToolStreamCollector::new();
        let mut content = String::new();
        let mut tool_call_messages = vec![];
        let mut tool_call_result_messages = vec![];
        while let Some(Ok(response)) = stream.next().await {
            let choice = &response.choices[0];
            // println!("{:?}", choice);
            if let Some(chunk) = &choice.delta.content {
                content.push_str(&chunk);
            }

            displayer.display_chunk(choice).await;

            let call_message = tool_stream_collector.add_data(choice);
            if let Some(call_message) = call_message {
                tool_call_messages.push(call_message.clone());
                let call = call_message.clone().try_into().unwrap();
                let call_tool_result = self.tools_repo.call_tool(call).await.unwrap();
                displayer.display_tool_call_result(&call_tool_result).await;
                tool_call_result_messages.push(call_tool_result_message(
                    &call_message.id,
                    &call_tool_result,
                ));
            }
        }
        self.history_repo
            .add(&assistant_message(content, tool_call_messages))
            .await
            .unwrap();
        for tool_call_result in tool_call_result_messages {
            self.history_repo.add(&tool_call_result).await.unwrap();
        }
        Ok(())
    }
}

fn user_message(prompt: &str) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
        content: ChatCompletionRequestUserMessageContent::Text(prompt.to_owned()),
        name: None,
    })
}

fn assistant_message(
    content: String,
    tool_calls: Vec<ChatCompletionMessageToolCall>,
) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
        content: Some(ChatCompletionRequestAssistantMessageContent::Text(content)),
        refusal: None,
        name: None,
        audio: None,
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        },
        function_call: None,
    })
}

fn call_tool_result_message(id: &str, result: &CallToolResult) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
        content: ChatCompletionRequestToolMessageContent::Array(
            result
                .content
                .iter()
                .map(|c| {
                    ChatCompletionRequestToolMessageContentPart::Text(
                        ChatCompletionRequestMessageContentPartText {
                            text: c.as_text().map(|c| c.text.clone()).unwrap_or("".to_owned()),
                        },
                    )
                })
                .collect(),
        ),
        tool_call_id: id.to_owned(),
    })
}
