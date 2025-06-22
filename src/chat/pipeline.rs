use std::sync::Arc;
use tokio_stream::StreamExt;

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, ChatCompletionTool, CreateChatCompletionRequest,
    },
    Client,
};

use crate::{
    history::{self, history_repo::HistoryRepo},
    tools::{tool::ToolCall, tool_stream_collector::ToolStreamCollector, tools_repo::ToolsRepo},
};

use super::displayer::ChunkDisplayer;

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
        chunk_displayer: Arc<dyn ChunkDisplayer>,
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
        let mut history_collector = vec![];
        while let Some(Ok(response)) = stream.next().await {
            let choice = &response.choices[0];
            // println!("{:?}", choice);
            if let Some(chunk) = &choice.delta.content {
                history_collector.push(chunk.clone());
            }

            chunk_displayer.display_chunk(choice).await;

            let call = tool_stream_collector.add_data(choice).unwrap();
            if let Some(call) = call {
                let call_tool_result = self.tools_repo.call_tool(call).await.unwrap();
                println!("{:?}", call_tool_result);
                for content in call_tool_result.content {
                    if let Some(text_content) = content.as_text() {
                        println!("Результат: `{}`", text_content.text);
                    }
                }
            }
        }
        self.history_repo
            .add(&assistant_message(&history_collector.join("")))
            .await
            .unwrap();
        Ok(())
    }
}

fn user_message(prompt: &str) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
        content: ChatCompletionRequestUserMessageContent::Text(prompt.to_owned()),
        name: None,
    })
}

fn assistant_message(content: &str) -> ChatCompletionRequestMessage {
    ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
        content: Some(ChatCompletionRequestAssistantMessageContent::Text(
            content.to_owned(),
        )),
        refusal: None,
        name: None,
        audio: None,
        tool_calls: None,
        function_call: None,
    })
}
