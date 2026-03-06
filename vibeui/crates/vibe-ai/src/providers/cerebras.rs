//! Cerebras provider — ultra-fast inference, OpenAI-compatible API.
//!
//! Supported models: llama3.1-70b, llama3.1-8b, llama-3.3-70b

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

const CEREBRAS_BASE_URL: &str = "https://api.cerebras.ai/v1";

#[derive(Debug, Serialize)]
struct CerebrasRequest {
    model: String,
    messages: Vec<CerebrasMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct CerebrasMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct CerebrasUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct CerebrasResponse {
    choices: Vec<CerebrasChoice>,
    #[serde(default)]
    usage: Option<CerebrasUsage>,
}

#[derive(Debug, Deserialize)]
struct CerebrasChoice {
    message: CerebrasMessage,
}

#[derive(Debug, Deserialize)]
struct CerebrasStreamResponse {
    choices: Vec<CerebrasStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct CerebrasStreamChoice {
    delta: CerebrasDelta,
}

#[derive(Debug, Deserialize)]
struct CerebrasDelta {
    content: Option<String>,
}

/// Cerebras provider — ultra-fast inference via dedicated hardware.
pub struct CerebrasProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl CerebrasProvider {
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
        self.config.api_url.clone().unwrap_or_else(|| CEREBRAS_BASE_URL.to_string())
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<CerebrasMessage> {
        let mut result: Vec<CerebrasMessage> = messages.iter().map(|m| CerebrasMessage {
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
impl AIProvider for CerebrasProvider {
    fn name(&self) -> &str { "Cerebras" }

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
        let api_key = self.config.api_key.as_ref().context("Cerebras API key not set (CEREBRAS_API_KEY)")?;
        let request = CerebrasRequest {
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
            .send().await.context("Cerebras request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Cerebras API error: {}", err);
        }
        let body: CerebrasResponse = resp.json().await.context("Failed to parse Cerebras response")?;
        let text = body.choices.first().context("No choices")?.message.content.clone();
        let usage = body.usage.map(|u| TokenUsage { prompt_tokens: u.prompt_tokens, completion_tokens: u.completion_tokens });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Cerebras API key not set")?;
        let request = CerebrasRequest {
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
            .send().await.context("Cerebras stream request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Cerebras API error: {}", err);
        }
        let stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(r) = serde_json::from_str::<CerebrasStreamResponse>(data) {
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
            provider_type: "cerebras".into(),
            api_key: Some("csk_test".into()),
            api_url: None,
            model: "llama3.1-70b".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_cerebras() {
        let p = CerebrasProvider::new(test_config());
        assert_eq!(p.name(), "Cerebras");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = CerebrasProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = CerebrasProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(CEREBRAS_BASE_URL, "https://api.cerebras.ai/v1");
    }

    #[test]
    fn cerebras_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"fast"}}],"usage":{"prompt_tokens":10,"completion_tokens":2}}"#;
        let resp: CerebrasResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "fast");
        assert_eq!(resp.usage.unwrap().prompt_tokens, 10);
    }

    // ── build_messages context injection ────────────────────────────────

    #[test]
    fn build_messages_without_context() {
        use crate::provider::MessageRole;
        let p = CerebrasProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::System, content: "You are helpful.".into() },
            Message { role: MessageRole::User, content: "Hello".into() },
        ];
        let result = p.build_messages(&msgs, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[0].content, "You are helpful.");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[1].content, "Hello");
    }

    #[test]
    fn build_messages_with_context_injects_into_last_user() {
        use crate::provider::MessageRole;
        let p = CerebrasProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::User, content: "Explain".into() },
        ];
        let result = p.build_messages(&msgs, Some("source code here".into()));
        assert_eq!(result.len(), 1);
        assert!(result[0].content.starts_with("Context:\nsource code here"));
        assert!(result[0].content.contains("User: Explain"));
    }

    #[test]
    fn build_messages_context_only_last_user_affected() {
        use crate::provider::MessageRole;
        let p = CerebrasProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::User, content: "First".into() },
            Message { role: MessageRole::Assistant, content: "Mid".into() },
            Message { role: MessageRole::User, content: "Last".into() },
        ];
        let result = p.build_messages(&msgs, Some("ctx".into()));
        // First user unchanged
        assert_eq!(result[0].content, "First");
        // Last user gets context
        assert!(result[2].content.starts_with("Context:\nctx"));
        assert!(result[2].content.contains("User: Last"));
    }

    #[test]
    fn build_messages_context_not_injected_when_last_is_assistant() {
        use crate::provider::MessageRole;
        let p = CerebrasProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::User, content: "Q".into() },
            Message { role: MessageRole::Assistant, content: "A".into() },
        ];
        let result = p.build_messages(&msgs, Some("ignored context".into()));
        assert_eq!(result[1].content, "A");
        assert_eq!(result[0].content, "Q");
    }

    #[test]
    fn build_messages_empty_input() {
        let p = CerebrasProvider::new(test_config());
        let result = p.build_messages(&[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_empty_input_with_context() {
        let p = CerebrasProvider::new(test_config());
        let result = p.build_messages(&[], Some("ctx".into()));
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_role_mapping() {
        use crate::provider::MessageRole;
        let p = CerebrasProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::System, content: "s".into() },
            Message { role: MessageRole::User, content: "u".into() },
            Message { role: MessageRole::Assistant, content: "a".into() },
        ];
        let result = p.build_messages(&msgs, None);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    // ── name / availability ─────────────────────────────────────────────

    #[test]
    fn name_returns_cerebras() {
        let p = CerebrasProvider::new(test_config());
        assert_eq!(p.name(), "Cerebras");
    }

    #[tokio::test]
    async fn availability_tracks_api_key() {
        let p_with = CerebrasProvider::new(test_config());
        assert!(p_with.is_available().await);

        let mut cfg = test_config();
        cfg.api_key = None;
        let p_without = CerebrasProvider::new(cfg);
        assert!(!p_without.is_available().await);
    }

    // ── base_url ────────────────────────────────────────────────────────

    #[test]
    fn base_url_custom_override() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://cerebras-proxy.example.com/v1".into());
        let p = CerebrasProvider::new(cfg);
        assert_eq!(p.base_url(), "https://cerebras-proxy.example.com/v1");
    }

    #[test]
    fn base_url_default_value() {
        let p = CerebrasProvider::new(test_config());
        assert_eq!(p.base_url(), "https://api.cerebras.ai/v1");
    }

    // ── request / response serialization ────────────────────────────────

    #[test]
    fn cerebras_request_serde_minimal() {
        let req = CerebrasRequest {
            model: "llama3.1-70b".into(),
            messages: vec![CerebrasMessage { role: "user".into(), content: "hi".into() }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["model"], "llama3.1-70b");
        assert_eq!(val["stream"], false);
        assert!(val.get("temperature").is_none());
        assert!(val.get("max_tokens").is_none());
    }

    #[test]
    fn cerebras_request_serde_full() {
        let req = CerebrasRequest {
            model: "llama-3.3-70b".into(),
            messages: vec![
                CerebrasMessage { role: "system".into(), content: "sys".into() },
                CerebrasMessage { role: "user".into(), content: "usr".into() },
            ],
            temperature: Some(0.8),
            max_tokens: Some(512),
            stream: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["temperature"], 0.8);
        assert_eq!(val["max_tokens"], 512);
        assert_eq!(val["stream"], true);
        assert_eq!(val["messages"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn cerebras_response_deser_no_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#;
        let resp: CerebrasResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "ok");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn cerebras_stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"tok"}}]}"#;
        let resp: CerebrasStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "tok");
    }

    #[test]
    fn cerebras_stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{"content":null}}]}"#;
        let resp: CerebrasStreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    #[test]
    fn cerebras_message_roundtrip() {
        let msg = CerebrasMessage { role: "user".into(), content: "test data".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: CerebrasMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }
}
