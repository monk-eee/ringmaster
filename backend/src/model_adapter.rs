use serde::{Deserialize, Serialize};
use std::env;

/// ADR-0011: mirrors MindLeak's own optional-model precedent. Extraction
/// must degrade cleanly and never block anything else when unconfigured.
#[derive(Debug)]
pub enum ModelAdapterError {
    NotConfigured,
    Request(reqwest::Error),
    UnexpectedResponse(String),
}

impl std::fmt::Display for ModelAdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "RINGMASTER_LLM_URL is not set; extraction is disabled"),
            Self::Request(error) => write!(f, "model request failed: {error}"),
            Self::UnexpectedResponse(reason) => write!(f, "unexpected model response: {reason}"),
        }
    }
}

impl std::error::Error for ModelAdapterError {}

pub struct ModelConfig {
    pub url: String,
    pub model: String,
}

impl ModelConfig {
    /// Reads RINGMASTER_LLM_URL / RINGMASTER_MODEL (ADR-0011). Returns None
    /// when unconfigured; callers must treat that as "extraction disabled",
    /// never as an error that blocks ingestion or storage.
    pub fn from_env() -> Option<Self> {
        let url = env::var("RINGMASTER_LLM_URL").ok()?;
        let model = env::var("RINGMASTER_MODEL").unwrap_or_else(|_| "default".to_string());
        Some(Self { url, model })
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

/// Calls an OpenAI-compatible chat-completion endpoint and returns the raw
/// model text. Returns a typed error and never panics when the endpoint is
/// unreachable or returns something unexpected (ADR-0011). No live model is
/// configured in this environment, so only the unreachable-endpoint path is
/// tested here.
pub async fn complete(config: &ModelConfig, prompt: &str) -> Result<String, ModelAdapterError> {
    let client = reqwest::Client::new();
    let request = ChatRequest {
        model: &config.model,
        messages: vec![ChatMessage { role: "user", content: prompt }],
    };
    let response = client
        .post(format!("{}/chat/completions", config.url.trim_end_matches('/')))
        .json(&request)
        .send()
        .await
        .map_err(ModelAdapterError::Request)?;
    let parsed: ChatResponse = response.json().await.map_err(ModelAdapterError::Request)?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .ok_or_else(|| ModelAdapterError::UnexpectedResponse("no choices in response".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn complete_returns_a_typed_error_without_panicking_when_unreachable() {
        let config = ModelConfig {
            url: "http://127.0.0.1:1".to_string(), // unroutable port: connection refused
            model: "test-model".to_string(),
        };
        let result = complete(&config, "extract obligations from: hello").await;
        assert!(
            matches!(result, Err(ModelAdapterError::Request(_))),
            "an unreachable endpoint must return a typed error, not panic"
        );
    }
}
