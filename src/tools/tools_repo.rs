use std::collections::HashMap;

use rmcp::{
    model::{CallToolRequestMethod, CallToolResult, ErrorData},
    ServiceError,
};

use crate::tools::mcp_server;

use super::{
    config::McpConfig,
    mcp_server::McpServer,
    tool::{Tool, ToolCall},
};

pub struct ToolsRepo {
    servers: HashMap<String, McpServer>,
}

impl ToolsRepo {
    pub async fn from_config(config: &McpConfig) -> Result<Self, (Self, Vec<mcp_server::Error>)> {
        let tasks = config.servers.iter().map(|(name, server_config)| {
            let transport_config = server_config.transport.clone();
            let name = name.to_string();
            async move {
                let server = mcp_server::new(&name, &transport_config).await?;
                Ok((name, server))
            }
        });
        let results = futures::future::join_all(tasks).await;
        let (servers, errors): (Vec<_>, Vec<_>) =
            results.into_iter().partition(|result| result.is_ok());

        let repo = Self {
            servers: servers.into_iter().filter_map(Result::ok).collect(),
        };

        if errors.is_empty() {
            Ok(repo)
        } else {
            let errors = errors.into_iter().filter_map(Result::err).collect();
            Err((repo, errors))
        }
    }

    pub async fn tools(&self) -> Result<Vec<Tool>, (Vec<Tool>, Vec<ServiceError>)> {
        let tasks = self.servers.iter().map(|(_, server)| async {
            Ok(server
                .list_all_tools()
                .await?
                .into_iter()
                .map(|t| t.into()))
        });
        let results = futures::future::join_all(tasks).await;
        let (tools, errors): (Vec<_>, Vec<_>) =
            results.into_iter().partition(|result| result.is_ok());

        let tools = tools
            .into_iter()
            .filter_map(Result::ok)
            .flatten()
            .collect::<Vec<_>>();

        if errors.is_empty() {
            Ok(tools)
        } else {
            let errors = errors.into_iter().filter_map(Result::err).collect();
            Err((tools, errors))
        }
    }

    pub async fn call_tool(&self, tool_call: ToolCall) -> Result<CallToolResult, ServiceError> {
        let tasks = self.servers.iter().map(|(_, server)| {
            let tool_call = tool_call.clone();
            async {
                let has_tool = server
                    .list_all_tools()
                    .await?
                    .into_iter()
                    .any(|t| t.name == tool_call.name);
                if has_tool {
                    server.call_tool(tool_call.into()).await
                } else {
                    Err(method_not_found())
                }
            }
        });

        futures::future::join_all(tasks)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .next()
            .ok_or(method_not_found())
    }
}

fn method_not_found() -> ServiceError {
    ServiceError::McpError(ErrorData::method_not_found::<CallToolRequestMethod>())
}
