use async_openai::types::{ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage};
use async_trait::async_trait;

use super::error::Error;

#[async_trait]
pub trait HistoryRepo {
    async fn set_system_message(&self, message: ChatCompletionRequestSystemMessage) -> Result<(), Error>;
    async fn add(&self, message: &ChatCompletionRequestMessage) -> Result<(), Error>;
    async fn history(&self) -> Result<Vec<ChatCompletionRequestMessage>, Error>;
}
