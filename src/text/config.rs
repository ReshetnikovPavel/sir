use std::path::PathBuf;

use secrecy::SecretString;
use serde::Deserialize;

use crate::config::from_env;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChatConfig {
    pub llm: OpenAIConfig,
    pub embedding: OpenAIConfig,
    pub system_prompt_path: PathBuf
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpenAIConfig {
    pub api_base: String,
    #[serde(deserialize_with = "from_env")]
    pub api_key: SecretString,
    pub model: String,
}
