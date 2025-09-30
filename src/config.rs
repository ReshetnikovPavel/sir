use std::path::Path;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Deserializer};

use crate::{
    audio::config::AudioConfig,
    mcp::config::McpConfig,
    text::{config::ChatConfig, context_service::ContextOptions},
};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(flatten)]
    pub audio: AudioConfig,
    #[serde(flatten)]
    pub chat: ChatConfig,
    #[serde(flatten)]
    pub mcp: McpConfig,
    pub context: ContextOptions,
}

impl Config {
    pub async fn load<T: AsRef<Path>>(path: T) -> Self {
        let content = tokio::fs::read_to_string(path)
            .await
            .expect("Can't open a config file");
        serde_json::from_str(&content).expect("Can't parse a config file")
    }
}

pub fn from_env<'de, D>(deserializer: D) -> Result<SecretString, D::Error>
where
    D: Deserializer<'de>,
{
    let secret = SecretString::deserialize(deserializer)?;
    let value = secret.expose_secret();
    if let Some(value) = value.strip_prefix('$') {
        Ok(SecretString::from(std::env::var(value).unwrap_or_else(
            |_| panic!("Environment variable {} must be set", value),
        )))
    } else {
        Ok(secret)
    }
}
