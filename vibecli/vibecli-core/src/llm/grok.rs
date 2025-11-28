//! Grok (xAI) LLM provider
//! API is compatible with OpenAI

use super::{LLMProvider, Message, MessageRole};
use async_trait::async_trait;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use futures::stream::Stream;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct GrokProvider {
    api_key: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct GrokRequest {
    model: String,
    messages: Vec<GrokMessage>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct GrokMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GrokResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: GrokMessage,
}

impl GrokProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

#[async_trait]
impl LLMProvider for GrokProvider {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let client = reqwest::Client::new();
        
        let grok_messages: Vec<GrokMessage> = messages
            .iter()
            .map(|m| GrokMessage {
                role: match m.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: m.content.clone(),
            })
            .collect();

        let request = GrokRequest {
            model: self.model.clone(),
            messages: grok_messages,
            stream: false,
        };

        let response = client
            .post("https://api.x.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Grok")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Grok API error: {}", error_text);
        }

        let grok_response: GrokResponse = response
            .json()
            .await
            .context("Failed to parse Grok response")?;

        Ok(grok_response.choices[0].message.content.clone())
    }

    async fn stream_chat(&self, _messages: &[Message]) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        // TODO: Implement streaming for Grok (similar to OpenAI SSE)
        anyhow::bail!("Streaming not yet implemented for Grok")
    }

    fn name(&self) -> &str {
        "Grok"
    }
}
