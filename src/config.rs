use serde::{Deserialize, Serialize};

use crate::tools::config::McpConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub mcp: McpConfig,
}

impl Config {
    pub async fn load(path: &str) -> Self {
        let content = tokio::fs::read_to_string(path)
            .await
            .expect("Can't open a config file");
        serde_json::from_str(&content).expect("Can't parse a config file")
    }
}
