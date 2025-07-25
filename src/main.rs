use std::{fs::read_to_string, sync::Arc};

use async_openai::{config::OpenAIConfig, Client};
use chat::pipeline::TextPipeline;
use config::Config;
use dotenv::dotenv;
use history::file_history_repo::FileHistoryRepo;
use secrecy::ExposeSecret;
use tools::tools_repo::ToolsRepo;

use crate::{
    audio::{audio_service::AudioService, openai_stt::OpenAISpeechToText},
    cli::runtime::CliRuntime,
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
    let history_repo = Arc::new(FileHistoryRepo {
        file_path: config.history.path.clone(),
    });

    let system_prompt =
        read_to_string(config.chat.system_prompt_path).expect("System prompt file does not exist");

    let text_pipeline = TextPipeline::new(
        &config.chat.llm.api_base,
        &config.chat.llm.api_key.expose_secret(),
        &config.chat.llm.model,
        &system_prompt,
        history_repo,
        tools_repo,
    )
    .await
    .expect("Cannot create a chat pipeline");

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
        text_pipeline: text_pipeline,
        audio_service: audio_service,
    };

    cli_runtime.run().await;
}
