use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{AudioInput, CreateTranscriptionRequest, InputSource},
    Client,
};

pub struct OpenAISpeechToText {
    pub client: Client<OpenAIConfig>,
    pub model: String,
}

impl OpenAISpeechToText {
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
