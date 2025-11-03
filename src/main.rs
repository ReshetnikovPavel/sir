use crate::domain::events::EventEmitter;
use crate::domain::messages::{Message, SystemMessage};
use crate::openai::embedding_model::EmbeddingModel;
use crate::openai::llm::LargeLanguageModel;
use crate::rag::tools_rag::ToolsRag;
use crate::voice_assistant::assistant::VoiceAssistant;
use crate::{domain::events::Event, text::context_service::ContextService};
use std::sync::Arc;
use std::{collections::HashMap, fs::read_to_string, path::PathBuf, thread};

use async_openai::{Client, config::OpenAIConfig};
use clap::Parser;
use config::Config;
use dotenv::dotenv;
use mcp::tools_repo::McpToolsRepo;
use secrecy::ExposeSecret;
use tokio::sync::mpsc::{self};

use crate::{
    cli::event_processor::CliEventProcessor, db::chat_repo::ChatRepo, text::pipeline::TextPipeline,
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

    let (tx, rx) = mpsc::channel::<Event>(128);
    let mut event_processor = CliEventProcessor {
        rx,
        stopwatches: HashMap::new(),
    };
    let _event_processor_handle = thread::spawn(async move || {
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

    VoiceAssistant::startup(config, text_pipeline).await;
}
