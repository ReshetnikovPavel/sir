use serde::Deserialize;

use crate::openai::config::{OpenAIConfig, TtsConfig};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioConfig {
    pub stt: OpenAIConfig,
    pub tts: TtsConfig,
}
