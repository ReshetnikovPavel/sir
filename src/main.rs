use std::{fs::read_to_string, sync::Arc, time::Duration};

use async_openai::{config::OpenAIConfig, Client};
use chat::pipeline::TextPipeline;
use config::Config;
use dotenv::dotenv;
use history::file_history_repo::FileHistoryRepo;
use log::warn;
use tools::tools_repo::ToolsRepo;

use crate::{
    audio::{audio_service::AudioService, openai_stt::OpenAISpeechToText},
    cli::runtime::CliRuntime, tools::tools_repo,
};

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

    let tools_repo_with_errors = ToolsRepo::from_config(&config.mcp).await;
    for error in tools_repo_with_errors.errors {
        log::error!("{}", error)
    }
    let tools_repo = tools_repo_with_errors.value;
    let tools_repo = Arc::new(tools_repo);
    let history_repo = Arc::new(FileHistoryRepo {
        file_path: "history.json".to_string(),
    });

    let api_base = std::env::var("OPENAI_API_BASE").expect("OPENAI_API_BASE must be set");
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let model = std::env::var("OPENAI_MODEL").expect("OPENAI_MODEL must be set");
    let system_prompt_path =
        std::env::var("SYSTEM_PROMPT_PATH").expect("SYSTEM_PROMPT_PATH must be set");
    let system_prompt =
        read_to_string(system_prompt_path).expect("System prompt file does not exist");

    let text_pipeline = TextPipeline::new(
        &api_base,
        &api_key,
        &model,
        &system_prompt,
        history_repo,
        tools_repo,
    )
    .await
    .expect("Cannot create a chat pipeline");
    let text_pipeline = Arc::new(text_pipeline);

    let stt = OpenAISpeechToText {
        client: Client::with_config(OpenAIConfig::new().with_api_base("http://127.0.0.1:8000/v1")),
        model: "whisper-tiny-ru-ct2".to_owned(),
    };
    let stt = Arc::new(stt);

    let audio_service = AudioService {
        stt,
        vad_record_duration: Duration::from_millis(500),
    };
    let audio_service = Arc::new(audio_service);

    let cli_runtime = CliRuntime {
        text_pipeline: text_pipeline.clone(),
        audio_service: audio_service.clone(),
    };

    cli_runtime.run().await;
}
