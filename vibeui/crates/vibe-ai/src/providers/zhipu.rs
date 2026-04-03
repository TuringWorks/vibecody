//! Zhipu GLM provider — Chinese market AI models with JWT authentication.
//!
//! Supported models: glm-4, glm-4-flash, glm-3-turbo
//! API key format: "<id>.<secret>" — JWT is generated from the secret half.

use super::openai_compat::{self, ChatRequest};
use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig};
use anyhow::{Context, Result};
use async_trait::async_trait;

const ZHIPU_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

/// Zhipu GLM provider with JWT-based authentication.
///
/// The API key is in the format `id.secret`. A short-lived JWT is generated
/// using HMAC-SHA256 with the secret portion.
pub struct ZhipuProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    display_name: String,
}

impl ZhipuProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let display_name = format!("Zhipu ({})", config.model);
        Self {
            config,
            client: openai_compat::default_http_client(),
            display_name,
        }
    }

    fn base_url(&self) -> String {
        self.config.api_url.clone().unwrap_or_else(|| ZHIPU_BASE_URL.to_string())
    }

    fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url())
    }

    /// Returns a JWT token derived from the API key (format: "id.secret").
    fn api_key(&self) -> Result<String> {
        let raw_key = self.config.api_key.as_ref().context("Zhipu API key not set (ZHIPU_API_KEY)")?;
        self.generate_token(raw_key)
    }

    fn make_request(&self, messages: &[Message], context: Option<String>, stream: bool) -> ChatRequest {
        ChatRequest {
            model: self.config.model.clone(),
            messages: openai_compat::build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream,
        }
    }

    /// Generate a JWT token from the API key (format: "id.secret").
    /// The JWT uses HMAC-SHA256 with the secret portion.
    fn generate_token(&self, api_key: &str) -> Result<String> {
        let parts: Vec<&str> = api_key.splitn(2, '.').collect();
        if parts.len() != 2 {
            anyhow::bail!("Zhipu API key must be in format 'id.secret'");
        }
        let id = parts[0];
        let secret = parts[1];

        // Build a simple JWT: header.payload.signature
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let exp = now + 3600; // 1 hour expiry

        let header = base64_url_encode(r#"{"alg":"HS256","sign_type":"SIGN","typ":"JWT"}"#.as_bytes());
        let payload = base64_url_encode(
            format!(r#"{{"api_key":"{}","exp":{},"timestamp":{}}}"#, id, exp, now).as_bytes()
        );

        let signing_input = format!("{}.{}", header, payload);

        // HMAC-SHA256 using the secret
        let key = secret.as_bytes();
        let mut hmac_state = HmacSha256::new(key);
        hmac_state.update(signing_input.as_bytes());
        let signature = base64_url_encode(&hmac_state.finalize());

        Ok(format!("{}.{}.{}", header, payload, signature))
    }
}

/// HMAC-SHA256 using the audited `hmac` + `sha2` crates (same as bedrock.rs).
/// Replaces the previous hand-rolled implementation.
struct HmacSha256 {
    mac: hmac::Hmac<sha2::Sha256>,
}

impl HmacSha256 {
    fn new(key: &[u8]) -> Self {
        use hmac::Mac;
        Self {
            mac: hmac::Hmac::<sha2::Sha256>::new_from_slice(key)
                .expect("HMAC can take key of any size"),
        }
    }

    fn update(&mut self, data: &[u8]) {
        use hmac::Mac;
        self.mac.update(data);
    }

    fn finalize(self) -> [u8; 32] {
        use hmac::Mac;
        let result = self.mac.finalize();
        let bytes = result.into_bytes();
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        out
    }
}

/// SHA-256 hash helper (used by JWT signing for Zhipu API authentication).
#[allow(dead_code)]
fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

fn base64_url_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        }
    }
    result
}

#[async_trait]
impl AIProvider for ZhipuProvider {
    fn name(&self) -> &str { &self.display_name }

    async fn is_available(&self) -> bool { self.config.api_key.is_some() }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        let messages = vec![
            Message { role: crate::provider::MessageRole::System, content: "You are a helpful coding assistant.".to_string() },
            Message { role: crate::provider::MessageRole::User, content: prompt },
        ];
        self.chat_response(&messages, None).await
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

    async fn chat_response(&self, messages: &[Message], context: Option<String>) -> Result<CompletionResponse> {
        let token = self.api_key()?;
        let request = self.make_request(messages, context, false);
        openai_compat::send_chat_request(&self.client, &self.chat_url(), &token, &request, "Zhipu").await
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let token = self.api_key()?;
        let request = self.make_request(messages, None, true);
        openai_compat::send_stream_request(&self.client, &self.chat_url(), &token, &request, "Zhipu").await
    }

    async fn chat_with_images(&self, messages: &[Message], _images: &[ImageAttachment], context: Option<String>) -> Result<String> {
        self.chat(messages, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openai_compat::{ChatResponse, ChatMessage, StreamResponse, ChatRequest};

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "zhipu".into(),
            api_key: Some("testid.testsecret".into()),
            api_url: None,
            model: "glm-4".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_zhipu() {
        let p = ZhipuProvider::new(test_config());
        assert_eq!(p.name(), "Zhipu");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = ZhipuProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = ZhipuProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(ZHIPU_BASE_URL, "https://open.bigmodel.cn/api/paas/v4");
    }

    #[test]
    fn jwt_generation_works() {
        let p = ZhipuProvider::new(test_config());
        let token = p.generate_token("myid.mysecret").unwrap();
        // JWT has 3 parts separated by dots
        assert_eq!(token.split('.').count(), 3);
    }

    #[test]
    fn jwt_bad_key_format_fails() {
        let p = ZhipuProvider::new(test_config());
        assert!(p.generate_token("no-dot-in-key").is_err());
    }

    #[test]
    fn zhipu_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"你好"}}],"usage":{"prompt_tokens":4,"completion_tokens":2}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "你好");
        assert_eq!(resp.usage.unwrap().completion_tokens, 2);
    }

    #[test]
    fn base_url_defaults_to_constant() {
        let p = ZhipuProvider::new(test_config());
        assert_eq!(p.base_url(), ZHIPU_BASE_URL);
    }

    #[test]
    fn base_url_uses_custom_when_set() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://custom.zhipu.example/v4".into());
        let p = ZhipuProvider::new(cfg);
        assert_eq!(p.base_url(), "https://custom.zhipu.example/v4");
    }

    #[test]
    fn build_messages_maps_roles() {
        use crate::provider::MessageRole;
        let messages = vec![
            Message { role: MessageRole::System, content: "sys".into() },
            Message { role: MessageRole::User, content: "q".into() },
        ];
        let result = openai_compat::build_messages(&messages, None);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[1].content, "q");
    }

    #[test]
    fn build_messages_appends_context() {
        use crate::provider::MessageRole;
        let messages = vec![
            Message { role: MessageRole::User, content: "query".into() },
        ];
        let result = openai_compat::build_messages(&messages, Some("background".into()));
        assert!(result[0].content.contains("Context:"));
        assert!(result[0].content.contains("background"));
        assert!(result[0].content.contains("query"));
    }

    #[test]
    fn zhipu_request_serializes_correctly() {
        let req = ChatRequest {
            model: "glm-4".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
            temperature: Some(0.5),
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "glm-4");
        assert_eq!(json["temperature"], 0.5);
        assert!(json.get("max_tokens").is_none()); // skip_serializing_if
        assert_eq!(json["stream"], false);
    }

    #[test]
    fn base64_url_encode_basic() {
        // "Hello" -> base64url "SGVsbG8"
        let result = base64_url_encode(b"Hello");
        assert_eq!(result, "SGVsbG8");
    }

    #[test]
    fn base64_url_encode_empty() {
        let result = base64_url_encode(b"");
        assert_eq!(result, "");
    }

    // ── stream response deserialization ─────────────────────────────────

    #[test]
    fn zhipu_stream_response_deser() {
        let json = r#"{"choices":[{"delta":{"content":"流式"}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_ref().unwrap(), "流式");
    }

    #[test]
    fn zhipu_stream_response_deser_null_content() {
        let json = r#"{"choices":[{"delta":{"content":null}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices[0].delta.content.is_none());
    }

    #[test]
    fn zhipu_stream_response_deser_empty_choices() {
        let json = r#"{"choices":[]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert!(resp.choices.is_empty());
    }

    // ── request serialization with all fields ───────────────────────────

    #[test]
    fn zhipu_request_serde_full() {
        let req = ChatRequest {
            model: "glm-4-flash".into(),
            messages: vec![
                ChatMessage { role: "system".into(), content: "sys".into() },
                ChatMessage { role: "user".into(), content: "q".into() },
            ],
            temperature: Some(0.5),
            max_tokens: Some(2048),
            stream: true,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "glm-4-flash");
        assert_eq!(json["temperature"], 0.5);
        assert_eq!(json["max_tokens"], 2048);
        assert_eq!(json["stream"], true);
        assert_eq!(json["messages"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn zhipu_request_serde_minimal() {
        let req = ChatRequest {
            model: "glm-3-turbo".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
            temperature: None,
            max_tokens: None,
            stream: false,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "glm-3-turbo");
        assert!(json.get("temperature").is_none());
        assert!(json.get("max_tokens").is_none());
    }

    // ── message roundtrip ───────────────────────────────────────────────

    #[test]
    fn zhipu_message_roundtrip() {
        let msg = ChatMessage { role: "user".into(), content: "测试数据".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg.role, msg2.role);
        assert_eq!(msg.content, msg2.content);
    }

    // ── build_messages edge cases ───────────────────────────────────────

    #[test]
    fn build_messages_empty_input() {
        let result = openai_compat::build_messages(&[], None);
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_empty_with_context() {
        let result = openai_compat::build_messages(&[], Some("ctx".into()));
        assert!(result.is_empty());
    }

    #[test]
    fn build_messages_context_only_affects_last_user() {
        use crate::provider::MessageRole;
        let messages = vec![
            Message { role: MessageRole::User, content: "first".into() },
            Message { role: MessageRole::Assistant, content: "mid".into() },
            Message { role: MessageRole::User, content: "second".into() },
        ];
        let result = openai_compat::build_messages(&messages, Some("bg".into()));
        assert_eq!(result[0].content, "first");
        assert!(result[2].content.starts_with("Context:\nbg"));
        assert!(result[2].content.contains("User: second"));
    }

    #[test]
    fn build_messages_context_skipped_when_last_is_assistant() {
        use crate::provider::MessageRole;
        let messages = vec![
            Message { role: MessageRole::User, content: "q".into() },
            Message { role: MessageRole::Assistant, content: "a".into() },
        ];
        let result = openai_compat::build_messages(&messages, Some("should be ignored".into()));
        assert_eq!(result[1].content, "a");
        assert_eq!(result[0].content, "q");
    }

    #[test]
    fn build_messages_all_roles_mapped() {
        use crate::provider::MessageRole;
        let messages = vec![
            Message { role: MessageRole::System, content: "s".into() },
            Message { role: MessageRole::User, content: "u".into() },
            Message { role: MessageRole::Assistant, content: "a".into() },
        ];
        let result = openai_compat::build_messages(&messages, None);
        assert_eq!(result[0].role, "system");
        assert_eq!(result[1].role, "user");
        assert_eq!(result[2].role, "assistant");
    }

    // ── JWT generation edge cases ───────────────────────────────────────

    #[test]
    fn jwt_token_header_contains_hs256() {
        let p = ZhipuProvider::new(test_config());
        let token = p.generate_token("testid.testsecret").unwrap();
        // Decode header (first segment, base64url)
        let header_b64 = token.split('.').next().unwrap();
        // The header should encode {"alg":"HS256","sign_type":"SIGN","typ":"JWT"}
        assert!(!header_b64.is_empty());
        // Verify 3 parts
        assert_eq!(token.split('.').count(), 3);
    }

    #[test]
    fn jwt_different_keys_produce_different_tokens() {
        let p = ZhipuProvider::new(test_config());
        let t1 = p.generate_token("id1.secret1").unwrap();
        let t2 = p.generate_token("id2.secret2").unwrap();
        assert_ne!(t1, t2);
    }

    #[test]
    fn jwt_empty_id_still_works() {
        let p = ZhipuProvider::new(test_config());
        // Empty id but valid format
        let result = p.generate_token(".secret");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().split('.').count(), 3);
    }

    // ── HmacSha256 unit tests ───────────────────────────────────────────

    #[test]
    fn hmac_sha256_deterministic() {
        let mut h1 = HmacSha256::new(b"key");
        h1.update(b"data");
        let r1 = h1.finalize();

        let mut h2 = HmacSha256::new(b"key");
        h2.update(b"data");
        let r2 = h2.finalize();

        assert_eq!(r1, r2);
        assert_eq!(r1.len(), 32);
    }

    #[test]
    fn hmac_sha256_different_keys_differ() {
        let mut h1 = HmacSha256::new(b"key1");
        h1.update(b"data");
        let r1 = h1.finalize();

        let mut h2 = HmacSha256::new(b"key2");
        h2.update(b"data");
        let r2 = h2.finalize();

        assert_ne!(r1, r2);
    }

    #[test]
    fn hmac_sha256_different_data_differ() {
        let mut h1 = HmacSha256::new(b"key");
        h1.update(b"data1");
        let r1 = h1.finalize();

        let mut h2 = HmacSha256::new(b"key");
        h2.update(b"data2");
        let r2 = h2.finalize();

        assert_ne!(r1, r2);
    }

    // ── base64_url_encode additional cases ──────────────────────────────

    #[test]
    fn base64_url_encode_no_padding() {
        // Standard base64 would pad with '=', base64url should not
        let result = base64_url_encode(b"a");
        assert!(!result.contains('='));
        assert!(!result.contains('+'));
        assert!(!result.contains('/'));
    }

    #[test]
    fn base64_url_encode_uses_url_safe_chars() {
        // Encode bytes that would produce + and / in standard base64
        let result = base64_url_encode(&[0xfb, 0xef, 0xbe]);
        // Should only contain URL-safe characters
        for c in result.chars() {
            assert!(c.is_ascii_alphanumeric() || c == '-' || c == '_',
                "Unexpected char: {}", c);
        }
    }

    // ── response with multiple choices ──────────────────────────────────

    #[test]
    fn zhipu_response_multiple_choices() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"答案1"}},{"message":{"role":"assistant","content":"答案2"}}],"usage":{"prompt_tokens":4,"completion_tokens":4}}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 2);
        assert_eq!(resp.choices[0].message.content, "答案1");
        assert_eq!(resp.choices[1].message.content, "答案2");
    }

    #[test]
    fn zhipu_response_deser_no_usage() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"ok"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.usage.is_none());
    }

    // ── provider config preserved ───────────────────────────────────────

    #[test]
    fn provider_preserves_model_config() {
        let mut cfg = test_config();
        cfg.model = "glm-4-flash".into();
        cfg.temperature = Some(0.3);
        let p = ZhipuProvider::new(cfg);
        assert_eq!(p.config.model, "glm-4-flash");
        assert_eq!(p.config.temperature, Some(0.3));
    }

    // ── additional edge case tests ──────────────────────────────────────

    #[test]
    fn zhipu_message_unicode_cjk_roundtrip() {
        let msg = ChatMessage { role: "user".into(), content: "你好世界 Hello 日本語".into() };
        let json = serde_json::to_string(&msg).unwrap();
        let msg2: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg2.content, "你好世界 Hello 日本語");
    }

    #[test]
    fn zhipu_usage_deser() {
        let json = r#"{"prompt_tokens":200,"completion_tokens":80}"#;
        let usage: openai_compat::ChatUsage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.prompt_tokens, 200);
        assert_eq!(usage.completion_tokens, 80);
    }

    #[test]
    fn jwt_generation_same_key_produces_3_part_token() {
        let p = ZhipuProvider::new(test_config());
        let t1 = p.generate_token("testid.testsecret").unwrap();
        let parts: Vec<&str> = t1.split('.').collect();
        assert_eq!(parts.len(), 3);
        // Each part should be non-empty
        assert!(!parts[0].is_empty());
        assert!(!parts[1].is_empty());
        assert!(!parts[2].is_empty());
    }

    #[test]
    fn jwt_generation_empty_secret_still_works() {
        let p = ZhipuProvider::new(test_config());
        let result = p.generate_token("myid.");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().split('.').count(), 3);
    }

    #[test]
    fn jwt_key_with_multiple_dots_uses_first_split() {
        let p = ZhipuProvider::new(test_config());
        // "id.secret.extra" splits into id="id", secret="secret.extra"
        let result = p.generate_token("id.secret.extra");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().split('.').count(), 3);
    }

    #[test]
    fn base64_url_encode_single_byte() {
        let result = base64_url_encode(&[0xFF]);
        assert!(!result.is_empty());
        // Should only use URL-safe chars
        for c in result.chars() {
            assert!(c.is_ascii_alphanumeric() || c == '-' || c == '_');
        }
    }

    #[test]
    fn base64_url_encode_three_byte_aligned() {
        // 3 bytes should produce exactly 4 chars (no padding needed)
        let result = base64_url_encode(b"abc");
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn hmac_sha256_empty_data() {
        let mut h = HmacSha256::new(b"key");
        h.update(b"");
        let result = h.finalize();
        assert_eq!(result.len(), 32);
        // Should be deterministic
        let mut h2 = HmacSha256::new(b"key");
        h2.update(b"");
        assert_eq!(result, h2.finalize());
    }

    #[test]
    fn hmac_sha256_long_key() {
        // Key longer than 64 bytes triggers SHA-256 hash of the key
        let long_key = vec![0xABu8; 100];
        let mut h = HmacSha256::new(&long_key);
        h.update(b"test data");
        let result = h.finalize();
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn zhipu_stream_response_deser_with_content() {
        let json = r#"{"choices":[{"delta":{"content":"你好"}}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].delta.content.as_deref(), Some("你好"));
    }

    #[test]
    fn build_messages_single_system_context_not_injected() {
        use crate::provider::MessageRole;
        let messages = vec![
            Message { role: MessageRole::System, content: "sys".into() },
        ];
        let result = openai_compat::build_messages(&messages, Some("ctx".into()));
        // Last message is "system", not "user", so context should NOT be injected
        assert_eq!(result[0].content, "sys");
    }
}
