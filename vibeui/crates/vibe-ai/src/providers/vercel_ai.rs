//! Vercel AI Gateway provider — unified proxy, OpenAI-compatible.
//!
//! Routes requests through the user's Vercel AI Gateway instance.
//! Requires `api_url` to be set (gateway endpoint).

use super::openai_compat::{self, ChatRequest};
use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig};
use anyhow::{Context, Result};
use async_trait::async_trait;

/// Vercel AI Gateway provider — unified proxy to multiple AI services.
pub struct VercelAIProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl VercelAIProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            client: openai_compat::default_http_client(),
        }
    }

    fn base_url(&self) -> Result<String> {
        self.config.api_url.clone().context("Vercel AI Gateway URL not set (vercel_ai.api_url in config)")
    }

    fn chat_url(&self) -> Result<String> {
        Ok(format!("{}/chat/completions", self.base_url()?))
    }

    fn api_key(&self) -> Result<&str> {
        self.config.api_key.as_deref().context("Vercel AI API key not set (VERCEL_AI_API_KEY)")
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
        let api_key = self.api_key()?;
        let url = self.chat_url()?;
        let request = self.make_request(messages, context, false);
        openai_compat::send_chat_request(&self.client, &url, api_key, &request, "Vercel AI").await
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.api_key()?;
        let url = self.chat_url()?;
        let request = self.make_request(messages, None, true);
        openai_compat::send_stream_request(&self.client, &url, api_key, &request, "Vercel AI").await
    }

    async fn chat_with_images(&self, messages: &[Message], _images: &[ImageAttachment], context: Option<String>) -> Result<String> {
        self.chat(messages, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openai_compat::{ChatResponse, ChatMessage, StreamResponse, ChatRequest};

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
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
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
        let messages = vec![
            Message { role: MessageRole::System, content: "sys".into() },
            Message { role: MessageRole::User, content: "usr".into() },
            Message { role: MessageRole::Assistant, content: "ast".into() },
        ];
        let result = openai_compat::build_messages(&messages, None);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    #[test]
    fn build_messages_appends_context() {
        use crate::provider::MessageRole;
        let messages = vec![
            Message { role: MessageRole::User, content: "ask".into() },
        ];
        let result = openai_compat::build_messages(&messages, Some("relevant info".into()));
        assert!(result[0].content.contains("Context:"));
        assert!(result[0].content.contains("relevant info"));
        assert!(result[0].content.contains("ask"));
    }

    #[test]
    fn vercel_ai_request_serializes_correctly() {
        let req = ChatRequest {
            model: "gpt-4o".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "test".into() }],
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
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "no usage");
        assert!(resp.usage.is_none());
    }

    // ── stream response deserialization ─────────────────────────────────

    #[test]
    fn vercel_ai_stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"streamed"}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "streamed");
    }

    #[test]
    fn vercel_ai_stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{"content":null}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    #[test]
    fn vercel_ai_stream_response_deser_empty_choices() {
        let json = r#"{"choices":[]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices.is_empty());
    }

    // ── request serialization edge cases ────────────────────────────────

    #[test]
    fn vercel_ai_request_serde_minimal() {
        let req = ChatRequest {
            model: "claude-3-opus".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "claude-3-opus");
        assert_eq!(json["stream"], false);
        // Optional fields omitted via skip_serializing_if
        assert!(json.get("temperature").is_none());
        assert!(json.get("max_tokens").is_none());
    }

    #[test]
    fn vercel_ai_request_serde_multiple_messages() {
        let req = ChatRequest {
            model: "gpt-4o".into(),
            messages: vec![
                ChatMessage { role: "system".into(), content: "sys".into() },
                ChatMessage { role: "user".into(), content: "u1".into() },
                ChatMessage { role: "assistant".into(), content: "a1".into() },
                ChatMessage { role: "user".into(), content: "u2".into() },
            ],
            temperature: Some(0.5),
            max_tokens: Some(4096),
            stream: true,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["messages"].as_array().unwrap().len(), 4);
        assert_eq!(json["temperature"], 0.5);
        assert_eq!(json["max_tokens"], 4096);
    }

    // ── message roundtrip ───────────────────────────────────────────────

    #[test]
    fn vercel_ai_message_roundtrip() {
        let msg = ChatMessage { role: "user".into(), content: "test input".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }

    // ── build_messages edge cases ───────────────────────────────────────

    #[test]
    fn build_messages_empty_input() {
        let result = openai_compat::build_messages(&[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_empty_input_with_context() {
        let result = openai_compat::build_messages(&[], Some("ctx".into()));
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_context_only_affects_last_user() {
        use crate::provider::MessageRole;
        let messages = vec![
            Message { role: MessageRole::User, content: "first".into() },
            Message { role: MessageRole::Assistant, content: "mid".into() },
            Message { role: MessageRole::User, content: "second".into() },
        ];
        let result = openai_compat::build_messages(&messages, Some("background".into()));
        // First user message unchanged
        assert_eq!(result[0].content, "first");
        // Last user message gets context
        assert!(result[2].content.starts_with("Context:\nbackground"));
        assert!(result[2].content.contains("User: second"));
    }

    #[test]
    fn build_messages_context_skipped_when_last_is_assistant() {
        use crate::provider::MessageRole;
        let messages = vec![
            Message { role: MessageRole::User, content: "q".into() },
            Message { role: MessageRole::Assistant, content: "a".into() },
        ];
        let result = openai_compat::build_messages(&messages, Some("some ctx".into()));
        assert_eq!(result[1].content, "a");
        assert_eq!(result[0].content, "q");
    }

    // ── availability edge cases ─────────────────────────────────────────

    #[tokio::test]
    async fn available_requires_both_key_and_url() {
        // Both present
        let p = VercelAIProvider::new(test_config());
        assert!(p.is_available().await);

        // Key only
        let mut cfg = test_config();
        cfg.api_url = None;
        assert!(!VercelAIProvider::new(cfg).is_available().await);

        // URL only
        let mut cfg2 = test_config();
        cfg2.api_key = None;
        assert!(!VercelAIProvider::new(cfg2).is_available().await);

        // Neither
        let cfg3 = ProviderConfig {
            provider_type: "vercel_ai".into(),
            api_key: None,
            api_url: None,
            model: "m".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        };
        assert!(!VercelAIProvider::new(cfg3).is_available().await);
    }

    // ── response with multiple choices ──────────────────────────────────

    #[test]
    fn vercel_ai_response_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"first"}},{"message":{"role":"assistant","content":"second"}}],"usage":{"prompt_tokens":5,"completion_tokens":2}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[0].message.content, "first");
        assert_eq!(resp.choices[1].message.content, "second");
    }

    // ── provider config is preserved ────────────────────────────────────

    #[test]
    fn provider_preserves_model_config() {
        let mut cfg = test_config();
        cfg.model = "custom-model-v2".into();
        let p = VercelAIProvider::new(cfg);
        assert_eq!(p.name(), "VercelAI");
        // Model is stored in config
        assert_eq!(p.config.model, "custom-model-v2");
    }
}
