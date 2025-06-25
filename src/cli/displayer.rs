use async_openai::types::ChatChoiceStream;
use async_trait::async_trait;
use rmcp::model::CallToolResult;

use crate::chat::displayer::Displayer;

pub struct CliChunkDisplayer {}

impl CliChunkDisplayer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Displayer for CliChunkDisplayer {
    async fn display_chunk(&self, chunk: &ChatChoiceStream) {
        if chunk.delta.tool_calls.is_some() {
            return;
        }
        if let Some(content) = &chunk.delta.content {
            print!("{}", content);
        }
        if let Some(_) = &chunk.finish_reason {
            println!();
        }
    }

    async fn display_tool_call_result(&self, result: &CallToolResult) {
        for content in &result.content {
            if let Some(text_content) = content.as_text() {
                println!("Результат: `{}`", text_content.text);
            }
        }
    }
}
