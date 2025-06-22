use std::{fs::read_to_string, sync::Arc};

use chat::pipeline::Pipeline;
use cli::runtime::cli_runtime;
use config::Config;
use dotenv::dotenv;
use history::file_history_repo::FileHistoryRepo;
use tools::tools_repo::ToolsRepo;

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

    let tools_repo = Arc::new(ToolsRepo::from_config(&config.mcp).await.expect("Can't create tools repo"));
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

    let mut pipeline = Pipeline::new(
        &api_base,
        &api_key,
        &model,
        &system_prompt,
        history_repo,
        tools_repo,
        ).await.expect("Cannot create a chat pipeline");

    cli_runtime(&mut pipeline).await;
}
