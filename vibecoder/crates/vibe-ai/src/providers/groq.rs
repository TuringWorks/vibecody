//! Groq provider — OpenAI-compatible ultra-fast inference.
//!
//! Supported models: llama-3.3-70b-versatile, llama-3.1-8b-instant,
//! mixtral-8x7b-32768, gemma2-9b-it, deepseek-r1-distill-llama-70b
//!
//! The implementation is [`crate::providers::compat`]; this file supplies the
//! three values that made it Groq rather than a neighbour, and keeps the
//! test suite that proves the migration changed nothing.

pub const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";

crate::openai_compat_provider!(GroqProvider, "Groq", GROQ_BASE_URL, "GROQ_API_KEY");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{AIProvider, Message, ProviderConfig};
    use crate::providers::openai_compat::{self, ChatRequest};
    use openai_compat::{ChatMessage, ChatResponse, StreamResponse};

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "groq".into(),
            api_key: Some("gsk_test".into()),
            api_url: None,
            model: "llama-3.3-70b-versatile".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
            effort: None,
        }
    }

    #[test]
    fn name_is_groq() {
        let p = GroqProvider::new(test_config());
        assert_eq!(p.name(), "Groq (llama-3.3-70b-versatile)");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = GroqProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = GroqProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(GROQ_BASE_URL, "https://api.groq.com/openai/v1");
    }

    #[test]
    fn response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"fast"}}],"usage":{"prompt_tokens":5,"completion_tokens":1}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "fast");
        assert_eq!(resp.usage.unwrap().completion_tokens, 1);
    }

    #[test]
    fn build_messages_no_context() {
        let messages = vec![
            Message {
                role: crate::provider::MessageRole::System,
                content: "sys".into(),
            },
            Message {
                role: crate::provider::MessageRole::User,
                content: "hello".into(),
            },
        ];
        let result = openai_compat::build_messages(&messages, None);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].content, "hello");
    }

    #[test]
    fn build_messages_with_context() {
        let messages = vec![Message {
            role: crate::provider::MessageRole::User,
            content: "explain this".into(),
        }];
        let result = openai_compat::build_messages(&messages, Some("fn bar() {}".into()));
        assert!(result[0].content.starts_with("Context:\nfn bar() {}"));
        assert!(result[0].content.ends_with("User: explain this"));
    }

    #[test]
    fn base_url_default() {
        let p = GroqProvider::new(test_config());
        assert_eq!(p.base_url(), "https://api.groq.com/openai/v1");
    }

    #[test]
    fn base_url_custom() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://custom.groq.proxy/v1".into());
        let p = GroqProvider::new(cfg);
        assert_eq!(p.base_url(), "https://custom.groq.proxy/v1");
    }

    #[test]
    fn request_omits_none_fields() {
        let req = ChatRequest {
            model: "llama-3.3-70b-versatile".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "hi".into(),
            }],
            temperature: None,
            max_tokens: None,
            stream: false,
            tools: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("temperature"));
        assert!(!json.contains("max_tokens"));
    }

    #[test]
    fn request_includes_set_fields() {
        let req = ChatRequest {
            model: "llama-3.3-70b-versatile".into(),
            messages: vec![],
            temperature: Some(0.5),
            max_tokens: Some(4096),
            stream: true,
            tools: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"temperature\""));
        assert!(json.contains("\"max_tokens\""));
        assert!(json.contains("\"stream\":true"));
    }

    #[test]
    fn response_deser_without_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "ok");
        assert!(resp.usage.is_none());
    }

    #[test]
    fn stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"chunk"}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_deref(), Some("chunk"));
    }

    #[test]
    fn stream_response_null_content() {
        let json = r#"{"choices":[{"delta":{}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    #[test]
    fn message_roundtrip() {
        let msg = ChatMessage {
            role: "assistant".into(),
            content: "fast response".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }

    #[test]
    fn response_deser_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"c1"}},{"message":{"role":"assistant","content":"c2"}}],"usage":{"prompt_tokens":5,"completion_tokens":2}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[1].message.content, "c2");
    }

    #[test]
    fn response_deser_empty_choices() {
        let json = r#"{"choices":[]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices.is_empty());
        assert!(resp.usage.is_none());
    }

    #[test]
    fn provider_preserves_model_config() {
        let mut cfg = test_config();
        cfg.model = "gemma2-9b-it".into();
        cfg.temperature = Some(0.9);
        cfg.max_tokens = Some(8192);
        let p = GroqProvider::new(cfg);
        assert_eq!(p.config().model, "gemma2-9b-it");
        assert_eq!(p.config().temperature, Some(0.9));
        assert_eq!(p.config().max_tokens, Some(8192));
    }

    #[test]
    fn unicode_content() {
        let msg = ChatMessage {
            role: "user".into(),
            content: "日本語テスト 🚀".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg2.content, "日本語テスト 🚀");
    }
}
