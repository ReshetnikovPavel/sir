use reqwest::multipart::{Form, Part};
use secrecy::ExposeSecret as _;
use serde::Deserialize;

use crate::openai::config::OpenAIConfig;

#[derive(Clone)]
pub struct SpeechToText {
    pub config: OpenAIConfig,
    pub client: reqwest::Client,
}


#[derive(Deserialize)]
struct Transcription {
    text: String,
}

impl SpeechToText {
    pub async fn transcribe(&self, audio: Vec<u8>) -> Result<String, anyhow::Error> {
        let url = self.config.api_base.clone() + "/audio/transcriptions";
        let form = Form::new()
            .part(
                "file",
                Part::bytes(audio)
                    .file_name("recording.wav")
                    .mime_str("audio/wav")?,
            )
            .text("model", self.config.model.to_string());

        let response = self
            .client
            .post(url)
            .multipart(form)
            .bearer_auth(self.config.api_key.expose_secret())
            .send()
            .await?
            .error_for_status()?;

        let response = response.json::<Transcription>().await?;
        Ok(response.text)
    }
}
