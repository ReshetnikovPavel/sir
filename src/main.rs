use std::{fs::read_to_string, path::PathBuf, rc::Rc};

use async_openai::{config::OpenAIConfig, Client};
use config::Config;
use context::context_service::ContextService;
use dotenv::dotenv;
use mcp::tools_repo::McpToolsRepo;
use secrecy::ExposeSecret;

use crate::{
    audio::{audio_service::AudioService, openai_stt::OpenAISpeechToText},
    cli::{event_processor::CliEventProcessor, runtime::CliRuntime},
    context::openai_embedding_model::OpenAIEmbeddingModel,
    db::chat_repo::ChatRepo,
    entities::messages,
    text::{openai_llm::OpenAILargeLanguageModel, pipeline::TextPipeline},
};

mod audio;
mod cli;
mod config;
mod context;
mod db;
mod entities;
mod mcp;
mod text;

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {
    dotenv().ok();
    env_logger::init();
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

    let tools_repo_with_errors = McpToolsRepo::from_config(&config.mcp).await;
    for error in tools_repo_with_errors.errors {
        log::error!("{}", error)
    }
    let tools_repo = Rc::new(tools_repo_with_errors.value);

    let embedding_model = OpenAIEmbeddingModel {
        client: Client::with_config(
            OpenAIConfig::new()
                .with_api_base(config.chat.embedding.api_base)
                .with_api_key(config.chat.embedding.api_key.expose_secret()),
        ),
        model: config.chat.embedding.model,
    };
    let embedding_model = Rc::new(embedding_model);

    let system_prompt =
        read_to_string(config.chat.system_prompt_path).expect("System prompt file does not exist");

    let context_service = ContextService {
        chat_repo: chat_repo.clone(),
        tools_repo: tools_repo.clone(),
        embedding_model: embedding_model.clone(),
        system_prompt: messages::SystemMessage {
            content: system_prompt,
        },
    };

    let event_processor = Rc::new(CliEventProcessor {});
    let llm = OpenAILargeLanguageModel {
        client: Client::with_config(
            OpenAIConfig::new()
                .with_api_base(config.chat.llm.api_base)
                .with_api_key(config.chat.llm.api_key.expose_secret()),
        ),
        model: config.chat.llm.model,
    };
    let text_pipeline = TextPipeline {
        top_n_tools: config.top_n_tools,
        llm,
        context_service,
        tools_repo: tools_repo.clone(),
        event_processor: event_processor.clone(),
    };

    let stt = OpenAISpeechToText {
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
