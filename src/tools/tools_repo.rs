use rmcp::model::Tool;

use crate::tools::mcp_server;

use super::{config::McpConfig, mcp_server::McpServer};

pub struct ToolsRepo {
    servers: Vec<McpServer>,
}

impl ToolsRepo {
    pub async fn from_config(config: &McpConfig) -> Result<Self, ()> {
        let mut servers = vec![];
        for server_config in &config.servers {
            let server = mcp_server::from_config(&server_config.transport)
                .await
                .unwrap();
            servers.push(server);
        }
        Ok(ToolsRepo { servers })
    }

    pub async fn tools(&self) -> Result<Vec<Tool>, ()> {
        let mut tools = vec![];
        for server in &self.servers {
            let mut server_tools = server.list_all_tools().await.unwrap();
            tools.append(&mut server_tools);
        }
        Ok(tools)
    }
}
