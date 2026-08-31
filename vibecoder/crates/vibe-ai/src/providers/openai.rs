//! OpenAI provider implementation (ChatGPT, Codex)

use crate::provider::{
    AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message,
    ProviderConfig, TokenUsage, StopReasonSink};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
    /// Reasoning effort (gap C5) — GPT-5.x / o-series only; omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
    /// Native tool schemas for this conversation.
    ///
    /// This provider sent none at all until now: the tools were described only
    /// in the ~15 KB XML catalogue in the system prompt, and the model's calls
    /// were regex-parsed back out of its prose, while the Chat Completions API
    /// had offered first-class function calling the whole time.
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parallel_tool_calls: Option<bool>,
}

impl OpenAIRequest {
    /// The same request with no tools, for an endpoint that rejects the field.
    ///
    /// Azure deployments and OpenAI-compatible proxies sitting behind this
    /// provider's `api_url` do not all implement function calling; the turn has
    /// to degrade to what worked before rather than fail, since the prompt
    /// still describes the tools in prose.
    fn without_tools(&self) -> Self {
        Self {
            model: self.model.clone(),
            messages: self.messages.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            stream: self.stream,
            reasoning_effort: self.reasoning_effort.clone(),
            tools: None,
            parallel_tool_calls: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: Value,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    #[serde(default)]
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    /// The shared reply type, not `OpenAIMessage`: a reply can carry tool calls
    /// and reasoning that a request never does, and reading it as the request
    /// shape is what silently discarded both.
    message: super::openai_compat::ResponseMessage,
}

/// OpenAI provider
pub struct OpenAIProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    display_name: String,
}

impl OpenAIProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let display_name = format!("OpenAI ({})", config.model);
        Self {
            display_name,
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    const DEFAULT_API_URL: &'static str = "https://api.openai.com/v1/chat/completions";

    fn api_url(&self) -> &str {
        self.config
            .api_url
            .as_deref()
            .unwrap_or(Self::DEFAULT_API_URL)
    }

    /// Map the per-request effort tier (gap C5) to OpenAI `reasoning_effort`, but
    /// only for reasoning-capable models (GPT-5.x / o-series / Codex). Returns
    /// `None` for chat-only models, which would 400 on an unknown field.
    fn reasoning_effort(&self) -> Option<String> {
        let m = self.config.model.to_lowercase();
        let reasoning_capable = m.starts_with("gpt-5")
            || m.starts_with("o1")
            || m.starts_with("o3")
            || m.starts_with("o4")
            || m.contains("codex");
        if !reasoning_capable {
            return None;
        }
        self.config
            .effort
            .map(|e| e.openai_reasoning_effort().to_string())
    }

    /// This pair's harness settings — tool transport, output cap, temperature.
    fn profile(&self) -> crate::harness::ModelProfile {
        crate::harness::profile_for("openai", &self.config.model)
    }

    /// Build one request, with tools attached when the conversation asked for
    /// them and this pair is on the native transport.
    fn build_request(
        &self,
        messages: &[Message],
        context: Option<String>,
        stream: bool,
    ) -> OpenAIRequest {
        let profile = self.profile();
        let tools = super::openai_compat::ChatRequest::tools_for(messages, &profile);
        OpenAIRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, context),
            // An explicit config value is the caller's decision and wins; the
            // profile only fills in what the caller left open.
            temperature: self.config.temperature.or(profile.temperature),
            max_tokens: self
                .config
                .max_tokens
                .or(profile.max_output_tokens.map(|t| t as usize)),
            reasoning_effort: self.reasoning_effort(),
            stream,
            parallel_tool_calls: super::openai_compat::ChatRequest::parallel_for(
                tools.as_ref(),
                &profile,
            ),
            tools,
        }
    }

    /// POST `request`, retrying once without tools if the endpoint rejects the
    /// field itself. Returns the successful response.
    async fn send(&self, api_key: &str, request: &OpenAIRequest) -> Result<reqwest::Response> {
        let post = |body: &OpenAIRequest| {
            self.client
                .post(self.api_url())
                .header("Authorization", format!("Bearer {}", api_key))
                .json(body)
                .send()
        };
        let response = post(request)
            .await
            .context("Failed to send request to OpenAI")?;
        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status().as_u16();
        let error_text = response.text().await?;
        if request.tools.is_none() || !super::openai_compat::is_tools_unsupported(&error_text) {
            anyhow::bail!("{}", Self::translate_api_error(status, &error_text));
        }
        tracing::warn!(
            model = %self.config.model,
            "OpenAI endpoint does not accept native tool definitions — retrying without them",
        );
        let retry = post(&request.without_tools())
            .await
            .context("Failed to send request to OpenAI")?;
        if !retry.status().is_success() {
            let status = retry.status().as_u16();
            let error_text = retry.text().await?;
            anyhow::bail!("{}", Self::translate_api_error(status, &error_text));
        }
        Ok(retry)
    }

    /// Translate a raw OpenAI API error response into a user-friendly message.
    fn translate_api_error(status: u16, body: &str) -> String {
        if let Ok(v) = serde_json::from_str::<Value>(body) {
            let msg = v
                .pointer("/error/message")
                .and_then(|m| m.as_str())
                .unwrap_or(body);
            return match status {
                401 => format!("Authentication failed: {}. Check your OPENAI_API_KEY or api_key_helper in config.", msg),
                403 => format!("Access denied: {}. Your API key may lack permissions for this model.", msg),
                429 => format!("Rate limited: {}. Wait a moment or check your OpenAI plan limits.", msg),
                404 => format!("Model not found: {}. Check your model name in config.", msg),
                503 => format!("OpenAI is temporarily overloaded: {}. Retry in a few seconds.", msg),
                _ => format!("OpenAI API error (HTTP {}): {}", status, msg),
            };
        }
        format!("OpenAI API error (HTTP {}): {}", status, body)
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<OpenAIMessage> {
        let mut openai_messages: Vec<OpenAIMessage> = messages
            .iter()
            .map(|m| OpenAIMessage {
                role: m.role.as_str().to_string(),
                content: Value::String(m.content.clone()),
            })
            .collect();

        if let Some(ctx) = context {
            if let Some(last_msg) = openai_messages.last_mut() {
                if last_msg.role == "user" {
                    if let Value::String(ref s) = last_msg.content.clone() {
                        last_msg.content =
                            Value::String(format!("Context:\n{}\n\nUser: {}", ctx, s));
                    }
                }
            }
        }
        openai_messages
    }

    /// Build messages with image content blocks for the last user message.
    fn build_vision_messages(
        &self,
        messages: &[Message],
        images: &[ImageAttachment],
        context: Option<String>,
    ) -> Vec<OpenAIMessage> {
        let mut openai_messages = self.build_messages(messages, context);

        if images.is_empty() {
            return openai_messages;
        }

        if let Some(last) = openai_messages.last_mut() {
            if last.role == "user" {
                let text = match &last.content {
                    Value::String(s) => s.clone(),
                    _ => String::new(),
                };

                let mut parts: Vec<Value> = images
                    .iter()
                    .map(|img| {
                        json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};base64,{}", img.media_type, img.base64)
                            }
                        })
                    })
                    .collect();

                parts.push(json!({ "type": "text", "text": text }));
                last.content = Value::Array(parts);
            }
        }

        openai_messages
    }
}

impl OpenAIProvider {
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
            .context("OpenAI API key not found")?;
        let request = self.build_request(messages, None, true); // context handled in build_messages
        let response = self.send(api_key, &request).await?;

        // The shared SSE parser rather than a second hand-rolled one. This
        // provider's own parser read `delta.content` and nothing else, so a
        // reasoning model's turn arrived empty and a native tool call was
        // dropped outright — both already solved once in `openai_compat`.
        Ok(super::openai_compat::parse_sse_stream_reporting(response, stop))
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
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
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("OpenAI API key not found")?;
        let request = self.build_request(messages, context, false);
        let response = self.send(api_key, &request).await?;

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        // `into_text` and not `.content`: a turn made entirely of tool calls
        // has an empty content, and reading only that field reports a model
        // that called a tool as one that said nothing.
        let text = openai_response
            .choices
            .into_iter()
            .next()
            .context("No choices in OpenAI response")?
            .message
            .into_text();

        let usage = openai_response.usage.map(|u| TokenUsage {
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

    fn supports_vision(&self) -> bool {
        // GPT-4 Vision, GPT-4o, and GPT-4-turbo models support images
        let m = &self.config.model;
        m.contains("gpt-4o")
            || m.contains("gpt-4-vision")
            || m.contains("gpt-4-turbo")
            || m == "gpt-4"
            || m.contains("o1")
    }

    async fn chat_with_images(
        &self,
        messages: &[Message],
        images: &[ImageAttachment],
        context: Option<String>,
    ) -> Result<String> {
        if images.is_empty() || !self.supports_vision() {
            return self.chat(messages, context).await;
        }

        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("OpenAI API key not found")?;
        // A vision turn is still a tool-capable turn: the model can look at a
        // screenshot and then call a tool about it, so the schemas ride along
        // here for the same reason they do on the text path.
        let profile = self.profile();
        let tools = super::openai_compat::ChatRequest::tools_for(messages, &profile);
        let request = OpenAIRequest {
            model: self.config.model.clone(),
            messages: self.build_vision_messages(messages, images, context),
            temperature: self.config.temperature.or(profile.temperature),
            max_tokens: self
                .config
                .max_tokens
                .or(profile.max_output_tokens.map(|t| t as usize)),
            reasoning_effort: self.reasoning_effort(),
            stream: false,
            parallel_tool_calls: super::openai_compat::ChatRequest::parallel_for(
                tools.as_ref(),
                &profile,
            ),
            tools,
        };

        let response = self.send(api_key, &request).await?;

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI vision response")?;

        Ok(openai_response
            .choices
            .into_iter()
            .next()
            .context("No choices in OpenAI vision response")?
            .message
            .into_text())
    }

    fn harness_profile(&self) -> crate::harness::ModelProfile {
        self.profile()
    }

    fn with_effort(
        &self,
        effort: crate::provider::Effort,
    ) -> Option<std::sync::Arc<dyn AIProvider>> {
        let mut cfg = self.config.clone();
        cfg.effort = Some(effort);
        Some(std::sync::Arc::new(OpenAIProvider::new(cfg)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MessageRole;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "openai".into(),
            api_key: Some("sk-test".into()),
            api_url: Some("https://api.openai.com".into()),
            model: "gpt-4o".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
            effort: None,
        }
    }

    #[test]
    fn name_is_openai() {
        let p = OpenAIProvider::new(test_config());
        assert!(p.name().starts_with("OpenAI ("));
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = OpenAIProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = OpenAIProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn supports_vision() {
        let p = OpenAIProvider::new(test_config());
        assert!(p.supports_vision());
    }

    #[test]
    fn build_messages_basic() {
        let p = OpenAIProvider::new(test_config());
        let msgs = vec![Message {
            role: MessageRole::User,
            content: "hello".into(),
        }];
        let result = p.build_messages(&msgs, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "user");
    }

    #[test]
    fn build_messages_with_context() {
        let p = OpenAIProvider::new(test_config());
        let msgs = vec![Message {
            role: MessageRole::User,
            content: "hello".into(),
        }];
        let result = p.build_messages(&msgs, Some("ctx".into()));
        assert_eq!(result.len(), 1);
        let content = result[0].content.as_str().unwrap();
        assert!(content.contains("ctx"));
        assert!(content.contains("hello"));
    }

    #[test]
    fn openai_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"hi"}}],"usage":{"prompt_tokens":3,"completion_tokens":1}}"#;
        let resp: OpenAIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.usage.unwrap().completion_tokens, 1);
    }

    // ── translate_api_error tests ────────────────────────────────────────────

    #[test]
    fn translate_401_auth() {
        let body = r#"{"error":{"message":"Incorrect API key provided"}}"#;
        let msg = OpenAIProvider::translate_api_error(401, body);
        assert!(msg.contains("Authentication failed"), "got: {msg}");
        assert!(msg.contains("OPENAI_API_KEY"));
    }

    #[test]
    fn translate_429_rate_limited() {
        let body = r#"{"error":{"message":"Rate limit exceeded"}}"#;
        let msg = OpenAIProvider::translate_api_error(429, body);
        assert!(msg.contains("Rate limited"), "got: {msg}");
    }

    #[test]
    fn translate_404_model() {
        let body = r#"{"error":{"message":"The model gpt-5 does not exist"}}"#;
        let msg = OpenAIProvider::translate_api_error(404, body);
        assert!(msg.contains("Model not found"), "got: {msg}");
    }

    #[test]
    fn translate_503_overloaded() {
        let body = r#"{"error":{"message":"Service unavailable"}}"#;
        let msg = OpenAIProvider::translate_api_error(503, body);
        assert!(msg.contains("temporarily overloaded"), "got: {msg}");
    }

    #[test]
    fn translate_500_generic() {
        let body = r#"{"error":{"message":"Internal error"}}"#;
        let msg = OpenAIProvider::translate_api_error(500, body);
        assert!(msg.contains("HTTP 500"), "got: {msg}");
    }

    #[test]
    fn translate_non_json() {
        let msg = OpenAIProvider::translate_api_error(502, "Bad Gateway");
        assert!(msg.contains("HTTP 502"), "got: {msg}");
        assert!(msg.contains("Bad Gateway"));
    }

    #[test]
    fn api_url_default() {
        let mut cfg = test_config();
        cfg.api_url = None;
        let p = OpenAIProvider::new(cfg);
        assert_eq!(p.api_url(), OpenAIProvider::DEFAULT_API_URL);
    }

    #[test]
    fn api_url_custom() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://my-proxy.com/v1/chat/completions".into());
        let p = OpenAIProvider::new(cfg);
        assert_eq!(p.api_url(), "https://my-proxy.com/v1/chat/completions");
    }
}
