use async_openai::{
    error::OpenAIError,
    types::{CreateEmbeddingRequest, EmbeddingInput},
    Client,
};

pub struct OpenAIEmbeddingModel {
    pub client: Client<async_openai::config::OpenAIConfig>,
    pub model: String,
}

impl OpenAIEmbeddingModel {
    pub async fn get_embedding(&self, input: String) -> Result<Vec<f32>, OpenAIError> {
        let request = CreateEmbeddingRequest {
            model: self.model.clone(),
            input: EmbeddingInput::String(input),
            encoding_format: None,
            user: None,
            dimensions: None,
        };
        let response = self.client.embeddings().create(request).await?;
        Ok(response.data.into_iter().next().unwrap().embedding)
    }

    pub async fn get_embeddings(&self, input: Vec<String>) -> Result<Vec<Vec<f32>>, OpenAIError> {
        let request = CreateEmbeddingRequest {
            model: self.model.clone(),
            input: EmbeddingInput::StringArray(input),
            encoding_format: None,
            user: None,
            dimensions: None,
        };
        let response = self.client.embeddings().create(request).await?;
        Ok(response.data.into_iter().map(|x| x.embedding).collect())
    }
}
