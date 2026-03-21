//! Together AI provider — open model hosting, OpenAI-compatible API.
//!
//! Supported models: meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo and other open models

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

const TOGETHER_BASE_URL: &str = "https://api.together.xyz/v1";

#[derive(Debug, Serialize)]
struct TogetherRequest {
    model: String,
    messages: Vec<TogetherMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct TogetherMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct TogetherUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct TogetherResponse {
    choices: Vec<TogetherChoice>,
    #[serde(default)]
    usage: Option<TogetherUsage>,
}

#[derive(Debug, Deserialize)]
struct TogetherChoice {
    message: TogetherMessage,
}

#[derive(Debug, Deserialize)]
struct TogetherStreamResponse {
    choices: Vec<TogetherStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct TogetherStreamChoice {
    delta: TogetherDelta,
}

#[derive(Debug, Deserialize)]
struct TogetherDelta {
    content: Option<String>,
}

/// Together AI provider — open model hosting with OpenAI-compatible API.
pub struct TogetherProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl TogetherProvider {
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
        self.config.api_url.clone().unwrap_or_else(|| TOGETHER_BASE_URL.to_string())
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<TogetherMessage> {
        let mut result: Vec<TogetherMessage> = messages.iter().map(|m| TogetherMessage {
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
impl AIProvider for TogetherProvider {
    fn name(&self) -> &str { "Together AI" }

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
        let api_key = self.config.api_key.as_ref().context("Together AI API key not set (TOGETHER_API_KEY)")?;
        let request = TogetherRequest {
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
            .send().await.context("Together AI request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Together AI API error: {}", err);
        }
        let body: TogetherResponse = resp.json().await.context("Failed to parse Together AI response")?;
        let text = body.choices.first().context("No choices")?.message.content.clone();
        let usage = body.usage.map(|u| TokenUsage { prompt_tokens: u.prompt_tokens, completion_tokens: u.completion_tokens });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Together AI API key not set")?;
        let request = TogetherRequest {
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
            .send().await.context("Together AI stream request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Together AI API error: {}", err);
        }
        let stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(r) = serde_json::from_str::<TogetherStreamResponse>(data) {
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
            provider_type: "together".into(),
            api_key: Some("sk-test".into()),
            api_url: None,
            model: "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_together_ai() {
        let p = TogetherProvider::new(test_config());
        assert_eq!(p.name(), "Together AI");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = TogetherProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = TogetherProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(TOGETHER_BASE_URL, "https://api.together.xyz/v1");
    }

    #[test]
    fn together_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"code"}}],"usage":{"prompt_tokens":8,"completion_tokens":3}}"#;
        let resp: TogetherResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "code");
        assert_eq!(resp.usage.unwrap().completion_tokens, 3);
    }

    #[test]
    fn build_messages_without_context() {
        use crate::provider::MessageRole;
        let p = TogetherProvider::new(test_config());
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
        let p = TogetherProvider::new(test_config());
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
        let p = TogetherProvider::new(test_config());
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
        let p = TogetherProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::User, content: "Q".into() },
            Message { role: MessageRole::Assistant, content: "A".into() },
        ];
        let result = p.build_messages(&msgs, Some("extra ctx".into()));
        assert_eq!(result[1].content, "A");
    }

    #[test]
    fn build_messages_empty_input() {
        let p = TogetherProvider::new(test_config());
        let result = p.build_messages(&[], Some("ctx".into()));
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_role_mapping_all_roles() {
        use crate::provider::MessageRole;
        let p = TogetherProvider::new(test_config());
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
        let p_with = TogetherProvider::new(test_config());
        assert!(p_with.is_available().await);

        let mut cfg = test_config();
        cfg.api_key = None;
        let p_without = TogetherProvider::new(cfg);
        assert!(!p_without.is_available().await);
    }

    #[test]
    fn base_url_custom_override() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://my-proxy.example.com/v1".into());
        let p = TogetherProvider::new(cfg);
        assert_eq!(p.base_url(), "https://my-proxy.example.com/v1");
    }

    #[test]
    fn base_url_default_value() {
        let p = TogetherProvider::new(test_config());
        assert_eq!(p.base_url(), "https://api.together.xyz/v1");
    }

    #[test]
    fn together_request_serde_minimal() {
        let req = TogetherRequest {
            model: "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo".into(),
            messages: vec![TogetherMessage { role: "user".into(), content: "test".into() }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["model"], "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo");
        assert_eq!(val["stream"], false);
        assert!(val.get("temperature").is_none());
        assert!(val.get("max_tokens").is_none());
    }

    #[test]
    fn together_request_serde_full() {
        let req = TogetherRequest {
            model: "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo".into(),
            messages: vec![
                TogetherMessage { role: "system".into(), content: "sys".into() },
                TogetherMessage { role: "user".into(), content: "usr".into() },
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
    fn together_response_deser_no_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"done"}}]}"#;
        let resp: TogetherResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "done");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn together_stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"token"}}]}"#;
        let resp: TogetherStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "token");
    }

    #[test]
    fn together_stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{"content":null}}]}"#;
        let resp: TogetherStreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    #[test]
    fn together_message_deser_roundtrip() {
        let msg = TogetherMessage { role: "user".into(), content: "hello world".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: TogetherMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }

    #[test]
    fn together_response_deser_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"x"}},{"message":{"role":"assistant","content":"y"}}]}"#;
        let resp: TogetherResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[1].message.content, "y");
    }

    #[test]
    fn together_response_deser_large_token_counts() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}],"usage":{"prompt_tokens":100000,"completion_tokens":50000}}"#;
        let resp: TogetherResponse = serde_json::from_str(json).unwrap();
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 100000);
        assert_eq!(usage.completion_tokens, 50000);
    }

    #[test]
    fn together_stream_response_deser_empty_string_content() {
        let json = r#"{"choices":[{"delta":{"content":""}}]}"#;
        let resp: TogetherStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "");
    }

    #[test]
    fn build_messages_empty_context_string_still_injects() {
        use crate::provider::MessageRole;
        let p = TogetherProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::User, content: "query".into() },
        ];
        let result = p.build_messages(&msgs, Some("".into()));
        assert!(result[0].content.starts_with("Context:\n"));
        assert!(result[0].content.contains("User: query"));
    }

    #[test]
    fn together_request_with_slash_model_name() {
        let req = TogetherRequest {
            model: "mistralai/Mixtral-8x7B-Instruct-v0.1".into(),
            messages: vec![
                TogetherMessage { role: "system".into(), content: "be helpful".into() },
                TogetherMessage { role: "user".into(), content: "solve this".into() },
            ],
            temperature: Some(0.0),
            max_tokens: Some(8192),
            stream: false,
        };
        let val = serde_json::to_value(&req).unwrap();
        assert_eq!(val["model"], "mistralai/Mixtral-8x7B-Instruct-v0.1");
        assert_eq!(val["messages"].as_array().unwrap().len(), 2);
        assert_eq!(val["max_tokens"], 8192);
    }

    #[test]
    fn together_request_streaming_model() {
        let req = TogetherRequest {
            model: "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo".into(),
            messages: vec![TogetherMessage { role: "user".into(), content: "hi".into() }],
            temperature: None,
            max_tokens: None,
            stream: true,
        };
        let val = serde_json::to_value(&req).unwrap();
        assert_eq!(val["model"], "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo");
        assert_eq!(val["stream"], true);
    }
}
