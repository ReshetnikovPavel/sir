use std::{
    fmt::Debug, io::Read, sync::{
        mpsc::{channel, Sender},
        Arc, Mutex,
    }, thread, time::{Duration, SystemTime}
};

use async_openai::{config::OpenAIConfig, error::OpenAIError, Client};
use cpal::{traits::HostTrait as _, SupportedStreamConfig};
use oww_rs::oww::{OwwModel, OWW_MODEL_CHUNK_SIZE};
use rodio::{DeviceTrait as _, Sink};
use secrecy::ExposeSecret as _;
use tokio::task::JoinHandle;
use voice_activity_detector_silero_v5::VoiceActivityDetector;

use crate::{
    audio::recording::Recording, config::Config, domain::messages::AssistantMessage, openai::{config::TtsConfig, stt::SpeechToText, tts::TextToSpeech}, text::pipeline::TextPipeline
};

const CHANNELS: u16 = 1;
const BITS_PER_SAMPLE: u16 = 16;
const COMMON_CHUNK_SIZE: usize = 1024;

const SILERO_SAMPLE_RATE: u32 = 16000;
const SILERO_CHUNK_SIZE: usize = 512;

const OWW_SAMPLE_RATE: u32 = 16000;
const OWW_CHUNK_SIZE: usize = 1280;

pub async fn startup(config: Config, text_pipeline: TextPipeline) {
    let (microphone_i16_sender, microphone_i16_receiver) = channel();
    let (microphone_f32_sender, microphone_f32_receiver) = channel();

    let vad_prob: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
    let vad_prob_clone = vad_prob.clone();
    let oww_prob: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
    let oww_prob_clone = oww_prob.clone();

    let _handle_vad = thread::spawn(move || {
        let mut vad = VoiceActivityDetector::builder()
            .chunk_size(SILERO_CHUNK_SIZE)
            .sample_rate(SILERO_SAMPLE_RATE)
            .build()
            .unwrap();

        loop {
            let (data, time): (Vec<i16>, SystemTime) = microphone_i16_receiver.recv().unwrap();
            if !SystemTime::now()
                .duration_since(time)
                .unwrap()
                .saturating_sub(Duration::from_millis(500))
                .is_zero()
            {
                dbg!("Skipped");
                continue;
            }
            let prob = vad.predict(data);
            let mut mutex_handle = vad_prob_clone.lock().unwrap();
            *mutex_handle = prob;
        }
    });

    let _handle_oww = thread::spawn(move || {
        let mut oww =
            OwwModel::new(oww_rs::config::SpeechUnlockType::OpenWakeWordAlexa, 0.1).unwrap();

        let mut buffer = vec![];
        loop {
            let (data, time): (Vec<f32>, SystemTime) = microphone_f32_receiver.recv().unwrap();
            let now = SystemTime::now();
            if !now
                .duration_since(time)
                .unwrap()
                .saturating_sub(Duration::from_millis(500))
                .is_zero()
            {
                dbg!("Skipped");
                continue;
            }

            buffer.extend(data);
            if buffer.len() < OWW_MODEL_CHUNK_SIZE {
                continue;
            }

            let detection = oww.detection(buffer[..OWW_MODEL_CHUNK_SIZE].to_vec());
            // dbg!(&detection);
            let mut mutex_handle = oww_prob_clone.lock().unwrap();
            *mutex_handle = detection.probability;
            drop(mutex_handle);
            buffer.clear();
        }
    });

    let stt = SpeechToText {
        client: Client::with_config(
            OpenAIConfig::new()
                .with_api_base(config.audio.stt.api_base)
                .with_api_key(config.audio.stt.api_key.expose_secret()),
        ),
        model: config.audio.stt.model,
    };

    let tts = TextToSpeech {
        config: TtsConfig {
            api_base: "http://127.0.0.1:8000/v1".to_owned(),
            api_key: "".into(),
            model: "".to_owned(),
            voice: "man".to_owned(),
        },
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

    let _stream1 = microphone_stream_i16(
        microphone_i16_sender,
        cpal::SupportedStreamConfig::new(
            1,
            cpal::SampleRate(SILERO_SAMPLE_RATE),
            cpal::SupportedBufferSize::Unknown,
            cpal::SampleFormat::I16,
        ),
    );
    let _stream1 = microphone_stream_f32(
        microphone_f32_sender,
        cpal::SupportedStreamConfig::new(
            1,
            cpal::SampleRate(OWW_SAMPLE_RATE),
            cpal::SupportedBufferSize::Unknown,
            cpal::SampleFormat::F32,
        ),
    );
    worker.work().await;
    // loop {}
}

fn microphone_stream_i16(
    output: Sender<(Vec<i16>, SystemTime)>,
    config: SupportedStreamConfig,
) -> anyhow::Result<cpal::Stream> {
    let host = cpal::default_host();
    let device = host.default_input_device().unwrap();
    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[i16], _: &_| {
            if let Err(e) = output.send((data.to_vec(), SystemTime::now())) {
                log::error!("{}", e)
            }
        },
        |e| log::error!("{}", e),
        None,
    )?;

    Ok(stream)
}

fn microphone_stream_f32(
    output: Sender<(Vec<f32>, SystemTime)>,
    config: SupportedStreamConfig,
) -> anyhow::Result<cpal::Stream> {
    let host = cpal::default_host();
    let device = host.default_input_device().unwrap();
    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &_| {
            if let Err(e) = output.send((data.to_vec(), SystemTime::now())) {
                log::error!("{}", e)
            }
        },
        |e| log::error!("{}", e),
        None,
    )?;

    Ok(stream)
}

pub struct VoiceAssistant {
    vad_prob: Arc<Mutex<f32>>,
    oww_prob: Arc<Mutex<f32>>,
    stt: Arc<SpeechToText>,
    text_pipeline: Arc<TextPipeline>,
    tts: Arc<TextToSpeech>,
    sink: Sink,
}

enum State {
    Idle,
    Listening(ListeningState),
    Transcribing(TranscribingState),
    TextProcessing(TextProcessingState),
    GeneratingSpeech(GeneratingSpeechState),
    Speaking
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

struct ListeningState {
    recording: Recording,
    lastest_speech_time: SystemTime,
}

struct TranscribingState {
    stt_thread_handle: JoinHandle<Result<String, OpenAIError>>,
}

struct TextProcessingState {
    text_pipeline_thread_handle: JoinHandle<Result<Vec<AssistantMessage>, anyhow::Error>>
}

struct GeneratingSpeechState {
    tts_thread_handle: JoinHandle<Result<Vec<u8>, OpenAIError>>
}

impl Default for State {
    fn default() -> Self {
        State::Idle
    }
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
            State::TextProcessing(text_processing_state) => self.text_processing(text_processing_state).await,
            State::GeneratingSpeech(generating_speech_state) => self.generating_speech(generating_speech_state).await,
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
            let text_pipeline_handle = tokio::spawn(async move { text_pipeline.answer_prompt(3, text).await });
            Ok(State::TextProcessing(TextProcessingState { text_pipeline_thread_handle: text_pipeline_handle }))
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
            Ok(State::GeneratingSpeech(GeneratingSpeechState { tts_thread_handle }))
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
        let speech_prob = self.vad_prob.lock().unwrap().clone();
        speech_prob > 0.5
    }

    fn is_wakeword(&self) -> bool {
        let wakeword_prob = self.oww_prob.lock().unwrap().clone();
        wakeword_prob > 0.5
    }
}
