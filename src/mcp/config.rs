use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;

use crate::domain::states::State;

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct McpConfig {
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    #[serde(flatten)]
    pub transport: McpServerTransportConfig,
    #[serde(default)]
    pub tools: HashMap<String, McpToolConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct McpToolConfig {
    #[serde(default)]
    pub on_response: State,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged, rename_all = "camelCase")]
pub enum McpServerTransportConfig {
    Sse {
        url: String,
    },
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        envs: HashMap<String, String>,
        log: Option<PathBuf>,
    },
}
