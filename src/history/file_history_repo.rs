use std::{
    fs::{read_to_string, OpenOptions},
    io::{self, Write}, path::PathBuf,
};

use async_openai::types::{ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage};
use async_trait::async_trait;

use super::history_repo::HistoryRepo;

pub struct FileHistoryRepo {
    pub file_path: PathBuf,
}

#[async_trait]
impl HistoryRepo for FileHistoryRepo {
    async fn add(&self, message: &ChatCompletionRequestMessage) -> Result<(), io::Error> {
        let json = serde_json::to_string(message)?;
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(&self.file_path)?;

        writeln!(file, "{}", json)?;
        Ok(())
    }

    async fn history(&self) -> Result<Vec<ChatCompletionRequestMessage>, io::Error> {
        Ok(read_to_string(&self.file_path)?
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect())
    }

    async fn set_system_message(
        &self,
        message: ChatCompletionRequestSystemMessage,
    ) -> Result<(), io::Error> {
        let message = ChatCompletionRequestMessage::System(message);
        let json = serde_json::to_string(&message)?;
        let contents = read_to_string(&self.file_path)?;
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
            .open(&self.file_path)?;
        file.write_all(new_contents.as_bytes())?;

        Ok(())
    }
}
