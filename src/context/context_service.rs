use std::rc::Rc;

use async_openai::types::ChatCompletionRequestMessage;
use thiserror::Error;
use uuid::Uuid;

use crate::{chat::messages, db::chat_repo::ChatRepo};

pub struct ContextService {
    pub chat_repo: Rc<ChatRepo>,
    pub system_prompt: String,
}

impl ContextService {
    pub async fn add_message(
        &self,
        chat_id: Uuid,
        message: &ChatCompletionRequestMessage,
    ) -> Result<(), Error> {
        self.chat_repo.add_message(chat_id, message).await?;
        Ok(())
    }

    pub async fn history(&self, chat_id: Uuid) -> Result<Vec<ChatCompletionRequestMessage>, Error> {
        let mut messages = vec![messages::system(&self.system_prompt)];
        messages.extend(self.chat_repo.get_messages(chat_id).await?);
        Ok(messages)
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    SQL(#[from] libsql::Error),
}
