use async_openai::error::OpenAIError;
use std::{
    io::{self, Read},
    time::Duration,
};
use thiserror::Error;
use tokio_stream::StreamExt as _;

use voice_activity_detector_silero_v5::{StreamExt as _, VoiceActivityDetector};

use crate::{
    audio::recording::{self, Recording},
    openai::stt::OpenAISpeechToText,
};

pub struct AudioService {
    pub stt: OpenAISpeechToText,
    pub vad_record_duration: Duration,
}

impl AudioService {
    pub async fn listen_input(&self) -> Result<String, Error> {
        let recording = Recording::start(false)?;
        println!("Started recording");
        loop {
            if self.is_speech().await? {
                break;
            }
        }
        loop {
            if !self.is_speech().await? {
                break;
            }
        }

        let mut file = recording.stop()?;
        println!("Stopped recording");

        let mut buf = vec![];
        let _ = file.read_to_end(&mut buf)?;

        println!("Starting transcribing...");
        let transcription = self.stt.transcribe(buf).await?;
        println!("Trascribed::: {}", transcription);

        Ok(transcription)
    }

    async fn is_speech(&self) -> Result<bool, Error> {
        let recording = Recording::start(true)?;
        println!("Started recording for vad");

        tokio::time::sleep(self.vad_record_duration).await;

        let file = recording.stop()?;
        println!("Stopped recording for vad");

        let mut reader = hound::WavReader::open(file)?;
        let spec = reader.spec();
        let mut vad = VoiceActivityDetector::builder()
            .chunk_size(1024_usize)
            .sample_rate(spec.sample_rate)
            .build()?;

        let chunks = reader.samples::<i16>().map_while(Result::ok);

        let mut chunks = tokio_stream::iter(chunks).label(&mut vad, 0.5, 10);
        Ok(chunks.any(|c| c.is_speech()).await)
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    StartRecording(#[from] recording::StartRecordingError),
    #[error(transparent)]
    StopRecording(#[from] recording::StopRecordingError),
    #[error(transparent)]
    Transcription(#[from] OpenAIError),
    #[error(transparent)]
    IOReadRecordedFile(#[from] io::Error),
    #[error(transparent)]
    HoundReadRecordedFile(#[from] hound::Error),
    #[error(transparent)]
    VoiceActivityDetector(#[from] voice_activity_detector_silero_v5::Error),
}
