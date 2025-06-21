use std::{fs::read_to_string, pin::Pin, sync::Arc};

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent, ChatCompletionResponseStream, CreateChatCompletionRequest
    },
    Client,
};
use rmcp::model::Tool;

use crate::{
    history::{error::Error, history_repo::HistoryRepo},
    tools::tools_repo::ToolsRepo,
};

pub struct LLM {
    client: Client<OpenAIConfig>,
    model: String,
    history_repo: Arc<dyn HistoryRepo>,
    tools_repo: Arc<ToolsRepo>,
}

fn tools_message(tools: &Vec<Tool>) -> String {
    let mut message =
        "\nУ тебя есть следующие инструменты, которые ты можешь использовать, чтобы исполнять запросы пользователя. Ты можешь использовать только их! Больше ничего! Это важно!\n"
            .to_owned();
    for tool in tools {
        message.push_str(&format!(
            "\ntool name: {}\ndescription: {}\nparameters: {}\n",
            tool.name,
            tool.description,
            serde_json::to_string_pretty(&tool.input_schema)
                .expect("failed to serialize tool parameters")
        ))
    }
    message.push_str(
        "\nЕсли тебе нужно воспользоваться инструментом, то верни сообщение в таком формате и больше ничего\n\
            Tool: <tool name>\n\
            Inputs: <inputs>\n",
    );
    message
}

impl LLM {
    pub async fn new(
        api_base: &str,
        api_key: &str,
        model: &str,
        system_prompt: &str,
        history_repo: Arc<dyn HistoryRepo>,
        tools_repo: Arc<ToolsRepo>,
    ) -> Result<Self, Error> {
        let config = OpenAIConfig::new()
            .with_api_base(api_base)
            .with_api_key(api_key);
        let client = Client::with_config(config);

        let tools = tools_repo.tools().await.unwrap();
        let mut system_message = system_prompt.to_owned();
        system_message.push_str(&tools_message(&tools));

        let system_message = ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(system_message),
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

    pub async fn from_env(
        history_repo: Arc<dyn HistoryRepo>,
        tools_repo: Arc<ToolsRepo>,
    ) -> Result<Self, Error> {
        let api_base = std::env::var("OPENAI_API_BASE").expect("OPENAI_API_BASE must be set");
        let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
        let model = std::env::var("OPENAI_MODEL").expect("OPENAI_MODEL must be set");
        let system_prompt_path =
            std::env::var("SYSTEM_PROMPT_PATH").expect("SYSTEM_PROMPT_PATH must be set");
        let system_prompt =
            read_to_string(system_prompt_path).expect("System prompt file does not exist");

        Self::new(
            &api_base,
            &api_key,
            &model,
            &system_prompt,
            history_repo,
            tools_repo,
        )
        .await
    }

    pub async fn get_full_response(&mut self, prompt: &str) -> Result<String, ()> {
        self.history_repo
            .add(&ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessage {
                    content: ChatCompletionRequestUserMessageContent::Text(prompt.to_owned()),
                    name: None,
                },
            ))
            .await
            .map_err(|_| ())?;
        let history = self.history_repo.history().await.map_err(|_| ())?;

        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages: history,
            ..Default::default()
        };

        let response = self.client.chat().create(request).await.map_err(|_| ())?;

        if let Some(choice) = response.choices.first() {
            if let Some(text) = &choice.message.content {
                let message = ChatCompletionRequestMessage::Assistant(
                    ChatCompletionRequestAssistantMessage {
                        content: Some(ChatCompletionRequestAssistantMessageContent::Text(
                            text.to_string(),
                        )),
                        refusal: None,
                        name: None,
                        audio: None,
                        tool_calls: None,
                        function_call: None,
                    },
                );
                self.history_repo.add(&message).await.map_err(|_| ())?;
                return Ok(text.to_string());
            }
            return Err(());
        }
        unreachable!();
    }

    pub async fn get_response_stream(&mut self, prompt: &str) -> Result<ChatCompletionResponseStream, ()> {
        self.history_repo
            .add(&ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessage {
                    content: ChatCompletionRequestUserMessageContent::Text(prompt.to_owned()),
                    name: None,
                },
            ))
            .await
            .map_err(|_| ())?;
        let history = self.history_repo.history().await.map_err(|_| ())?;

        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages: history,
            stream: Some(true),
            ..Default::default()
        };

        Ok(self.client.chat().create_stream(request).await.map_err(|_| ())?)
    }
}
