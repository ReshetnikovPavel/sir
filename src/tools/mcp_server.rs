use std::process::Stdio;

use rmcp::{service::RunningService, RoleClient, ServiceExt};

use super::config::McpServerTransportConfig;

pub type McpServer = RunningService<RoleClient, ()>;

pub async fn from_config(
    config: &McpServerTransportConfig,
) -> Result<RunningService<RoleClient, ()>, ()> {
    let client = match config {
        McpServerTransportConfig::Sse { url } => {
            let transport = rmcp::transport::sse::SseTransport::start(url.to_owned())
                .await
                .unwrap();
            ().serve(transport).await.unwrap()
        }
        McpServerTransportConfig::Stdio {
            command,
            args,
            envs,
        } => {
            let transport = rmcp::transport::child_process::TokioChildProcess::new(
                tokio::process::Command::new(command)
                    .args(args)
                    .envs(envs)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit()),
            )
            .unwrap();
            ().serve(transport).await.unwrap()
        }
    };
    Ok(client)
}
