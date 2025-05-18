use cli::cli;
use dotenv::dotenv;
use llm::LLM;

pub(crate) mod cli;
pub(crate) mod llm;

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {
    dotenv().ok();
    let mut llm = LLM::from_env();
    let _ = cli(&mut llm).await;
}
