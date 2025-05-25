use serde::{Deserialize, Serialize};

use crate::tools::config::McpConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub mcp: McpConfig,
}

impl Config {
    pub async fn load(path: &str) -> Result<Self, ()> {
        let content = tokio::fs::read_to_string(path).await.unwrap();
        let config: Self = serde_json::from_str(&content).unwrap();
        Ok(config)
    }
}
