use async_openai::types::ChatChoiceStream;
use async_trait::async_trait;
use rmcp::model::CallToolResult;

#[async_trait]
pub trait Displayer {
    async fn display_chunk(&self, choice: &ChatChoiceStream);
    async fn display_tool_call_result(&self, result: &CallToolResult);
}
