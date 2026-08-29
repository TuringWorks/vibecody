//! Mistral AI provider — native API, OpenAI-compatible.
//!
//! Supported models: mistral-large-latest, mistral-medium-latest,
//! mistral-small-latest, codestral-latest, open-mistral-nemo
//!
//! The implementation is [`crate::providers::compat`]; this file supplies the
//! three values that made it Mistral rather than a neighbour, and keeps the
//! test suite that proves the migration changed nothing.

pub const MISTRAL_BASE_URL: &str = "https://api.mistral.ai/v1";

crate::openai_compat_provider!(
    MistralProvider,
    "mistral",
    "Mistral",
    MISTRAL_BASE_URL,
    "MISTRAL_API_KEY"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{AIProvider, Message, ProviderConfig};
    use crate::providers::openai_compat::{self, ChatRequest};
    use openai_compat::{ChatMessage, ChatResponse, StreamResponse};

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "mistral".into(),
            api_key: Some("test_key".into()),
            api_url: None,
            model: "mistral-large-latest".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
            effort: None,
        }
    }

    #[test]
    fn name_is_mistral() {
        let p = MistralProvider::new(test_config());
        assert_eq!(p.name(), "Mistral (mistral-large-latest)");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = MistralProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = MistralProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(MISTRAL_BASE_URL, "https://api.mistral.ai/v1");
    }

    #[test]
    fn mistral_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"hello"}}],"usage":{"prompt_tokens":5,"completion_tokens":1}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "hello");
        assert_eq!(resp.usage.unwrap().completion_tokens, 1);
    }

    // ── build_messages context injection ────────────────────────────────

    #[test]
    fn build_messages_without_context() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message {
                role: MessageRole::System,
                content: "Be helpful.".into(),
            },
            Message {
                role: MessageRole::User,
                content: "Hello".into(),
            },
        ];
        let result = openai_compat::build_messages(&msgs, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[0].content, "Be helpful.");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[1].content, "Hello");
    }

    #[test]
    fn build_messages_with_context_prepends_to_last_user() {
        use crate::provider::MessageRole;
        let msgs = vec![Message {
            role: MessageRole::User,
            content: "Explain this".into(),
        }];
        let result = openai_compat::build_messages(&msgs, Some("file.rs contents".into()));
        assert_eq!(result.len(), 1);
        assert!(result[0].content.starts_with("Context:\nfile.rs contents"));
        assert!(result[0].content.contains("User: Explain this"));
    }

    #[test]
    fn build_messages_context_only_affects_last_user_message() {
        use crate::provider::MessageRole;
        let msgs = vec![
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
        let result = openai_compat::build_messages(&msgs, Some("ctx".into()));
        // First user message unchanged
        assert_eq!(result[0].content, "First");
        // Last message is user, gets context
        assert!(result[2].content.starts_with("Context:\nctx"));
        assert!(result[2].content.contains("User: Second"));
    }

    #[test]
    fn build_messages_context_skipped_when_last_is_not_user() {
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
        let result = openai_compat::build_messages(&msgs, Some("some context".into()));
        // Last message is assistant, context should NOT be injected
        assert_eq!(result[1].content, "A");
        assert_eq!(result[0].content, "Q");
    }

    #[test]
    fn build_messages_empty_messages() {
        let result = openai_compat::build_messages(&[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_role_mapping() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message {
                role: MessageRole::System,
                content: "sys".into(),
            },
            Message {
                role: MessageRole::User,
                content: "usr".into(),
            },
            Message {
                role: MessageRole::Assistant,
                content: "ast".into(),
            },
        ];
        let result = openai_compat::build_messages(&msgs, None);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    // ── name / availability ─────────────────────────────────────────────

    #[test]
    fn name_returns_correct_string() {
        let p = MistralProvider::new(test_config());
        assert_eq!(p.name(), "Mistral (mistral-large-latest)");
        // Verify it is a static str, not dynamic
        let name: &str = p.name();
        assert!(!name.is_empty());
    }

    #[tokio::test]
    async fn availability_reflects_api_key_presence() {
        let mut cfg = test_config();
        cfg.api_key = Some("key".into());
        let p = MistralProvider::new(cfg);
        assert!(p.is_available().await);

        let mut cfg2 = test_config();
        cfg2.api_key = None;
        let p2 = MistralProvider::new(cfg2);
        assert!(!p2.is_available().await);
    }

    // ── base_url ────────────────────────────────────────────────────────

    #[test]
    fn base_url_uses_custom_when_provided() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://custom.mistral.example.com/v1".into());
        let p = MistralProvider::new(cfg);
        assert_eq!(p.base_url(), "https://custom.mistral.example.com/v1");
    }

    #[test]
    fn base_url_falls_back_to_default() {
        let p = MistralProvider::new(test_config());
        assert_eq!(p.base_url(), "https://api.mistral.ai/v1");
    }

    // ── request serialization ───────────────────────────────────────────

    #[test]
    fn mistral_request_serde_stream_false() {
        let req = ChatRequest {
            model: "mistral-large-latest".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
            tools: None,
            parallel_tool_calls: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["model"], "mistral-large-latest");
        assert_eq!(val["stream"], false);
        // temperature and max_tokens should be omitted (skip_serializing_if)
        assert!(val.get("temperature").is_none());
        assert!(val.get("max_tokens").is_none());
    }

    #[test]
    fn mistral_request_serde_with_all_fields() {
        let req = ChatRequest {
            model: "codestral-latest".into(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: "sys".into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: "code".into(),
                },
            ],
            temperature: Some(0.5),
            max_tokens: Some(2048),
            stream: true,
            tools: None,
            parallel_tool_calls: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["temperature"], 0.5);
        assert_eq!(val["max_tokens"], 2048);
        assert_eq!(val["stream"], true);
        assert_eq!(val["messages"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn mistral_response_deser_no_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "ok");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn mistral_stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"tok"}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "tok");
    }

    #[test]
    fn mistral_stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{"content":null}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    // ── response parsing edge cases ───────────────────────────────────

    #[test]
    fn mistral_response_deser_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"a"}},{"message":{"role":"assistant","content":"b"}}],"usage":{"prompt_tokens":3,"completion_tokens":2}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[1].message.content, "b");
    }

    #[test]
    fn mistral_message_serde_roundtrip() {
        let msg = ChatMessage {
            role: "system".into(),
            content: "Be precise.".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }

    #[test]
    fn mistral_response_deser_zero_tokens() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":""}}],"usage":{"prompt_tokens":0,"completion_tokens":0}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "");
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
    }

    // ── build_messages with empty context string ──────────────────────

    #[test]
    fn build_messages_empty_context_string_still_injects() {
        use crate::provider::MessageRole;
        let msgs = vec![Message {
            role: MessageRole::User,
            content: "query".into(),
        }];
        let result = openai_compat::build_messages(&msgs, Some("".into()));
        assert!(result[0].content.starts_with("Context:\n"));
        assert!(result[0].content.contains("User: query"));
    }

    // ── config edge cases ─────────────────────────────────────────────

    #[test]
    fn mistral_request_with_different_model() {
        let req = ChatRequest {
            model: "open-mistral-nemo".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: Some(1.0),
            max_tokens: Some(100),
            stream: false,
            tools: None,
            parallel_tool_calls: None,
        };
        let val = serde_json::to_value(&req).unwrap();
        assert_eq!(val["model"], "open-mistral-nemo");
        assert_eq!(val["temperature"], 1.0);
        assert_eq!(val["max_tokens"], 100);
    }
}
