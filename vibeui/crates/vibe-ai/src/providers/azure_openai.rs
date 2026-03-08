//! Azure OpenAI provider.
//!
//! Endpoint format: `https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}`
//!
//! Config:
//! ```toml
//! [azure_openai]
//! enabled = true
//! api_url = "https://myresource.openai.azure.com"
//! model = "gpt-4o"          # Azure deployment name
//! api_key = "..."
//! api_version = "2024-12-01-preview"   # optional
//! ```

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_API_VERSION: &str = "2024-12-01-preview";

#[derive(Debug, Serialize)]
struct AzRequest {
    messages: Vec<AzMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct AzMessage {
    role: String,
    content: Value,
}

#[derive(Debug, Deserialize)]
struct AzUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AzResponse {
    choices: Vec<AzChoice>,
    #[serde(default)]
    usage: Option<AzUsage>,
}

#[derive(Debug, Deserialize)]
struct AzChoice {
    message: AzMessage,
}

#[derive(Debug, Deserialize)]
struct AzStreamResponse {
    choices: Vec<AzStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct AzStreamChoice {
    delta: AzDelta,
}

#[derive(Debug, Deserialize)]
struct AzDelta {
    content: Option<String>,
}

/// Azure OpenAI provider.
pub struct AzureOpenAIProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    api_version: String,
}

impl AzureOpenAIProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            api_version: DEFAULT_API_VERSION.to_string(),
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    /// Build the full Azure endpoint URL for the deployment.
    fn endpoint_url(&self) -> String {
        let base = self.config.api_url.as_deref().unwrap_or("");
        let base = base.trim_end_matches('/');
        let deployment = &self.config.model;
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            base, deployment, self.api_version
        )
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<AzMessage> {
        let mut result: Vec<AzMessage> = messages.iter().map(|m| AzMessage {
            role: m.role.as_str().to_string(),
            content: Value::String(m.content.clone()),
        }).collect();
        if let Some(ctx) = context {
            if let Some(last) = result.last_mut() {
                if last.role == "user" {
                    if let Value::String(ref s) = last.content.clone() {
                        last.content = Value::String(format!("Context:\n{}\n\nUser: {}", ctx, s));
                    }
                }
            }
        }
        result
    }
}

#[async_trait]
impl AIProvider for AzureOpenAIProvider {
    fn name(&self) -> &str { "AzureOpenAI" }

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
        let api_key = self.config.api_key.as_ref().context("Azure OpenAI API key not set (AZURE_OPENAI_API_KEY)")?;
        let request = AzRequest {
            messages: self.build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };
        let resp = self.client.post(self.endpoint_url())
            .header("api-key", api_key.as_str())
            .json(&request)
            .send().await.context("Azure OpenAI request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Azure OpenAI error: {}", err);
        }
        let body: AzResponse = resp.json().await.context("Failed to parse Azure OpenAI response")?;
        let content = body.choices.first().context("No choices")?.message.content.clone();
        let text = match content { Value::String(s) => s, v => v.to_string() };
        let usage = body.usage.map(|u| TokenUsage { prompt_tokens: u.prompt_tokens, completion_tokens: u.completion_tokens });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Azure OpenAI API key not set")?;
        let request = AzRequest {
            messages: self.build_messages(messages, None),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: true,
        };
        let resp = self.client.post(self.endpoint_url())
            .header("api-key", api_key.as_str())
            .json(&request)
            .send().await.context("Azure OpenAI stream failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Azure OpenAI error: {}", err);
        }
        let stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(r) = serde_json::from_str::<AzStreamResponse>(data) {
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
            provider_type: "azure_openai".into(),
            api_key: Some("az-test-key".into()),
            api_url: Some("https://myresource.openai.azure.com".into()),
            model: "gpt-4o".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_azure_openai() {
        let p = AzureOpenAIProvider::new(test_config());
        assert_eq!(p.name(), "AzureOpenAI");
    }

    #[tokio::test]
    async fn is_available_with_key_and_url() {
        let p = AzureOpenAIProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = AzureOpenAIProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_url() {
        let mut cfg = test_config();
        cfg.api_url = None;
        let p = AzureOpenAIProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn endpoint_url_assembly() {
        let p = AzureOpenAIProvider::new(test_config());
        let url = p.endpoint_url();
        assert!(url.starts_with("https://myresource.openai.azure.com/openai/deployments/gpt-4o/chat/completions"));
        assert!(url.contains("api-version=2024-12-01-preview"));
    }

    #[test]
    fn with_api_version() {
        let p = AzureOpenAIProvider::new(test_config()).with_api_version("2025-01-01");
        let url = p.endpoint_url();
        assert!(url.contains("api-version=2025-01-01"));
    }

    #[test]
    fn default_api_version() {
        assert_eq!(DEFAULT_API_VERSION, "2024-12-01-preview");
    }

    #[test]
    fn endpoint_url_strips_trailing_slash() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://myresource.openai.azure.com/".into());
        let p = AzureOpenAIProvider::new(cfg);
        let url = p.endpoint_url();
        assert!(!url.contains("azure.com//openai"));
        assert!(url.starts_with("https://myresource.openai.azure.com/openai/deployments/"));
    }

    #[test]
    fn build_messages_maps_roles() {
        use crate::provider::MessageRole;
        let p = AzureOpenAIProvider::new(test_config());
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
    fn build_messages_appends_context_to_last_user() {
        use crate::provider::MessageRole;
        let p = AzureOpenAIProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "hello".into() },
        ];
        let result = p.build_messages(&messages, Some("ctx".into()));
        let content = result[0].content.as_str().unwrap();
        assert!(content.contains("Context:\nctx"));
        assert!(content.contains("hello"));
    }

    #[test]
    fn az_request_skips_none_optional_fields() {
        let req = AzRequest {
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("temperature").is_none());
        assert!(json.get("max_tokens").is_none());
    }

    #[test]
    fn az_response_deser_with_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"hi"}}],"usage":{"prompt_tokens":10,"completion_tokens":5}}"#;
        let resp: AzResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "hi");
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
    }

    // ── response parsing variants ─────────────────────────────────────

    #[test]
    fn az_response_deser_no_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#;
        let resp: AzResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "ok");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn az_response_deser_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"first"}},{"message":{"role":"assistant","content":"second"}}]}"#;
        let resp: AzResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[1].message.content, "second");
    }

    #[test]
    fn az_stream_response_deser_with_content() {
        let json = r#"{"choices":[{"delta":{"content":"token"}}]}"#;
        let resp: AzStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "token");
    }

    #[test]
    fn az_stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{"content":null}}]}"#;
        let resp: AzStreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    // ── request serialization ─────────────────────────────────────────

    #[test]
    fn az_request_includes_present_optional_fields() {
        let req = AzRequest {
            messages: vec![AzMessage { role: "user".into(), content: serde_json::Value::String("hi".into()) }],
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: true,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json["temperature"].as_f64().unwrap() - 0.7 < 0.001);
        assert_eq!(json["max_tokens"], 4096);
        assert_eq!(json["stream"], true);
    }

    // ── build_messages edge cases ─────────────────────────────────────

    #[test]
    fn build_messages_empty_list() {
        let p = AzureOpenAIProvider::new(test_config());
        let result = p.build_messages(&[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_context_ignored_when_last_is_assistant() {
        use crate::provider::MessageRole;
        let p = AzureOpenAIProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "Q".into() },
            Message { role: MessageRole::Assistant, content: "A".into() },
        ];
        let result = p.build_messages(&messages, Some("ignored ctx".into()));
        assert_eq!(result[1].content, serde_json::Value::String("A".into()));
    }

    #[test]
    fn build_messages_empty_context_with_messages() {
        use crate::provider::MessageRole;
        let p = AzureOpenAIProvider::new(test_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "hello".into() },
        ];
        let result = p.build_messages(&messages, Some("".into()));
        let content = result[0].content.as_str().unwrap();
        assert!(content.contains("Context:"));
        assert!(content.contains("hello"));
    }

    // ── endpoint URL edge cases ───────────────────────────────────────

    #[test]
    fn endpoint_url_with_empty_api_url() {
        let mut cfg = test_config();
        cfg.api_url = Some("".into());
        let p = AzureOpenAIProvider::new(cfg);
        let url = p.endpoint_url();
        assert!(url.contains("/openai/deployments/gpt-4o/chat/completions"));
    }

    #[test]
    fn endpoint_url_uses_model_as_deployment() {
        let mut cfg = test_config();
        cfg.model = "gpt-35-turbo".into();
        let p = AzureOpenAIProvider::new(cfg);
        let url = p.endpoint_url();
        assert!(url.contains("/deployments/gpt-35-turbo/"));
    }

    // ── az_message roundtrip ──────────────────────────────────────────

    #[test]
    fn az_message_roundtrip_serde() {
        let msg = AzMessage { role: "user".into(), content: serde_json::Value::String("test data".into()) };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: AzMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }
}
