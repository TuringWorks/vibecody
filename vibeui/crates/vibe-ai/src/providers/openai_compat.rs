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

/// Parse an SSE byte stream from an OpenAI-compatible API into a CompletionStream.
///
/// Handles `data: [DONE]`, malformed JSON (silently skipped), and
/// extracts `choices[0].delta.content` from each SSE event.
pub fn parse_sse_stream(response: reqwest::Response) -> CompletionStream {
    response
        .bytes_stream()
        .map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        continue;
                    }
                    if let Ok(r) = serde_json::from_str::<StreamResponse>(data) {
                        if let Some(c) = r.choices.first().and_then(|ch| ch.delta.content.as_ref())
                        {
                            content.push_str(c);
                        }
                    }
                }
            }
            Ok(content)
        })
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
    let usage = body
        .usage
        .map(|u| TokenUsage {
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
        let msgs = vec![
            Message { role: MessageRole::User, content: "hello".into() },
        ];
        let result = build_messages(&msgs, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "user");
        assert_eq!(result[0].content, "hello");
    }

    #[test]
    fn build_messages_with_context() {
        let msgs = vec![
            Message { role: MessageRole::User, content: "explain".into() },
        ];
        let result = build_messages(&msgs, Some("file.rs contents".into()));
        assert!(result[0].content.contains("Context:\nfile.rs contents"));
        assert!(result[0].content.contains("User: explain"));
    }

    #[test]
    fn build_messages_context_only_appends_to_user() {
        let msgs = vec![
            Message { role: MessageRole::System, content: "sys".into() },
        ];
        let result = build_messages(&msgs, Some("ctx".into()));
        // System message should NOT get context injected
        assert_eq!(result[0].content, "sys");
    }

    #[test]
    fn build_messages_preserves_roles() {
        let msgs = vec![
            Message { role: MessageRole::System, content: "sys".into() },
            Message { role: MessageRole::User, content: "u1".into() },
            Message { role: MessageRole::Assistant, content: "a1".into() },
            Message { role: MessageRole::User, content: "u2".into() },
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
            messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
            temperature: Some(0.7),
            max_tokens: None,
            stream: false,
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
}
