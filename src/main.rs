use std::{fs::read_to_string, sync::Arc};

use async_openai::{config::OpenAIConfig, Client};
use chat::pipeline::Pipeline;
use cli::runtime::cli_runtime;
use config::Config;
use dotenv::dotenv;
use history::file_history_repo::FileHistoryRepo;
use log::warn;
use tools::tools_repo::ToolsRepo;

use crate::audio::openai_stt::OpenAISpeechToText;

mod audio;
mod chat;
mod cli;
mod config;
mod history;
mod tools;

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {
    dotenv().ok();
    env_logger::init();
    let config = Config::load("config.json").await;

    let tools_repo = match ToolsRepo::from_config(&config.mcp).await {
        Ok(r) => r,
        Err((r, e)) => {
            warn!("Unable to load MCP servers: {:?}", e);
            r
        }
    };
    let tools_repo = Arc::new(tools_repo);
    let history_repo = Arc::new(FileHistoryRepo {
        file_path: "history.json".to_string(),
    });

    let stt = OpenAISpeechToText {
        client: Client::with_config(OpenAIConfig::new().with_api_base("http://127.0.0.1:8000/v1")),
        model: "whisper-tiny-ru-ct2".to_owned(),
    };
    let stt = Arc::new(stt);

    let api_base = std::env::var("OPENAI_API_BASE").expect("OPENAI_API_BASE must be set");
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let model = std::env::var("OPENAI_MODEL").expect("OPENAI_MODEL must be set");
    let system_prompt_path =
        std::env::var("SYSTEM_PROMPT_PATH").expect("SYSTEM_PROMPT_PATH must be set");
    let system_prompt =
        read_to_string(system_prompt_path).expect("System prompt file does not exist");

    let mut pipeline = Pipeline::new(
        &api_base,
        &api_key,
        &model,
        &system_prompt,
        history_repo,
        tools_repo,
    )
    .await
    .expect("Cannot create a chat pipeline");

    cli_runtime(&mut pipeline, stt.clone()).await;
}
