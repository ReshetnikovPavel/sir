use async_openai::error::OpenAIError;
use secrecy::ExposeSecret as _;

use crate::openai::config::TtsConfig;

pub struct TextToSpeech {
    pub config: TtsConfig,
    pub client: reqwest::Client,
}

impl TextToSpeech {
    pub async fn speech(&self, input: &str) -> Result<Vec<u8>, OpenAIError> {
        let url = self.config.api_base.clone() + "/audio/speech";
        let body = serde_json::json!({
            "input": input,
            "model": self.config.model,
            "voice": self.config.voice,
            "response_format": "wav"
        });
        let response = self
            .client
            .post(url)
            .body(body.to_string())
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_key.expose_secret()),
            )
            .send()
            .await?
            .error_for_status()?;

        let content = response.bytes().await?.to_vec();

        Ok(content)
    }
}
