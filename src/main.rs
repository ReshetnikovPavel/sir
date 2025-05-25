use std::sync::Arc;

use cli::cli;
use config::Config;
use dotenv::dotenv;
use history::file_history_repo::FileHistoryRepo;
use tools::tools_repo::ToolsRepo;
use crate::llm::llm::LLM;

mod cli;
mod history;
mod llm;
mod tools;
mod config;

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {
    dotenv().ok();
    env_logger::init();
    let config = Config::load("config.json").await.unwrap();

    let tools_repo = Arc::new(ToolsRepo::from_config(&config.mcp).await.unwrap());
    let history_repo = Arc::new(FileHistoryRepo {
        file_path: "history.json".to_string(),
    });

    let mut llm = LLM::from_env(history_repo, tools_repo).await.unwrap();
    let _ = cli(&mut llm).await;
}
