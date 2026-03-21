//! SambaNova provider — fast inference, OpenAI-compatible API.
//!
//! Supported models: Meta-Llama-3.1-70B-Instruct and other hosted models

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

const SAMBANOVA_BASE_URL: &str = "https://api.sambanova.ai/v1";

#[derive(Debug, Serialize)]
struct SambaNovaRequest {
    model: String,
    messages: Vec<SambaNovaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct SambaNovaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct SambaNovaUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct SambaNovaResponse {
    choices: Vec<SambaNovaChoice>,
    #[serde(default)]
    usage: Option<SambaNovaUsage>,
}

#[derive(Debug, Deserialize)]
struct SambaNovaChoice {
    message: SambaNovaMessage,
}

#[derive(Debug, Deserialize)]
struct SambaNovaStreamResponse {
    choices: Vec<SambaNovaStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct SambaNovaStreamChoice {
    delta: SambaNovaDelta,
}

#[derive(Debug, Deserialize)]
struct SambaNovaDelta {
    content: Option<String>,
}

/// SambaNova provider — fast inference with OpenAI-compatible API.
pub struct SambaNovaProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl SambaNovaProvider {
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
        self.config.api_url.clone().unwrap_or_else(|| SAMBANOVA_BASE_URL.to_string())
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<SambaNovaMessage> {
        let mut result: Vec<SambaNovaMessage> = messages.iter().map(|m| SambaNovaMessage {
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
impl AIProvider for SambaNovaProvider {
    fn name(&self) -> &str { "SambaNova" }

    async fn is_available(&self) -> bool { self.config.api_key.is_some() }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        let messages = vec![
            Message { role: crate::provider::MessageRole::System, content: "You are a helpful coding assistant specializing in code completion.".to_string() },
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
            Message { role: crate::provider::MessageRole::System, content: "You are a helpful coding assistant specializing in code completion.".to_string() },
            Message { role: crate::provider::MessageRole::User, content: prompt },
        ];
        self.stream_chat(&messages).await
    }

    async fn chat_response(&self, messages: &[Message], context: Option<String>) -> Result<CompletionResponse> {
        let api_key = self.config.api_key.as_ref().context("SambaNova API key not set (SAMBANOVA_API_KEY)")?;
        let request = SambaNovaRequest {
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
            .send().await.context("SambaNova request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("SambaNova API error: {}", err);
        }
        let body: SambaNovaResponse = resp.json().await.context("Failed to parse SambaNova response")?;
        let text = body.choices.first().context("No choices")?.message.content.clone();
        let usage = body.usage.map(|u| TokenUsage { prompt_tokens: u.prompt_tokens, completion_tokens: u.completion_tokens });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("SambaNova API key not set")?;
        let request = SambaNovaRequest {
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
            .send().await.context("SambaNova stream request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("SambaNova API error: {}", err);
        }
        let stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(r) = serde_json::from_str::<SambaNovaStreamResponse>(data) {
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
            provider_type: "sambanova".into(),
            api_key: Some("sk-test".into()),
            api_url: None,
            model: "Meta-Llama-3.1-70B-Instruct".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_sambanova() {
        let p = SambaNovaProvider::new(test_config());
        assert_eq!(p.name(), "SambaNova");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = SambaNovaProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = SambaNovaProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(SAMBANOVA_BASE_URL, "https://api.sambanova.ai/v1");
    }

    #[test]
    fn sambanova_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"code"}}],"usage":{"prompt_tokens":8,"completion_tokens":3}}"#;
        let resp: SambaNovaResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "code");
        assert_eq!(resp.usage.unwrap().completion_tokens, 3);
    }

    #[test]
    fn build_messages_without_context() {
        use crate::provider::MessageRole;
        let p = SambaNovaProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::System, content: "System prompt".into() },
            Message { role: MessageRole::User, content: "Help me".into() },
        ];
        let result = p.build_messages(&msgs, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[0].content, "System prompt");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[1].content, "Help me");
    }

    #[test]
    fn build_messages_with_context_prepends_to_last_user() {
        use crate::provider::MessageRole;
        let p = SambaNovaProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::User, content: "Review this".into() },
        ];
        let result = p.build_messages(&msgs, Some("fn main() {}".into()));
        assert_eq!(result.len(), 1);
        assert!(result[0].content.starts_with("Context:\nfn main() {}"));
        assert!(result[0].content.contains("User: Review this"));
    }

    #[test]
    fn build_messages_context_only_modifies_last_user() {
        use crate::provider::MessageRole;
        let p = SambaNovaProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::User, content: "First Q".into() },
            Message { role: MessageRole::Assistant, content: "First A".into() },
            Message { role: MessageRole::User, content: "Second Q".into() },
        ];
        let result = p.build_messages(&msgs, Some("context data".into()));
        assert_eq!(result[0].content, "First Q");
        assert!(result[2].content.starts_with("Context:\ncontext data"));
        assert!(result[2].content.contains("User: Second Q"));
    }

    #[test]
    fn build_messages_context_ignored_when_last_is_assistant() {
        use crate::provider::MessageRole;
        let p = SambaNovaProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::User, content: "Q".into() },
            Message { role: MessageRole::Assistant, content: "A".into() },
        ];
        let result = p.build_messages(&msgs, Some("extra ctx".into()));
        assert_eq!(result[1].content, "A");
    }

    #[test]
    fn build_messages_empty_input() {
        let p = SambaNovaProvider::new(test_config());
        let result = p.build_messages(&[], Some("ctx".into()));
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_role_mapping_all_roles() {
        use crate::provider::MessageRole;
        let p = SambaNovaProvider::new(test_config());
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

    #[tokio::test]
    async fn availability_tracks_api_key() {
        let p_with = SambaNovaProvider::new(test_config());
        assert!(p_with.is_available().await);

        let mut cfg = test_config();
        cfg.api_key = None;
        let p_without = SambaNovaProvider::new(cfg);
        assert!(!p_without.is_available().await);
    }

    #[test]
    fn base_url_custom_override() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://my-proxy.example.com/v1".into());
        let p = SambaNovaProvider::new(cfg);
        assert_eq!(p.base_url(), "https://my-proxy.example.com/v1");
    }

    #[test]
    fn base_url_default_value() {
        let p = SambaNovaProvider::new(test_config());
        assert_eq!(p.base_url(), "https://api.sambanova.ai/v1");
    }

    #[test]
    fn sambanova_request_serde_minimal() {
        let req = SambaNovaRequest {
            model: "Meta-Llama-3.1-70B-Instruct".into(),
            messages: vec![SambaNovaMessage { role: "user".into(), content: "test".into() }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["model"], "Meta-Llama-3.1-70B-Instruct");
        assert_eq!(val["stream"], false);
        assert!(val.get("temperature").is_none());
        assert!(val.get("max_tokens").is_none());
    }

    #[test]
    fn sambanova_request_serde_full() {
        let req = SambaNovaRequest {
            model: "Meta-Llama-3.1-70B-Instruct".into(),
            messages: vec![
                SambaNovaMessage { role: "system".into(), content: "sys".into() },
                SambaNovaMessage { role: "user".into(), content: "usr".into() },
            ],
            temperature: Some(0.3),
            max_tokens: Some(4096),
            stream: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["temperature"], 0.3);
        assert_eq!(val["max_tokens"], 4096);
        assert_eq!(val["stream"], true);
        assert_eq!(val["messages"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn sambanova_response_deser_no_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"done"}}]}"#;
        let resp: SambaNovaResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "done");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn sambanova_stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"token"}}]}"#;
        let resp: SambaNovaStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "token");
    }

    #[test]
    fn sambanova_stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{"content":null}}]}"#;
        let resp: SambaNovaStreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    #[test]
    fn sambanova_message_deser_roundtrip() {
        let msg = SambaNovaMessage { role: "user".into(), content: "hello world".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: SambaNovaMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }

    #[test]
    fn sambanova_response_deser_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"x"}},{"message":{"role":"assistant","content":"y"}}]}"#;
        let resp: SambaNovaResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[1].message.content, "y");
    }

    #[test]
    fn sambanova_response_deser_large_token_counts() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}],"usage":{"prompt_tokens":100000,"completion_tokens":50000}}"#;
        let resp: SambaNovaResponse = serde_json::from_str(json).unwrap();
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 100000);
        assert_eq!(usage.completion_tokens, 50000);
    }

    #[test]
    fn sambanova_stream_response_deser_empty_string_content() {
        let json = r#"{"choices":[{"delta":{"content":""}}]}"#;
        let resp: SambaNovaStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "");
    }

    #[test]
    fn build_messages_empty_context_string_still_injects() {
        use crate::provider::MessageRole;
        let p = SambaNovaProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::User, content: "query".into() },
        ];
        let result = p.build_messages(&msgs, Some("".into()));
        assert!(result[0].content.starts_with("Context:\n"));
        assert!(result[0].content.contains("User: query"));
    }

    #[test]
    fn sambanova_request_llama_8b_model() {
        let req = SambaNovaRequest {
            model: "Meta-Llama-3.1-8B-Instruct".into(),
            messages: vec![
                SambaNovaMessage { role: "system".into(), content: "be helpful".into() },
                SambaNovaMessage { role: "user".into(), content: "solve this".into() },
            ],
            temperature: Some(0.0),
            max_tokens: Some(8192),
            stream: false,
        };
        let val = serde_json::to_value(&req).unwrap();
        assert_eq!(val["model"], "Meta-Llama-3.1-8B-Instruct");
        assert_eq!(val["messages"].as_array().unwrap().len(), 2);
        assert_eq!(val["max_tokens"], 8192);
    }

    #[test]
    fn sambanova_request_streaming_model() {
        let req = SambaNovaRequest {
            model: "Meta-Llama-3.1-405B-Instruct".into(),
            messages: vec![SambaNovaMessage { role: "user".into(), content: "hi".into() }],
            temperature: None,
            max_tokens: None,
            stream: true,
        };
        let val = serde_json::to_value(&req).unwrap();
        assert_eq!(val["model"], "Meta-Llama-3.1-405B-Instruct");
        assert_eq!(val["stream"], true);
    }
}
