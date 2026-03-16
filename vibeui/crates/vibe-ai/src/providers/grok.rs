//! xAI Grok provider implementation

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

// Grok API is compatible with OpenAI
#[derive(Debug, Serialize)]
struct GrokRequest {
    model: String,
    messages: Vec<GrokMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct GrokMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GrokResponse {
    choices: Vec<GrokChoice>,
}

#[derive(Debug, Deserialize)]
struct GrokChoice {
    message: GrokMessage,
}

#[derive(Debug, Deserialize)]
struct GrokStreamResponse {
    choices: Vec<GrokStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct GrokStreamChoice {
    delta: GrokDelta,
}

#[derive(Debug, Deserialize)]
struct GrokDelta {
    content: Option<String>,
}

/// Grok provider
pub struct GrokProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    display_name: String,
}

impl GrokProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let display_name = format!("Grok ({})", config.model);
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

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<GrokMessage> {
        let mut grok_messages: Vec<GrokMessage> = messages
            .iter()
            .map(|m| GrokMessage {
                role: m.role.as_str().to_string(),
                content: m.content.clone(),
            })
            .collect();

        if let Some(ctx) = context {
            if let Some(last_msg) = grok_messages.last_mut() {
                if last_msg.role == "user" {
                    last_msg.content = format!("Context:\n{}\n\nUser: {}", ctx, last_msg.content);
                }
            }
        }
        grok_messages
    }
}

#[async_trait]
impl AIProvider for GrokProvider {
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
            Message { role: crate::provider::MessageRole::System, content: "You are a helpful coding assistant.".to_string() },
            Message { role: crate::provider::MessageRole::User, content: prompt },
        ];

        let response_text = self.chat(&messages, None).await?;

        Ok(CompletionResponse {
            text: response_text,
            model: self.config.model.clone(),
            usage: None,
        })
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

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let api_key = self.config.api_key.as_ref().context("Grok API key not found")?;
        let request = GrokRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };

        let response = self.client
            .post("https://api.x.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Grok")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Grok API error: {}", error_text);
        }

        let grok_response: GrokResponse = response.json().await.context("Failed to parse Grok response")?;
        
        grok_response.choices.first()
            .map(|c| c.message.content.clone())
            .context("No choices in Grok response")
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Grok API key not found")?;
        let request = GrokRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, None),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: true,
        };

        let response = self.client
            .post("https://api.x.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Grok")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Grok API error: {}", error_text);
        }

        let stream = response.bytes_stream();
        
        let completion_stream = stream
            .map(|chunk| {
                let chunk = chunk?;
                let chunk_str = String::from_utf8_lossy(&chunk);
                let mut content = String::new();
                
                for line in chunk_str.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            continue;
                        }
                        if let Ok(response) = serde_json::from_str::<GrokStreamResponse>(data) {
                            if let Some(choice) = response.choices.first() {
                                if let Some(delta_content) = &choice.delta.content {
                                    content.push_str(delta_content);
                                }
                            }
                        }
                    }
                }
                Ok(content)
            })
            .boxed();

        Ok(completion_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "grok".into(),
            api_key: Some("xai-test".into()),
            api_url: None,
            model: "grok-2".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_grok() {
        let p = GrokProvider::new(test_config());
        assert!(p.name().starts_with("Grok ("));
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = GrokProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = GrokProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn grok_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"hi"}}]}"#;
        let resp: GrokResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "hi");
    }

    // ── build_messages: context injection ────────────────────────────────

    #[test]
    fn build_messages_no_context_passthrough() {
        let p = GrokProvider::new(test_config());
        let messages = vec![
            Message { role: crate::provider::MessageRole::System, content: "sys".into() },
            Message { role: crate::provider::MessageRole::User, content: "hello".into() },
        ];
        let result = p.build_messages(&messages, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[0].content, "sys");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[1].content, "hello");
    }

    #[test]
    fn build_messages_context_appended_to_last_user() {
        let p = GrokProvider::new(test_config());
        let messages = vec![
            Message { role: crate::provider::MessageRole::User, content: "explain this".into() },
        ];
        let result = p.build_messages(&messages, Some("fn foo() {}".into()));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "user");
        assert!(result[0].content.starts_with("Context:\nfn foo() {}"));
        assert!(result[0].content.ends_with("User: explain this"));
    }

    #[test]
    fn build_messages_context_not_injected_when_last_is_assistant() {
        let p = GrokProvider::new(test_config());
        let messages = vec![
            Message { role: crate::provider::MessageRole::User, content: "hi".into() },
            Message { role: crate::provider::MessageRole::Assistant, content: "hello".into() },
        ];
        let result = p.build_messages(&messages, Some("some context".into()));
        // Context should NOT be injected because last message role is "assistant"
        assert_eq!(result.len(), 2);
        assert_eq!(result[1].role, "assistant");
        assert_eq!(result[1].content, "hello"); // unchanged
    }

    #[test]
    fn build_messages_context_injected_into_correct_last_user() {
        let p = GrokProvider::new(test_config());
        let messages = vec![
            Message { role: crate::provider::MessageRole::System, content: "system prompt".into() },
            Message { role: crate::provider::MessageRole::User, content: "first question".into() },
            Message { role: crate::provider::MessageRole::Assistant, content: "first answer".into() },
            Message { role: crate::provider::MessageRole::User, content: "second question".into() },
        ];
        let result = p.build_messages(&messages, Some("ctx data".into()));
        assert_eq!(result.len(), 4);
        // First user message should be unchanged
        assert_eq!(result[1].content, "first question");
        // Last user message should have context injected
        assert!(result[3].content.contains("ctx data"));
        assert!(result[3].content.contains("second question"));
    }

    #[test]
    fn build_messages_empty_messages() {
        let p = GrokProvider::new(test_config());
        let result = p.build_messages(&[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_empty_messages_with_context() {
        let p = GrokProvider::new(test_config());
        // Context with no messages — no last_mut to inject into
        let result = p.build_messages(&[], Some("ctx".into()));
        assert!(result.is_empty());
    }

    // ── request serde ────────────────────────────────────────────────────

    #[test]
    fn grok_request_omits_none_temperature() {
        let req = GrokRequest {
            model: "grok-2".into(),
            messages: vec![GrokMessage { role: "user".into(), content: "hi".into() }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("temperature"), "temperature should be omitted when None");
        assert!(!json.contains("max_tokens"), "max_tokens should be omitted when None");
    }

    #[test]
    fn grok_request_includes_temperature_when_set() {
        let req = GrokRequest {
            model: "grok-2".into(),
            messages: vec![],
            temperature: Some(0.3),
            max_tokens: Some(2048),
            stream: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"temperature\""));
        assert!(json.contains("\"max_tokens\""));
    }

    #[test]
    fn grok_stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"tok"}}]}"#;
        let resp: GrokStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_deref(), Some("tok"));
    }

    #[test]
    fn grok_stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{}}]}"#;
        let resp: GrokStreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    // ── role mapping ─────────────────────────────────────────────────────

    #[test]
    fn build_messages_maps_roles_correctly() {
        let p = GrokProvider::new(test_config());
        let messages = vec![
            Message { role: crate::provider::MessageRole::System, content: "s".into() },
            Message { role: crate::provider::MessageRole::User, content: "u".into() },
            Message { role: crate::provider::MessageRole::Assistant, content: "a".into() },
        ];
        let result = p.build_messages(&messages, None);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    // ── request serialization roundtrip ─────────────────────────────────

    #[test]
    fn grok_request_model_field_preserved() {
        let req = GrokRequest {
            model: "grok-3-mini".into(),
            messages: vec![GrokMessage { role: "user".into(), content: "test".into() }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "grok-3-mini");
        assert_eq!(json["stream"], false);
    }

    #[test]
    fn grok_request_stream_true_serialized() {
        let req = GrokRequest {
            model: "grok-2".into(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            stream: true,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["stream"], true);
    }

    #[test]
    fn grok_message_roundtrip() {
        let msg = GrokMessage { role: "user".into(), content: "hello world".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: GrokMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }

    #[test]
    fn grok_response_deser_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"a1"}},{"message":{"role":"assistant","content":"a2"}}]}"#;
        let resp: GrokResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[0].message.content, "a1");
        assert_eq!(resp.choices[1].message.content, "a2");
    }

    #[test]
    fn grok_response_deser_empty_choices() {
        let json = r#"{"choices":[]}"#;
        let resp: GrokResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices.is_empty());
    }

    #[test]
    fn grok_stream_response_deser_multiple_choices() {
        let json = r#"{"choices":[{"delta":{"content":"a"}},{"delta":{"content":"b"}}]}"#;
        let resp: GrokStreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[1].delta.content.as_deref(), Some("b"));
    }

    #[test]
    fn grok_message_unicode_content() {
        let msg = GrokMessage { role: "user".into(), content: "こんにちは 🌍".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: GrokMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg2.content, "こんにちは 🌍");
    }

    #[test]
    fn grok_request_temperature_boundary_values() {
        let req = GrokRequest {
            model: "grok-2".into(),
            messages: vec![],
            temperature: Some(0.0),
            max_tokens: Some(1),
            stream: false,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["temperature"], 0.0);
        assert_eq!(json["max_tokens"], 1);
    }

    #[test]
    fn provider_preserves_model_config() {
        let mut cfg = test_config();
        cfg.model = "grok-3".into();
        cfg.temperature = Some(0.7);
        cfg.max_tokens = Some(4096);
        let p = GrokProvider::new(cfg);
        assert_eq!(p.config.model, "grok-3");
        assert_eq!(p.config.temperature, Some(0.7));
        assert_eq!(p.config.max_tokens, Some(4096));
    }
}
