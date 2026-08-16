//! Cerebras — OpenAI-compatible inference.
//!
//! The implementation is [`crate::providers::compat`]; this file supplies the
//! three things that made it Cerebras rather than any of its neighbours, and
//! keeps the test suite that proves the migration changed nothing.

use super::openai_compat::{self, ChatRequest};
use crate::provider::{AIProvider, Message, ProviderConfig};

pub const CEREBRAS_BASE_URL: &str = "https://api.cerebras.ai/v1";

crate::openai_compat_provider!(
    CerebrasProvider,
    "Cerebras",
    CEREBRAS_BASE_URL,
    "CEREBRAS_API_KEY"
);

#[cfg(test)]
mod tests {
    use super::*;
    use openai_compat::{ChatMessage, ChatResponse, StreamResponse};

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "cerebras".into(),
            api_key: Some("csk_test".into()),
            api_url: None,
            model: "llama3.1-70b".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
            effort: None,
        }
    }

    #[test]
    fn name_is_cerebras() {
        let p = CerebrasProvider::new(test_config());
        assert_eq!(p.name(), "Cerebras (llama3.1-70b)");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = CerebrasProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = CerebrasProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(CEREBRAS_BASE_URL, "https://api.cerebras.ai/v1");
    }

    #[test]
    fn cerebras_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"fast"}}],"usage":{"prompt_tokens":10,"completion_tokens":2}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "fast");
        assert_eq!(resp.usage.unwrap().prompt_tokens, 10);
    }

    // ── build_messages context injection ────────────────────────────────

    #[test]
    fn build_messages_without_context() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message {
                role: MessageRole::System,
                content: "You are helpful.".into(),
            },
            Message {
                role: MessageRole::User,
                content: "Hello".into(),
            },
        ];
        let result = openai_compat::build_messages(&msgs, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[0].content, "You are helpful.");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[1].content, "Hello");
    }

    #[test]
    fn build_messages_with_context_injects_into_last_user() {
        use crate::provider::MessageRole;
        let msgs = vec![Message {
            role: MessageRole::User,
            content: "Explain".into(),
        }];
        let result = openai_compat::build_messages(&msgs, Some("source code here".into()));
        assert_eq!(result.len(), 1);
        assert!(result[0].content.starts_with("Context:\nsource code here"));
        assert!(result[0].content.contains("User: Explain"));
    }

    #[test]
    fn build_messages_context_only_last_user_affected() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message {
                role: MessageRole::User,
                content: "First".into(),
            },
            Message {
                role: MessageRole::Assistant,
                content: "Mid".into(),
            },
            Message {
                role: MessageRole::User,
                content: "Last".into(),
            },
        ];
        let result = openai_compat::build_messages(&msgs, Some("ctx".into()));
        // First user unchanged
        assert_eq!(result[0].content, "First");
        // Last user gets context
        assert!(result[2].content.starts_with("Context:\nctx"));
        assert!(result[2].content.contains("User: Last"));
    }

    #[test]
    fn build_messages_context_not_injected_when_last_is_assistant() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message {
                role: MessageRole::User,
                content: "Q".into(),
            },
            Message {
                role: MessageRole::Assistant,
                content: "A".into(),
            },
        ];
        let result = openai_compat::build_messages(&msgs, Some("ignored context".into()));
        assert_eq!(result[1].content, "A");
        assert_eq!(result[0].content, "Q");
    }

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
    fn build_messages_role_mapping() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message {
                role: MessageRole::System,
                content: "s".into(),
            },
            Message {
                role: MessageRole::User,
                content: "u".into(),
            },
            Message {
                role: MessageRole::Assistant,
                content: "a".into(),
            },
        ];
        let result = openai_compat::build_messages(&msgs, None);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    // ── name / availability ─────────────────────────────────────────────

    #[test]
    fn name_returns_cerebras() {
        let p = CerebrasProvider::new(test_config());
        assert_eq!(p.name(), "Cerebras (llama3.1-70b)");
    }

    #[tokio::test]
    async fn availability_tracks_api_key() {
        let p_with = CerebrasProvider::new(test_config());
        assert!(p_with.is_available().await);

        let mut cfg = test_config();
        cfg.api_key = None;
        let p_without = CerebrasProvider::new(cfg);
        assert!(!p_without.is_available().await);
    }

    // ── base_url ────────────────────────────────────────────────────────

    #[test]
    fn base_url_custom_override() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://cerebras-proxy.example.com/v1".into());
        let p = CerebrasProvider::new(cfg);
        assert_eq!(p.base_url(), "https://cerebras-proxy.example.com/v1");
    }

    #[test]
    fn base_url_default_value() {
        let p = CerebrasProvider::new(test_config());
        assert_eq!(p.base_url(), "https://api.cerebras.ai/v1");
    }

    // ── request / response serialization ────────────────────────────────

    #[test]
    fn cerebras_request_serde_minimal() {
        let req = ChatRequest {
            model: "llama3.1-70b".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_string(&req).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["model"], "llama3.1-70b");
        assert_eq!(val["stream"], false);
        assert!(val.get("temperature").is_none());
        assert!(val.get("max_tokens").is_none());
    }

    #[test]
    fn cerebras_request_serde_full() {
        let req = ChatRequest {
            model: "llama-3.3-70b".into(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: "sys".into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: "usr".into(),
                },
            ],
            temperature: Some(0.8),
            max_tokens: Some(512),
            stream: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["temperature"], 0.8);
        assert_eq!(val["max_tokens"], 512);
        assert_eq!(val["stream"], true);
        assert_eq!(val["messages"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn cerebras_response_deser_no_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "ok");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn cerebras_stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"tok"}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "tok");
    }

    #[test]
    fn cerebras_stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{"content":null}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    #[test]
    fn cerebras_message_roundtrip() {
        let msg = ChatMessage {
            role: "user".into(),
            content: "test data".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }

    // ── response parsing edge cases ───────────────────────────────────

    #[test]
    fn cerebras_response_deser_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"a"}},{"message":{"role":"assistant","content":"b"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[1].message.content, "b");
    }

    #[test]
    fn cerebras_response_deser_zero_token_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":""}}],"usage":{"prompt_tokens":0,"completion_tokens":0}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
    }

    #[test]
    fn cerebras_stream_response_deser_empty_content() {
        let json = r#"{"choices":[{"delta":{"content":""}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "");
    }

    // ── build_messages with empty context string ──────────────────────

    #[test]
    fn build_messages_empty_context_still_injects() {
        use crate::provider::MessageRole;
        let msgs = vec![Message {
            role: MessageRole::User,
            content: "q".into(),
        }];
        let result = openai_compat::build_messages(&msgs, Some("".into()));
        assert!(result[0].content.starts_with("Context:\n"));
        assert!(result[0].content.contains("User: q"));
    }

    // ── request with different model names ────────────────────────────

    #[test]
    fn cerebras_request_different_model() {
        let req = ChatRequest {
            model: "llama3.1-8b".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "test".into(),
            }],
            temperature: Some(0.0),
            max_tokens: Some(1),
            stream: false,
        };
        let val = serde_json::to_value(&req).unwrap();
        assert_eq!(val["model"], "llama3.1-8b");
        assert_eq!(val["temperature"], 0.0);
        assert_eq!(val["max_tokens"], 1);
    }
}
