use std::{fmt::Debug, time::SystemTime};

use async_openai::error::OpenAIError;
use tokio::task::JoinHandle;

use crate::{audio::recording::Recording, domain::messages::AssistantMessage};

#[derive(Default)]
pub enum State {
    #[default]
    Idle,
    Listening(ListeningState),
    Transcribing(TranscribingState),
    TextProcessing(TextProcessingState),
    GeneratingSpeech(GeneratingSpeechState),
    Speaking,
}

impl Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Listening(_) => write!(f, "Listening"),
            Self::Transcribing(_) => write!(f, "Transcribing"),
            Self::TextProcessing(_) => write!(f, "TextProcessing"),
            State::GeneratingSpeech(_) => write!(f, "GeneratingSpeech"),
            State::Speaking => write!(f, "Speaking"),
        }
    }
}

pub struct ListeningState {
    pub recording: Recording,
    pub lastest_speech_time: SystemTime,
}

pub struct TranscribingState {
    pub stt_thread_handle: JoinHandle<Result<String, OpenAIError>>,
}

pub struct TextProcessingState {
    pub text_pipeline_thread_handle: JoinHandle<Result<Vec<AssistantMessage>, anyhow::Error>>,
}

pub struct GeneratingSpeechState {
    pub tts_thread_handle: JoinHandle<Result<Vec<u8>, OpenAIError>>,
}
