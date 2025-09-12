use crate::audio::audio_service::AudioService;
use crate::domain::events::EventEmitter;
use crate::domain::messages::{Message, SystemMessage};
use crate::openai::embedding_model::EmbeddingModel;
use crate::openai::llm::LargeLanguageModel;
use crate::openai::stt::SpeechToText;
use crate::rag::tools_rag::ToolsRag;
use crate::{domain::events::Event, text::context_service::ContextService};
use std::{collections::HashMap, fs::read_to_string, path::PathBuf, rc::Rc, thread};

use async_openai::{config::OpenAIConfig, Client};
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

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {
    dotenv().ok();
    env_logger::init();

    let (tx, rx) = mpsc::channel::<Event>(128);
    let mut event_processor = CliEventProcessor {
        rx,
        stopwatches: HashMap::new(),
    };
    let event_processor_handle = thread::spawn(async move || {
        event_processor.run().await;
    });

    tokio::join!(event_processor_handle.join().unwrap(), startup(tx));
}

async fn startup(tx: Sender<Event>) {
    let event_emitter = Rc::new(EventEmitter { tx });
    let config = Config::load("config.json").await;

    let db = libsql::Builder::new_local("sir.db")
        .build()
        .await
        .expect("Unable to access the database");
    let db_connection = db.connect().expect("Unable to connect to the database");
    let db_connection = Rc::new(db_connection);
    let chat_repo = ChatRepo::init(db_connection)
        .await
        .expect("Unable to initialize chat repository");
    let chat_repo = Rc::new(chat_repo);

    event_emitter.emit(Event::StartLoadingTools).await;
    let tools_repo_with_errors = McpToolsRepo::from_config(&config.mcp).await;
    for error in tools_repo_with_errors.errors {
        log::error!("{}", error)
    }
    let tools_repo = Rc::new(tools_repo_with_errors.value);
    event_emitter.emit(Event::FinishLoadingTools).await;

    let embedding_model = EmbeddingModel {
        client: Client::with_config(
            OpenAIConfig::new()
                .with_api_base(config.chat.embedding.api_base)
                .with_api_key(config.chat.embedding.api_key.expose_secret()),
        ),
        model: config.chat.embedding.model,
    };
    let embedding_model = Rc::new(embedding_model);

    let tools_result = tools_repo.tools().await;
    let tools = tools_result.value;
    for error in tools_result.errors {
        event_emitter.emit(Event::Error(error.into())).await;
    }
    let tools_rag = ToolsRag::new(embedding_model.clone(), tools).await.unwrap();

    let system_prompt =
        read_to_string(config.chat.system_prompt_path).expect("System prompt file does not exist");

    let context_service = ContextService {
        tools_rag,
        chat_repo: chat_repo.clone(),
        system_prompt: Message::System(SystemMessage {
            content: system_prompt,
        }),
        event_emitter: event_emitter.clone(),
        options: config.context
    };

    let llm = LargeLanguageModel {
        client: Client::with_config(
            OpenAIConfig::new()
                .with_api_base(config.chat.llm.api_base)
                .with_api_key(config.chat.llm.api_key.expose_secret()),
        ),
        model: config.chat.llm.model,
    };
    let text_pipeline = TextPipeline {
        llm,
        context_service,
        chat_repo: chat_repo.clone(),
        tools_repo: tools_repo.clone(),
        event_emitter: event_emitter.clone(),
    };

    let stt = SpeechToText {
        client: Client::with_config(
            OpenAIConfig::new()
                .with_api_base(config.audio.stt.api_base)
                .with_api_key(config.audio.stt.api_key.expose_secret()),
        ),
        model: config.audio.stt.model,
    };

    let audio_service = AudioService {
        stt,
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
