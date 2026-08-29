//! Claude AI provider implementation

use crate::provider::{
    AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message,
    ProviderConfig, TokenUsage,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
struct ThinkingConfig {
    #[serde(rename = "type")]
    thinking_type: String,
    budget_tokens: u32,
}

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    /// Extended thinking — only serialized when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingConfig>,
    /// Native tool schemas, in Anthropic's flattened shape.
    ///
    /// This provider sent none until now: the tools existed only as the ~15 KB
    /// XML catalogue in the system prompt, and the model's calls were
    /// regex-parsed back out of its prose, while the Messages API had offered
    /// first-class tool use the whole time.
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
}

impl ClaudeRequest {
    /// The same request with no tools, for an endpoint or model that rejects
    /// the field.
    fn without_tools(&self) -> Self {
        Self {
            model: self.model.clone(),
            messages: self.messages.clone(),
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            stream: self.stream,
            system: self.system.clone(),
            thinking: self.thinking.as_ref().map(|t| ThinkingConfig {
                thinking_type: t.thinking_type.clone(),
                budget_tokens: t.budget_tokens,
            }),
            tools: None,
        }
    }
}

/// Supports both text-only (String) and vision (array of content blocks).
#[derive(Debug, Clone, Serialize)]
struct ClaudeMessage {
    role: String,
    content: Value, // String or array for vision
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    /// Raw blocks — see [`render_content_blocks`] for why they are not typed.
    content: Vec<Value>,
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

/// Every block of a Claude reply, rendered into the one text format the agent
/// parses.
///
/// Blocks are read as raw JSON rather than a tagged enum on purpose. A reply
/// is a *list* of blocks and they are not all text: a turn that calls a tool
/// carries a `tool_use` block with no `text` field at all. Modelling that as
/// `#[serde(tag = "type")]` would make an unfamiliar or untagged block fail
/// the whole reply — turning a turn the model completed successfully into an
/// API error. Nothing here can fail: an unrecognised block contributes
/// nothing and the rest of the turn still arrives.
///
/// All blocks are rendered, not just the first. A turn is routinely prose
/// *then* a tool call, and reading `content[0]` alone silently dropped
/// whichever came second.
fn render_content_blocks(blocks: &[Value]) -> String {
    let mut out = String::new();
    for block in blocks {
        // An absent `type` is treated as text, which is what a block carrying
        // a `text` field and nothing else can only be.
        match block.get("type").and_then(|t| t.as_str()) {
            Some("tool_use") | None if block.get("name").is_some() => {
                if let Some(name) = block.get("name").and_then(|n| n.as_str()) {
                    out.push_str(&crate::tools::render_tool_call(name, block.get("input")));
                }
            }
            Some("thinking") => {
                if let Some(text) = block
                    .get("thinking")
                    .and_then(|t| t.as_str())
                    .filter(|t| !t.is_empty())
                {
                    out.push_str("<thinking>");
                    out.push_str(text);
                    out.push_str("</thinking>\n");
                }
            }
            Some("text") | None => {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    out.push_str(text);
                }
            }
            // A block type this build does not know about. Ignored rather than
            // fatal: a new one must not break a working reply.
            Some(_) => {}
        }
    }
    out
}

#[derive(Debug, Deserialize)]
struct ClaudeStreamResponse {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<ClaudeDelta>,
    /// Present on `content_block_start`; says what kind of block is opening.
    content_block: Option<ClaudeStreamBlock>,
}

#[derive(Debug, Deserialize)]
struct ClaudeStreamBlock {
    #[serde(rename = "type")]
    block_type: String,
    /// The tool's name, on a `tool_use` block.
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeDelta {
    #[serde(default)]
    text: Option<String>,
    /// Extended-thinking text, streamed as `thinking_delta`.
    #[serde(default)]
    thinking: Option<String>,
    /// A tool call's arguments, streamed as `input_json_delta` in fragments of
    /// a JSON string that are only parseable once the block closes.
    #[serde(default)]
    partial_json: Option<String>,
}

/// Assembles Claude's SSE event stream into the one text format the agent
/// parses.
///
/// State spans events for the same two reasons the OpenAI-compatible
/// accumulator's does — a `<thinking>` wrapper opens once, and a tool call's
/// arguments arrive in fragments — plus a third specific to this API: the
/// tool's *name* arrives on `content_block_start` while its arguments arrive
/// on later `content_block_delta` events, so the name has to be held.
#[derive(Debug, Default)]
struct ClaudeSseAccumulator {
    thinking_open: bool,
    tool_name: Option<String>,
    tool_args: String,
    /// Bytes not yet forming a whole line. An HTTP chunk boundary falls
    /// wherever the network puts it, and a truncated event parses as nothing.
    line_buf: String,
}

impl ClaudeSseAccumulator {
    fn push(&mut self, text: &str) -> String {
        self.line_buf.push_str(text);
        let consumed = self.line_buf.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let complete: String = self.line_buf.drain(..consumed).collect();
        self.parse_lines(&complete)
    }

    fn parse_lines(&mut self, text: &str) -> String {
        let mut out = String::new();
        for line in text.lines() {
            let Some(data) = line.trim_end_matches('\r').strip_prefix("data: ") else {
                continue;
            };
            let Ok(event) = serde_json::from_str::<ClaudeStreamResponse>(data) else {
                continue;
            };
            match event.event_type.as_str() {
                "content_block_start" => {
                    // A new block ends whatever the last one was.
                    out.push_str(&self.flush_tool_call());
                    if let Some(block) = event.content_block {
                        if block.block_type == "tool_use" {
                            out.push_str(&self.close_thinking());
                            self.tool_name = block.name;
                        }
                    }
                }
                "content_block_delta" => {
                    let Some(delta) = event.delta else { continue };
                    if let Some(thinking) = delta.thinking.filter(|t| !t.is_empty()) {
                        if !self.thinking_open {
                            out.push_str("<thinking>");
                            self.thinking_open = true;
                        }
                        out.push_str(&thinking);
                    }
                    if let Some(text) = delta.text.filter(|t| !t.is_empty()) {
                        out.push_str(&self.close_thinking());
                        out.push_str(&text);
                    }
                    if let Some(fragment) = delta.partial_json {
                        self.tool_args.push_str(&fragment);
                    }
                }
                "content_block_stop" => out.push_str(&self.flush_tool_call()),
                _ => {}
            }
        }
        out
    }

    /// Close anything still open at the end of the stream, including a final
    /// event that arrived without a trailing newline.
    fn finish(&mut self) -> String {
        let tail = std::mem::take(&mut self.line_buf);
        let mut out = self.parse_lines(&tail);
        out.push_str(&self.flush_tool_call());
        out.push_str(&self.close_thinking());
        out
    }

    fn close_thinking(&mut self) -> String {
        match std::mem::take(&mut self.thinking_open) {
            true => "</thinking>\n".to_string(),
            false => String::new(),
        }
    }

    /// Emit the in-flight tool call, if any, as `<tool_call>` markup.
    fn flush_tool_call(&mut self) -> String {
        let Some(name) = self.tool_name.take() else {
            self.tool_args.clear();
            return String::new();
        };
        let args = std::mem::take(&mut self.tool_args);
        let parsed = serde_json::from_str::<Value>(&args).ok();
        crate::tools::render_tool_call(&name, parsed.as_ref())
    }
}

/// Claude AI provider (Anthropic)
pub struct ClaudeProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    display_name: String,
}

impl ClaudeProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let display_name = format!("Claude ({})", config.model);
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

    const DEFAULT_API_URL: &'static str = "https://api.anthropic.com/v1/messages";

    fn api_url(&self) -> &str {
        self.config
            .api_url
            .as_deref()
            .unwrap_or(Self::DEFAULT_API_URL)
    }

    /// Translate a raw Claude API error response into a user-friendly message.
    fn translate_api_error(status: u16, body: &str) -> String {
        // Try to parse as JSON to extract the message field
        if let Ok(v) = serde_json::from_str::<Value>(body) {
            let msg = v
                .pointer("/error/message")
                .and_then(|m| m.as_str())
                .unwrap_or(body);
            let err_type = v
                .pointer("/error/type")
                .and_then(|t| t.as_str())
                .unwrap_or("");
            return match (status, err_type) {
                (401, _) => format!("Authentication failed: {}. Check your ANTHROPIC_API_KEY or api_key_helper in config.", msg),
                (403, _) => format!("Access denied: {}. Your API key may lack permissions for this model.", msg),
                (429, _) => format!("Rate limited: {}. Wait a moment or check your Anthropic plan limits.", msg),
                (404, _) => format!("Model not found: {}. Check your model name in config.", msg),
                (529, _) | (503, _) => format!("Claude is temporarily overloaded: {}. Retry in a few seconds.", msg),
                _ => format!("Claude API error (HTTP {}): {}", status, msg),
            };
        }
        format!("Claude API error (HTTP {}): {}", status, body)
    }

    fn build_messages(
        &self,
        messages: &[Message],
        context: Option<String>,
    ) -> (Vec<ClaudeMessage>, Option<String>) {
        let mut claude_messages = Vec::new();
        let mut system_prompt = None;

        for m in messages {
            if let crate::provider::MessageRole::System = m.role {
                system_prompt = Some(m.content.clone());
            } else {
                let content = if let Some(ctx) = context.as_ref() {
                    if m.role == crate::provider::MessageRole::User
                        && claude_messages
                            .iter()
                            .all(|cm: &ClaudeMessage| cm.role != "user")
                    {
                        Value::String(format!("Context:\n{}\n\nUser: {}", ctx, m.content))
                    } else {
                        Value::String(m.content.clone())
                    }
                } else {
                    Value::String(m.content.clone())
                };
                claude_messages.push(ClaudeMessage {
                    role: m.role.as_str().to_string(),
                    content,
                });
            }
        }

        (claude_messages, system_prompt)
    }

    /// Build the optional extended-thinking config from provider settings.
    ///
    /// An explicit `thinking_budget_tokens` always wins; otherwise the per-request
    /// `effort` tier (gap C5) derives the budget — `Effort::Low` disables thinking.
    fn thinking_config(&self) -> Option<ThinkingConfig> {
        let budget = self
            .config
            .thinking_budget_tokens
            .or_else(|| self.config.effort.and_then(|e| e.claude_thinking_budget()))?;
        Some(ThinkingConfig {
            thinking_type: "enabled".to_string(),
            budget_tokens: budget,
        })
    }

    /// Build a vision request message with text + images.
    fn build_vision_messages(
        &self,
        messages: &[Message],
        images: &[ImageAttachment],
    ) -> (Vec<ClaudeMessage>, Option<String>) {
        let mut claude_messages = Vec::new();
        let mut system_prompt = None;

        for (i, m) in messages.iter().enumerate() {
            if let crate::provider::MessageRole::System = m.role {
                system_prompt = Some(m.content.clone());
                continue;
            }
            // Attach images to the last user message.
            let is_last_user =
                m.role == crate::provider::MessageRole::User && i == messages.len() - 1;
            let content = if is_last_user && !images.is_empty() {
                let mut blocks: Vec<Value> = images
                    .iter()
                    .map(|img| {
                        serde_json::json!({
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": img.media_type,
                                "data": img.base64,
                            }
                        })
                    })
                    .collect();
                blocks.push(serde_json::json!({ "type": "text", "text": m.content }));
                Value::Array(blocks)
            } else {
                Value::String(m.content.clone())
            };
            claude_messages.push(ClaudeMessage {
                role: m.role.as_str().to_string(),
                content,
            });
        }

        (claude_messages, system_prompt)
    }
}

impl ClaudeProvider {
    fn profile(&self) -> crate::harness::ModelProfile {
        crate::harness::profile_for("claude", &self.config.model)
    }

    /// Tool schemas for this conversation, in Anthropic's shape.
    fn tools_for(&self, messages: &[Message]) -> Option<Vec<Value>> {
        let profile = self.profile();
        if !profile.sends_tool_schemas() {
            return None;
        }
        let defs =
            crate::tools::tool_definitions_for(messages.iter().map(|m| m.content.as_str()))?;
        Some(crate::tools::to_anthropic_shape(&defs))
    }

    /// Assemble a request from already-built messages.
    ///
    /// Takes the messages rather than building them because the vision path
    /// builds a different shape but wants every other decision here identical.
    fn assemble(
        &self,
        claude_messages: Vec<ClaudeMessage>,
        system: Option<String>,
        tools: Option<Vec<Value>>,
        stream: bool,
    ) -> ClaudeRequest {
        let profile = self.profile();
        ClaudeRequest {
            model: self.config.model.clone(),
            messages: claude_messages,
            // The existing 16_384 stays the floor for every model that has no
            // configured cap. It is not a measured limit for any particular
            // model and this change does not make it one — it is what shipped,
            // kept so nothing regresses on the way to being told a real figure.
            max_tokens: self
                .config
                .max_tokens
                .or(profile.max_output_tokens.map(|t| t as usize))
                .or(Some(16_384)),
            temperature: self.config.temperature.or(profile.temperature),
            stream,
            system,
            thinking: self.thinking_config(),
            tools,
        }
    }

    /// POST `request`, retrying once without tools if the API rejects the
    /// field itself rather than anything about the conversation.
    async fn send(&self, api_key: &str, request: &ClaudeRequest) -> Result<reqwest::Response> {
        let post = |body: &ClaudeRequest| {
            self.client
                .post(self.api_url())
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(body)
                .send()
        };
        let response = post(request)
            .await
            .context("Failed to send request to Claude")?;
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status().as_u16();
        let error_text = response.text().await?;
        if request.tools.is_none()
            || !super::openai_compat::is_tools_unsupported(&error_text)
        {
            anyhow::bail!("{}", Self::translate_api_error(status, &error_text));
        }
        tracing::warn!(
            model = %self.config.model,
            "Claude endpoint does not accept tool definitions — retrying without them",
        );
        let retry = post(&request.without_tools())
            .await
            .context("Failed to send request to Claude")?;
        if !retry.status().is_success() {
            let status = retry.status().as_u16();
            let error_text = retry.text().await?;
            anyhow::bail!("{}", Self::translate_api_error(status, &error_text));
        }
        Ok(retry)
    }
}

#[async_trait]
impl AIProvider for ClaudeProvider {
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
        let messages = vec![Message {
            role: crate::provider::MessageRole::User,
            content: prompt,
        }];
        self.chat_response(&messages, None).await
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );

        let messages = vec![Message {
            role: crate::provider::MessageRole::User,
            content: prompt,
        }];

        self.stream_chat(&messages).await
    }

    async fn chat_response(
        &self,
        messages: &[Message],
        context: Option<String>,
    ) -> Result<CompletionResponse> {
        let api_key = self
            .config
            .resolve_api_key()
            .await
            .context("Claude API key not found")?;
        let (claude_messages, system) = self.build_messages(messages, context);

        let request = self.assemble(claude_messages, system, self.tools_for(messages), false);
        let response = self.send(&api_key, &request).await?;

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .context("Failed to parse Claude response")?;

        // Every block, not just the first: a turn is routinely prose *then* a
        // tool call, and reading `content[0]` dropped whichever came second.
        let text = render_content_blocks(&claude_response.content);

        let usage = claude_response.usage.map(|u| TokenUsage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
        });

        Ok(CompletionResponse {
            text,
            model: self.config.model.clone(),
            usage,
        })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let api_key = self
            .config
            .resolve_api_key()
            .await
            .context("Claude API key not found")?;
        let (claude_messages, system) = self.build_messages(messages, context);

        let request = self.assemble(claude_messages, system, self.tools_for(messages), false);
        let response = self.send(&api_key, &request).await?;

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .context("Failed to parse Claude response")?;

        Ok(render_content_blocks(&claude_response.content))
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self
            .config
            .resolve_api_key()
            .await
            .context("Claude API key not found")?;
        let (claude_messages, system) = self.build_messages(messages, None);

        let request = self.assemble(claude_messages, system, self.tools_for(messages), true);
        let response = self.send(&api_key, &request).await?;

        let acc = std::sync::Arc::new(std::sync::Mutex::new(ClaudeSseAccumulator::default()));
        let tail = acc.clone();
        Ok(response
            .bytes_stream()
            .map(move |chunk| {
                let chunk = chunk?;
                let text = String::from_utf8_lossy(&chunk);
                Ok(acc
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(text.as_ref()))
            })
            // A stream that ends without closing its last block still has to
            // release whatever the accumulator is holding.
            .chain(futures::stream::once(async move {
                Ok(tail.lock().unwrap_or_else(|e| e.into_inner()).finish())
            }))
            .boxed())
    }

    fn harness_profile(&self) -> crate::harness::ModelProfile {
        self.profile()
    }

    fn supports_vision(&self) -> bool {
        // Claude 3+ models support vision.
        self.config.model.contains("claude-3")
            || self.config.model.contains("claude-sonnet")
            || self.config.model.contains("claude-opus")
            || self.config.model.contains("claude-haiku")
    }

    async fn chat_with_images(
        &self,
        messages: &[Message],
        images: &[ImageAttachment],
        _context: Option<String>,
    ) -> Result<String> {
        let api_key = self
            .config
            .resolve_api_key()
            .await
            .context("Claude API key not found")?;
        let (claude_messages, system) = self.build_vision_messages(messages, images);

        // A vision turn is still a tool-capable turn: the model can read a
        // screenshot and then call a tool about it.
        let request = self.assemble(claude_messages, system, self.tools_for(messages), false);
        let response = self.send(&api_key, &request).await?;

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .context("Failed to parse Claude vision response")?;
        Ok(render_content_blocks(&claude_response.content))
    }

    fn with_effort(
        &self,
        effort: crate::provider::Effort,
    ) -> Option<std::sync::Arc<dyn AIProvider>> {
        let mut cfg = self.config.clone();
        cfg.effort = Some(effort);
        Some(std::sync::Arc::new(ClaudeProvider::new(cfg)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MessageRole;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "claude".into(),
            api_key: Some("test-key".into()),
            api_url: Some("https://api.anthropic.com".into()),
            model: "claude-sonnet-4-20250514".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
            effort: None,
        }
    }

    #[test]
    fn name_is_claude() {
        let p = ClaudeProvider::new(test_config());
        assert!(p.name().starts_with("Claude ("));
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = ClaudeProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn is_not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = ClaudeProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn supports_vision() {
        let p = ClaudeProvider::new(test_config());
        assert!(p.supports_vision());
    }

    #[test]
    fn build_messages_extracts_system() {
        let p = ClaudeProvider::new(test_config());
        let msgs = vec![
            Message {
                role: MessageRole::System,
                content: "sys".into(),
            },
            Message {
                role: MessageRole::User,
                content: "hi".into(),
            },
        ];
        let (claude_msgs, sys) = p.build_messages(&msgs, None);
        assert_eq!(sys.as_deref(), Some("sys"));
        assert_eq!(claude_msgs.len(), 1);
        assert_eq!(claude_msgs[0].role, "user");
    }

    #[test]
    fn claude_request_serde() {
        let req = ClaudeRequest {
            model: "claude-sonnet-4-20250514".into(),
            messages: vec![ClaudeMessage {
                role: "user".into(),
                content: Value::String("hi".into()),
            }],
            max_tokens: Some(1024),
            temperature: None,
            stream: false,
            system: None,
            thinking: None,
            tools: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "claude-sonnet-4-20250514");
        assert!(json.get("system").is_none()); // skip_serializing_if
        assert!(json.get("thinking").is_none());
    }

    #[test]
    fn claude_response_deser() {
        let json =
            r#"{"content":[{"text":"hello world"}],"usage":{"input_tokens":10,"output_tokens":5}}"#;
        let resp: ClaudeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(render_content_blocks(&resp.content), "hello world");
        assert_eq!(resp.usage.unwrap().output_tokens, 5);
    }

    // ── content blocks ───────────────────────────────────────────────────

    /// The reason this provider needed changing at all: the Messages API
    /// returns tool calls as `tool_use` blocks, and nothing here read them.
    #[test]
    fn a_tool_use_block_becomes_a_tool_call() {
        let json = r#"{"content":[
            {"type":"tool_use","id":"tu_1","name":"read_file","input":{"path":"src/main.rs"}}
        ]}"#;
        let resp: ClaudeResponse = serde_json::from_str(json).unwrap();
        let text = render_content_blocks(&resp.content);
        assert_eq!(
            text,
            r#"<tool_call name="read_file"><path>src/main.rs</path></tool_call>"#
        );
        assert_eq!(crate::tools::parse_tool_calls(&text).len(), 1);
    }

    /// A turn is routinely prose *then* a tool call. Reading `content[0]`
    /// alone — which is what shipped — dropped whichever came second.
    #[test]
    fn prose_and_a_tool_call_in_one_turn_both_survive() {
        let json = r#"{"content":[
            {"type":"text","text":"Let me look at that file.\n"},
            {"type":"tool_use","name":"read_file","input":{"path":"a.rs"}}
        ]}"#;
        let resp: ClaudeResponse = serde_json::from_str(json).unwrap();
        let text = render_content_blocks(&resp.content);
        assert!(text.starts_with("Let me look at that file."));
        assert_eq!(crate::tools::parse_tool_calls(&text).len(), 1);
    }

    #[test]
    fn two_tool_calls_in_one_turn_both_survive() {
        let json = r#"{"content":[
            {"type":"tool_use","name":"read_file","input":{"path":"a.rs"}},
            {"type":"tool_use","name":"read_file","input":{"path":"b.rs"}}
        ]}"#;
        let resp: ClaudeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            crate::tools::parse_tool_calls(&render_content_blocks(&resp.content)).len(),
            2
        );
    }

    #[test]
    fn a_thinking_block_is_wrapped() {
        let json = r#"{"content":[
            {"type":"thinking","thinking":"weighing it up"},
            {"type":"text","text":"done"}
        ]}"#;
        let resp: ClaudeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            render_content_blocks(&resp.content),
            "<thinking>weighing it up</thinking>\ndone"
        );
    }

    /// A block type this build has never seen must contribute nothing and
    /// break nothing — never turn a completed turn into an API error.
    #[test]
    fn an_unknown_block_type_is_ignored_not_fatal() {
        let json = r#"{"content":[
            {"type":"some_future_block","payload":{"a":1}},
            {"type":"text","text":"still here"}
        ]}"#;
        let resp: ClaudeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(render_content_blocks(&resp.content), "still here");
    }

    // ── streaming ────────────────────────────────────────────────────────

    #[test]
    fn a_streamed_tool_call_is_assembled_across_events() {
        let mut acc = ClaudeSseAccumulator::default();
        let mut out = String::new();
        for event in [
            r#"data: {"type":"content_block_start","content_block":{"type":"tool_use","name":"read_file"}}"#,
            r#"data: {"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{\"path\":"}}"#,
            r#"data: {"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"\"a.rs\"}"}}"#,
            r#"data: {"type":"content_block_stop"}"#,
        ] {
            out.push_str(&acc.push(&format!("{event}\n")));
        }
        out.push_str(&acc.finish());
        assert_eq!(
            out,
            r#"<tool_call name="read_file"><path>a.rs</path></tool_call>"#
        );
    }

    #[test]
    fn streamed_text_is_unchanged() {
        let mut acc = ClaudeSseAccumulator::default();
        let mut out = String::new();
        out.push_str(&acc.push(
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n",
        ));
        out.push_str(&acc.finish());
        assert_eq!(out, "hi");
    }

    /// Extended thinking streams as its own delta type; before this it was
    /// dropped, so a high-effort turn showed nothing until the prose began.
    #[test]
    fn streamed_thinking_is_wrapped_once_and_closed_by_prose() {
        let mut acc = ClaudeSseAccumulator::default();
        let mut out = String::new();
        for event in [
            r#"data: {"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"weigh"}}"#,
            r#"data: {"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"ing"}}"#,
            r#"data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"done"}}"#,
        ] {
            out.push_str(&acc.push(&format!("{event}\n")));
        }
        out.push_str(&acc.finish());
        assert_eq!(out, "<thinking>weighing</thinking>\ndone");
    }

    /// A turn that is only thinking still has to close its wrapper.
    #[test]
    fn a_thinking_only_stream_is_closed_at_the_end() {
        let mut acc = ClaudeSseAccumulator::default();
        let mut out = String::new();
        out.push_str(&acc.push(
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"hm\"}}\n",
        ));
        out.push_str(&acc.finish());
        assert_eq!(out, "<thinking>hm</thinking>\n");
    }

    /// The same chunk-boundary hazard as every other SSE parser here.
    #[test]
    fn a_claude_event_split_across_chunks_is_not_lost() {
        let mut acc = ClaudeSseAccumulator::default();
        let mut out = String::new();
        out.push_str(&acc.push(
            "data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_de",
        ));
        out.push_str(&acc.push("lta\",\"text\":\"split\"}}\n"));
        out.push_str(&acc.finish());
        assert_eq!(out, "split");
    }

    #[test]
    fn a_stream_ending_without_a_block_stop_still_emits_the_call() {
        let mut acc = ClaudeSseAccumulator::default();
        let mut out = String::new();
        out.push_str(&acc.push(
            "data: {\"type\":\"content_block_start\",\"content_block\":{\"type\":\"tool_use\",\"name\":\"build\"}}\n",
        ));
        out.push_str(&acc.finish());
        assert_eq!(out, r#"<tool_call name="build"></tool_call>"#);
    }

    // ── translate_api_error tests ────────────────────────────────────────────

    #[test]
    fn translate_401_auth_error() {
        let body = r#"{"error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        let msg = ClaudeProvider::translate_api_error(401, body);
        assert!(msg.contains("Authentication failed"), "got: {msg}");
        assert!(msg.contains("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn translate_429_rate_limit() {
        let body = r#"{"error":{"type":"rate_limit_error","message":"Too many requests"}}"#;
        let msg = ClaudeProvider::translate_api_error(429, body);
        assert!(msg.contains("Rate limited"), "got: {msg}");
        assert!(msg.contains("plan limits"));
    }

    #[test]
    fn translate_404_model_not_found() {
        let body = r#"{"error":{"type":"not_found_error","message":"model: foo-bar not found"}}"#;
        let msg = ClaudeProvider::translate_api_error(404, body);
        assert!(msg.contains("Model not found"), "got: {msg}");
    }

    #[test]
    fn translate_529_overloaded() {
        let body =
            r#"{"error":{"type":"overloaded_error","message":"Claude is overloaded right now"}}"#;
        let msg = ClaudeProvider::translate_api_error(529, body);
        assert!(msg.contains("temporarily overloaded"), "got: {msg}");
    }

    #[test]
    fn translate_503_service_unavailable() {
        let body = r#"{"error":{"type":"api_error","message":"Internal server error"}}"#;
        let msg = ClaudeProvider::translate_api_error(503, body);
        assert!(msg.contains("temporarily overloaded"), "got: {msg}");
    }

    #[test]
    fn translate_403_access_denied() {
        let body = r#"{"error":{"type":"permission_error","message":"Not allowed"}}"#;
        let msg = ClaudeProvider::translate_api_error(403, body);
        assert!(msg.contains("Access denied"), "got: {msg}");
    }

    #[test]
    fn translate_unknown_status() {
        let body = r#"{"error":{"type":"unknown","message":"Something happened"}}"#;
        let msg = ClaudeProvider::translate_api_error(500, body);
        assert!(msg.contains("HTTP 500"), "got: {msg}");
        assert!(msg.contains("Something happened"));
    }

    #[test]
    fn translate_non_json_body() {
        let msg = ClaudeProvider::translate_api_error(502, "Bad Gateway");
        assert!(msg.contains("HTTP 502"), "got: {msg}");
        assert!(msg.contains("Bad Gateway"));
    }

    #[test]
    fn api_url_uses_default_when_none() {
        let mut cfg = test_config();
        cfg.api_url = None;
        let p = ClaudeProvider::new(cfg);
        assert_eq!(p.api_url(), ClaudeProvider::DEFAULT_API_URL);
    }

    #[test]
    fn api_url_uses_custom_when_set() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://custom.api.com/v1/messages".into());
        let p = ClaudeProvider::new(cfg);
        assert_eq!(p.api_url(), "https://custom.api.com/v1/messages");
    }
}
