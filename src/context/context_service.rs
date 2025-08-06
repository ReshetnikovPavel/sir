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
        let system_message = messages::system(&self.system_prompt);
        let chat = self.chat_repo.get_messages(chat_id).await?;
        let chat = self.without_old_tool_calls(chat);
        let mut messages = vec![system_message];
        messages.extend(chat);
        Ok(messages)
    }

    fn without_old_tool_calls(
        &self,
        mut messages: Vec<ChatCompletionRequestMessage>,
    ) -> Vec<ChatCompletionRequestMessage> {
        let latest_tool = messages.iter().rposition(messages::is_tool).unwrap_or(0);

        let user = messages[..latest_tool]
            .iter()
            .rposition(messages::is_user)
            .unwrap_or(0);

        let after_latest_tool = messages.split_off(user);
        messages
            .into_iter()
            .filter(|m| !messages::is_tool(&m) && !messages::is_assistant_with_tool_call(&m))
            .chain(after_latest_tool)
            .collect()
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    SQL(#[from] libsql::Error),
}
