use std::{
    fs::{self, OpenOptions},
    io,
    process::Stdio,
};
use thiserror::Error;

use rmcp::{service::RunningService, transport::sse::SseTransportError, RoleClient, ServiceExt};

use super::config::McpServerTransportConfig;

pub type McpServer = RunningService<RoleClient, ()>;

pub async fn new(
    name: &str,
    config: &McpServerTransportConfig,
) -> Result<RunningService<RoleClient, ()>, Error> {
    let client = match config {
        McpServerTransportConfig::Sse { url } => {
            let transport = rmcp::transport::sse::SseTransport::start(url.to_owned()).await?;
            ().serve(transport).await?
        }
        McpServerTransportConfig::Stdio {
            command,
            args,
            envs,
            log,
        } => {
            let log = log.clone().unwrap_or(format!("logs/{}.log", name).into());
            if let Some(parent) = log.parent() {
                fs::create_dir_all(parent)?;
            }
            let stdout = OpenOptions::new().create(true).append(true).open(&log)?;
            let stderr = OpenOptions::new().create(true).append(true).open(&log)?;

            let transport = rmcp::transport::child_process::TokioChildProcess::new(
                tokio::process::Command::new(command)
                    .args(args)
                    .envs(envs)
                    .stdout(Stdio::from(stdout))
                    .stderr(Stdio::from(stderr)),
            )?;
            ().serve(transport).await?
        }
    };
    Ok(client)
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error starting MCP SSE server")]
    Sse(#[from] SseTransportError),
    #[error("Error starting MCP child process server")]
    ChildProcess(#[from] io::Error),
}
