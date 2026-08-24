//! Shared infrastructure for OpenAI-compatible providers.
//!
//! Most AI providers (Groq, Mistral, DeepSeek, Cerebras, Grok, OpenRouter,
//! Perplexity, Together, Fireworks, SambaNova, MiniMax, Zhipu, VercelAI)
//! use the same OpenAI-compatible API schema. This module extracts the
//! duplicated types, HTTP client construction, SSE stream parsing, and
//! message building into shared utilities.

use crate::provider::{CompletionResponse, CompletionStream, Message, TokenUsage};
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
}

impl ChatRequest {
    /// The tools this conversation expects, or `None` for a plain chat.
    ///
    /// Providers call this instead of hand-writing the field so one rule —
    /// `vibe_ai::tools::tool_definitions_for` — decides for all of them.
    pub fn tools_for(messages: &[Message]) -> Option<Vec<serde_json::Value>> {
        crate::tools::tool_definitions_for(messages.iter().map(|m| m.content.as_str()))
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
    pub message: ChatMessage,
}

#[derive(Debug, Deserialize)]
pub struct StreamResponse {
    pub choices: Vec<StreamChoice>,
}

#[derive(Debug, Deserialize)]
pub struct StreamChoice {
    pub delta: StreamDelta,
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
    pub tool_calls: Option<Vec<StreamToolCall>>,
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

#[derive(Debug, Deserialize)]
pub struct StreamToolCall {
    #[serde(default)]
    pub function: Option<StreamToolFunction>,
}

#[derive(Debug, Deserialize)]
pub struct StreamToolFunction {
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
}

impl SseAccumulator {
    /// Feed one chunk of SSE text; returns the text to emit downstream.
    fn push(&mut self, text: &str) -> String {
        let mut out = String::new();
        for line in text.lines() {
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
    fn finish(&mut self) -> String {
        let mut out = self.flush_tool_call();
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
            Ok(tail_acc.lock().unwrap_or_else(|e| e.into_inner()).finish())
        }))
        .boxed()
}

// ── Shared Chat Response Helper ─────────────────────────────────────────────

/// Send a non-streaming chat request and parse the response.
///
/// This handles the common pattern of: POST JSON → check status → parse body → extract text+usage.
pub async fn send_chat_request(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    request: &ChatRequest,
    provider_label: &str,
) -> Result<CompletionResponse> {
    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(request)
        .send()
        .await
        .with_context(|| format!("{} request failed", provider_label))?;

    if !resp.status().is_success() {
        let err = resp.text().await?;
        anyhow::bail!("{} API error: {}", provider_label, err);
    }

    let body: ChatResponse = resp
        .json()
        .await
        .with_context(|| format!("Failed to parse {} response", provider_label))?;
    let text = body
        .choices
        .first()
        .context("No choices")?
        .message
        .content
        .clone();
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
    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(request)
        .send()
        .await
        .with_context(|| format!("{} stream request failed", provider_label))?;

    if !resp.status().is_success() {
        let err = resp.text().await?;
        anyhow::bail!("{} API error: {}", provider_label, err);
    }

    Ok(parse_sse_stream(resp))
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
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
    #[test]
    fn tools_ride_along_only_when_the_conversation_asked_for_them() {
        let chat = [Message {
            role: crate::provider::MessageRole::System,
            content: format!("preamble\n{}\n- write_file", crate::tools::CHAT_TOOL_PROMPT_MARKER),
        }];
        let defs = ChatRequest::tools_for(&chat).expect("chat conversations carry tools");
        assert_eq!(defs.len(), crate::tools::CHAT_TOOL_NAMES.len());

        let plain = [Message {
            role: crate::provider::MessageRole::User,
            content: "what is a monad".to_string(),
        }];
        assert!(ChatRequest::tools_for(&plain).is_none());
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
        };
        let json = serde_json::to_string(&req).expect("serialize");
        assert!(!json.contains("tools"), "None tools must not hit the wire: {json}");
    }
}
