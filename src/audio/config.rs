use std::time::Duration;

use duration_str::deserialize_duration;
use secrecy::SecretString;
use serde::Deserialize;

use crate::config::from_env;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AudioConfig {
    pub stt: OpenAIConfig,
    pub vad: VoiceActivationDetectorConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VoiceActivationDetectorConfig {
    #[serde(deserialize_with = "deserialize_duration")]
    pub record_duration: Duration,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OpenAIConfig {
    pub api_base: String,
    #[serde(deserialize_with = "from_env")]
    pub api_key: SecretString,
    pub model: String,
}
