use std::io;

use async_openai::types::{ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage};
use async_trait::async_trait;

#[async_trait]
pub trait HistoryRepo {
    async fn set_system_message(&self, message: ChatCompletionRequestSystemMessage) -> Result<(), io::Error>;
    async fn add(&self, message: &ChatCompletionRequestMessage) -> Result<(), io::Error>;
    async fn history(&self) -> Result<Vec<ChatCompletionRequestMessage>, io::Error>;
}
