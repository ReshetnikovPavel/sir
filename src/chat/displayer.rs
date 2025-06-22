use async_openai::types::ChatChoiceStream;
use async_trait::async_trait;

#[async_trait]
pub trait ChunkDisplayer {
    async fn display_chunk(&self, choice: &ChatChoiceStream);
}
