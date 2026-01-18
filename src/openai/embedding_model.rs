use secrecy::ExposeSecret as _;
use serde::Deserialize;

use crate::openai::config::OpenAIConfig;

pub struct EmbeddingModel {
    pub client: reqwest::Client,
    pub config: OpenAIConfig,
}

#[derive(Deserialize)]
struct Embedding {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct Embeddings {
    data: Vec<Embedding>,
}

impl EmbeddingModel {
    pub async fn get_embedding(&self, input: String) -> anyhow::Result<Vec<f32>> {
        let url = self.config.api_base.clone() + "/embeddings";
        let body = serde_json::json!({
            "input": input,
            "model": self.config.model,
        });

        let response = self
            .client
            .post(url)
            .body(body.to_string())
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_key.expose_secret()),
            )
            .send()
            .await?
            .error_for_status()?;

        let response = response
            .json::<Embeddings>()
            .await?
            .data
            .into_iter()
            .next()
            .ok_or(anyhow::Error::msg("Embedding vector is empty"))?;

        Ok(response.embedding)
    }

    pub async fn get_embeddings(&self, input: Vec<String>) -> Result<Vec<Vec<f32>>, anyhow::Error> {
        if input.is_empty() {
            return Ok(vec![]);
        }

        let url = self.config.api_base.clone() + "/embeddings";
        let body = serde_json::json!({
            "input": input,
            "model": self.config.model,
        });
        let response = self
            .client
            .post(url)
            .body(body.to_string())
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_key.expose_secret()),
            )
            .send()
            .await?
            .error_for_status()?;

        let response = response.json::<Embeddings>().await.unwrap();
        Ok(response.data.into_iter().map(|emb| emb.embedding).collect())
    }
}
