use serde::{Deserialize, Serialize};
use std::env;

/// ADR-0018: mirrors model_adapter.rs's optional-model pattern (ADR-0011).
/// Embedding generation must degrade cleanly and never block ingestion,
/// extraction, or storage when unconfigured.
#[derive(Debug)]
pub enum EmbeddingAdapterError {
    NotConfigured,
    Request(reqwest::Error),
    UnexpectedResponse(String),
}

impl std::fmt::Display for EmbeddingAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "RINGMASTER_EMBEDDING_URL is not set; embedding is disabled"),
            Self::Request(error) => write!(f, "embedding request failed: {error}"),
            Self::UnexpectedResponse(reason) => write!(f, "unexpected embedding response: {reason}"),
        }
    }
}

impl std::error::Error for EmbeddingAdapterError {}

pub struct EmbeddingConfig {
    pub url: String,
    pub model: String,
}

impl EmbeddingConfig {
    /// Reads RINGMASTER_EMBEDDING_URL / RINGMASTER_EMBEDDING_MODEL
    /// (ADR-0018). Returns None when unconfigured; callers must treat that
    /// as "embedding disabled", never as an error that blocks anything else.
    pub fn from_env() -> Option<Self> {
        let url = env::var("RINGMASTER_EMBEDDING_URL").ok()?;
        let model = env::var("RINGMASTER_EMBEDDING_MODEL").unwrap_or_else(|_| "default".to_string());
        Some(Self { url, model })
    }
}

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

/// Calls an OpenAI-compatible embeddings endpoint and returns the vector.
/// Returns a typed error and never panics when the endpoint is unreachable
/// or returns something unexpected (ADR-0018).
pub async fn embed(config: &EmbeddingConfig, text: &str) -> Result<Vec<f32>, EmbeddingAdapterError> {
    let client = reqwest::Client::new();
    let request = EmbeddingRequest { model: &config.model, input: text };
    let response = client
        .post(format!("{}/embeddings", config.url.trim_end_matches('/')))
        .json(&request)
        .send()
        .await
        .map_err(EmbeddingAdapterError::Request)?;
    let parsed: EmbeddingResponse = response.json().await.map_err(EmbeddingAdapterError::Request)?;
    parsed
        .data
        .into_iter()
        .next()
        .map(|data| data.embedding)
        .ok_or_else(|| EmbeddingAdapterError::UnexpectedResponse("no data in response".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn embed_returns_a_typed_error_without_panicking_when_unreachable() {
        let config = EmbeddingConfig { url: "http://127.0.0.1:1".to_string(), model: "test-model".to_string() };
        let result = embed(&config, "some text").await;
        assert!(matches!(result, Err(EmbeddingAdapterError::Request(_))));
    }

    #[test]
    fn from_env_returns_none_when_unconfigured() {
        std::env::remove_var("RINGMASTER_EMBEDDING_URL");
        assert!(EmbeddingConfig::from_env().is_none());
    }
}
