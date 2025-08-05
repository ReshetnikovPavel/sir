use std::{fs::read_to_string, path::PathBuf, rc::Rc};

use async_openai::{config::OpenAIConfig, Client};
use chat::pipeline::TextPipeline;
use config::Config;
use dotenv::dotenv;
use context::context_service::ContextService;
use secrecy::ExposeSecret;
use tools::tools_repo::ToolsRepo;

use crate::{
    audio::{audio_service::AudioService, openai_stt::OpenAISpeechToText},
    cli::runtime::CliRuntime,
    db::chat_repo::ChatRepo,
};

mod audio;
mod chat;
mod cli;
mod config;
mod db;
mod context;
mod tools;
mod types;

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

    let system_prompt =
        read_to_string(config.chat.system_prompt_path).expect("System prompt file does not exist");

    let history_service = ContextService {
        chat_repo: chat_repo.clone(),
        system_prompt,
    };

    let tools_repo_with_errors = ToolsRepo::from_config(&config.mcp).await;
    for error in tools_repo_with_errors.errors {
        log::error!("{}", error)
    }
    let tools_repo = tools_repo_with_errors.value;

    let text_pipeline = TextPipeline {
        client: Client::with_config(
            OpenAIConfig::new()
                .with_api_base(config.chat.llm.api_base)
                .with_api_key(config.chat.llm.api_key.expose_secret()),
        ),
        model: config.chat.llm.model,
        history_service,
        tools_repo,
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
        last_chat_id_path: PathBuf::from("last_chat_id.txt")
    };

    cli_runtime.run().await;
}
