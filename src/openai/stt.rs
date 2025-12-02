use async_openai::{
    Client,
    config::OpenAIConfig,
    error::OpenAIError,
    types::{AudioInput, CreateTranscriptionRequest, InputSource},
};

#[derive(Clone)]
pub struct SpeechToText {
    pub client: Client<OpenAIConfig>,
    pub model: String,
}

impl SpeechToText {
    pub async fn transcribe(&self, audio: Vec<u8>) -> Result<String, OpenAIError> {
        let request = CreateTranscriptionRequest {
            file: AudioInput {
                source: InputSource::VecU8 {
                    filename: "recording.wav".to_owned(),
                    vec: audio,
                },
            },
            model: self.model.clone(),
            prompt: None,
            response_format: None,
            temperature: None,
            language: None,
            timestamp_granularities: None,
        };
        let response = self.client.audio().transcribe(request).await?;
        Ok(response.text)
    }
}
