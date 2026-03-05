//! Zhipu GLM provider — Chinese market AI models with JWT authentication.
//!
//! Supported models: glm-4, glm-4-flash, glm-3-turbo
//! API key format: "<id>.<secret>" — JWT is generated from the secret half.

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

const ZHIPU_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

#[derive(Debug, Serialize)]
struct ZhipuRequest {
    model: String,
    messages: Vec<ZhipuMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ZhipuMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ZhipuUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ZhipuResponse {
    choices: Vec<ZhipuChoice>,
    #[serde(default)]
    usage: Option<ZhipuUsage>,
}

#[derive(Debug, Deserialize)]
struct ZhipuChoice {
    message: ZhipuMessage,
}

#[derive(Debug, Deserialize)]
struct ZhipuStreamResponse {
    choices: Vec<ZhipuStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct ZhipuStreamChoice {
    delta: ZhipuDelta,
}

#[derive(Debug, Deserialize)]
struct ZhipuDelta {
    content: Option<String>,
}

/// Zhipu GLM provider with JWT-based authentication.
///
/// The API key is in the format `id.secret`. A short-lived JWT is generated
/// using HMAC-SHA256 with the secret portion.
pub struct ZhipuProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl ZhipuProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    fn base_url(&self) -> String {
        self.config.api_url.clone().unwrap_or_else(|| ZHIPU_BASE_URL.to_string())
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

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<ZhipuMessage> {
        let mut result: Vec<ZhipuMessage> = messages.iter().map(|m| ZhipuMessage {
            role: m.role.as_str().to_string(),
            content: m.content.clone(),
        }).collect();
        if let Some(ctx) = context {
            if let Some(last) = result.last_mut() {
                if last.role == "user" {
                    last.content = format!("Context:\n{}\n\nUser: {}", ctx, last.content);
                }
            }
        }
        result
    }
}

/// Minimal HMAC-SHA256 implementation (avoids adding hmac/sha2 crates).
struct HmacSha256 {
    inner_key: [u8; 64],
    outer_key: [u8; 64],
    data: Vec<u8>,
}

impl HmacSha256 {
    fn new(key: &[u8]) -> Self {
        let mut padded_key = [0u8; 64];
        if key.len() > 64 {
            // SHA-256 hash the key if too long
            let hash = sha256(key);
            padded_key[..32].copy_from_slice(&hash);
        } else {
            padded_key[..key.len()].copy_from_slice(key);
        }

        let mut inner_key = [0x36u8; 64];
        let mut outer_key = [0x5cu8; 64];
        for i in 0..64 {
            inner_key[i] ^= padded_key[i];
            outer_key[i] ^= padded_key[i];
        }
        Self { inner_key, outer_key, data: Vec::new() }
    }

    fn update(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    fn finalize(&self) -> [u8; 32] {
        // inner hash = SHA-256(inner_key || data)
        let mut inner_input = Vec::with_capacity(64 + self.data.len());
        inner_input.extend_from_slice(&self.inner_key);
        inner_input.extend_from_slice(&self.data);
        let inner_hash = sha256(&inner_input);

        // outer hash = SHA-256(outer_key || inner_hash)
        let mut outer_input = Vec::with_capacity(64 + 32);
        outer_input.extend_from_slice(&self.outer_key);
        outer_input.extend_from_slice(&inner_hash);
        sha256(&outer_input)
    }
}

/// Simple SHA-256 (we already have sha2 in the workspace via bedrock.rs).
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
    fn name(&self) -> &str { "Zhipu" }

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
        let api_key = self.config.api_key.as_ref().context("Zhipu API key not set (ZHIPU_API_KEY)")?;
        let token = self.generate_token(api_key)?;
        let request = ZhipuRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };
        let url = format!("{}/chat/completions", self.base_url());
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request)
            .send().await.context("Zhipu request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Zhipu API error: {}", err);
        }
        let body: ZhipuResponse = resp.json().await.context("Failed to parse Zhipu response")?;
        let text = body.choices.first().context("No choices")?.message.content.clone();
        let usage = body.usage.map(|u| TokenUsage { prompt_tokens: u.prompt_tokens, completion_tokens: u.completion_tokens });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Zhipu API key not set")?;
        let token = self.generate_token(api_key)?;
        let request = ZhipuRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, None),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: true,
        };
        let url = format!("{}/chat/completions", self.base_url());
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&request)
            .send().await.context("Zhipu stream request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Zhipu API error: {}", err);
        }
        let stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(r) = serde_json::from_str::<ZhipuStreamResponse>(data) {
                        if let Some(c) = r.choices.first().and_then(|ch| ch.delta.content.as_ref()) {
                            content.push_str(c);
                        }
                    }
                }
            }
            Ok(content)
        }).boxed();
        Ok(stream)
    }

    async fn chat_with_images(&self, messages: &[Message], _images: &[ImageAttachment], context: Option<String>) -> Result<String> {
        self.chat(messages, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let resp: ZhipuResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "你好");
        assert_eq!(resp.usage.unwrap().completion_tokens, 2);
    }
}
