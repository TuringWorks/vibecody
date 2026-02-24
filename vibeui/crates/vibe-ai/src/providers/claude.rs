//! Claude AI provider implementation

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

/// Supports both text-only (String) and vision (array of content blocks).
#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: Value,  // String or array for vision
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
struct ClaudeContent {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeStreamResponse {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<ClaudeDelta>,
}

#[derive(Debug, Deserialize)]
struct ClaudeDelta {
    text: Option<String>,
}

/// Claude AI provider (Anthropic)
pub struct ClaudeProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> (Vec<ClaudeMessage>, Option<String>) {
        let mut claude_messages = Vec::new();
        let mut system_prompt = None;

        for m in messages {
            if let crate::provider::MessageRole::System = m.role {
                system_prompt = Some(m.content.clone());
            } else {
                let content = if let Some(ctx) = context.as_ref() {
                    if m.role == crate::provider::MessageRole::User
                        && claude_messages.iter().all(|cm: &ClaudeMessage| cm.role != "user")
                    {
                        Value::String(format!("Context:\n{}\n\nUser: {}", ctx, m.content))
                    } else {
                        Value::String(m.content.clone())
                    }
                } else {
                    Value::String(m.content.clone())
                };
                claude_messages.push(ClaudeMessage {
                    role: format!("{:?}", m.role).to_lowercase(),
                    content,
                });
            }
        }

        (claude_messages, system_prompt)
    }

    /// Build a vision request message with text + images.
    fn build_vision_messages(
        &self,
        messages: &[Message],
        images: &[ImageAttachment],
    ) -> (Vec<ClaudeMessage>, Option<String>) {
        let mut claude_messages = Vec::new();
        let mut system_prompt = None;

        for (i, m) in messages.iter().enumerate() {
            if let crate::provider::MessageRole::System = m.role {
                system_prompt = Some(m.content.clone());
                continue;
            }
            // Attach images to the last user message.
            let is_last_user = m.role == crate::provider::MessageRole::User
                && i == messages.len() - 1;
            let content = if is_last_user && !images.is_empty() {
                let mut blocks: Vec<Value> = images
                    .iter()
                    .map(|img| {
                        serde_json::json!({
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": img.media_type,
                                "data": img.base64,
                            }
                        })
                    })
                    .collect();
                blocks.push(serde_json::json!({ "type": "text", "text": m.content }));
                Value::Array(blocks)
            } else {
                Value::String(m.content.clone())
            };
            claude_messages.push(ClaudeMessage {
                role: format!("{:?}", m.role).to_lowercase(),
                content,
            });
        }

        (claude_messages, system_prompt)
    }
}

#[async_trait]
impl AIProvider for ClaudeProvider {
    fn name(&self) -> &str {
        "Claude"
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
            Message { role: crate::provider::MessageRole::User, content: prompt },
        ];

        let response_text = self.chat(&messages, None).await?;

        Ok(CompletionResponse {
            text: response_text,
            model: self.config.model.clone(),
        })
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        
        let messages = vec![
            Message { role: crate::provider::MessageRole::User, content: prompt },
        ];

        self.stream_chat(&messages).await
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let api_key = self.config.api_key.as_ref().context("Claude API key not found")?;
        let (claude_messages, system) = self.build_messages(messages, context);

        let request = ClaudeRequest {
            model: self.config.model.clone(),
            messages: claude_messages,
            max_tokens: self.config.max_tokens.or(Some(4096)), // Default max tokens for Claude
            temperature: self.config.temperature,
            stream: false,
            system,
        };

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Claude")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Claude API error: {}", error_text);
        }

        let claude_response: ClaudeResponse = response.json().await.context("Failed to parse Claude response")?;
        
        claude_response.content.first()
            .map(|c| c.text.clone())
            .context("No content in Claude response")
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Claude API key not found")?;
        let (claude_messages, system) = self.build_messages(messages, None);

        let request = ClaudeRequest {
            model: self.config.model.clone(),
            messages: claude_messages,
            max_tokens: self.config.max_tokens.or(Some(4096)),
            temperature: self.config.temperature,
            stream: true,
            system,
        };

        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Claude")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Claude API error: {}", error_text);
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
                        if let Ok(response) = serde_json::from_str::<ClaudeStreamResponse>(data) {
                            if response.event_type == "content_block_delta" {
                                if let Some(delta) = response.delta {
                                    if let Some(text) = delta.text {
                                        content.push_str(&text);
                                    }
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

    fn supports_vision(&self) -> bool {
        // Claude 3+ models support vision.
        self.config.model.contains("claude-3") || self.config.model.contains("claude-sonnet")
            || self.config.model.contains("claude-opus") || self.config.model.contains("claude-haiku")
    }

    async fn chat_with_images(
        &self,
        messages: &[Message],
        images: &[ImageAttachment],
        _context: Option<String>,
    ) -> Result<String> {
        let api_key = self.config.api_key.as_ref().context("Claude API key not found")?;
        let (claude_messages, system) = self.build_vision_messages(messages, images);

        let request = ClaudeRequest {
            model: self.config.model.clone(),
            messages: claude_messages,
            max_tokens: self.config.max_tokens.or(Some(4096)),
            temperature: self.config.temperature,
            stream: false,
            system,
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send vision request to Claude")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Claude vision API error: {}", error_text);
        }

        let claude_response: ClaudeResponse =
            response.json().await.context("Failed to parse Claude vision response")?;
        claude_response
            .content
            .first()
            .map(|c| c.text.clone())
            .context("No content in Claude vision response")
    }
}
