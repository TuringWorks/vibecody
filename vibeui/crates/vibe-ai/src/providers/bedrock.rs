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
}

impl BedrockProvider {
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
    fn name(&self) -> &str { "Bedrock" }

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
