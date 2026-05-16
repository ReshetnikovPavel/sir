use crate::domain::events::EventEmitter;
use crate::domain::messages::{Message, SystemMessage};
use crate::openai::embedding_model::EmbeddingModel;
use crate::openai::llm::LargeLanguageModel;
use crate::rag::tools_rag::ToolsRag;
use crate::voice_assistant::assistant::VoiceAssistant;
use crate::{domain::events::Event, text::context_service::ContextService};
use std::env;
use std::process::{Command, exit};
use std::sync::Arc;
use std::{fs::read_to_string, path::PathBuf};

use anyhow::anyhow;
use config::Config;
use dotenv::dotenv;
use mcp::tools_repo::McpToolsRepo;
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

fn print_help() {
    println!(
        "Usage: sir [OPTION]...
Run AI assistant.

-c, --config config path to use"
    )
}

struct Args {
    config: PathBuf,
}

impl Args {
    fn parse() -> anyhow::Result<Self> {
        let mut args = env::args().skip(1);
        let mut config = PathBuf::from("config.json");
        while let Some(arg) = args.next() {
            match arg.as_ref() {
                "-h" | "--help" => {
                    print_help();
                    exit(0)
                }
                "-c" | "--config" => {
                    config = args
                        .next()
                        .ok_or(anyhow!("No `{}` value provided", arg))?
                        .into()
                }
                arg if arg.starts_with('-') => Err(anyhow!("Unknown option: `{}`", arg))?,
                arg => Err(anyhow!("Positional arguments are not supported: `{}`", arg))?,
            }
        }
        Ok(Self { config })
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {
    dotenv().ok();
    env_logger::init();

    let args = Args::parse().expect("Failed to parse arguments");

    let (event_sender, event_receiver) = channel(16);
    let config = Config::load(args.config).await;

    let procs = config
        .procs
        .iter()
        .map(|p| {
            Command::new(p.command.clone())
                .args(p.args.clone())
                .spawn()
                .expect("Cound not run a command")
        })
        .collect::<Vec<_>>();
    log::info!("Started {} processes", procs.len());

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
    let (tools, errors) = McpToolsRepo::from_config(&config.mcp).await;
    for error in errors {
        log::error!("{}", error)
    }
    let tools_repo = Arc::new(tools);

    let embedding_model = EmbeddingModel {
        client: reqwest::Client::new(),
        config: config.chat.embedding.clone(),
    };
    let embedding_model = Arc::new(embedding_model);

    log::info!("Initiating tools RAG");
    let (tools, errors) = tools_repo.tools().await;
    for error in errors {
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

    VoiceAssistant::startup(config, text_pipeline, event_emitter).await;
}
