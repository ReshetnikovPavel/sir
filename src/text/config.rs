use std::path::PathBuf;

use serde::Deserialize;

use crate::openai::config::OpenAIConfig;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChatConfig {
    pub llm: OpenAIConfig,
    pub embedding: OpenAIConfig,
    pub system_prompt_path: PathBuf,
}
