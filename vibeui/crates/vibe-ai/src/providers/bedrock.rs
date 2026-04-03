//! AWS Bedrock provider — Claude, Llama 3, Mistral via the Converse API.
//!
//! Auth: AWS_ACCESS_KEY_ID + AWS_SECRET_ACCESS_KEY + AWS_REGION (or config fields).
//!
//! Default model: anthropic.claude-3-sonnet-20240229-v1:0
//! Other models:  meta.llama3-8b-instruct-v1:0
//!                mistral.mistral-7b-instruct-v0:2
//!                amazon.titan-text-express-v1

use crate::provider::{
    AIProvider, CodeContext, CompletionResponse, CompletionStream,
    ImageAttachment, Message, MessageRole, ProviderConfig, TokenUsage,
};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use futures::stream;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ── SigV4 helpers ─────────────────────────────────────────────────────────────

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac =
        <Hmac<Sha256>>::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn derive_signing_key(secret: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k = hmac_sha256(format!("AWS4{}", secret).as_bytes(), date.as_bytes());
    let k = hmac_sha256(&k, region.as_bytes());
    let k = hmac_sha256(&k, service.as_bytes());
    hmac_sha256(&k, b"aws4_request")
}

/// Build the Authorization header for a Bedrock Converse POST request.
fn sigv4_auth_header(
    access_key: &str,
    secret_key: &str,
    region: &str,
    host: &str,
    path: &str,
    payload: &[u8],
    datetime: &str, // "20240101T120000Z"
) -> String {
    let date = &datetime[..8]; // "20240101"
    let service = "bedrock";

    let payload_hash = sha256_hex(payload);

    let canonical_headers = format!(
        "content-type:application/json\nhost:{}\nx-amz-date:{}\n",
        host, datetime
    );
    let signed_headers = "content-type;host;x-amz-date";

    let canonical_request = format!(
        "POST\n{}\n\n{}\n{}\n{}",
        path, canonical_headers, signed_headers, payload_hash
    );

    let credential_scope = format!("{}/{}/{}/aws4_request", date, region, service);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        datetime,
        credential_scope,
        sha256_hex(canonical_request.as_bytes())
    );

    let signing_key = derive_signing_key(secret_key, date, region, service);
    let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));

    format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        access_key, credential_scope, signed_headers, signature
    )
}

// ── Converse API types ────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ConverseRequest {
    messages: Vec<ConverseMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<Vec<SystemBlock>>,
    #[serde(rename = "inferenceConfig", skip_serializing_if = "Option::is_none")]
    inference_config: Option<InferenceConfig>,
}

#[derive(Debug, Serialize)]
struct SystemBlock {
    text: String,
}

#[derive(Debug, Serialize)]
struct InferenceConfig {
    #[serde(rename = "maxTokens", skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize)]
struct ConverseMessage {
    role: String,
    content: Vec<ContentBlock>,
}

#[derive(Debug, Serialize)]
struct ContentBlock {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ConverseResponse {
    output: ConverseOutput,
    usage: Option<ConverseUsage>,
}

#[derive(Debug, Deserialize)]
struct ConverseOutput {
    message: ConverseOutMessage,
}

#[derive(Debug, Deserialize)]
struct ConverseOutMessage {
    content: Vec<ConverseOutContent>,
}

#[derive(Debug, Deserialize)]
struct ConverseOutContent {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConverseUsage {
    #[serde(rename = "inputTokens")]
    input_tokens: Option<u32>,
    #[serde(rename = "outputTokens")]
    output_tokens: Option<u32>,
}

// ── Provider ──────────────────────────────────────────────────────────────────

pub struct BedrockProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    display_name: String,
}

impl BedrockProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let display_name = format!("Bedrock ({})", config.model);
        Self {
            config,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(90))
                .connect_timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            display_name,
        }
    }

    fn region(&self) -> String {
        self.config
            .api_url
            .clone()
            .unwrap_or_else(|| std::env::var("AWS_REGION")
                .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
                .unwrap_or_else(|_| "us-east-1".to_string()))
    }

    fn access_key(&self) -> String {
        std::env::var("AWS_ACCESS_KEY_ID").unwrap_or_default()
    }

    fn secret_key(&self) -> String {
        // api_key field doubles as AWS_SECRET_ACCESS_KEY when set in config
        self.config.api_key.clone()
            .or_else(|| std::env::var("AWS_SECRET_ACCESS_KEY").ok())
            .unwrap_or_default()
    }

    /// UTC datetime string "YYYYMMDDTHHmmssZ"
    fn utc_datetime() -> String {
        // Build ISO-8601 compact UTC datetime using SystemTime (no chrono dep)
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Simple broken-down UTC time from epoch seconds
        let s = secs % 60;
        let m = (secs / 60) % 60;
        let h = (secs / 3600) % 24;
        let days = secs / 86400; // days since 1970-01-01
        // Gregorian calendar
        let (y, mo, d) = epoch_days_to_ymd(days);
        format!("{:04}{:02}{:02}T{:02}{:02}{:02}Z", y, mo, d, h, m, s)
    }

    fn build_converse_request(
        &self,
        messages: &[Message],
        context: Option<String>,
    ) -> ConverseRequest {
        let mut system_blocks: Vec<SystemBlock> = Vec::new();
        let mut chat_msgs: Vec<ConverseMessage> = Vec::new();

        for msg in messages {
            match msg.role {
                MessageRole::System => {
                    system_blocks.push(SystemBlock { text: msg.content.clone() });
                }
                MessageRole::User => {
                    let text = if let Some(ref ctx) = context {
                        format!("Context:\n{}\n\nUser: {}", ctx, msg.content)
                    } else {
                        msg.content.clone()
                    };
                    chat_msgs.push(ConverseMessage {
                        role: "user".to_string(),
                        content: vec![ContentBlock { text }],
                    });
                }
                MessageRole::Assistant => {
                    chat_msgs.push(ConverseMessage {
                        role: "assistant".to_string(),
                        content: vec![ContentBlock { text: msg.content.clone() }],
                    });
                }
            }
        }

        ConverseRequest {
            messages: chat_msgs,
            system: if system_blocks.is_empty() { None } else { Some(system_blocks) },
            inference_config: Some(InferenceConfig {
                max_tokens: self.config.max_tokens,
                temperature: self.config.temperature,
            }),
        }
    }

    async fn converse(&self, messages: &[Message], context: Option<String>) -> Result<ConverseResponse> {
        let region = self.region();
        let model_id = &self.config.model;
        let host = format!("bedrock-runtime.{}.amazonaws.com", region);
        // Percent-encode the model ID (colons become %3A, slashes become %2F)
        let encoded_model: String = model_id.chars().map(|c| match c {
            ':' => "%3A".to_string(),
            '/' => "%2F".to_string(),
            c => c.to_string(),
        }).collect();
        let path = format!("/model/{}/converse", encoded_model);

        let body =
            serde_json::to_vec(&self.build_converse_request(messages, context))?;

        let datetime = Self::utc_datetime();
        let access_key = self.access_key();
        let secret_key = self.secret_key();

        if access_key.is_empty() {
            bail!("AWS_ACCESS_KEY_ID not set");
        }
        if secret_key.is_empty() {
            bail!("AWS_SECRET_ACCESS_KEY not set (or config.api_key)");
        }

        let auth = sigv4_auth_header(
            &access_key, &secret_key, &region, &host, &path, &body, &datetime,
        );

        let url = format!("https://{}{}", host, path);
        let resp = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-amz-date", &datetime)
            .header("Authorization", auth)
            .body(body)
            .send()
            .await
            .context("Bedrock request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("Bedrock API error {}: {}", status, text);
        }

        resp.json::<ConverseResponse>().await.context("Failed to parse Bedrock response")
    }
}

// Gregorian calendar conversion (no external dep)
fn epoch_days_to_ymd(z: u64) -> (u32, u32, u32) {
    let z = z as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32)
}

#[async_trait]
impl AIProvider for BedrockProvider {
    fn name(&self) -> &str { &self.display_name }

    async fn is_available(&self) -> bool {
        !self.access_key().is_empty() && !self.secret_key().is_empty()
    }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        let prompt = format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        );
        let messages = vec![
            Message { role: MessageRole::System, content: "You are a helpful coding assistant. Output only the completion, no explanation.".to_string() },
            Message { role: MessageRole::User, content: prompt },
        ];
        self.chat_response(&messages, None).await
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let resp = self.complete(context).await?;
        Ok(Box::pin(stream::once(async move { Ok(resp.text) })))
    }

    async fn chat_response(&self, messages: &[Message], context: Option<String>) -> Result<CompletionResponse> {
        let resp = self.converse(messages, context).await?;
        let text = resp.output.message.content
            .iter()
            .filter_map(|c| c.text.clone())
            .collect::<Vec<_>>()
            .join("");
        let usage = resp.usage.map(|u| TokenUsage {
            prompt_tokens: u.input_tokens.unwrap_or(0),
            completion_tokens: u.output_tokens.unwrap_or(0),
        });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        // Bedrock streaming uses HTTP/2 event streams; fall back to single-shot wrapped as stream
        let text = self.chat(messages, None).await?;
        Ok(Box::pin(stream::once(async move { Ok(text) })))
    }

    async fn chat_with_images(&self, messages: &[Message], _images: &[ImageAttachment], context: Option<String>) -> Result<String> {
        // Claude-on-Bedrock supports images via the Converse API, but we skip for now
        self.chat(messages, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── sha256_hex ───────────────────────────────────────────────────────

    #[test]
    fn sha256_hex_empty() {
        // SHA-256 of empty string is well-known
        let h = sha256_hex(b"");
        assert_eq!(h, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn sha256_hex_hello() {
        let h = sha256_hex(b"hello");
        assert_eq!(h, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }

    // ── hmac_sha256 ──────────────────────────────────────────────────────

    #[test]
    fn hmac_sha256_deterministic() {
        let a = hmac_sha256(b"key", b"data");
        let b = hmac_sha256(b"key", b"data");
        assert_eq!(a, b);
        assert_eq!(a.len(), 32); // SHA-256 output is 32 bytes
    }

    #[test]
    fn hmac_sha256_different_keys_differ() {
        let a = hmac_sha256(b"key1", b"data");
        let b = hmac_sha256(b"key2", b"data");
        assert_ne!(a, b);
    }

    // ── derive_signing_key ───────────────────────────────────────────────

    #[test]
    fn derive_signing_key_deterministic() {
        let k1 = derive_signing_key("secret", "20240101", "us-east-1", "bedrock");
        let k2 = derive_signing_key("secret", "20240101", "us-east-1", "bedrock");
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 32);
    }

    #[test]
    fn derive_signing_key_differs_by_date() {
        let k1 = derive_signing_key("secret", "20240101", "us-east-1", "bedrock");
        let k2 = derive_signing_key("secret", "20240102", "us-east-1", "bedrock");
        assert_ne!(k1, k2);
    }

    #[test]
    fn derive_signing_key_differs_by_region() {
        let k1 = derive_signing_key("secret", "20240101", "us-east-1", "bedrock");
        let k2 = derive_signing_key("secret", "20240101", "eu-west-1", "bedrock");
        assert_ne!(k1, k2);
    }

    // ── epoch_days_to_ymd ────────────────────────────────────────────────

    #[test]
    fn epoch_days_to_ymd_unix_epoch() {
        // Day 0 = 1970-01-01
        assert_eq!(epoch_days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn epoch_days_to_ymd_known_dates() {
        // 2000-01-01 = day 10957
        assert_eq!(epoch_days_to_ymd(10957), (2000, 1, 1));
        // 2024-01-01 = day 19723
        assert_eq!(epoch_days_to_ymd(19723), (2024, 1, 1));
    }

    #[test]
    fn epoch_days_to_ymd_leap_day() {
        // 2024-02-29 = day 19782 (2024 is a leap year)
        assert_eq!(epoch_days_to_ymd(19782), (2024, 2, 29));
    }

    #[test]
    fn epoch_days_to_ymd_end_of_year() {
        // 2023-12-31 = day 19722
        assert_eq!(epoch_days_to_ymd(19722), (2023, 12, 31));
    }

    // ── sigv4_auth_header ────────────────────────────────────────────────

    #[test]
    fn sigv4_auth_header_format() {
        let header = sigv4_auth_header(
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
            "us-east-1",
            "bedrock-runtime.us-east-1.amazonaws.com",
            "/model/test/converse",
            b"{}",
            "20240101T120000Z",
        );
        assert!(header.starts_with("AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/"));
        assert!(header.contains("us-east-1/bedrock/aws4_request"));
        assert!(header.contains("SignedHeaders=content-type;host;x-amz-date"));
        assert!(header.contains("Signature="));
    }

    #[test]
    fn sigv4_auth_header_deterministic() {
        let a = sigv4_auth_header("AK", "SK", "us-east-1", "h", "/p", b"{}", "20240101T120000Z");
        let b = sigv4_auth_header("AK", "SK", "us-east-1", "h", "/p", b"{}", "20240101T120000Z");
        assert_eq!(a, b);
    }

    #[test]
    fn sigv4_auth_header_differs_by_payload() {
        let a = sigv4_auth_header("AK", "SK", "us-east-1", "h", "/p", b"{}", "20240101T120000Z");
        let b = sigv4_auth_header("AK", "SK", "us-east-1", "h", "/p", b"{\"x\":1}", "20240101T120000Z");
        assert_ne!(a, b);
    }

    // ── epoch_days_to_ymd — additional known dates ──────────────────────

    #[test]
    fn epoch_days_to_ymd_2000_01_01() {
        // 2000-01-01 is day 10957 from Unix epoch
        assert_eq!(epoch_days_to_ymd(10957), (2000, 1, 1));
    }

    #[test]
    fn epoch_days_to_ymd_leap_year_2000_feb29() {
        // 2000 is a leap year (divisible by 400); 2000-02-29 is day 10957+59 = 11016
        assert_eq!(epoch_days_to_ymd(11016), (2000, 2, 29));
    }

    #[test]
    fn epoch_days_to_ymd_non_leap_1900_equivalent() {
        // Day 1 = 1970-01-02
        assert_eq!(epoch_days_to_ymd(1), (1970, 1, 2));
    }

    #[test]
    fn epoch_days_to_ymd_mid_year() {
        // 1970-07-01 = day 181 (31+28+31+30+31+30 = 181)
        assert_eq!(epoch_days_to_ymd(181), (1970, 7, 1));
    }

    #[test]
    fn epoch_days_to_ymd_2024_leap_feb28() {
        // 2024-02-28 = day 19781 (day before the leap day we already test)
        assert_eq!(epoch_days_to_ymd(19781), (2024, 2, 28));
    }

    #[test]
    fn epoch_days_to_ymd_2024_mar01() {
        // 2024-03-01 = day 19783 (day after the leap day)
        assert_eq!(epoch_days_to_ymd(19783), (2024, 3, 1));
    }

    #[test]
    fn epoch_days_to_ymd_far_future() {
        // 2100-01-01 = day 47482
        assert_eq!(epoch_days_to_ymd(47482), (2100, 1, 1));
    }

    // ── utc_datetime format ─────────────────────────────────────────────

    #[test]
    fn utc_datetime_format_matches_pattern() {
        let dt = BedrockProvider::utc_datetime();
        // Must be exactly 16 characters: YYYYMMDDTHHMMSSZ
        assert_eq!(dt.len(), 16, "utc_datetime should be 16 chars: {}", dt);
        assert!(dt.ends_with('Z'), "utc_datetime should end with Z: {}", dt);
        assert_eq!(&dt[8..9], "T", "utc_datetime should have T at position 8: {}", dt);
        // All chars except T and Z should be digits
        for (i, c) in dt.chars().enumerate() {
            if i == 8 {
                assert_eq!(c, 'T');
            } else if i == 15 {
                assert_eq!(c, 'Z');
            } else {
                assert!(c.is_ascii_digit(), "char at position {} should be a digit, got '{}'", i, c);
            }
        }
    }

    #[test]
    fn utc_datetime_year_is_reasonable() {
        let dt = BedrockProvider::utc_datetime();
        let year: u32 = dt[..4].parse().unwrap();
        assert!(year >= 2024 && year <= 2100, "year {} seems unreasonable", year);
    }

    // ── model ID percent-encoding ───────────────────────────────────────

    #[test]
    fn model_id_colon_percent_encoding() {
        let model_id = "anthropic.claude-3-sonnet-20240229-v1:0";
        let encoded: String = model_id.chars().map(|c| match c {
            ':' => "%3A".to_string(),
            '/' => "%2F".to_string(),
            c => c.to_string(),
        }).collect();
        assert_eq!(encoded, "anthropic.claude-3-sonnet-20240229-v1%3A0");
        assert!(!encoded.contains(':'));
    }

    #[test]
    fn model_id_slash_percent_encoding() {
        let model_id = "us.meta/llama3-8b-instruct-v1:0";
        let encoded: String = model_id.chars().map(|c| match c {
            ':' => "%3A".to_string(),
            '/' => "%2F".to_string(),
            c => c.to_string(),
        }).collect();
        assert_eq!(encoded, "us.meta%2Fllama3-8b-instruct-v1%3A0");
        assert!(!encoded.contains(':'));
        assert!(!encoded.contains('/'));
    }

    #[test]
    fn model_id_no_special_chars_unchanged() {
        let model_id = "amazon.titan-text-express-v1";
        let encoded: String = model_id.chars().map(|c| match c {
            ':' => "%3A".to_string(),
            '/' => "%2F".to_string(),
            c => c.to_string(),
        }).collect();
        assert_eq!(encoded, model_id);
    }

    // ── build_converse_request ──────────────────────────────────────────

    fn test_bedrock_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "bedrock".into(),
            api_key: Some("test_secret_key".into()),
            api_url: Some("us-east-1".into()),
            model: "anthropic.claude-3-sonnet-20240229-v1:0".into(),
            temperature: Some(0.7),
            max_tokens: Some(1024),
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn build_converse_request_system_message_placement() {
        let provider = BedrockProvider::new(test_bedrock_config());
        let messages = vec![
            Message { role: MessageRole::System, content: "You are helpful.".into() },
            Message { role: MessageRole::User, content: "Hello".into() },
        ];
        let req = provider.build_converse_request(&messages, None);

        // System messages should be in the system blocks, not in chat messages
        assert!(req.system.is_some());
        let sys_blocks = req.system.as_ref().unwrap();
        assert_eq!(sys_blocks.len(), 1);
        assert_eq!(sys_blocks[0].text, "You are helpful.");

        // Only the user message should appear in messages
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, "user");
        assert_eq!(req.messages[0].content[0].text, "Hello");
    }

    #[test]
    fn build_converse_request_context_injection() {
        let provider = BedrockProvider::new(test_bedrock_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "What is this?".into() },
        ];
        let req = provider.build_converse_request(&messages, Some("file: main.rs".into()));

        assert_eq!(req.messages.len(), 1);
        assert!(req.messages[0].content[0].text.starts_with("Context:\nfile: main.rs"));
        assert!(req.messages[0].content[0].text.contains("User: What is this?"));
    }

    #[test]
    fn build_converse_request_no_context_leaves_message_intact() {
        let provider = BedrockProvider::new(test_bedrock_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "Hello".into() },
        ];
        let req = provider.build_converse_request(&messages, None);

        assert_eq!(req.messages[0].content[0].text, "Hello");
    }

    #[test]
    fn build_converse_request_no_system_messages_yields_none() {
        let provider = BedrockProvider::new(test_bedrock_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "Hi".into() },
            Message { role: MessageRole::Assistant, content: "Hello".into() },
        ];
        let req = provider.build_converse_request(&messages, None);

        assert!(req.system.is_none());
    }

    #[test]
    fn build_converse_request_multiple_system_messages() {
        let provider = BedrockProvider::new(test_bedrock_config());
        let messages = vec![
            Message { role: MessageRole::System, content: "Rule 1".into() },
            Message { role: MessageRole::System, content: "Rule 2".into() },
            Message { role: MessageRole::User, content: "Go".into() },
        ];
        let req = provider.build_converse_request(&messages, None);

        let sys = req.system.as_ref().unwrap();
        assert_eq!(sys.len(), 2);
        assert_eq!(sys[0].text, "Rule 1");
        assert_eq!(sys[1].text, "Rule 2");
    }

    #[test]
    fn build_converse_request_assistant_role_mapping() {
        let provider = BedrockProvider::new(test_bedrock_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "Hi".into() },
            Message { role: MessageRole::Assistant, content: "Hello!".into() },
            Message { role: MessageRole::User, content: "How?".into() },
        ];
        let req = provider.build_converse_request(&messages, None);

        assert_eq!(req.messages.len(), 3);
        assert_eq!(req.messages[0].role, "user");
        assert_eq!(req.messages[1].role, "assistant");
        assert_eq!(req.messages[1].content[0].text, "Hello!");
        assert_eq!(req.messages[2].role, "user");
    }

    #[test]
    fn build_converse_request_inference_config() {
        let provider = BedrockProvider::new(test_bedrock_config());
        let messages = vec![
            Message { role: MessageRole::User, content: "test".into() },
        ];
        let req = provider.build_converse_request(&messages, None);

        let ic = req.inference_config.as_ref().unwrap();
        assert_eq!(ic.max_tokens, Some(1024));
        assert_eq!(ic.temperature, Some(0.7));
    }

    #[test]
    fn build_converse_request_serializes_to_valid_json() {
        let provider = BedrockProvider::new(test_bedrock_config());
        let messages = vec![
            Message { role: MessageRole::System, content: "Be helpful".into() },
            Message { role: MessageRole::User, content: "Hello".into() },
        ];
        let req = provider.build_converse_request(&messages, Some("ctx".into()));
        let json = serde_json::to_string(&req).unwrap();

        // Verify it round-trips as valid JSON
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(val["system"].is_array());
        assert!(val["messages"].is_array());
        assert!(val["inferenceConfig"].is_object());
    }

    // ── Bedrock provider name ───────────────────────────────────────────

    #[test]
    fn bedrock_provider_name() {
        let provider = BedrockProvider::new(test_bedrock_config());
        assert_eq!(provider.name(), "Bedrock");
    }

    // ── ConverseResponse deserialization ─────────────────────────────────

    #[test]
    fn converse_response_deser_full() {
        let json = r#"{
            "output": {
                "message": {
                    "content": [{"text": "Hello world"}]
                }
            },
            "usage": {
                "inputTokens": 10,
                "outputTokens": 5
            }
        }"#;
        let resp: ConverseResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.output.message.content[0].text.as_ref().unwrap(), "Hello world");
        let usage = resp.usage.unwrap();
        assert_eq!(usage.input_tokens.unwrap(), 10);
        assert_eq!(usage.output_tokens.unwrap(), 5);
    }

    #[test]
    fn converse_response_deser_no_usage() {
        let json = r#"{
            "output": {
                "message": {
                    "content": [{"text": "test"}]
                }
            }
        }"#;
        let resp: ConverseResponse = serde_json::from_str(json).unwrap();
        assert!(resp.usage.is_none());
    }

    #[test]
    fn converse_response_deser_multiple_content_blocks() {
        let json = r#"{
            "output": {
                "message": {
                    "content": [{"text": "part1"}, {"text": "part2"}]
                }
            }
        }"#;
        let resp: ConverseResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.output.message.content.len(), 2);
        assert_eq!(resp.output.message.content[1].text.as_ref().unwrap(), "part2");
    }

    // ── converse response edge cases ──────────────────────────────────

    #[test]
    fn converse_response_deser_null_text_in_content_block() {
        let json = r#"{
            "output": {
                "message": {
                    "content": [{"text": null}]
                }
            }
        }"#;
        let resp: ConverseResponse = serde_json::from_str(json).unwrap();
        assert!(resp.output.message.content[0].text.is_none());
    }

    #[test]
    fn converse_response_deser_partial_usage() {
        let json = r#"{
            "output": {
                "message": {
                    "content": [{"text": "ok"}]
                }
            },
            "usage": {
                "inputTokens": 42
            }
        }"#;
        let resp: ConverseResponse = serde_json::from_str(json).unwrap();
        let usage = resp.usage.unwrap();
        assert_eq!(usage.input_tokens, Some(42));
        assert!(usage.output_tokens.is_none());
    }

    // ── converse request serialization ────────────────────────────────

    #[test]
    fn converse_request_skips_none_system() {
        let req = ConverseRequest {
            messages: vec![ConverseMessage {
                role: "user".into(),
                content: vec![ContentBlock { text: "hi".into() }],
            }],
            system: None,
            inference_config: None,
        };
        let val = serde_json::to_value(&req).unwrap();
        assert!(val.get("system").is_none());
        assert!(val.get("inferenceConfig").is_none());
    }

    #[test]
    fn inference_config_serialization() {
        let ic = InferenceConfig {
            max_tokens: Some(2048),
            temperature: Some(0.5),
        };
        let val = serde_json::to_value(&ic).unwrap();
        assert_eq!(val["maxTokens"], 2048);
        assert_eq!(val["temperature"], 0.5);
    }

    #[test]
    fn inference_config_skips_none_fields() {
        let ic = InferenceConfig {
            max_tokens: None,
            temperature: None,
        };
        let val = serde_json::to_value(&ic).unwrap();
        assert!(val.get("maxTokens").is_none());
        assert!(val.get("temperature").is_none());
    }

    // ── build_converse_request empty messages ─────────────────────────

    #[test]
    fn build_converse_request_empty_messages() {
        let provider = BedrockProvider::new(test_bedrock_config());
        let req = provider.build_converse_request(&[], None);
        assert!(req.messages.is_empty());
        assert!(req.system.is_none());
    }

    // ── sigv4 differences ─────────────────────────────────────────────

    #[test]
    fn sigv4_auth_header_differs_by_region() {
        let a = sigv4_auth_header("AK", "SK", "us-east-1", "h", "/p", b"{}", "20240101T120000Z");
        let b = sigv4_auth_header("AK", "SK", "eu-west-1", "h", "/p", b"{}", "20240101T120000Z");
        assert_ne!(a, b);
    }

    #[test]
    fn sigv4_auth_header_differs_by_datetime() {
        let a = sigv4_auth_header("AK", "SK", "us-east-1", "h", "/p", b"{}", "20240101T120000Z");
        let b = sigv4_auth_header("AK", "SK", "us-east-1", "h", "/p", b"{}", "20240102T120000Z");
        assert_ne!(a, b);
    }

    #[test]
    fn sigv4_auth_header_differs_by_access_key() {
        let a = sigv4_auth_header("AK1", "SK", "us-east-1", "h", "/p", b"{}", "20240101T120000Z");
        let b = sigv4_auth_header("AK2", "SK", "us-east-1", "h", "/p", b"{}", "20240101T120000Z");
        assert_ne!(a, b);
        assert!(a.contains("AK1"));
        assert!(b.contains("AK2"));
    }
}
