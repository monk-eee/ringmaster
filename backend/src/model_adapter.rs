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
    /// ADR-0065: bearer token for hosted OpenAI-compatible providers. `None`
    /// for keyless endpoints (Ollama), which send no `Authorization` header.
    pub api_key: Option<String>,
}

impl ModelConfig {
    /// Reads RINGMASTER_LLM_URL / RINGMASTER_MODEL / RINGMASTER_LLM_API_KEY
    /// (ADR-0011, ADR-0065). Returns None when the URL is unset; callers must
    /// treat that as "extraction disabled", never as an error that blocks
    /// ingestion or storage. The API key is optional -- a URL with no key is
    /// valid (Ollama), a URL with a key is valid (hosted).
    pub fn from_env() -> Option<Self> {
        let url = env::var("RINGMASTER_LLM_URL").ok()?;
        let model = env::var("RINGMASTER_MODEL").unwrap_or_else(|_| "default".to_string());
        let api_key = env::var("RINGMASTER_LLM_API_KEY").ok().filter(|key| !key.trim().is_empty());
        Some(Self { url, model, api_key })
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
    let mut request_builder = client.post(format!("{}/chat/completions", config.url.trim_end_matches('/'))).json(&request);
    if let Some(api_key) = &config.api_key {
        request_builder = request_builder.bearer_auth(api_key);
    }
    let response = request_builder.send().await.map_err(ModelAdapterError::Request)?;
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
            api_key: None,
        };
        let result = complete(&config, "extract obligations from: hello").await;
        assert!(
            matches!(result, Err(ModelAdapterError::Request(_))),
            "an unreachable endpoint must return a typed error, not panic"
        );
    }
}
