//! Fireworks AI provider — fast inference, OpenAI-compatible API.
//!
//! Supported models: accounts/fireworks/models/llama-v3p1-70b-instruct and other hosted models

use super::openai_compat::{self, ChatRequest};
use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig};
use anyhow::{Context, Result};
use async_trait::async_trait;

const FIREWORKS_BASE_URL: &str = "https://api.fireworks.ai/inference/v1";

/// Fireworks AI provider — fast inference with OpenAI-compatible API.
pub struct FireworksProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    display_name: String,
}

impl FireworksProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let display_name = format!("Fireworks AI ({})", config.model);
        Self {
            config,
            client: openai_compat::default_http_client(),
            display_name,
        }
    }

    fn base_url(&self) -> String {
        self.config.api_url.clone().unwrap_or_else(|| FIREWORKS_BASE_URL.to_string())
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url())
    }

    fn api_key(&self) -> Result<&str> {
        self.config.api_key.as_deref().context("Fireworks AI API key not set (FIREWORKS_API_KEY)")
    }

    fn make_request(&self, messages: &[Message], context: Option<String>, stream: bool) -> ChatRequest {
        ChatRequest {
            model: self.config.model.clone(),
            messages: openai_compat::build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream,
        }
    }
}

#[async_trait]
impl AIProvider for FireworksProvider {
    fn name(&self) -> &str { &self.display_name }

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
        let api_key = self.api_key()?;
        let request = self.make_request(messages, context, false);
        openai_compat::send_chat_request(&self.client, &self.chat_url(), api_key, &request, "Fireworks AI").await
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.api_key()?;
        let request = self.make_request(messages, None, true);
        openai_compat::send_stream_request(&self.client, &self.chat_url(), api_key, &request, "Fireworks AI").await
    }

    async fn chat_with_images(&self, messages: &[Message], _images: &[ImageAttachment], context: Option<String>) -> Result<String> {
        self.chat(messages, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openai_compat::{ChatResponse, ChatMessage, StreamResponse};

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "fireworks".into(),
            api_key: Some("sk-test".into()),
            api_url: None,
            model: "accounts/fireworks/models/llama-v3p1-70b-instruct".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_fireworks_ai() {
        let p = FireworksProvider::new(test_config());
        assert_eq!(p.name(), "Fireworks AI (accounts/fireworks/models/llama-v3p1-70b-instruct)");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = FireworksProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = FireworksProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(FIREWORKS_BASE_URL, "https://api.fireworks.ai/inference/v1");
    }

    #[test]
    fn response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"code"}}],"usage":{"prompt_tokens":8,"completion_tokens":3}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "code");
        assert_eq!(resp.usage.unwrap().completion_tokens, 3);
    }

    #[test]
    fn build_messages_without_context() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message { role: MessageRole::System, content: "System prompt".into() },
            Message { role: MessageRole::User, content: "Help me".into() },
        ];
        let result = openai_compat::build_messages(&msgs, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[0].content, "System prompt");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[1].content, "Help me");
    }

    #[test]
    fn build_messages_with_context_prepends_to_last_user() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message { role: MessageRole::User, content: "Review this".into() },
        ];
        let result = openai_compat::build_messages(&msgs, Some("fn main() {}".into()));
        assert_eq!(result.len(), 1);
        assert!(result[0].content.starts_with("Context:\nfn main() {}"));
        assert!(result[0].content.contains("User: Review this"));
    }

    #[test]
    fn build_messages_context_only_modifies_last_user() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message { role: MessageRole::User, content: "First Q".into() },
            Message { role: MessageRole::Assistant, content: "First A".into() },
            Message { role: MessageRole::User, content: "Second Q".into() },
        ];
        let result = openai_compat::build_messages(&msgs, Some("context data".into()));
        assert_eq!(result[0].content, "First Q");
        assert!(result[2].content.starts_with("Context:\ncontext data"));
        assert!(result[2].content.contains("User: Second Q"));
    }

    #[test]
    fn build_messages_context_ignored_when_last_is_assistant() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message { role: MessageRole::User, content: "Q".into() },
            Message { role: MessageRole::Assistant, content: "A".into() },
        ];
        let result = openai_compat::build_messages(&msgs, Some("extra ctx".into()));
        assert_eq!(result[1].content, "A");
    }

    #[test]
    fn build_messages_empty_input() {
        let result = openai_compat::build_messages(&[], Some("ctx".into()));
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_role_mapping_all_roles() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message { role: MessageRole::System, content: "s".into() },
            Message { role: MessageRole::User, content: "u".into() },
            Message { role: MessageRole::Assistant, content: "a".into() },
        ];
        let result = openai_compat::build_messages(&msgs, None);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    #[tokio::test]
    async fn availability_tracks_api_key() {
        let p_with = FireworksProvider::new(test_config());
        assert!(p_with.is_available().await);

        let mut cfg = test_config();
        cfg.api_key = None;
        let p_without = FireworksProvider::new(cfg);
        assert!(!p_without.is_available().await);
    }

    #[test]
    fn base_url_custom_override() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://my-proxy.example.com/v1".into());
        let p = FireworksProvider::new(cfg);
        assert_eq!(p.base_url(), "https://my-proxy.example.com/v1");
    }

    #[test]
    fn base_url_default_value() {
        let p = FireworksProvider::new(test_config());
        assert_eq!(p.base_url(), "https://api.fireworks.ai/inference/v1");
    }

    #[test]
    fn request_serde_minimal() {
        let req = ChatRequest {
            model: "accounts/fireworks/models/llama-v3p1-70b-instruct".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "test".into() }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["model"], "accounts/fireworks/models/llama-v3p1-70b-instruct");
        assert_eq!(val["stream"], false);
        assert!(val.get("temperature").is_none());
        assert!(val.get("max_tokens").is_none());
    }

    #[test]
    fn request_serde_full() {
        let req = ChatRequest {
            model: "accounts/fireworks/models/llama-v3p1-70b-instruct".into(),
            messages: vec![
                ChatMessage { role: "system".into(), content: "sys".into() },
                ChatMessage { role: "user".into(), content: "usr".into() },
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
    fn response_deser_no_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"done"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "done");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"token"}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "token");
    }

    #[test]
    fn stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{"content":null}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    #[test]
    fn message_deser_roundtrip() {
        let msg = ChatMessage { role: "user".into(), content: "hello world".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }

    #[test]
    fn response_deser_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"x"}},{"message":{"role":"assistant","content":"y"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[1].message.content, "y");
    }

    #[test]
    fn response_deser_large_token_counts() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}],"usage":{"prompt_tokens":100000,"completion_tokens":50000}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 100000);
        assert_eq!(usage.completion_tokens, 50000);
    }

    #[test]
    fn stream_response_deser_empty_string_content() {
        let json = r#"{"choices":[{"delta":{"content":""}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "");
    }

    #[test]
    fn build_messages_empty_context_string_still_injects() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message { role: MessageRole::User, content: "query".into() },
        ];
        let result = openai_compat::build_messages(&msgs, Some("".into()));
        assert!(result[0].content.starts_with("Context:\n"));
        assert!(result[0].content.contains("User: query"));
    }

    #[test]
    fn request_mixtral_model() {
        let req = ChatRequest {
            model: "accounts/fireworks/models/mixtral-8x7b-instruct".into(),
            messages: vec![
                ChatMessage { role: "system".into(), content: "be helpful".into() },
                ChatMessage { role: "user".into(), content: "solve this".into() },
            ],
            temperature: Some(0.0),
            max_tokens: Some(8192),
            stream: false,
        };
        let val = serde_json::to_value(&req).unwrap();
        assert_eq!(val["model"], "accounts/fireworks/models/mixtral-8x7b-instruct");
        assert_eq!(val["messages"].as_array().unwrap().len(), 2);
        assert_eq!(val["max_tokens"], 8192);
    }

    #[test]
    fn request_streaming_model() {
        let req = ChatRequest {
            model: "accounts/fireworks/models/llama-v3p1-8b-instruct".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
            temperature: None,
            max_tokens: None,
            stream: true,
        };
        let val = serde_json::to_value(&req).unwrap();
        assert_eq!(val["model"], "accounts/fireworks/models/llama-v3p1-8b-instruct");
        assert_eq!(val["stream"], true);
    }
}
