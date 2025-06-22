use async_openai::types::ChatChoiceStream;
use async_trait::async_trait;

use crate::chat::displayer::ChunkDisplayer;

pub struct CliChunkDisplayer {}

impl CliChunkDisplayer {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ChunkDisplayer for CliChunkDisplayer {
    async fn display_chunk(&self, chunk: &ChatChoiceStream) {
        if let Some(content) = &chunk.delta.content {
            print!("{}", content);
        }
        if let Some(_) = &chunk.finish_reason {
            println!();
        }
    }
}
