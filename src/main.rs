use crate::domain::events::EventEmitter;
use crate::domain::messages::{Message, SystemMessage};
use crate::openai::embedding_model::EmbeddingModel;
use crate::openai::llm::LargeLanguageModel;
use crate::rag::tools_rag::ToolsRag;
use crate::voice_assistant::assistant::VoiceAssistant;
use crate::{domain::events::Event, text::context_service::ContextService};
use std::sync::Arc;
use std::{fs::read_to_string, path::PathBuf, thread};

use async_openai::{Client, config::OpenAIConfig};
use clap::Parser;
use config::Config;
use dotenv::dotenv;
use mcp::tools_repo::McpToolsRepo;
use secrecy::ExposeSecret;
use tokio::sync::mpsc::channel;

use crate::{db::chat_repo::ChatRepo, text::pipeline::TextPipeline, tui::events::EventProcessor};

mod audio;
mod config;
mod db;
mod domain;
mod mcp;
mod openai;
mod rag;
mod text;
mod tui;
mod voice_assistant;

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

    let (event_sender, event_receiver) = channel(16);
    let config = Config::load(args.config).await;

    log::info!("Connecting to the database");
    let db = libsql::Builder::new_local("sir.db")
        .build()
        .await
        .expect("Unable to access the database");
    let db_connection = db.connect().expect("Unable to connect to the database");
    let db_connection = Arc::new(db_connection);

    log::info!("Creating chat repository");
    let chat_repo = ChatRepo::init(db_connection)
        .await
        .expect("Unable to initialize chat repository");

    let chat_repo = Arc::new(chat_repo);

    log::info!("Starting event processor");
    let mut event_processor = EventProcessor {
        event_receiver,
        chat_repo: chat_repo.clone(),
    };
    let _event_processor_handle = tokio::spawn(async move { event_processor.run().await });

    let event_emitter = Arc::new(EventEmitter { event_sender });

    log::info!("Loading tools");
    let tools_repo_with_errors = McpToolsRepo::from_config(&config.mcp).await;
    for error in tools_repo_with_errors.errors {
        log::error!("{}", error)
    }
    let tools_repo = Arc::new(tools_repo_with_errors.value);

    let embedding_model = EmbeddingModel {
        client: Client::with_config(
            OpenAIConfig::new()
                .with_api_base(config.chat.embedding.api_base.clone())
                .with_api_key(config.chat.embedding.api_key.expose_secret()),
        ),
        model: config.chat.embedding.model.clone(),
    };
    let embedding_model = Arc::new(embedding_model);

    log::info!("Initiating tools RAG");
    let tools_result = tools_repo.tools().await;
    let tools = tools_result.value;
    for error in tools_result.errors {
        event_emitter.emit(Event::Error(error.into())).await;
    }
    let tools_rag = Arc::new(ToolsRag::new(embedding_model.clone(), tools).await.unwrap());

    log::info!("Reading System prompt");
    let system_prompt = read_to_string(config.chat.system_prompt_path.clone())
        .expect("System prompt file does not exist");

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
        model: config.chat.llm.model.clone(),
        config: config.chat.llm.clone(),
        client: reqwest::Client::new(),
    };
    let text_pipeline = TextPipeline {
        llm: Arc::new(llm),
        context_service: Arc::new(context_service),
        chat_repo: chat_repo.clone(),
        tools_repo: tools_repo.clone(),
        event_emitter: event_emitter.clone(),
    };

    log::info!("Starting voice assistant");
    VoiceAssistant::startup(config, text_pipeline, event_emitter).await;
}
