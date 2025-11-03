use std::{
    io::Read,
    sync::{mpsc::channel, Arc, Mutex},
    time::{Duration, SystemTime},
};

use async_openai::{config::OpenAIConfig, Client};
use rodio::Sink;
use secrecy::ExposeSecret as _;

use crate::{
    audio::{microphone::microphone_stream, recording::Recording},
    config::Config,
    openai::{stt::SpeechToText, tts::TextToSpeech},
    text::pipeline::TextPipeline,
    voice_assistant::{
        daemons,
        state::{GeneratingSpeechState, ListeningState, TextProcessingState, TranscribingState},
    },
};

use super::state::State;

pub struct VoiceAssistant {
    vad_prob: Arc<Mutex<f32>>,
    oww_prob: Arc<Mutex<f32>>,
    stt: Arc<SpeechToText>,
    text_pipeline: Arc<TextPipeline>,
    tts: Arc<TextToSpeech>,
    sink: Sink,
}

impl VoiceAssistant {
    pub async fn work(self) -> ! {
        dbg!("Voice Assistant started");
        let mut state = State::Idle;
        loop {
            state = self
                .handle(state)
                .await
                .inspect_err(|e| log::error!("{}", e))
                .unwrap();
        }
    }

    async fn handle(&self, state: State) -> anyhow::Result<State> {
        dbg!(&state);
        match state {
            State::Idle => self.idle().await,
            State::Listening(listening_state) => self.listening(listening_state).await,
            State::Transcribing(transcribing_state) => self.transcribing(transcribing_state).await,
            State::TextProcessing(text_processing_state) => {
                self.text_processing(text_processing_state).await
            }
            State::GeneratingSpeech(generating_speech_state) => {
                self.generating_speech(generating_speech_state).await
            }
            State::Speaking => self.speaking().await,
        }
    }

    async fn idle(&self) -> anyhow::Result<State> {
        if self.is_wakeword() {
            Ok(State::Listening(ListeningState {
                recording: Recording::start()?,
                lastest_speech_time: SystemTime::now(),
            }))
        } else {
            Ok(State::Idle)
        }
    }

    async fn listening(&self, state: ListeningState) -> anyhow::Result<State> {
        if self.is_speech() {
            Ok(State::Listening(ListeningState {
                recording: state.recording,
                lastest_speech_time: SystemTime::now(),
            }))
        } else if SystemTime::now()
            .duration_since(state.lastest_speech_time)?
            .saturating_sub(Duration::from_secs(1))
            .is_zero()
        {
            Ok(State::Listening(state))
        } else {
            let mut file = state.recording.stop()?;
            let mut recorded_data = vec![];
            file.read_to_end(&mut recorded_data)?;

            let stt = self.stt.clone();
            let stt_thread_handle =
                tokio::spawn(async move { stt.transcribe(recorded_data).await });

            Ok(State::Transcribing(TranscribingState { stt_thread_handle }))
        }
    }

    async fn transcribing(&self, state: TranscribingState) -> anyhow::Result<State> {
        if state.stt_thread_handle.is_finished() {
            let text = state.stt_thread_handle.await??;
            dbg!(&text);
            let text_pipeline = self.text_pipeline.clone();
            let text_pipeline_handle =
                tokio::spawn(async move { text_pipeline.answer_prompt(3, text).await });
            Ok(State::TextProcessing(TextProcessingState {
                text_pipeline_thread_handle: text_pipeline_handle,
            }))
        } else {
            Ok(State::Transcribing(state))
        }
    }

    async fn text_processing(&self, state: TextProcessingState) -> anyhow::Result<State> {
        if state.text_pipeline_thread_handle.is_finished() {
            let messages = state.text_pipeline_thread_handle.await??;
            let request = messages.into_iter().map(|m| m.content).collect::<String>();
            let tts = self.tts.clone();
            let tts_thread_handle = tokio::spawn(async move { tts.speech(&request).await });
            Ok(State::GeneratingSpeech(GeneratingSpeechState {
                tts_thread_handle,
            }))
        } else {
            Ok(State::TextProcessing(state))
        }
    }

    async fn generating_speech(&self, state: GeneratingSpeechState) -> anyhow::Result<State> {
        if state.tts_thread_handle.is_finished() {
            let data = state.tts_thread_handle.await??;
            let cursor = std::io::Cursor::new(data);
            let decoder = rodio::Decoder::new(cursor)?;
            self.sink.append(decoder);
            Ok(State::Speaking)
        } else {
            Ok(State::GeneratingSpeech(state))
        }
    }

    async fn speaking(&self) -> anyhow::Result<State> {
        if self.sink.empty() {
            Ok(State::Idle)
        } else {
            Ok(State::Speaking)
        }
    }

    fn is_speech(&self) -> bool {
        let speech_prob = *self.vad_prob.lock().unwrap();
        speech_prob > 0.5
    }

    fn is_wakeword(&self) -> bool {
        let wakeword_prob = *self.oww_prob.lock().unwrap();
        wakeword_prob > 0.5
    }

    pub async fn startup(config: Config, text_pipeline: TextPipeline) -> ! {
        let (microphone_i16_sender, microphone_i16_receiver) = channel();
        let (microphone_f32_sender, microphone_f32_receiver) = channel();

        let vad_prob: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
        let oww_prob: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));

        let _handle_vad = daemons::vad(microphone_i16_receiver, vad_prob.clone());
        let _handle_oww = daemons::oww(microphone_f32_receiver, oww_prob.clone());

        let stt = SpeechToText {
            client: Client::with_config(
                OpenAIConfig::new()
                    .with_api_base(config.audio.stt.api_base)
                    .with_api_key(config.audio.stt.api_key.expose_secret()),
            ),
            model: config.audio.stt.model,
        };

        let tts = TextToSpeech {
            config: config.audio.tts,
            client: reqwest::Client::new(),
        };

        let stream_handle = rodio::OutputStreamBuilder::open_default_stream().unwrap();
        let sink = rodio::Sink::connect_new(stream_handle.mixer());
        let worker = VoiceAssistant {
            vad_prob,
            oww_prob,
            stt: Arc::new(stt),
            text_pipeline: Arc::new(text_pipeline),
            tts: Arc::new(tts),
            sink,
        };

        let _microphone_i16_handle = microphone_stream(
            microphone_i16_sender,
            cpal::SupportedStreamConfig::new(
                1,
                cpal::SampleRate(daemons::SILERO_SAMPLE_RATE),
                cpal::SupportedBufferSize::Unknown,
                cpal::SampleFormat::I16,
            ),
        );

        let _microphone_f32_sender = microphone_stream(
            microphone_f32_sender,
            cpal::SupportedStreamConfig::new(
                1,
                cpal::SampleRate(daemons::OWW_SAMPLE_RATE),
                cpal::SupportedBufferSize::Unknown,
                cpal::SampleFormat::F32,
            ),
        );

        worker.work().await;
    }
}
