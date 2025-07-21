use std::{io, sync::Arc};
use tokio_stream::StreamExt;

use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionTool, ChatCompletionToolChoiceOption, CreateChatCompletionRequest},
    Client,
};

use crate::{
    history::history_repo::HistoryRepo,
    tools::{tool_stream_collector::ToolStreamCollector, tools_repo::ToolsRepo},
};

use super::{displayer::Displayer, messages};

pub struct TextPipeline {
    client: Client<OpenAIConfig>,
    model: String,
    history_repo: Arc<dyn HistoryRepo>,
    tools_repo: Arc<ToolsRepo>,
}

impl TextPipeline {
    pub async fn new(
        api_base: &str,
        api_key: &str,
        model: &str,
        system_prompt: &str,
        history_repo: Arc<dyn HistoryRepo>,
        tools_repo: Arc<ToolsRepo>,
    ) -> Result<Self, io::Error> {
        let config = OpenAIConfig::new()
            .with_api_base(api_base)
            .with_api_key(api_key);
        let client = Client::with_config(config);
        let system_message = messages::system(system_prompt);
        history_repo.set_system_message(system_message).await?;

        Ok(Self {
            client,
            model: model.to_owned(),
            history_repo,
            tools_repo,
        })
    }

    pub async fn process(&self, prompt: &str, displayer: Arc<dyn Displayer>) -> anyhow::Result<()> {
        self.history_repo.add(&messages::user(prompt)).await?;

        let tools = self
            .tools_repo
            .tools()
            .await
            .unwrap()
            .iter()
            .map(|t| t.clone().into())
            .collect::<Vec<ChatCompletionTool>>();

        loop {
            let history = self.history_repo.history().await?;

            let request = CreateChatCompletionRequest {
                model: self.model.clone(),
                messages: history,
                stream: Some(true),
                tools: Some(tools.clone()),
                tool_choice: Some(ChatCompletionToolChoiceOption::Auto),
                ..Default::default()
            };

            let mut stream = self.client.chat().create_stream(request).await?;

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
                    let call = call_message.clone().try_into()?;
                    let call_tool_result = self
                        .tools_repo
                        .call_tool(&call)
                        .await
                        .expect(&format!("{:?}", &call));
                    displayer.display_tool_call_result(&call_tool_result).await;
                    tool_call_result_messages.push(messages::call_tool_result(
                        &call_message.id,
                        &call_tool_result,
                    ));
                }
            }
            self.history_repo
                .add(&messages::assistant(content, tool_call_messages))
                .await?;

            for tool_call_result in &tool_call_result_messages {
                self.history_repo.add(&tool_call_result).await?;
            }

            if tool_call_result_messages.is_empty() {
                break;
            }
        }
        Ok(())
    }
}
