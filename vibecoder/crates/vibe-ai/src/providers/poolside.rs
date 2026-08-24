//! Poolside AI provider — OpenAI-compatible API at `https://inference.poolside.ai/v1`.
//!
//! API keys start with `sky_` and are passed as `POOLSIDE_API_KEY` (env)
//! or stored encrypted in ProfileStore via `vibecli set-key poolside <key>`.
//!
//! Supported models: poolside/laguna-s-2.1, poolside/laguna-xs-2.1, poolside/laguna-m-1
//! (see <https://docs.poolside.ai/get-started/supported-models>).

use super::openai_compat::{self, ChatRequest};
use crate::provider::{
    AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message,
    ProviderConfig,
};
use anyhow::{Context, Result};
use async_trait::async_trait;

const POOLSIDE_BASE_URL: &str = "https://inference.poolside.ai/v1";

/// Poolside AI provider — OpenAI-compatible endpoint, purpose-built coding models.
pub struct PoolsideProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    display_name: String,
}

impl PoolsideProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let display_name = format!("Poolside ({})", config.model);
        Self {
            config,
            client: openai_compat::default_http_client(),
            display_name,
        }
    }

    fn base_url(&self) -> String {
        self.config
            .api_url
            .clone()
            .unwrap_or_else(|| POOLSIDE_BASE_URL.to_string())
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url())
    }

    fn api_key(&self) -> Result<&str> {
        self.config
            .api_key
            .as_deref()
            .context("Poolside API key not set (POOLSIDE_API_KEY)")
    }

    fn make_request(
        &self,
        messages: &[Message],
        context: Option<String>,
        stream: bool,
    ) -> ChatRequest {
        ChatRequest {
            model: self.config.model.clone(),
            messages: openai_compat::build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream,
            tools: ChatRequest::tools_for(messages),
        }
    }
}

#[async_trait]
impl AIProvider for PoolsideProvider {
    fn name(&self) -> &str {
        &self.display_name
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
            Message {
                role: crate::provider::MessageRole::System,
                content: "You are a helpful coding assistant.".to_string(),
            },
            Message {
                role: crate::provider::MessageRole::User,
                content: prompt,
            },
        ];
        self.chat_response(&messages, None).await
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        let messages = vec![
            Message {
                role: crate::provider::MessageRole::System,
                content: "You are a helpful coding assistant.".to_string(),
            },
            Message {
                role: crate::provider::MessageRole::User,
                content: prompt,
            },
        ];
        self.stream_chat(&messages).await
    }

    async fn chat_response(
        &self,
        messages: &[Message],
        context: Option<String>,
    ) -> Result<CompletionResponse> {
        let api_key = self.api_key()?;
        let request = self.make_request(messages, context, false);
        openai_compat::send_chat_request(
            &self.client,
            &self.chat_url(),
            api_key,
            &request,
            "Poolside",
        )
        .await
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.api_key()?;
        let request = self.make_request(messages, None, true);
        openai_compat::send_stream_request(
            &self.client,
            &self.chat_url(),
            api_key,
            &request,
            "Poolside",
        )
        .await
    }

    async fn chat_with_images(
        &self,
        messages: &[Message],
        _images: &[ImageAttachment],
        context: Option<String>,
    ) -> Result<String> {
        self.chat(messages, context).await
    }

    fn supports_vision(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openai_compat::{ChatMessage, ChatResponse};

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "poolside".into(),
            api_key: Some("sky_test_key".into()),
            api_url: None,
            model: "poolside/laguna-s-2.1".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
            effort: None,
        }
    }

    #[test]
    fn name_is_poolside() {
        let p = PoolsideProvider::new(test_config());
        assert_eq!(p.name(), "Poolside (poolside/laguna-s-2.1)");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = PoolsideProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = PoolsideProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(POOLSIDE_BASE_URL, "https://inference.poolside.ai/v1");
    }

    #[test]
    fn response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"fast"}}],"usage":{"prompt_tokens":5,"completion_tokens":1}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "fast");
        assert_eq!(resp.usage.unwrap().completion_tokens, 1);
    }

    #[test]
    fn base_url_default() {
        let p = PoolsideProvider::new(test_config());
        assert_eq!(p.base_url(), "https://inference.poolside.ai/v1");
    }

    #[test]
    fn base_url_custom() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://custom.poolside.proxy/v1".into());
        let p = PoolsideProvider::new(cfg);
        assert_eq!(p.base_url(), "https://custom.poolside.proxy/v1");
    }

    #[test]
    fn request_omits_none_fields() {
        let req = ChatRequest {
            model: "poolside/laguna-s-2.1".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
            tools: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("temperature"));
        assert!(!json.contains("max_tokens"));
    }

    #[test]
    fn provider_preserves_model_config() {
        let mut cfg = test_config();
        cfg.model = "poolside/laguna-xs-2.1".into();
        cfg.temperature = Some(0.9);
        cfg.max_tokens = Some(8192);
        let p = PoolsideProvider::new(cfg);
        assert_eq!(p.config.model, "poolside/laguna-xs-2.1");
        assert_eq!(p.config.temperature, Some(0.9));
        assert_eq!(p.config.max_tokens, Some(8192));
    }

    #[test]
    fn api_key_error_when_missing() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = PoolsideProvider::new(cfg);
        let result = p.api_key();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("POOLSIDE_API_KEY"));
    }
}
