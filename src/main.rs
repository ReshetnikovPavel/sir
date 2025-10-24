use crate::audio::audio_service::AudioService;
use crate::db::chat_repo;
use crate::domain::events::EventEmitter;
use crate::domain::messages::{Message, SystemMessage};
use crate::openai::config::TtsConfig;
use crate::openai::embedding_model::EmbeddingModel;
use crate::openai::llm::LargeLanguageModel;
use crate::openai::stt::SpeechToText;
use crate::openai::tts::TextToSpeech;
use crate::rag::tools_rag::ToolsRag;
use crate::{domain::events::Event, text::context_service::ContextService};
use std::sync::Arc;
use std::{collections::HashMap, fs::read_to_string, path::PathBuf, rc::Rc, thread};

use async_openai::{config::OpenAIConfig, Client};
use clap::Parser;
use config::Config;
use dotenv::dotenv;
use mcp::tools_repo::McpToolsRepo;
use secrecy::ExposeSecret;
use tokio::sync::mpsc::{self, Sender};

use crate::{
    cli::{event_processor::CliEventProcessor, runtime::CliRuntime},
    db::chat_repo::ChatRepo,
    text::pipeline::TextPipeline,
};

mod audio;
mod cli;
mod config;
mod db;
mod domain;
mod mcp;
mod openai;
mod rag;
mod text;

/// SIR AI
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.json")]
    pub config: PathBuf,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {

    dotenv().ok();
    env_logger::init();

    let args = Args::parse();

    let (tx, rx) = mpsc::channel::<Event>(128);
    let mut event_processor = CliEventProcessor {
        rx,
        stopwatches: HashMap::new(),
    };
    let event_processor_handle = thread::spawn(async move || {
        event_processor.run().await;
    });

    let event_emitter = Arc::new(EventEmitter { tx });
    let config = Config::load(args.config).await;

    let db = libsql::Builder::new_local("sir.db")
        .build()
        .await
        .expect("Unable to access the database");
    let db_connection = db.connect().expect("Unable to connect to the database");
    let db_connection = Arc::new(db_connection);
    let chat_repo = ChatRepo::init(db_connection)
        .await
        .expect("Unable to initialize chat repository");
    let chat_repo = Arc::new(chat_repo);

    event_emitter.emit(Event::StartLoadingTools).await;
    let tools_repo_with_errors = McpToolsRepo::from_config(&config.mcp).await;
    for error in tools_repo_with_errors.errors {
        log::error!("{}", error)
    }
    let tools_repo = Arc::new(tools_repo_with_errors.value);
    event_emitter.emit(Event::FinishLoadingTools).await;

    let embedding_model = EmbeddingModel {
        client: Client::with_config(
            OpenAIConfig::new()
                .with_api_base(config.chat.embedding.api_base.clone())
                .with_api_key(config.chat.embedding.api_key.expose_secret()),
        ),
        model: config.chat.embedding.model.clone(),
    };
    let embedding_model = Arc::new(embedding_model);

    let tools_result = tools_repo.tools().await;
    let tools = tools_result.value;
    for error in tools_result.errors {
        event_emitter.emit(Event::Error(error.into())).await;
    }
    let tools_rag = Arc::new(ToolsRag::new(embedding_model.clone(), tools).await.unwrap());

    let system_prompt =
        read_to_string(config.chat.system_prompt_path.clone()).expect("System prompt file does not exist");

    let context_service = ContextService {
        tools_rag,
        chat_repo: chat_repo.clone(),
        system_prompt: Message::System(SystemMessage {
            content: system_prompt,
        }),
        event_emitter: event_emitter.clone(),
        options: config.context.clone(),
    };

    let llm = LargeLanguageModel {
        client: Client::with_config(
            OpenAIConfig::new()
                .with_api_base(config.chat.llm.api_base.clone())
                .with_api_key(config.chat.llm.api_key.expose_secret()),
        ),
        model: config.chat.llm.model.clone(),
    };
    let text_pipeline = TextPipeline {
        llm: Arc::new(llm),
        context_service: Arc::new(context_service),
        chat_repo: chat_repo.clone(),
        tools_repo: tools_repo.clone(),
        event_emitter: event_emitter.clone(),
    };

    tokio::join!(event_processor_handle.join().unwrap(), audio::voice_assistant::startup(config, text_pipeline));

    // tokio::join!(event_processor_handle.join().unwrap(), startup(tx, args, text_pipeline));
}

async fn startup(tx: Sender<Event>, args: Args, text_pipeline: TextPipeline, config: Config, chat_repo: Rc<ChatRepo>) {
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
    let audio_service = AudioService {
        stt,
        tts,
        vad_record_duration: config.audio.vad.record_duration,
    };

    let cli_runtime = CliRuntime {
        text_pipeline,
        audio_service,
        chat_repo,
        last_chat_id_path: PathBuf::from("last_chat_id.txt"),
    };
    cli_runtime.run().await;
}
