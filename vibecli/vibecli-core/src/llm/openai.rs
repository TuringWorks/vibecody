//! OpenAI LLM provider

use super::{LLMProvider, Message, MessageRole};
use async_trait::async_trait;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use futures::stream::Stream;
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    api_key: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: OpenAIMessage,
}

impl OpenAIProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let client = reqwest::Client::new();
        
        let openai_messages: Vec<OpenAIMessage> = messages
            .iter()
            .map(|m| OpenAIMessage {
                role: match m.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: m.content.clone(),
            })
            .collect();

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: openai_messages,
        };

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        Ok(openai_response.choices[0].message.content.clone())
    }

    async fn stream_chat(&self, _messages: &[Message]) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        // TODO: Implement streaming for OpenAI
        // OpenAI supports streaming via SSE (Server-Sent Events)
        anyhow::bail!("Streaming not yet implemented for OpenAI")
    }

    fn name(&self) -> &str {
        "OpenAI"
    }
}
