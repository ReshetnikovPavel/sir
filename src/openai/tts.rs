use async_openai::error::OpenAIError;
use reqwest::Client;

use crate::openai::config::TtsConfig;

pub struct TextToSpeech {
    pub config: TtsConfig,
    pub client: Client,
}

impl TextToSpeech {
    pub async fn speech(&self, input: &str) -> Result<Vec<u8>, OpenAIError> {
        let url = self.config.api_base.clone() + "/audio/speech";
        let json = serde_json::json!({
            "input": input,
            "model": self.config.model,
            "voice": self.config.voice,
            "response_format": "wav"
        });

        let response = self
            .client
            .post(url)
            .body(json.to_string())
            .send()
            .await?
            .error_for_status()?;

        let content = response.bytes().await?.to_vec();

        Ok(content)
    }
}
