//! OpenRouter provider — 300+ models via unified OpenAI-compatible API.
//!
//! Set OPENROUTER_API_KEY. Model format: "anthropic/claude-3.5-sonnet",
//! "google/gemini-flash-1.5", "meta-llama/llama-3.3-70b-instruct", etc.

use crate::provider::{
    AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message,
    ProviderConfig, TokenUsage, StopReasonSink};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Serialize)]
struct ORRequest {
    model: String,
    messages: Vec<ORMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
    /// Native tool schemas. OpenRouter proxies the OpenAI function-calling
    /// shape to every upstream that supports it; this provider sent none, so
    /// its models learned about tools only from the system prompt's catalogue.
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parallel_tool_calls: Option<bool>,
}

impl ORRequest {
    /// The same request without tools. OpenRouter fronts hundreds of upstreams
    /// and not all of them implement function calling, so the turn degrades to
    /// the prose path rather than failing.
    fn without_tools(&self) -> Self {
        Self {
            model: self.model.clone(),
            messages: self.messages.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            stream: self.stream,
            tools: None,
            parallel_tool_calls: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ORMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ORUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ORResponse {
    choices: Vec<ORChoice>,
    #[serde(default)]
    usage: Option<ORUsage>,
}

#[derive(Debug, Deserialize)]
struct ORChoice {
    /// The shared reply type, which carries tool calls and reasoning.
    message: super::openai_compat::ResponseMessage,
}

/// OpenRouter provider — access 300+ models through a single API key.
pub struct OpenRouterProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    /// Site URL for OpenRouter attribution (optional).
    site_url: String,
    /// App name for OpenRouter attribution (optional).
    app_name: String,
    display_name: String,
}

impl OpenRouterProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let display_name = format!("OpenRouter ({})", config.model);
        Self {
            display_name,
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            site_url: "https://github.com/vibecody/vibecody".to_string(),
            app_name: "VibeCody".to_string(),
        }
    }

    fn base_url(&self) -> String {
        self.config
            .api_url
            .clone()
            .unwrap_or_else(|| OPENROUTER_BASE_URL.to_string())
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<ORMessage> {
        let mut result: Vec<ORMessage> = messages
            .iter()
            .map(|m| ORMessage {
                role: m.role.as_str().to_string(),
                content: m.content.clone(),
            })
            .collect();
        if let Some(ctx) = context {
            if let Some(last) = result.last_mut() {
                if last.role == "user" {
                    last.content = format!("Context:\n{}\n\nUser: {}", ctx, last.content);
                }
            }
        }
        result
    }

    fn client_with_headers(&self, api_key: &str) -> reqwest::RequestBuilder {
        self.client
            .post(format!("{}/chat/completions", self.base_url()))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("HTTP-Referer", &self.site_url)
            .header("X-Title", &self.app_name)
    }

    fn profile(&self) -> crate::harness::ModelProfile {
        crate::harness::profile_for("openrouter", &self.config.model)
    }

    fn build_request(
        &self,
        messages: &[Message],
        context: Option<String>,
        stream: bool,
    ) -> ORRequest {
        let profile = self.profile();
        let tools = super::openai_compat::ChatRequest::tools_for(messages, &profile);
        ORRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, context),
            temperature: self.config.temperature.or(profile.temperature),
            max_tokens: self
                .config
                .max_tokens
                .or(profile.max_output_tokens.map(|t| t as usize)),
            stream,
            parallel_tool_calls: super::openai_compat::ChatRequest::parallel_for(
                tools.as_ref(),
                &profile,
            ),
            tools,
        }
    }

    /// POST `request`, retrying once without tools if the upstream rejects the
    /// field itself.
    async fn send(&self, api_key: &str, request: &ORRequest) -> Result<reqwest::Response> {
        let resp = self
            .client_with_headers(api_key)
            .json(request)
            .send()
            .await
            .context("OpenRouter request failed")?;
        if resp.status().is_success() {
            return Ok(resp);
        }
        let err = resp.text().await?;
        if request.tools.is_none() || !super::openai_compat::is_tools_unsupported(&err) {
            anyhow::bail!("OpenRouter API error: {}", err);
        }
        tracing::warn!(
            model = %self.config.model,
            "OpenRouter upstream does not accept native tool definitions — retrying without them",
        );
        let retry = self
            .client_with_headers(api_key)
            .json(&request.without_tools())
            .send()
            .await
            .context("OpenRouter request failed")?;
        if !retry.status().is_success() {
            let err = retry.text().await?;
            anyhow::bail!("OpenRouter API error: {}", err);
        }
        Ok(retry)
    }
}

impl OpenRouterProvider {
    /// The streaming body shared by [`AIProvider::stream_chat`] and
    /// [`AIProvider::stream_chat_reporting`].
    ///
    /// `stop` is `None` on the plain path, leaving that caller exactly as
    /// it was; the reporting path passes a sink and gets the endpoint's
    /// finish reason back out of the final SSE chunk.
    async fn stream_chat_inner(
        &self,
        messages: &[Message],
        stop: Option<StopReasonSink>,
    ) -> Result<CompletionStream> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("OpenRouter API key not set")?;
        let request = self.build_request(messages, None, true);
        let resp = self.send(api_key, &request).await?;
        // The shared SSE parser. OpenRouter is where reasoning models are most
        // often reached, and this loop read `delta.content` alone — the reason
        // `openai_compat` learned to read OpenRouter's `reasoning` field in the
        // first place, a fix this provider was not benefiting from.
        Ok(super::openai_compat::parse_sse_stream_reporting(resp, stop))
    }
}

#[async_trait]
impl AIProvider for OpenRouterProvider {
    fn name(&self) -> &str {
        &self.display_name
    }

    async fn is_available(&self) -> bool {
        self.config.api_key.is_some()
    }

    /// OpenRouter publishes `context_length` per model on `/models`, and it is
    /// the number that matters here: the same model id can be routed to
    /// different upstreams with different windows, so only OpenRouter knows.
    /// The listing is public, so this works before a key is configured.
    async fn context_window(&self) -> Option<usize> {
        crate::context_window::cached("OpenRouter", &self.config.model, || async {
            let url = format!("{}/models", self.base_url());
            let body = self
                .client
                .get(&url)
                .send()
                .await
                .ok()?
                .json::<serde_json::Value>()
                .await
                .ok()?;
            crate::context_window::from_models_list(&body, &self.config.model)
        })
        .await
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
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("OpenRouter API key not set (OPENROUTER_API_KEY)")?;
        let request = self.build_request(messages, context, false);
        let resp = self.send(api_key, &request).await?;
        let body: ORResponse = resp
            .json()
            .await
            .context("Failed to parse OpenRouter response")?;
        // `into_text`, not `.content`: a tool-call turn has an empty content.
        let text = body
            .choices
            .into_iter()
            .next()
            .context("No choices")?
            .message
            .into_text();
        let usage = body.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
        });
        Ok(CompletionResponse {
            text,
            model: self.config.model.clone(),
            usage,
        })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        self.stream_chat_inner(messages, None).await
    }

    async fn stream_chat_reporting(
        &self,
        messages: &[Message],
        stop: StopReasonSink,
    ) -> Result<CompletionStream> {
        self.stream_chat_inner(messages, Some(stop)).await
    }

    fn harness_profile(&self) -> crate::harness::ModelProfile {
        self.profile()
    }

    async fn chat_with_images(
        &self,
        messages: &[Message],
        _images: &[ImageAttachment],
        context: Option<String>,
    ) -> Result<String> {
        // OpenRouter passes through vision to underlying providers that support it
        self.chat(messages, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "openrouter".into(),
            api_key: Some("sk-or-test".into()),
            api_url: None,
            model: "anthropic/claude-3.5-sonnet".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
            effort: None,
        }
    }

    #[test]
    fn name_is_openrouter() {
        let p = OpenRouterProvider::new(test_config());
        assert!(p.name().starts_with("OpenRouter ("));
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = OpenRouterProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = OpenRouterProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(OPENROUTER_BASE_URL, "https://openrouter.ai/api/v1");
    }

    #[test]
    fn or_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}],"usage":{"prompt_tokens":2,"completion_tokens":1}}"#;
        let resp: ORResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "ok");
    }

    #[test]
    fn base_url_defaults_to_constant() {
        let p = OpenRouterProvider::new(test_config());
        assert_eq!(p.base_url(), OPENROUTER_BASE_URL);
    }

    #[test]
    fn base_url_uses_custom_when_set() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://custom.proxy.com/v1".into());
        let p = OpenRouterProvider::new(cfg);
        assert_eq!(p.base_url(), "https://custom.proxy.com/v1");
    }

    #[test]
    fn build_messages_maps_roles() {
        use crate::provider::MessageRole;
        let p = OpenRouterProvider::new(test_config());
        let messages = vec![
            Message {
                role: MessageRole::System,
                content: "sys".into(),
            },
            Message {
                role: MessageRole::User,
                content: "q".into(),
            },
            Message {
                role: MessageRole::Assistant,
                content: "a".into(),
            },
        ];
        let result = p.build_messages(&messages, None);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    #[test]
    fn build_messages_appends_context() {
        use crate::provider::MessageRole;
        let p = OpenRouterProvider::new(test_config());
        let messages = vec![Message {
            role: MessageRole::User,
            content: "query".into(),
        }];
        let result = p.build_messages(&messages, Some("extra context".into()));
        assert!(result[0].content.contains("Context:"));
        assert!(result[0].content.contains("extra context"));
        assert!(result[0].content.contains("query"));
    }

    #[test]
    fn or_request_serializes_correctly() {
        let req = ORRequest {
            model: "anthropic/claude-3.5-sonnet".into(),
            messages: vec![ORMessage {
                role: "user".into(),
                content: "test".into(),
            }],
            temperature: Some(0.5),
            max_tokens: Some(2048),
            stream: false,
            tools: None,
            parallel_tool_calls: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "anthropic/claude-3.5-sonnet");
        assert_eq!(json["temperature"], 0.5);
        assert_eq!(json["max_tokens"], 2048);
        assert_eq!(json["stream"], false);
    }

    // ── request serialization edge cases ──────────────────────────────

    #[test]
    fn or_request_skips_none_optional_fields() {
        let req = ORRequest {
            model: "meta-llama/llama-3.3-70b-instruct".into(),
            messages: vec![ORMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
            tools: None,
            parallel_tool_calls: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("temperature").is_none());
        assert!(json.get("max_tokens").is_none());
    }

    #[test]
    fn or_request_stream_true() {
        let req = ORRequest {
            model: "openai/gpt-4o".into(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: true,
            tools: None,
            parallel_tool_calls: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["stream"], true);
    }

    // ── response parsing variants ─────────────────────────────────────

    #[test]
    fn or_response_deser_no_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"done"}}]}"#;
        let resp: ORResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "done");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn or_response_deser_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"a"}},{"message":{"role":"assistant","content":"b"}}],"usage":{"prompt_tokens":5,"completion_tokens":2}}"#;
        let resp: ORResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[1].message.content, "b");
    }

    #[test]
    fn or_stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"tok"}}]}"#;
        let resp: super::super::openai_compat::StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "tok");
    }

    #[test]
    fn or_stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{"content":null}}]}"#;
        let resp: super::super::openai_compat::StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    // ── build_messages edge cases ─────────────────────────────────────

    #[test]
    fn build_messages_empty_list() {
        let p = OpenRouterProvider::new(test_config());
        let result = p.build_messages(&[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_context_ignored_when_last_is_assistant() {
        use crate::provider::MessageRole;
        let p = OpenRouterProvider::new(test_config());
        let messages = vec![
            Message {
                role: MessageRole::User,
                content: "Q".into(),
            },
            Message {
                role: MessageRole::Assistant,
                content: "A".into(),
            },
        ];
        let result = p.build_messages(&messages, Some("ctx".into()));
        assert_eq!(result[1].content, "A");
        assert_eq!(result[0].content, "Q");
    }

    #[test]
    fn build_messages_multi_turn_context_only_last_user() {
        use crate::provider::MessageRole;
        let p = OpenRouterProvider::new(test_config());
        let messages = vec![
            Message {
                role: MessageRole::User,
                content: "First".into(),
            },
            Message {
                role: MessageRole::Assistant,
                content: "Reply".into(),
            },
            Message {
                role: MessageRole::User,
                content: "Second".into(),
            },
        ];
        let result = p.build_messages(&messages, Some("ctx".into()));
        assert_eq!(result[0].content, "First");
        assert!(result[2].content.contains("Context:\nctx"));
        assert!(result[2].content.contains("Second"));
    }

    // ── or_message roundtrip ──────────────────────────────────────────

    #[test]
    fn or_message_serde_roundtrip() {
        let msg = ORMessage {
            role: "assistant".into(),
            content: "hello world".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: ORMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }

    // ── provider defaults ─────────────────────────────────────────────

    #[test]
    fn default_site_url_and_app_name() {
        let p = OpenRouterProvider::new(test_config());
        assert!(p.site_url.contains("vibecody"));
        assert_eq!(p.app_name, "VibeCody");
    }
}
