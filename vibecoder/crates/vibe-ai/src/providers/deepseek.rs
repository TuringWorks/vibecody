//! DeepSeek provider — code-focused models, OpenAI-compatible API.
//!
//! Supported models: deepseek-chat (V3), deepseek-reasoner (R1), deepseek-coder (legacy)
//!
//! The implementation is [`crate::providers::compat`]; this file supplies the
//! three values that made it DeepSeek rather than a neighbour, and keeps the
//! test suite that proves the migration changed nothing.

pub const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/v1";

crate::openai_compat_provider!(
    DeepSeekProvider,
    "DeepSeek",
    DEEPSEEK_BASE_URL,
    "DEEPSEEK_API_KEY"
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{AIProvider, Message, ProviderConfig};
    use crate::providers::openai_compat::{self, ChatRequest};
    use openai_compat::{ChatMessage, ChatResponse, StreamResponse};

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "deepseek".into(),
            api_key: Some("sk-test".into()),
            api_url: None,
            model: "deepseek-coder".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
            effort: None,
        }
    }

    #[test]
    fn name_is_deepseek() {
        let p = DeepSeekProvider::new(test_config());
        assert_eq!(p.name(), "DeepSeek (deepseek-coder)");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = DeepSeekProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = DeepSeekProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(DEEPSEEK_BASE_URL, "https://api.deepseek.com/v1");
    }

    #[test]
    fn deepseek_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"code"}}],"usage":{"prompt_tokens":8,"completion_tokens":3}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "code");
        assert_eq!(resp.usage.unwrap().completion_tokens, 3);
    }

    // ── build_messages context injection ────────────────────────────────

    #[test]
    fn build_messages_without_context() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message {
                role: MessageRole::System,
                content: "System prompt".into(),
            },
            Message {
                role: MessageRole::User,
                content: "Help me".into(),
            },
        ];
        let result = openai_compat::build_messages(&msgs, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[0].content, "System prompt");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[1].content, "Help me");
    }

    #[test]
    fn build_messages_with_context_prepends_to_last_user() {
        use crate::provider::MessageRole;
        let msgs = vec![Message {
            role: MessageRole::User,
            content: "Review this".into(),
        }];
        let result = openai_compat::build_messages(&msgs, Some("fn main() {}".into()));
        assert_eq!(result.len(), 1);
        assert!(result[0].content.starts_with("Context:\nfn main() {}"));
        assert!(result[0].content.contains("User: Review this"));
    }

    #[test]
    fn build_messages_context_only_modifies_last_user() {
        use crate::provider::MessageRole;
        let msgs = vec![
            Message {
                role: MessageRole::User,
                content: "First Q".into(),
            },
            Message {
                role: MessageRole::Assistant,
                content: "First A".into(),
            },
            Message {
                role: MessageRole::User,
                content: "Second Q".into(),
            },
        ];
        let result = openai_compat::build_messages(&msgs, Some("context data".into()));
        // First user message should be unchanged
        assert_eq!(result[0].content, "First Q");
        // Last user message should have context injected
        assert!(result[2].content.starts_with("Context:\ncontext data"));
        assert!(result[2].content.contains("User: Second Q"));
    }

    #[test]
    fn build_messages_context_ignored_when_last_is_assistant() {
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
        let result = openai_compat::build_messages(&msgs, Some("extra ctx".into()));
        // Last message is assistant, context NOT injected
        assert_eq!(result[1].content, "A");
    }

    #[test]
    fn build_messages_empty_input() {
        let result = openai_compat::build_messages(&[], Some("ctx".into()));
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_role_mapping_all_roles() {
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
    fn name_returns_deepseek() {
        let p = DeepSeekProvider::new(test_config());
        assert_eq!(p.name(), "DeepSeek (deepseek-coder)");
    }

    #[tokio::test]
    async fn availability_tracks_api_key() {
        let p_with = DeepSeekProvider::new(test_config());
        assert!(p_with.is_available().await);

        let mut cfg = test_config();
        cfg.api_key = None;
        let p_without = DeepSeekProvider::new(cfg);
        assert!(!p_without.is_available().await);
    }

    // ── base_url ────────────────────────────────────────────────────────

    #[test]
    fn base_url_custom_override() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://my-proxy.example.com/v1".into());
        let p = DeepSeekProvider::new(cfg);
        assert_eq!(p.base_url(), "https://my-proxy.example.com/v1");
    }

    #[test]
    fn base_url_default_value() {
        let p = DeepSeekProvider::new(test_config());
        assert_eq!(p.base_url(), "https://api.deepseek.com/v1");
    }

    // ── request serialization ───────────────────────────────────────────

    #[test]
    fn deepseek_request_serde_minimal() {
        let req = ChatRequest {
            model: "deepseek-coder".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "test".into(),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
            tools: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["model"], "deepseek-coder");
        assert_eq!(val["stream"], false);
        // Optional fields should be omitted
        assert!(val.get("temperature").is_none());
        assert!(val.get("max_tokens").is_none());
    }

    #[test]
    fn deepseek_request_serde_full() {
        let req = ChatRequest {
            model: "deepseek-chat".into(),
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
            temperature: Some(0.3),
            max_tokens: Some(4096),
            stream: true,
            tools: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["temperature"], 0.3);
        assert_eq!(val["max_tokens"], 4096);
        assert_eq!(val["stream"], true);
        assert_eq!(val["messages"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn deepseek_response_deser_no_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"done"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "done");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn deepseek_stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"token"}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "token");
    }

    #[test]
    fn deepseek_stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{"content":null}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    #[test]
    fn deepseek_message_deser_roundtrip() {
        let msg = ChatMessage {
            role: "user".into(),
            content: "hello world".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }

    // ── response parsing edge cases ───────────────────────────────────

    #[test]
    fn deepseek_response_deser_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"x"}},{"message":{"role":"assistant","content":"y"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[1].message.content, "y");
    }

    #[test]
    fn deepseek_response_deser_large_token_counts() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}],"usage":{"prompt_tokens":100000,"completion_tokens":50000}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        let usage = resp.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 100000);
        assert_eq!(usage.completion_tokens, 50000);
    }

    #[test]
    fn deepseek_stream_response_deser_empty_string_content() {
        let json = r#"{"choices":[{"delta":{"content":""}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "");
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

    // ── request with different model names ────────────────────────────

    #[test]
    fn deepseek_request_reasoner_model() {
        let req = ChatRequest {
            model: "deepseek-reasoner".into(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: "think step by step".into(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: "solve this".into(),
                },
            ],
            temperature: Some(0.0),
            max_tokens: Some(8192),
            stream: false,
            tools: None,
        };
        let val = serde_json::to_value(&req).unwrap();
        assert_eq!(val["model"], "deepseek-reasoner");
        assert_eq!(val["messages"].as_array().unwrap().len(), 2);
        assert_eq!(val["max_tokens"], 8192);
    }

    #[test]
    fn deepseek_request_chat_model() {
        let req = ChatRequest {
            model: "deepseek-chat".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: None,
            max_tokens: None,
            stream: true,
            tools: None,
        };
        let val = serde_json::to_value(&req).unwrap();
        assert_eq!(val["model"], "deepseek-chat");
        assert_eq!(val["stream"], true);
    }
}
