use std::fs::read_to_string;

use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, CreateChatCompletionRequest,
    },
    Client,
};

pub struct LLM {
    client: Client<OpenAIConfig>,
    model: String,
    messages: Vec<ChatCompletionRequestMessage>,
}

impl LLM {
    pub fn new(api_base: &str, api_key: &str, model: &str, system_prompt: &str) -> Self {
        let config = OpenAIConfig::new()
            .with_api_base(api_base)
            .with_api_key(api_key);
        let client = Client::with_config(config);

        Self {
            client,
            model: model.to_owned(),
            messages: vec![ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessage {
                    content: ChatCompletionRequestSystemMessageContent::Text(
                        system_prompt.to_owned(),
                    ),
                    name: None,
                },
            )],
        }
    }

    pub fn from_env() -> Self {
        let api_base = std::env::var("OPENAI_API_BASE").expect("OPENAI_API_BASE must be set");
        let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
        let model = std::env::var("OPENAI_MODEL").expect("OPENAI_MODEL must be set");
        let system_prompt_path =
            std::env::var("SYSTEM_PROMPT_PATH").expect("SYSTEM_PROMPT_PATH must be set");
        let system_prompt =
            read_to_string(system_prompt_path).expect("System prompt file does not exist");

        Self::new(&api_base, &api_key, &model, &system_prompt)
    }

    pub async fn ask(&mut self, prompt: &str) -> Result<Option<String>, OpenAIError> {
        self.messages.push(ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(prompt.to_owned()),
                name: None,
            },
        ));

        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages: self.messages.clone(),
            ..Default::default()
        };

        let response = self.client.chat().create(request).await?;
        if let Some(choice) = response.choices.first() {
            return Ok(choice.message.content.clone());
        }
        unreachable!();
    }
}
