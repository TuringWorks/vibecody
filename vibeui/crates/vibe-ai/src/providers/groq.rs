//! Groq provider — OpenAI-compatible ultra-fast inference.
//!
//! Supported models: llama-3.3-70b-versatile, llama-3.1-8b-instant,
//! mixtral-8x7b-32768, gemma2-9b-it, deepseek-r1-distill-llama-70b

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";

#[derive(Debug, Serialize)]
struct GroqRequest {
    model: String,
    messages: Vec<GroqMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroqMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GroqUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
    #[serde(default)]
    usage: Option<GroqUsage>,
}

#[derive(Debug, Deserialize)]
struct GroqChoice {
    message: GroqMessage,
}

#[derive(Debug, Deserialize)]
struct GroqStreamResponse {
    choices: Vec<GroqStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct GroqStreamChoice {
    delta: GroqDelta,
}

#[derive(Debug, Deserialize)]
struct GroqDelta {
    content: Option<String>,
}

/// Groq provider — OpenAI-compatible endpoint, ultra-fast inference.
pub struct GroqProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl GroqProvider {
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

    fn base_url(&self) -> String {
        self.config.api_url.clone().unwrap_or_else(|| GROQ_BASE_URL.to_string())
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<GroqMessage> {
        let mut result: Vec<GroqMessage> = messages.iter().map(|m| GroqMessage {
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
impl AIProvider for GroqProvider {
    fn name(&self) -> &str { "Groq" }

    async fn is_available(&self) -> bool { self.config.api_key.is_some() }

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
        let api_key = self.config.api_key.as_ref().context("Groq API key not set (GROQ_API_KEY)")?;
        let request = GroqRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };
        let url = format!("{}/chat/completions", self.base_url());
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send().await.context("Groq request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Groq API error: {}", err);
        }
        let body: GroqResponse = resp.json().await.context("Failed to parse Groq response")?;
        let text = body.choices.first().context("No choices")?.message.content.clone();
        let usage = body.usage.map(|u| TokenUsage { prompt_tokens: u.prompt_tokens, completion_tokens: u.completion_tokens });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Groq API key not set")?;
        let request = GroqRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, None),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: true,
        };
        let url = format!("{}/chat/completions", self.base_url());
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send().await.context("Groq stream request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Groq API error: {}", err);
        }
        let stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(r) = serde_json::from_str::<GroqStreamResponse>(data) {
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
        // Groq doesn't support vision for most models; fall back to text
        self.chat(messages, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "groq".into(),
            api_key: Some("gsk_test".into()),
            api_url: None,
            model: "llama-3.3-70b-versatile".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_groq() {
        let p = GroqProvider::new(test_config());
        assert_eq!(p.name(), "Groq");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = GroqProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = GroqProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(GROQ_BASE_URL, "https://api.groq.com/openai/v1");
    }

    #[test]
    fn groq_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"fast"}}],"usage":{"prompt_tokens":5,"completion_tokens":1}}"#;
        let resp: GroqResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "fast");
        assert_eq!(resp.usage.unwrap().completion_tokens, 1);
    }

    // ── build_messages: context injection ────────────────────────────────

    #[test]
    fn build_messages_no_context_passthrough() {
        let p = GroqProvider::new(test_config());
        let messages = vec![
            Message { role: crate::provider::MessageRole::System, content: "sys".into() },
            Message { role: crate::provider::MessageRole::User, content: "hello".into() },
        ];
        let result = p.build_messages(&messages, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[0].content, "sys");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[1].content, "hello");
    }

    #[test]
    fn build_messages_context_appended_to_last_user() {
        let p = GroqProvider::new(test_config());
        let messages = vec![
            Message { role: crate::provider::MessageRole::User, content: "explain this".into() },
        ];
        let result = p.build_messages(&messages, Some("fn bar() {}".into()));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "user");
        assert!(result[0].content.starts_with("Context:\nfn bar() {}"));
        assert!(result[0].content.ends_with("User: explain this"));
    }

    #[test]
    fn build_messages_context_not_injected_when_last_is_assistant() {
        let p = GroqProvider::new(test_config());
        let messages = vec![
            Message { role: crate::provider::MessageRole::User, content: "hi".into() },
            Message { role: crate::provider::MessageRole::Assistant, content: "hello back".into() },
        ];
        let result = p.build_messages(&messages, Some("some context".into()));
        assert_eq!(result.len(), 2);
        assert_eq!(result[1].role, "assistant");
        assert_eq!(result[1].content, "hello back"); // unchanged
    }

    #[test]
    fn build_messages_context_injected_into_last_user_in_multi_turn() {
        let p = GroqProvider::new(test_config());
        let messages = vec![
            Message { role: crate::provider::MessageRole::System, content: "prompt".into() },
            Message { role: crate::provider::MessageRole::User, content: "q1".into() },
            Message { role: crate::provider::MessageRole::Assistant, content: "a1".into() },
            Message { role: crate::provider::MessageRole::User, content: "q2".into() },
        ];
        let result = p.build_messages(&messages, Some("ctx".into()));
        assert_eq!(result.len(), 4);
        // First user message untouched
        assert_eq!(result[1].content, "q1");
        // Last user message has context injected
        assert!(result[3].content.contains("ctx"));
        assert!(result[3].content.contains("q2"));
    }

    #[test]
    fn build_messages_empty_messages() {
        let p = GroqProvider::new(test_config());
        let result = p.build_messages(&[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_empty_messages_with_context() {
        let p = GroqProvider::new(test_config());
        let result = p.build_messages(&[], Some("ctx".into()));
        assert!(result.is_empty());
    }

    // ── role mapping ─────────────────────────────────────────────────────

    #[test]
    fn build_messages_maps_roles_correctly() {
        let p = GroqProvider::new(test_config());
        let messages = vec![
            Message { role: crate::provider::MessageRole::System, content: "s".into() },
            Message { role: crate::provider::MessageRole::User, content: "u".into() },
            Message { role: crate::provider::MessageRole::Assistant, content: "a".into() },
        ];
        let result = p.build_messages(&messages, None);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    // ── base_url ─────────────────────────────────────────────────────────

    #[test]
    fn base_url_default() {
        let p = GroqProvider::new(test_config());
        assert_eq!(p.base_url(), "https://api.groq.com/openai/v1");
    }

    #[test]
    fn base_url_custom() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://custom.groq.proxy/v1".into());
        let p = GroqProvider::new(cfg);
        assert_eq!(p.base_url(), "https://custom.groq.proxy/v1");
    }

    // ── request serde ────────────────────────────────────────────────────

    #[test]
    fn groq_request_omits_none_fields() {
        let req = GroqRequest {
            model: "llama-3.3-70b-versatile".into(),
            messages: vec![GroqMessage { role: "user".into(), content: "hi".into() }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("temperature"), "temperature should be omitted when None");
        assert!(!json.contains("max_tokens"), "max_tokens should be omitted when None");
    }

    #[test]
    fn groq_request_includes_set_fields() {
        let req = GroqRequest {
            model: "llama-3.3-70b-versatile".into(),
            messages: vec![],
            temperature: Some(0.5),
            max_tokens: Some(4096),
            stream: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"temperature\""));
        assert!(json.contains("\"max_tokens\""));
        assert!(json.contains("\"stream\":true"));
    }

    #[test]
    fn groq_response_deser_without_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#;
        let resp: GroqResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "ok");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn groq_stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"chunk"}}]}"#;
        let resp: GroqStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_deref(), Some("chunk"));
    }

    #[test]
    fn groq_stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{}}]}"#;
        let resp: GroqStreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }
}
