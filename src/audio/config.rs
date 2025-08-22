use std::time::Duration;

use duration_str::deserialize_duration;
use serde::Deserialize;

use crate::openai::config::OpenAIConfig;

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
