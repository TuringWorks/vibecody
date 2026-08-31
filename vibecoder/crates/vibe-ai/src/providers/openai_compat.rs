//! Shared infrastructure for OpenAI-compatible providers.
//!
//! Most AI providers (Groq, Mistral, DeepSeek, Cerebras, Grok, OpenRouter,
//! Perplexity, Together, Fireworks, SambaNova, MiniMax, Zhipu, VercelAI)
//! use the same OpenAI-compatible API schema. This module extracts the
//! duplicated types, HTTP client construction, SSE stream parsing, and
//! message building into shared utilities.

use crate::provider::{CompletionResponse, CompletionStream, Message, TokenUsage, record_stop_reason, StopReason, StopReasonSink};
use anyhow::{Context, Result};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

// ── Shared Request/Response Types ───────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    pub stream: bool,
    /// Native tool schemas for this conversation, when it asked for tools.
    ///
    /// Every local OpenAI-compatible server (LM Studio, llama.cpp, vLLM, Jan)
    /// speaks this schema. Describing tools only in the system prompt leaves a
    /// tool-trained model nothing to call — it narrates its intent and returns
    /// an empty turn. The response half of the round trip already exists:
    /// native calls are transcribed back to `<tool_call>` markup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
    /// Whether the model may emit several tool calls in one turn.
    ///
    /// Absent unless the pair's profile says something. Sending
    /// `parallel_tool_calls` to an endpoint that has never heard of it is a
    /// 400 on providers that reject unknown fields, so silence is the only
    /// safe default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
}

impl ChatRequest {
    /// The tools this conversation expects, or `None` for a plain chat or a
    /// pair configured onto the prose path.
    ///
    /// Providers call this instead of hand-writing the field so two rules
    /// decide for all of them: `tools::tool_definitions_for` answers *which*
    /// vocabulary this conversation asked for, and the pair's
    /// [`ModelProfile`](crate::harness::ModelProfile) answers whether the
    /// schemas go on the wire at all. Both have to agree — a conversation with
    /// no tool prompt gets no schemas even on a native pair, and a pair pinned
    /// to `Prose` gets none even when the prompt asks for tools.
    pub fn tools_for(
        messages: &[Message],
        profile: &crate::harness::ModelProfile,
    ) -> Option<Vec<serde_json::Value>> {
        if !profile.sends_tool_schemas() {
            return None;
        }
        crate::tools::tool_definitions_for(messages.iter().map(|m| m.content.as_str()))
    }

    /// `parallel_tool_calls` for this request: only meaningful alongside
    /// schemas, so it stays absent when there are none.
    pub fn parallel_for(
        tools: Option<&Vec<serde_json::Value>>,
        profile: &crate::harness::ModelProfile,
    ) -> Option<bool> {
        tools.and(profile.parallel_tool_calls)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
    #[serde(default)]
    pub usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: ResponseMessage,
}

/// The assistant message in a **non-streaming** reply.
///
/// Separate from [`ChatMessage`], which is what we send, because the two
/// genuinely differ: a reply can carry structured tool calls and reasoning that
/// a request never does. Sharing one type is how the non-streaming path came to
/// drop native tool calls on the floor — `ChatMessage` had `content` and
/// nothing else, so a turn whose whole substance was a `tool_calls` array
/// deserialised to an empty string and the caller saw a model that had said
/// nothing.
#[derive(Debug, Deserialize)]
pub struct ResponseMessage {
    /// Absent when the turn was entirely tool calls or entirely reasoning.
    ///
    /// Both *missing* and *explicitly null* have to be tolerated: OpenAI sends
    /// `"content": null` for precisely the tool-call turn this type exists to
    /// read, and `#[serde(default)]` alone does not accept a null into a
    /// `String` — the whole response would fail to parse.
    #[serde(default, deserialize_with = "null_as_empty_string")]
    pub content: String,
    #[serde(default)]
    pub tool_calls: Option<Vec<NativeToolCall>>,
    /// Reasoning, under either of the two names vendors use for it.
    #[serde(default)]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
}

/// `null` and a missing field both read as an empty string.
fn null_as_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

impl ResponseMessage {
    /// This message as the text the rest of the stack consumes: reasoning in a
    /// `<thinking>` wrapper, then prose, then any tool calls transcribed to
    /// `<tool_call>` markup.
    ///
    /// The same shape the streaming accumulator produces, so a caller cannot
    /// tell which path a turn came down — which is the point, since the agent
    /// parses one format.
    pub fn into_text(self) -> String {
        let mut out = String::new();
        if let Some(reasoning) = self
            .reasoning_content
            .as_deref()
            .or(self.reasoning.as_deref())
            .filter(|t| !t.is_empty())
        {
            out.push_str("<thinking>");
            out.push_str(reasoning);
            out.push_str("</thinking>\n");
        }
        out.push_str(&self.content);
        for call in self.tool_calls.iter().flatten() {
            let Some(function) = call.function.as_ref() else {
                continue;
            };
            let Some(name) = function.name.as_deref() else {
                continue;
            };
            let args = function
                .arguments
                .as_deref()
                .and_then(|a| serde_json::from_str::<serde_json::Value>(a).ok());
            out.push_str(&crate::tools::render_tool_call(name, args.as_ref()));
        }
        out
    }
}

#[derive(Debug, Deserialize)]
pub struct StreamResponse {
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
pub struct StreamChoice {
    pub delta: StreamDelta,
    /// Why generation ended, present only on the final choice of the stream.
    ///
    /// OpenAI's vocabulary: `"stop"`, `"length"`, `"tool_calls"`,
    /// `"content_filter"`. The field was absent here, so serde dropped it and a
    /// reply cut off at `max_tokens` was indistinguishable from a finished one
    /// across every OpenAI-compatible provider at once.
    #[serde(default)]
    pub finish_reason: Option<String>,
}

/// Map an OpenAI-shaped `finish_reason` onto [`StopReason`].
///
/// `None` in, `None` out: a chunk without the field is simply not the final
/// one, and must not be read as a clean finish.
pub fn openai_stop_reason(finish_reason: Option<&str>) -> Option<StopReason> {
    match finish_reason? {
        "stop" => Some(StopReason::Natural),
        "length" | "max_tokens" => Some(StopReason::Length),
        "tool_calls" | "function_call" => Some(StopReason::ToolUse),
        "content_filter" => Some(StopReason::Filtered),
        other => Some(StopReason::Other(other.to_string())),
    }
}

#[derive(Debug, Deserialize)]
pub struct StreamDelta {
    pub content: Option<String>,
    /// Reasoning tokens. DeepSeek-R1 and Zhipu/GLM call this
    /// `reasoning_content`; OpenRouter and several proxies call it
    /// `reasoning`. A turn made entirely of reasoning leaves `content` empty,
    /// so dropping both fields renders that turn as an empty response.
    #[serde(default)]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub reasoning: Option<String>,
    /// Native (structured) tool calls, emitted by some hosted models instead
    /// of the `<tool_call>` markup the agent parses. Transcribed on the way out.
    #[serde(default)]
    pub tool_calls: Option<Vec<NativeToolCall>>,
}

impl StreamDelta {
    /// The reasoning text for this delta, whichever field carried it.
    fn reasoning_text(&self) -> Option<&str> {
        self.reasoning_content
            .as_deref()
            .or(self.reasoning.as_deref())
            .filter(|t| !t.is_empty())
    }
}

/// One structured tool call, in either a streaming delta or a complete reply.
///
/// Named for what it is rather than where it arrives: the non-streaming path
/// deserialises the identical shape, and giving it a second name would invite
/// a second, subtly different transcription.
#[derive(Debug, Deserialize)]
pub struct NativeToolCall {
    #[serde(default)]
    pub function: Option<NativeToolFunction>,
}

#[derive(Debug, Deserialize)]
pub struct NativeToolFunction {
    #[serde(default)]
    pub name: Option<String>,
    /// OpenAI streams arguments as a JSON *string*, assembled across deltas.
    #[serde(default)]
    pub arguments: Option<String>,
}

// ── Shared HTTP Client Factory ──────────────────────────────────────────────

/// Create the standard HTTP client used by all providers.
///
/// - Request timeout: 90 seconds (allows streaming completions)
/// - Connect timeout: 10 seconds (fail fast on unreachable hosts)
pub fn default_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

// ── Shared Message Builder ──────────────────────────────────────────────────

/// Convert vibe-ai Messages into OpenAI-compatible ChatMessages,
/// optionally injecting context into the last user message.
pub fn build_messages(messages: &[Message], context: Option<String>) -> Vec<ChatMessage> {
    let mut result: Vec<ChatMessage> = messages
        .iter()
        .map(|m| ChatMessage {
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

// ── Shared SSE Stream Parser ────────────────────────────────────────────────

/// Assembles one SSE stream into the text the rest of the stack consumes.
///
/// State has to span chunks: reasoning arrives token-by-token (so the
/// `<thinking>` wrapper opens once and closes when prose or a tool call
/// follows), and a native tool call's arguments arrive as JSON string
/// fragments that are only parseable once complete.
#[derive(Debug, Default)]
struct SseAccumulator {
    thinking_open: bool,
    tool_name: Option<String>,
    tool_args: String,
    /// The stop reason from the final chunk, when the endpoint sent one.
    ///
    /// Held on the accumulator rather than returned, because `push` returns the
    /// text to emit downstream and the reason is not text.
    stop_reason: Option<StopReason>,
    /// Bytes received that do not yet end a line.
    ///
    /// An HTTP chunk boundary falls wherever the network puts it, not on a
    /// newline. Splitting each chunk with `lines()` therefore hands the parser
    /// a truncated JSON event and then an orphan fragment — both fail to parse,
    /// both are skipped as "malformed", and the token in them is gone with no
    /// error anywhere. The Copilot provider had already hit this and grown its
    /// own line buffer; this is that fix, in the parser every provider shares.
    line_buf: String,
}

impl SseAccumulator {
    /// Feed one chunk of SSE text; returns the text to emit downstream.
    ///
    /// Only whole lines are parsed. A trailing partial line is held until the
    /// chunk that completes it arrives.
    fn push(&mut self, text: &str) -> String {
        self.line_buf.push_str(text);
        let consumed = self.line_buf.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let complete: String = self.line_buf.drain(..consumed).collect();
        self.parse_lines(&complete)
    }

    fn parse_lines(&mut self, text: &str) -> String {
        let mut out = String::new();
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            let Some(data) = line.strip_prefix("data: ") else {
                continue;
            };
            if data == "[DONE]" {
                out.push_str(&self.finish());
                continue;
            }
            // Malformed / partial JSON is skipped, as before.
            let Ok(response) = serde_json::from_str::<StreamResponse>(data) else {
                continue;
            };
            // The final chunk carries `finish_reason`; capture it before the
            // `delta` guard below, which skips chunks with no choices.
            if let Some(reason) = response
                .choices
                .first()
                .and_then(|c| openai_stop_reason(c.finish_reason.as_deref()))
            {
                self.stop_reason = Some(reason);
            }
            let Some(delta) = response.choices.first().map(|c| &c.delta) else {
                continue;
            };

            if let Some(reasoning) = delta.reasoning_text() {
                if !self.thinking_open {
                    out.push_str("<thinking>");
                    self.thinking_open = true;
                }
                out.push_str(reasoning);
            }

            let content = delta.content.as_deref().unwrap_or("");
            if self.thinking_open && (!content.is_empty() || delta.tool_calls.is_some()) {
                out.push_str("</thinking>\n");
                self.thinking_open = false;
            }
            out.push_str(content);

            for call in delta.tool_calls.iter().flatten() {
                let Some(function) = call.function.as_ref() else {
                    continue;
                };
                // A named delta starts a call; unnamed deltas extend the
                // arguments of the one in flight.
                if let Some(name) = function.name.as_deref() {
                    out.push_str(&self.flush_tool_call());
                    self.tool_name = Some(name.to_string());
                }
                if let Some(args) = function.arguments.as_deref() {
                    self.tool_args.push_str(args);
                }
            }
        }
        out
    }

    /// Close anything still open at the end of the stream.
    ///
    /// A stream that ends without a trailing newline leaves its last event in
    /// `line_buf`; it is a complete event, just unterminated, so it is parsed
    /// here rather than discarded.
    fn finish(&mut self) -> String {
        let tail = std::mem::take(&mut self.line_buf);
        let mut out = self.parse_lines(&tail);
        out.push_str(&self.flush_tool_call());
        if self.thinking_open {
            out.push_str("</thinking>\n");
            self.thinking_open = false;
        }
        out
    }

    /// Emit the in-flight tool call, if any, as `<tool_call>` markup.
    fn flush_tool_call(&mut self) -> String {
        let Some(name) = self.tool_name.take() else {
            self.tool_args.clear();
            return String::new();
        };
        let args = std::mem::take(&mut self.tool_args);
        let parsed = serde_json::from_str::<serde_json::Value>(&args).ok();
        crate::tools::render_tool_call(&name, parsed.as_ref())
    }
}

/// Parse an SSE byte stream from an OpenAI-compatible API into a CompletionStream.
///
/// Handles `data: [DONE]` and malformed JSON (silently skipped), and extracts
/// content, reasoning, and native tool calls from `choices[0].delta`.
pub fn parse_sse_stream(response: reqwest::Response) -> CompletionStream {
    parse_sse_stream_reporting(response, None)
}

/// [`parse_sse_stream`], additionally recording why the stream ended.
///
/// One parser serves every OpenAI-compatible provider, so capturing the reason
/// here is what makes `finish_reason` reach the UI for all of them at once
/// rather than provider by provider.
pub fn parse_sse_stream_reporting(
    response: reqwest::Response,
    stop: Option<StopReasonSink>,
) -> CompletionStream {
    let acc = std::sync::Arc::new(std::sync::Mutex::new(SseAccumulator::default()));
    let tail_acc = acc.clone();
    response
        .bytes_stream()
        .map(move |chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            Ok(acc
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(text.as_ref()))
        })
        // A stream that ends without `[DONE]` still has to release whatever
        // the accumulator is holding.
        .chain(futures::stream::once(async move {
            let mut guard = tail_acc.lock().unwrap_or_else(|e| e.into_inner());
            let tail = guard.finish();
            // Reported once the stream is drained, so a caller reading the sink
            // after the last chunk sees the reason rather than a race.
            if let (Some(sink), Some(reason)) = (stop.as_ref(), guard.stop_reason.clone()) {
                record_stop_reason(sink, reason);
            }
            Ok(tail)
        }))
        .boxed()
}

// ── Shared Chat Response Helper ─────────────────────────────────────────────

/// Send a non-streaming chat request and parse the response.
///
/// This handles the common pattern of: POST JSON → check status → parse body → extract text+usage.
/// True when an error body is the endpoint refusing the `tools` field itself
/// rather than anything about the conversation.
///
/// Not every OpenAI-compatible endpoint implements function calling —
/// Perplexity's chat models, some self-hosted servers, older proxies. Sending
/// tools to one of those must degrade to what worked before, not fail the
/// turn: the tools are described in the system prompt as well.
pub fn is_tools_unsupported(body: &str) -> bool {
    let body = body.to_ascii_lowercase();
    (body.contains("tool") || body.contains("function"))
        && (body.contains("not support")
            || body.contains("unsupported")
            || body.contains("unrecognized")
            || body.contains("unknown field")
            || body.contains("unexpected keyword")
            || body.contains("invalid parameter"))
}

/// The same request with no `tools` field.
fn without_tools(request: &ChatRequest) -> ChatRequest {
    ChatRequest {
        model: request.model.clone(),
        messages: request.messages.clone(),
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        stream: request.stream,
        tools: None,
        // Dropped with the tools it qualifies: an endpoint that rejected
        // `tools` will reject the switch that configures them just as fast.
        parallel_tool_calls: None,
    }
}

pub async fn send_chat_request(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    request: &ChatRequest,
    provider_label: &str,
) -> Result<CompletionResponse> {
    let post = |body: &ChatRequest| {
        client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(body)
            .send()
    };
    let resp = post(request)
        .await
        .with_context(|| format!("{} request failed", provider_label))?;

    let resp = if resp.status().is_success() {
        resp
    } else {
        let err = resp.text().await?;
        // An endpoint without function calling gets the turn again without the
        // tools field — the prompt still describes the tools in prose.
        if request.tools.is_none() || !is_tools_unsupported(&err) {
            anyhow::bail!("{} API error: {}", provider_label, err);
        }
        tracing::warn!(
            provider = provider_label,
            "Endpoint does not accept native tool definitions — retrying without them",
        );
        let retry = post(&without_tools(request))
            .await
            .with_context(|| format!("{} request failed", provider_label))?;
        if !retry.status().is_success() {
            let err = retry.text().await?;
            anyhow::bail!("{} API error: {}", provider_label, err);
        }
        retry
    };

    let body: ChatResponse = resp
        .json()
        .await
        .with_context(|| format!("Failed to parse {} response", provider_label))?;
    // `into_text` and not `.content`: a turn made entirely of tool calls has an
    // empty `content`, and reading only that field is how this path used to
    // report a model that had in fact called a tool as one that said nothing.
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
        model: request.model.clone(),
        usage,
    })
}

/// Send a streaming chat request and return an SSE-parsed CompletionStream.
pub async fn send_stream_request(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    request: &ChatRequest,
    provider_label: &str,
) -> Result<CompletionStream> {
    send_stream_request_reporting(client, url, api_key, request, provider_label, None).await
}

/// [`send_stream_request`], additionally recording why the stream ended.
pub async fn send_stream_request_reporting(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    request: &ChatRequest,
    provider_label: &str,
    stop: Option<StopReasonSink>,
) -> Result<CompletionStream> {
    let post = |body: &ChatRequest| {
        client
            .post(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(body)
            .send()
    };
    let resp = post(request)
        .await
        .with_context(|| format!("{} stream request failed", provider_label))?;

    if resp.status().is_success() {
        return Ok(parse_sse_stream_reporting(resp, stop));
    }

    // Same fallback as the non-streaming path.
    let err = resp.text().await?;
    if request.tools.is_none() || !is_tools_unsupported(&err) {
        anyhow::bail!("{} API error: {}", provider_label, err);
    }
    tracing::warn!(
        provider = provider_label,
        "Endpoint does not accept native tool definitions — retrying the stream without them",
    );
    let retry = post(&without_tools(request))
        .await
        .with_context(|| format!("{} stream request failed", provider_label))?;
    if !retry.status().is_success() {
        let err = retry.text().await?;
        anyhow::bail!("{} API error: {}", provider_label, err);
    }
    Ok(parse_sse_stream_reporting(retry, stop))
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shared_parser_tells_a_capped_reply_from_a_finished_one() {
        // One parser serves every OpenAI-compatible provider, so this is the
        // single place that decides whether all of them can distinguish a reply
        // cut at `max_tokens` from one that finished. `finish_reason` was not a
        // field here, so none of them could.
        let mut acc = SseAccumulator::default();
        acc.push("data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n");
        assert_eq!(acc.stop_reason, None, "no reason until the final chunk");

        acc.push("data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"length\"}]}\n");
        assert_eq!(acc.stop_reason, Some(StopReason::Length));

        let mut done = SseAccumulator::default();
        done.push("data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n");
        assert_eq!(done.stop_reason, Some(StopReason::Natural));
    }

    #[test]
    fn an_endpoint_that_sends_no_finish_reason_stays_unknown() {
        // Plenty of proxies omit it. Absent must stay absent rather than being
        // read as a clean finish.
        let mut acc = SseAccumulator::default();
        acc.push("data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n");
        acc.push("data: [DONE]\n");
        assert_eq!(acc.stop_reason, None);
    }
    use crate::provider::MessageRole;

    #[test]
    fn build_messages_basic() {
        let msgs = vec![Message {
            role: MessageRole::User,
            content: "hello".into(),
        }];
        let result = build_messages(&msgs, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "user");
        assert_eq!(result[0].content, "hello");
    }

    #[test]
    fn build_messages_with_context() {
        let msgs = vec![Message {
            role: MessageRole::User,
            content: "explain".into(),
        }];
        let result = build_messages(&msgs, Some("file.rs contents".into()));
        assert!(result[0].content.contains("Context:\nfile.rs contents"));
        assert!(result[0].content.contains("User: explain"));
    }

    #[test]
    fn build_messages_context_only_appends_to_user() {
        let msgs = vec![Message {
            role: MessageRole::System,
            content: "sys".into(),
        }];
        let result = build_messages(&msgs, Some("ctx".into()));
        // System message should NOT get context injected
        assert_eq!(result[0].content, "sys");
    }

    #[test]
    fn build_messages_preserves_roles() {
        let msgs = vec![
            Message {
                role: MessageRole::System,
                content: "sys".into(),
            },
            Message {
                role: MessageRole::User,
                content: "u1".into(),
            },
            Message {
                role: MessageRole::Assistant,
                content: "a1".into(),
            },
            Message {
                role: MessageRole::User,
                content: "u2".into(),
            },
        ];
        let result = build_messages(&msgs, None);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
        assert_eq!(result[3].role, "user");
    }

    #[test]
    fn default_client_returns_valid_client() {
        let client = default_http_client();
        // Just verify it doesn't panic and returns a client
        assert!(!format!("{:?}", client).is_empty());
    }

    #[test]
    fn chat_request_serializes_correctly() {
        let req = ChatRequest {
            model: "gpt-4".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: Some(0.7),
            max_tokens: None,
            stream: false,
            tools: None,
            parallel_tool_calls: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"model\":\"gpt-4\""));
        assert!(json.contains("\"temperature\":0.7"));
        assert!(!json.contains("max_tokens")); // skip_serializing_if None
    }

    #[test]
    fn chat_response_deserializes() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"hello"}}],"usage":{"prompt_tokens":5,"completion_tokens":3}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "hello");
        assert_eq!(resp.usage.unwrap().prompt_tokens, 5);
    }

    #[test]
    fn stream_response_deserializes() {
        let json = r#"{"choices":[{"delta":{"content":"hi"}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_deref(), Some("hi"));
    }

    #[test]
    fn stream_response_null_content() {
        let json = r#"{"choices":[{"delta":{}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content, None);
    }

    // ── SSE assembly ───────────────────────────────────────────────────
    //
    // Reasoning models (DeepSeek-R1, GLM) put a turn in `reasoning_content`
    // and leave `content` empty; some hosted models answer with a structured
    // `tool_calls` delta instead of the markup our prompt asks for. Dropping
    // either renders the turn as an empty response.

    fn sse(events: &[&str]) -> String {
        events.iter().map(|e| format!("data: {e}\n")).collect()
    }

    #[test]
    fn content_only_stream_is_unchanged() {
        let mut acc = SseAccumulator::default();
        let out = acc.push(&sse(&[
            r#"{"choices":[{"delta":{"content":"hi"}}]}"#,
            "[DONE]",
        ]));
        assert_eq!(out, "hi");
    }

    #[test]
    fn reasoning_is_wrapped_once_across_deltas() {
        let mut acc = SseAccumulator::default();
        let out = acc.push(&sse(&[
            r#"{"choices":[{"delta":{"reasoning_content":"the "}}]}"#,
            r#"{"choices":[{"delta":{"reasoning_content":"plan"}}]}"#,
            r#"{"choices":[{"delta":{"content":"Answer."}}]}"#,
            "[DONE]",
        ]));
        assert_eq!(out, "<thinking>the plan</thinking>\nAnswer.");
    }

    #[test]
    fn openrouter_reasoning_field_is_also_read() {
        let mut acc = SseAccumulator::default();
        let out = acc.push(&sse(&[
            r#"{"choices":[{"delta":{"reasoning":"hmm"}}]}"#,
            "[DONE]",
        ]));
        assert_eq!(out, "<thinking>hmm</thinking>\n");
    }

    #[test]
    fn reasoning_only_turn_is_closed_at_stream_end() {
        let mut acc = SseAccumulator::default();
        // No [DONE] — the transport just ends.
        let mut out = acc.push(&sse(&[
            r#"{"choices":[{"delta":{"reasoning_content":"x"}}]}"#,
        ]));
        out.push_str(&acc.finish());
        assert_eq!(out, "<thinking>x</thinking>\n");
    }

    #[test]
    fn native_tool_call_is_assembled_across_deltas() {
        let mut acc = SseAccumulator::default();
        let out = acc.push(&sse(&[
            r#"{"choices":[{"delta":{"tool_calls":[{"function":{"name":"list_directory","arguments":"{\"path\":"}}]}}]}"#,
            r#"{"choices":[{"delta":{"tool_calls":[{"function":{"arguments":"\"src\"}"}}]}}]}"#,
            "[DONE]",
        ]));
        assert_eq!(
            out,
            r#"<tool_call name="list_directory"><path>src</path></tool_call>"#
        );
        assert_eq!(crate::tools::parse_tool_calls(&out).len(), 1);
    }

    #[test]
    fn two_tool_calls_are_emitted_separately() {
        let mut acc = SseAccumulator::default();
        let out = acc.push(&sse(&[
            r#"{"choices":[{"delta":{"tool_calls":[{"function":{"name":"read_file","arguments":"{\"path\":\"a.rs\"}"}}]}}]}"#,
            r#"{"choices":[{"delta":{"tool_calls":[{"function":{"name":"read_file","arguments":"{\"path\":\"b.rs\"}"}}]}}]}"#,
            "[DONE]",
        ]));
        let calls = crate::tools::parse_tool_calls(&out);
        assert_eq!(calls.len(), 2);
    }

    #[test]
    fn unparseable_tool_arguments_still_emit_the_call() {
        let mut acc = SseAccumulator::default();
        let out = acc.push(&sse(&[
            r#"{"choices":[{"delta":{"tool_calls":[{"function":{"name":"build","arguments":"{oops"}}]}}]}"#,
            "[DONE]",
        ]));
        assert_eq!(out, r#"<tool_call name="build"></tool_call>"#);
    }

    #[test]
    fn malformed_events_are_skipped() {
        let mut acc = SseAccumulator::default();
        let out =
            acc.push("data: not json\ndata: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n");
        assert_eq!(out, "ok");
    }

    /// Both halves of the round trip live here: a chat conversation must go
    /// out with tool schemas, and a plain one must not — an endpoint that is
    /// handed tools it was never asked for answers with calls the panel has
    /// nowhere to run.
    /// An HTTP chunk boundary falls wherever the network puts it. Splitting
    /// each chunk on newlines hands the parser a truncated event and then an
    /// orphan fragment, both of which are skipped as malformed — the token in
    /// them vanishes with no error. This is the case that proves the buffer.
    #[test]
    fn an_event_split_across_chunks_is_not_lost() {
        let mut acc = SseAccumulator::default();
        let mut out = String::new();
        // One event, cut in half mid-JSON.
        out.push_str(&acc.push("data: {\"choices\":[{\"delta\":{\"cont"));
        out.push_str(&acc.push("ent\":\"hello\"}}]}\n"));
        out.push_str(&acc.finish());
        assert_eq!(out, "hello");
    }

    /// The boundary can also fall between events rather than inside one.
    #[test]
    fn events_split_at_a_chunk_boundary_all_arrive() {
        let mut acc = SseAccumulator::default();
        let mut out = String::new();
        out.push_str(&acc.push("data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\ndata: {\"cho"));
        out.push_str(&acc.push("ices\":[{\"delta\":{\"content\":\"b\"}}]}\n"));
        out.push_str(&acc.finish());
        assert_eq!(out, "ab");
    }

    /// A stream whose final event has no trailing newline is still a complete
    /// event — dropping it would lose the last token of every such reply.
    #[test]
    fn a_final_event_without_a_newline_is_still_parsed() {
        let mut acc = SseAccumulator::default();
        let mut out = String::new();
        out.push_str(&acc.push("data: {\"choices\":[{\"delta\":{\"content\":\"tail\"}}]}"));
        out.push_str(&acc.finish());
        assert_eq!(out, "tail");
    }

    /// A tool call split across chunks has to survive the same way — this is
    /// the shape that actually drives the agent loop.
    #[test]
    fn a_tool_call_split_across_chunks_survives() {
        let mut acc = SseAccumulator::default();
        let mut out = String::new();
        out.push_str(&acc.push(
            "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"function\":{\"name\":\"read_f",
        ));
        out.push_str(&acc.push(
            "ile\",\"arguments\":\"{\\\"path\\\":\\\"a.rs\\\"}\"}}]}}]}\n",
        ));
        out.push_str(&acc.finish());
        assert_eq!(out, r#"<tool_call name="read_file"><path>a.rs</path></tool_call>"#);
    }

    /// CRLF line endings are legal in SSE and some proxies emit them.
    #[test]
    fn crlf_line_endings_parse() {
        let mut acc = SseAccumulator::default();
        let mut out = String::new();
        out.push_str(&acc.push("data: {\"choices\":[{\"delta\":{\"content\":\"x\"}}]}\r\n"));
        out.push_str(&acc.finish());
        assert_eq!(out, "x");
    }

    #[test]
    fn tools_ride_along_only_when_the_conversation_asked_for_them() {
        let chat = [Message {
            role: crate::provider::MessageRole::System,
            content: format!(
                "preamble\n{}\n- write_file",
                crate::tools::CHAT_TOOL_PROMPT_MARKER
            ),
        }];
        let native = crate::harness::ModelProfile::native_tools();
        let defs = ChatRequest::tools_for(&chat, &native).expect("chat conversations carry tools");
        assert_eq!(defs.len(), crate::tools::CHAT_TOOL_NAMES.len());

        let plain = [Message {
            role: crate::provider::MessageRole::User,
            content: "what is a monad".to_string(),
        }];
        assert!(ChatRequest::tools_for(&plain, &native).is_none());
    }

    /// Both conditions have to hold. A pair pinned to the prose path gets no
    /// schemas even when the conversation's prompt asks for tools — that
    /// setting is the escape hatch for a model whose native tool calling is
    /// worse than its prose, so it has to actually suppress them.
    #[test]
    fn a_prose_pair_sends_no_schemas_even_for_a_tool_conversation() {
        let chat = [Message {
            role: crate::provider::MessageRole::System,
            content: format!(
                "preamble\n{}\n- write_file",
                crate::tools::CHAT_TOOL_PROMPT_MARKER
            ),
        }];
        let prose = crate::harness::ModelProfile::conservative();
        assert!(ChatRequest::tools_for(&chat, &prose).is_none());
    }

    #[test]
    fn parallel_tool_calls_is_absent_without_tools() {
        let profile = crate::harness::ModelProfile {
            parallel_tool_calls: Some(true),
            ..crate::harness::ModelProfile::native_tools()
        };
        // Set alongside schemas...
        let tools = vec![serde_json::json!({"type": "function"})];
        assert_eq!(
            ChatRequest::parallel_for(Some(&tools), &profile),
            Some(true)
        );
        // ...and absent without them, since it would qualify nothing and some
        // endpoints reject unknown fields outright.
        assert_eq!(ChatRequest::parallel_for(None, &profile), None);
    }

    /// A turn whose entire substance is a tool call has an empty `content`.
    /// Reading only that field is how the non-streaming path used to report a
    /// model that had called a tool as one that said nothing at all.
    #[test]
    fn a_non_streaming_tool_call_survives_into_the_text() {
        let body: ChatResponse = serde_json::from_str(
            r#"{"choices":[{"message":{"content":"","tool_calls":[{"function":{"name":"read_file","arguments":"{\"path\":\"src/main.rs\"}"}}]}}]}"#,
        )
        .expect("parses");
        let text = body
            .choices
            .into_iter()
            .next()
            .expect("one choice")
            .message
            .into_text();
        assert_eq!(
            text,
            r#"<tool_call name="read_file"><path>src/main.rs</path></tool_call>"#
        );
        assert_eq!(crate::tools::parse_tool_calls(&text).len(), 1);
    }

    /// The two paths have to produce the same shape or the agent parses one of
    /// them and not the other.
    #[test]
    fn a_non_streaming_reply_wraps_reasoning_like_the_stream_does() {
        let body: ChatResponse = serde_json::from_str(
            r#"{"choices":[{"message":{"content":"done","reasoning_content":"thinking hard"}}]}"#,
        )
        .expect("parses");
        let text = body
            .choices
            .into_iter()
            .next()
            .expect("one choice")
            .message
            .into_text();
        assert_eq!(text, "<thinking>thinking hard</thinking>\ndone");
    }

    /// OpenAI sends `"content": null` for exactly the turn this type exists to
    /// read. `#[serde(default)]` does not accept a null into a `String`, so
    /// without the custom deserializer the whole response fails to parse and
    /// the turn is reported as an API error rather than a tool call.
    #[test]
    fn a_null_content_parses_rather_than_failing_the_turn() {
        let body: ChatResponse = serde_json::from_str(
            r#"{"choices":[{"message":{"content":null,"tool_calls":[{"function":{"name":"build","arguments":"{}"}}]}}]}"#,
        )
        .expect("a null content must parse");
        let text = body
            .choices
            .into_iter()
            .next()
            .expect("one choice")
            .message
            .into_text();
        assert_eq!(text, r#"<tool_call name="build"></tool_call>"#);
    }

    /// A reply with neither reasoning nor tool calls must come through exactly
    /// as it did before this path learned about either.
    #[test]
    fn a_plain_reply_is_unchanged() {
        let body: ChatResponse =
            serde_json::from_str(r#"{"choices":[{"message":{"content":"just prose"}}]}"#)
                .expect("parses");
        assert_eq!(
            body.choices
                .into_iter()
                .next()
                .expect("one choice")
                .message
                .into_text(),
            "just prose"
        );
    }

    #[test]
    fn tools_are_omitted_from_the_wire_when_absent() {
        let req = ChatRequest {
            model: "m".into(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: false,
            tools: None,
            parallel_tool_calls: None,
        };
        let json = serde_json::to_string(&req).expect("serialize");
        assert!(
            !json.contains("tools"),
            "None tools must not hit the wire: {json}"
        );
    }

    /// An endpoint without function calling must degrade to the behaviour that
    /// worked before tools were sent, not fail the turn.
    #[test]
    fn tool_rejections_are_recognised_across_wordings() {
        for body in [
            r#"{"error":{"message":"This model does not support tools"}}"#,
            r#"{"error":{"message":"Unsupported parameter: 'tools'"}}"#,
            r#"{"error":"unknown field `tools`"}"#,
            r#"{"error":{"message":"Invalid parameter: function calling is not enabled"}}"#,
        ] {
            assert!(is_tools_unsupported(body), "should be recognised: {body}");
        }
    }

    /// Everything else is a real error and must surface as one — a rate limit
    /// retried without tools would silently lose tool calling for the session.
    #[test]
    fn ordinary_errors_are_not_mistaken_for_tool_rejections() {
        for body in [
            r#"{"error":{"message":"Rate limit exceeded"}}"#,
            r#"{"error":{"message":"Invalid API key"}}"#,
            r#"{"error":{"message":"context length exceeded"}}"#,
        ] {
            assert!(
                !is_tools_unsupported(body),
                "should not be recognised: {body}"
            );
        }
    }

    #[test]
    fn dropping_tools_keeps_the_rest_of_the_request() {
        let req = ChatRequest {
            model: "m".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: Some(0.2),
            max_tokens: Some(512),
            stream: true,
            tools: Some(crate::tools::chat_tool_definitions()),
            parallel_tool_calls: None,
        };
        let stripped = without_tools(&req);
        assert!(stripped.tools.is_none());
        assert_eq!(stripped.model, "m");
        assert_eq!(stripped.temperature, Some(0.2));
        assert_eq!(stripped.max_tokens, Some(512));
        assert!(stripped.stream);
        assert_eq!(stripped.messages.len(), 1);
    }
}
