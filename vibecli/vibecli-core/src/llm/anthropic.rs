//! Anthropic (Claude) LLM provider

use super::{LLMProvider, Message, MessageRole};
use async_trait::async_trait;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use futures::stream::Stream;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    api_key: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    text: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let client = reqwest::Client::new();
        
        let anthropic_messages: Vec<AnthropicMessage> = messages
            .iter()
            .filter(|m| !matches!(m.role, MessageRole::System))
            .map(|m| AnthropicMessage {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => "user".to_string(), // Fallback
                },
                content: m.content.clone(),
            })
            .collect();

        let request = AnthropicRequest {
            model: self.model.clone(),
            messages: anthropic_messages,
            max_tokens: 4096,
        };

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Anthropic")?;

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        Ok(anthropic_response.content[0].text.clone())
    }

    async fn stream_chat(&self, _messages: &[Message]) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        // TODO: Implement streaming for Anthropic
        // Anthropic supports streaming via SSE
        anyhow::bail!("Streaming not yet implemented for Anthropic")
    }

    fn name(&self) -> &str {
        "Anthropic (Claude)"
    }
}
