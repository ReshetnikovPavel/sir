use std::{
    fs::{read_to_string, OpenOptions},
    io::Write,
};

use async_openai::types::{ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage};
use async_trait::async_trait;

use super::{
    error::{log_map, Error},
    history_repo::HistoryRepo,
};

pub struct FileHistoryRepo {
    pub file_path: String,
}

#[async_trait]
impl HistoryRepo for FileHistoryRepo {
    async fn add(&self, message: &ChatCompletionRequestMessage) -> Result<(), Error> {
        let json = serde_json::to_string(message).map_err(log_map)?;
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(&self.file_path)
            .map_err(log_map)?;

        writeln!(file, "{}", json).map_err(log_map)?;
        Ok(())
    }

    async fn history(&self) -> Result<Vec<ChatCompletionRequestMessage>, Error> {
        Ok(read_to_string(&self.file_path)
            .map_err(log_map)?
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect())
    }

    async fn set_system_message(
        &self,
        message: ChatCompletionRequestSystemMessage,
    ) -> Result<(), Error> {
        let message = ChatCompletionRequestMessage::System(message);
        let json = serde_json::to_string(&message).map_err(log_map)?;
        let contents = read_to_string(&self.file_path).map_err(log_map)?;
        let mut lines: Vec<&str> = contents.lines().collect();
        if !lines.is_empty() {
            lines[0] = &json;
        } else {
            lines.push(&json);
        }

        let mut new_contents = lines.join("\n");
        new_contents.push('\n');
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.file_path)
            .map_err(log_map)?;
        file.write_all(new_contents.as_bytes()).map_err(log_map)?;

        Ok(())
    }
}
