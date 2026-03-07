//! Claude AI provider implementation

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
struct ThinkingConfig {
    #[serde(rename = "type")]
    thinking_type: String,
    budget_tokens: u32,
}

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
    /// Extended thinking — only serialized when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingConfig>,
}

/// Supports both text-only (String) and vision (array of content blocks).
#[derive(Debug, Serialize)]
struct ClaudeMessage {
    role: String,
    content: Value,  // String or array for vision
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
    #[serde(default)]
    usage: Option<ClaudeUsage>,
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
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    const DEFAULT_API_URL: &'static str = "https://api.anthropic.com/v1/messages";

    fn api_url(&self) -> &str {
        self.config.api_url.as_deref().unwrap_or(Self::DEFAULT_API_URL)
    }

    /// Translate a raw Claude API error response into a user-friendly message.
    fn translate_api_error(status: u16, body: &str) -> String {
        // Try to parse as JSON to extract the message field
        if let Ok(v) = serde_json::from_str::<Value>(body) {
            let msg = v.pointer("/error/message")
                .and_then(|m| m.as_str())
                .unwrap_or(body);
            let err_type = v.pointer("/error/type")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            return match (status, err_type) {
                (401, _) => format!("Authentication failed: {}. Check your ANTHROPIC_API_KEY or api_key_helper in config.", msg),
                (403, _) => format!("Access denied: {}. Your API key may lack permissions for this model.", msg),
                (429, _) => format!("Rate limited: {}. Wait a moment or check your Anthropic plan limits.", msg),
                (404, _) => format!("Model not found: {}. Check your model name in config.", msg),
                (529, _) | (503, _) => format!("Claude is temporarily overloaded: {}. Retry in a few seconds.", msg),
                _ => format!("Claude API error (HTTP {}): {}", status, msg),
            };
        }
        format!("Claude API error (HTTP {}): {}", status, body)
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
                    role: m.role.as_str().to_string(),
                    content,
                });
            }
        }

        (claude_messages, system_prompt)
    }

    /// Build the optional extended-thinking config from provider settings.
    fn thinking_config(&self) -> Option<ThinkingConfig> {
        self.config.thinking_budget_tokens.map(|budget| ThinkingConfig {
            thinking_type: "enabled".to_string(),
            budget_tokens: budget,
        })
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
                role: m.role.as_str().to_string(),
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
        self.chat_response(&messages, None).await
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

    async fn chat_response(&self, messages: &[Message], context: Option<String>) -> Result<CompletionResponse> {
        let api_key = self.config.resolve_api_key().await.context("Claude API key not found")?;
        let (claude_messages, system) = self.build_messages(messages, context);

        let request = ClaudeRequest {
            model: self.config.model.clone(),
            messages: claude_messages,
            max_tokens: self.config.max_tokens.or(Some(4096)),
            temperature: self.config.temperature,
            stream: false,
            system,
            thinking: self.thinking_config(),
        };

        let response = self.client
            .post(self.api_url())
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Claude")?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await?;
            anyhow::bail!("{}", Self::translate_api_error(status, &error_text));
        }

        let claude_response: ClaudeResponse = response.json().await.context("Failed to parse Claude response")?;

        let text = claude_response.content.first()
            .map(|c| c.text.clone())
            .context("No content in Claude response")?;

        let usage = claude_response.usage.map(|u| TokenUsage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
        });

        Ok(CompletionResponse {
            text,
            model: self.config.model.clone(),
            usage,
        })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let api_key = self.config.resolve_api_key().await.context("Claude API key not found")?;
        let (claude_messages, system) = self.build_messages(messages, context);

        let request = ClaudeRequest {
            model: self.config.model.clone(),
            messages: claude_messages,
            max_tokens: self.config.max_tokens.or(Some(4096)),
            temperature: self.config.temperature,
            stream: false,
            system,
            thinking: self.thinking_config(),
        };

        let response = self.client
            .post(self.api_url())
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Claude")?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await?;
            anyhow::bail!("{}", Self::translate_api_error(status, &error_text));
        }

        let claude_response: ClaudeResponse = response.json().await.context("Failed to parse Claude response")?;
        
        claude_response.content.first()
            .map(|c| c.text.clone())
            .context("No content in Claude response")
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.resolve_api_key().await.context("Claude API key not found")?;
        let (claude_messages, system) = self.build_messages(messages, None);

        let request = ClaudeRequest {
            model: self.config.model.clone(),
            messages: claude_messages,
            max_tokens: self.config.max_tokens.or(Some(4096)),
            temperature: self.config.temperature,
            stream: true,
            system,
            thinking: self.thinking_config(),
        };

        let response = self.client
            .post(self.api_url())
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Claude")?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await?;
            anyhow::bail!("{}", Self::translate_api_error(status, &error_text));
        }

        let stream = response.bytes_stream();
        
        let completion_stream = stream
            .map(|chunk| {
                let chunk = chunk?;
                let chunk_str = String::from_utf8_lossy(&chunk);
                let mut content = String::new();
                
                for line in chunk_str.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
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
        let api_key = self.config.resolve_api_key().await.context("Claude API key not found")?;
        let (claude_messages, system) = self.build_vision_messages(messages, images);

        let request = ClaudeRequest {
            model: self.config.model.clone(),
            messages: claude_messages,
            max_tokens: self.config.max_tokens.or(Some(4096)),
            temperature: self.config.temperature,
            stream: false,
            system,
            thinking: self.thinking_config(),
        };

        let response = self
            .client
            .post(self.api_url())
            .header("x-api-key", &api_key)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MessageRole;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "claude".into(),
            api_key: Some("test-key".into()),
            api_url: Some("https://api.anthropic.com".into()),
            model: "claude-sonnet-4-20250514".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_claude() {
        let p = ClaudeProvider::new(test_config());
        assert_eq!(p.name(), "Claude");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = ClaudeProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn is_not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = ClaudeProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn supports_vision() {
        let p = ClaudeProvider::new(test_config());
        assert!(p.supports_vision());
    }

    #[test]
    fn build_messages_extracts_system() {
        let p = ClaudeProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::System, content: "sys".into() },
            Message { role: MessageRole::User, content: "hi".into() },
        ];
        let (claude_msgs, sys) = p.build_messages(&msgs, None);
        assert_eq!(sys.as_deref(), Some("sys"));
        assert_eq!(claude_msgs.len(), 1);
        assert_eq!(claude_msgs[0].role, "user");
    }

    #[test]
    fn claude_request_serde() {
        let req = ClaudeRequest {
            model: "claude-sonnet-4-20250514".into(),
            messages: vec![ClaudeMessage { role: "user".into(), content: Value::String("hi".into()) }],
            max_tokens: Some(1024),
            temperature: None,
            stream: false,
            system: None,
            thinking: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "claude-sonnet-4-20250514");
        assert!(json.get("system").is_none()); // skip_serializing_if
        assert!(json.get("thinking").is_none());
    }

    #[test]
    fn claude_response_deser() {
        let json = r#"{"content":[{"text":"hello world"}],"usage":{"input_tokens":10,"output_tokens":5}}"#;
        let resp: ClaudeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.content[0].text, "hello world");
        assert_eq!(resp.usage.unwrap().output_tokens, 5);
    }
}
