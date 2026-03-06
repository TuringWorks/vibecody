//! Vercel AI Gateway provider — unified proxy, OpenAI-compatible.
//!
//! Routes requests through the user's Vercel AI Gateway instance.
//! Requires `api_url` to be set (gateway endpoint).

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct VercelAIRequest {
    model: String,
    messages: Vec<VercelAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct VercelAIMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct VercelAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct VercelAIResponse {
    choices: Vec<VercelAIChoice>,
    #[serde(default)]
    usage: Option<VercelAIUsage>,
}

#[derive(Debug, Deserialize)]
struct VercelAIChoice {
    message: VercelAIMessage,
}

#[derive(Debug, Deserialize)]
struct VercelAIStreamResponse {
    choices: Vec<VercelAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct VercelAIStreamChoice {
    delta: VercelAIDelta,
}

#[derive(Debug, Deserialize)]
struct VercelAIDelta {
    content: Option<String>,
}

/// Vercel AI Gateway provider — unified proxy to multiple AI services.
pub struct VercelAIProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl VercelAIProvider {
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

    fn base_url(&self) -> Result<String> {
        self.config.api_url.clone().context("Vercel AI Gateway URL not set (vercel_ai.api_url in config)")
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<VercelAIMessage> {
        let mut result: Vec<VercelAIMessage> = messages.iter().map(|m| VercelAIMessage {
            role: m.role.as_str().to_string(),
            content: m.content.clone(),
        }).collect();
        if let Some(ctx) = context {
            if let Some(last) = result.last_mut() {
                if last.role == "user" {
                    last.content = format!("Context:\n{}\n\nUser: {}", ctx, last.content);
                }
            }
        }
        result
    }
}

#[async_trait]
impl AIProvider for VercelAIProvider {
    fn name(&self) -> &str { "VercelAI" }

    async fn is_available(&self) -> bool {
        self.config.api_key.is_some() && self.config.api_url.is_some()
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
        self.chat_response(&messages, None).await
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

    async fn chat_response(&self, messages: &[Message], context: Option<String>) -> Result<CompletionResponse> {
        let api_key = self.config.api_key.as_ref().context("Vercel AI API key not set (VERCEL_AI_API_KEY)")?;
        let base_url = self.base_url()?;
        let request = VercelAIRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };
        let url = format!("{}/chat/completions", base_url);
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send().await.context("Vercel AI request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Vercel AI Gateway error: {}", err);
        }
        let body: VercelAIResponse = resp.json().await.context("Failed to parse Vercel AI response")?;
        let text = body.choices.first().context("No choices")?.message.content.clone();
        let usage = body.usage.map(|u| TokenUsage { prompt_tokens: u.prompt_tokens, completion_tokens: u.completion_tokens });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Vercel AI API key not set")?;
        let base_url = self.base_url()?;
        let request = VercelAIRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, None),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: true,
        };
        let url = format!("{}/chat/completions", base_url);
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send().await.context("Vercel AI stream request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Vercel AI Gateway error: {}", err);
        }
        let stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(r) = serde_json::from_str::<VercelAIStreamResponse>(data) {
                        if let Some(c) = r.choices.first().and_then(|ch| ch.delta.content.as_ref()) {
                            content.push_str(c);
                        }
                    }
                }
            }
            Ok(content)
        }).boxed();
        Ok(stream)
    }

    async fn chat_with_images(&self, messages: &[Message], _images: &[ImageAttachment], context: Option<String>) -> Result<String> {
        self.chat(messages, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "vercel_ai".into(),
            api_key: Some("vai_test".into()),
            api_url: Some("https://my-gateway.vercel.app/v1".into()),
            model: "gpt-4o".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_vercel_ai() {
        let p = VercelAIProvider::new(test_config());
        assert_eq!(p.name(), "VercelAI");
    }

    #[tokio::test]
    async fn is_available_with_key_and_url() {
        let p = VercelAIProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = VercelAIProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_url() {
        let mut cfg = test_config();
        cfg.api_url = None;
        let p = VercelAIProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn vercel_ai_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"proxied"}}],"usage":{"prompt_tokens":6,"completion_tokens":1}}"#;
        let resp: VercelAIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "proxied");
        assert_eq!(resp.usage.unwrap().prompt_tokens, 6);
    }

    #[test]
    fn base_url_returns_configured_value() {
        let p = VercelAIProvider::new(test_config());
        assert_eq!(p.base_url().unwrap(), "https://my-gateway.vercel.app/v1");
    }

    #[test]
    fn base_url_errors_when_not_set() {
        let mut cfg = test_config();
        cfg.api_url = None;
        let p = VercelAIProvider::new(cfg);
        assert!(p.base_url().is_err());
    }

    #[test]
    fn build_messages_maps_roles() {
        use crate::provider::MessageRole;
        let p = VercelAIProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::System, content: "sys".into() },
            Message { role: MessageRole::User, content: "usr".into() },
            Message { role: MessageRole::Assistant, content: "ast".into() },
        ];
        let result = p.build_messages(&messages, None);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    #[test]
    fn build_messages_appends_context() {
        use crate::provider::MessageRole;
        let p = VercelAIProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "ask".into() },
        ];
        let result = p.build_messages(&messages, Some("relevant info".into()));
        assert!(result[0].content.contains("Context:"));
        assert!(result[0].content.contains("relevant info"));
        assert!(result[0].content.contains("ask"));
    }

    #[test]
    fn vercel_ai_request_serializes_correctly() {
        let req = VercelAIRequest {
            model: "gpt-4o".into(),
            messages: vec![VercelAIMessage { role: "user".into(), content: "test".into() }],
            temperature: Some(0.25),
            max_tokens: Some(512),
            stream: true,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["temperature"], 0.25);
        assert_eq!(json["max_tokens"], 512);
        assert_eq!(json["stream"], true);
    }

    #[test]
    fn vercel_ai_response_without_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"no usage"}}]}"#;
        let resp: VercelAIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "no usage");
        assert!(resp.usage.is_none());
    }
}
