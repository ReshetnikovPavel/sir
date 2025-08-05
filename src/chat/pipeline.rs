use std::sync::Arc;
use tokio_stream::StreamExt;

use async_openai::{
    config::OpenAIConfig,
    types::{ChatCompletionTool, ChatCompletionToolChoiceOption, CreateChatCompletionRequest},
    Client,
};
use uuid::Uuid;

use crate::{
    context::context_service::ContextService,
    tools::{tool_stream_collector::ToolStreamCollector, tools_repo::ToolsRepo},
};

use super::{displayer::Displayer, messages};

pub struct TextPipeline {
    pub client: Client<OpenAIConfig>,
    pub model: String,
    pub history_service: ContextService,
    pub tools_repo: ToolsRepo,
}

impl TextPipeline {
    pub async fn process(
        &self,
        chat_id: Uuid,
        prompt: &str,
        displayer: Arc<dyn Displayer>,
    ) -> anyhow::Result<()> {
        self.history_service
            .add_message(chat_id, &messages::user(prompt))
            .await?;

        let tools_with_errors = self.tools_repo.tools().await;
        for error in tools_with_errors.errors {
            log::error!("{}", error)
        }

        let tools = tools_with_errors
            .value
            .iter()
            .map(|t| t.clone().into())
            .collect::<Vec<ChatCompletionTool>>();

        loop {
            let history = self.history_service.history(chat_id).await?;

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
            self.history_service
                .add_message(chat_id, &messages::assistant(content, tool_call_messages))
                .await?;

            for tool_call_result in &tool_call_result_messages {
                self.history_service
                    .add_message(chat_id, &tool_call_result)
                    .await?;
            }

            if tool_call_result_messages.is_empty() {
                break;
            }
        }
        Ok(())
    }
}
