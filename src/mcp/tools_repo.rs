use std::collections::HashMap;

use rmcp::{
    ServiceError,
    model::{CallToolRequestMethod, CallToolResult, ErrorData},
};

use crate::{
    domain::tools::{Tool, ToolCall},
    mcp::{self, config::McpToolConfig},
};

use super::{config::McpConfig, server::McpServer};

type ServerName = String;
type ToolName = String;

pub struct McpToolsRepo {
    servers: HashMap<ServerName, McpServer>,
    tool_configs: HashMap<ToolName, McpToolConfig>,
}

impl McpToolsRepo {
    pub async fn from_config(config: &McpConfig) -> (Self, Vec<mcp::server::Error>) {
        let tasks = config.mcp_servers.iter().map(|(name, server_config)| {
            let transport_config = server_config.transport.clone();
            let name = name.to_string();
            async move {
                let server = mcp::server::new(&name, &transport_config).await?;
                Ok((name, server))
            }
        });
        let results = futures::future::join_all(tasks).await;
        let (servers, errors): (Vec<_>, Vec<_>) =
            results.into_iter().partition(|result| result.is_ok());

        let tool_configs = config
            .mcp_servers
            .values()
            .flat_map(|server_config| {
                server_config
                    .tools
                    .iter()
                    .map(|(tool_name, tool_config)| (tool_name.to_owned(), tool_config.clone()))
            })
            .collect::<HashMap<_, _>>();

        let repo = Self {
            servers: servers.into_iter().filter_map(Result::ok).collect(),
            tool_configs,
        };
        let errors = errors.into_iter().filter_map(Result::err).collect();

        (repo, errors)
    }

    pub async fn tools(&self) -> (Vec<Tool>, Vec<ServiceError>) {
        let tasks = self.servers.iter().map(|(server_name, server)| async {
            Ok(server.list_all_tools().await?.into_iter().map(|t| {
                Tool::new(
                    t.clone(),
                    server_name.clone(),
                    self.tool_configs
                        .get(&String::from(t.name.clone()))
                        .map(|config| config.on_response)
                        .unwrap_or_default(),
                )
            }))
        });
        let results = futures::future::join_all(tasks).await;
        let (tools, errors): (Vec<_>, Vec<_>) =
            results.into_iter().partition(|result| result.is_ok());

        let tools = tools
            .into_iter()
            .filter_map(Result::ok)
            .flatten()
            .collect::<Vec<_>>();

        let errors = errors.into_iter().filter_map(Result::err).collect();

        (tools, errors)
    }

    pub async fn call_tools(
        &self,
        tool_calls: Vec<ToolCall>,
    ) -> Vec<Result<CallToolResult, ServiceError>> {
        let tasks = tool_calls
            .into_iter()
            .map(|tool_call| async { self.call_tool(tool_call).await });
        futures::future::join_all(tasks).await
    }

    pub async fn call_tool(&self, tool_call: ToolCall) -> Result<CallToolResult, ServiceError> {
        let server = self
            .servers
            .get(&tool_call.server_name)
            .ok_or(method_not_found())?;
        server.call_tool(tool_call.into()).await
    }
}

fn method_not_found() -> ServiceError {
    ServiceError::McpError(ErrorData::method_not_found::<CallToolRequestMethod>())
}
