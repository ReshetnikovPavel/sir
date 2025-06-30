use std::collections::HashMap; 

use rmcp::{model::{CallToolRequestMethod, CallToolResult, ErrorData}, ServiceError};

use crate::tools::mcp_server;

use super::{config::McpConfig, mcp_server::McpServer, tool::{Tool, ToolCall}};

pub struct ToolsRepo {
    servers: HashMap<String, McpServer>,
}

impl ToolsRepo {
    pub async fn from_config(config: &McpConfig) -> Result<Self, ()> {
        let mut servers = HashMap::with_capacity(config.servers.capacity());
        for (name, server_config) in &config.servers {
            let server = mcp_server::from_config(&server_config.transport)
                .await
                .unwrap();
            servers.insert(name.to_string(), server);
        }
        Ok(ToolsRepo { servers })
    }

    pub async fn tools(&self) -> Result<Vec<Tool>, ()> {
        let mut tools = vec![];
        for (_, server) in &self.servers {
            let mut server_tools = server
                .list_all_tools()
                .await
                .unwrap()
                .iter()
                .map(|t| t.clone().into())
                .collect::<Vec<Tool>>();
            tools.append(&mut server_tools);
        }
        // println!("{:?}", tools);
        Ok(tools)
    }

    pub async fn call_tool(&self, tool_call: ToolCall) -> Result<CallToolResult, ServiceError> {
        for (_, server) in &self.servers {
            let find_tool_result = server
                .list_all_tools()
                .await
                .unwrap()
                .iter()
                .find_map(|t| (t.name == tool_call.name).then(|| t.clone()));

            if find_tool_result.is_some() {
                return server.call_tool(tool_call.clone().into()).await;
            }
        }
        Err(ServiceError::McpError(ErrorData::method_not_found::<CallToolRequestMethod>()))
    }
}
