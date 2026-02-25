//! xAI Grok provider implementation

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

// Grok API is compatible with OpenAI
#[derive(Debug, Serialize)]
struct GrokRequest {
    model: String,
    messages: Vec<GrokMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct GrokMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GrokResponse {
    choices: Vec<GrokChoice>,
}

#[derive(Debug, Deserialize)]
struct GrokChoice {
    message: GrokMessage,
}

#[derive(Debug, Deserialize)]
struct GrokStreamResponse {
    choices: Vec<GrokStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct GrokStreamChoice {
    delta: GrokDelta,
}

#[derive(Debug, Deserialize)]
struct GrokDelta {
    content: Option<String>,
}

/// Grok provider
pub struct GrokProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl GrokProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<GrokMessage> {
        let mut grok_messages: Vec<GrokMessage> = messages
            .iter()
            .map(|m| GrokMessage {
                role: format!("{:?}", m.role).to_lowercase(),
                content: m.content.clone(),
            })
            .collect();

        if let Some(ctx) = context {
            if let Some(last_msg) = grok_messages.last_mut() {
                if last_msg.role == "user" {
                    last_msg.content = format!("Context:\n{}\n\nUser: {}", ctx, last_msg.content);
                }
            }
        }
        grok_messages
    }
}

#[async_trait]
impl AIProvider for GrokProvider {
    fn name(&self) -> &str {
        "Grok"
    }

    async fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        
        let messages = vec![
            Message { role: crate::provider::MessageRole::System, content: "You are a helpful coding assistant.".to_string() },
            Message { role: crate::provider::MessageRole::User, content: prompt },
        ];

        let response_text = self.chat(&messages, None).await?;

        Ok(CompletionResponse {
            text: response_text,
            model: self.config.model.clone(),
            usage: None,
        })
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        
        let messages = vec![
            Message { role: crate::provider::MessageRole::System, content: "You are a helpful coding assistant.".to_string() },
            Message { role: crate::provider::MessageRole::User, content: prompt },
        ];

        self.stream_chat(&messages).await
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let api_key = self.config.api_key.as_ref().context("Grok API key not found")?;
        let request = GrokRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };

        let response = self.client
            .post("https://api.x.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Grok")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Grok API error: {}", error_text);
        }

        let grok_response: GrokResponse = response.json().await.context("Failed to parse Grok response")?;
        
        grok_response.choices.first()
            .map(|c| c.message.content.clone())
            .context("No choices in Grok response")
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Grok API key not found")?;
        let request = GrokRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, None),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: true,
        };

        let response = self.client
            .post("https://api.x.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Grok")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Grok API error: {}", error_text);
        }

        let stream = response.bytes_stream();
        
        let completion_stream = stream
            .map(|chunk| {
                let chunk = chunk?;
                let chunk_str = String::from_utf8_lossy(&chunk);
                let mut content = String::new();
                
                for line in chunk_str.lines() {
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" {
                            continue;
                        }
                        if let Ok(response) = serde_json::from_str::<GrokStreamResponse>(data) {
                            if let Some(choice) = response.choices.first() {
                                if let Some(delta_content) = &choice.delta.content {
                                    content.push_str(delta_content);
                                }
                            }
                        }
                    }
                }
                Ok(content)
            })
            .boxed();

        Ok(completion_stream)
    }
}
