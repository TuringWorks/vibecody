//! Google Gemini provider implementation
//!
//! Native provider for Google Gemini 2.5 Pro/Flash models via the
//! Generative Language API (`generativelanguage.googleapis.com`).

use crate::provider::{
    AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::fmt;

// ─── Model catalogue ────────────────────────────────────────────────────────

/// Supported Gemini model variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeminiModel {
    Gemini25Pro,
    Gemini25Flash,
    Gemini20Flash,
    Gemini20FlashLite,
}

impl GeminiModel {
    /// API model identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Gemini25Pro => "gemini-2.5-pro",
            Self::Gemini25Flash => "gemini-2.5-flash",
            Self::Gemini20Flash => "gemini-2.0-flash",
            Self::Gemini20FlashLite => "gemini-2.0-flash-lite",
        }
    }

    /// Context window size in tokens.
    pub fn context_window(&self) -> usize {
        match self {
            Self::Gemini25Pro => 1_048_576,
            Self::Gemini25Flash => 1_048_576,
            Self::Gemini20Flash => 1_048_576,
            Self::Gemini20FlashLite => 1_048_576,
        }
    }

    /// Maximum output tokens.
    pub fn max_output(&self) -> usize {
        match self {
            Self::Gemini25Pro => 65_536,
            Self::Gemini25Flash => 65_536,
            Self::Gemini20Flash => 8_192,
            Self::Gemini20FlashLite => 8_192,
        }
    }
}

impl fmt::Display for GeminiModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ─── Safety types ───────────────────────────────────────────────────────────

/// Harm category for safety settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HarmCategory {
    #[serde(rename = "HARM_CATEGORY_HATE_SPEECH")]
    HateSpeech,
    #[serde(rename = "HARM_CATEGORY_HARASSMENT")]
    Harassment,
    #[serde(rename = "HARM_CATEGORY_SEXUALLY_EXPLICIT")]
    SexuallyExplicit,
    #[serde(rename = "HARM_CATEGORY_DANGEROUS_CONTENT")]
    DangerousContent,
}

impl fmt::Display for HarmCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HateSpeech => write!(f, "HARM_CATEGORY_HATE_SPEECH"),
            Self::Harassment => write!(f, "HARM_CATEGORY_HARASSMENT"),
            Self::SexuallyExplicit => write!(f, "HARM_CATEGORY_SEXUALLY_EXPLICIT"),
            Self::DangerousContent => write!(f, "HARM_CATEGORY_DANGEROUS_CONTENT"),
        }
    }
}

/// Threshold for blocking harmful content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarmBlockThreshold {
    BlockNone,
    BlockLowAndAbove,
    BlockMediumAndAbove,
    BlockHighAndAbove,
}

impl fmt::Display for HarmBlockThreshold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BlockNone => write!(f, "BLOCK_NONE"),
            Self::BlockLowAndAbove => write!(f, "BLOCK_LOW_AND_ABOVE"),
            Self::BlockMediumAndAbove => write!(f, "BLOCK_MEDIUM_AND_ABOVE"),
            Self::BlockHighAndAbove => write!(f, "BLOCK_HIGH_AND_ABOVE"),
        }
    }
}

/// A single safety setting pairing category with threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetySetting {
    pub category: HarmCategory,
    pub threshold: HarmBlockThreshold,
}

// ─── Request / response types ───────────────────────────────────────────────

/// Top-level request body for `generateContent` / `streamGenerateContent`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiRequest {
    pub model: String,
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<Content>,
    pub generation_config: GenerationConfig,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub safety_settings: Vec<SafetySetting>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDeclaration>>,
}

/// A single content message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
}

/// A content part — text, inline data, function call, or function response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Part {
    Text {
        text: String,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: InlineDataPayload,
    },
    FunctionCall {
        #[serde(rename = "functionCall")]
        function_call: FunctionCallPayload,
    },
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: FunctionResponsePayload,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineDataPayload {
    pub mime_type: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallPayload {
    pub name: String,
    pub args: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionResponsePayload {
    pub name: String,
    pub response: String,
}

/// Generation parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// Tool declaration for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDeclaration {
    pub function_declarations: Vec<FunctionDeclaration>,
}

/// A single function declaration inside a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<String>,
}

// ─── Response types ─────────────────────────────────────────────────────────

/// Top-level response from `generateContent`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiResponse {
    pub candidates: Option<Vec<Candidate>>,
    pub usage_metadata: Option<UsageMetadata>,
}

/// A single candidate in the response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub content: CandidateContent,
    pub finish_reason: Option<String>,
    #[serde(default)]
    pub safety_ratings: Vec<SafetyRating>,
}

#[derive(Debug, Deserialize)]
pub struct CandidateContent {
    pub parts: Vec<CandidateTextPart>,
}

#[derive(Debug, Deserialize)]
pub struct CandidateTextPart {
    pub text: String,
}

/// Safety rating on a response candidate.
#[derive(Debug, Deserialize)]
pub struct SafetyRating {
    pub category: String,
    pub probability: String,
    #[serde(default)]
    pub blocked: bool,
}

/// Token usage metadata.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetadata {
    pub prompt_token_count: u32,
    pub candidates_token_count: u32,
    pub total_token_count: u32,
}

// ─── Error type ─────────────────────────────────────────────────────────────

/// Provider-specific errors.
#[derive(Debug)]
pub enum GeminiError {
    ApiKeyMissing,
    RequestFailed(String),
    ResponseParseError,
    SafetyBlocked(String),
    QuotaExceeded,
    ModelNotFound,
    InvalidConfig,
}

impl fmt::Display for GeminiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApiKeyMissing => write!(f, "Gemini API key is missing"),
            Self::RequestFailed(msg) => write!(f, "Gemini request failed: {}", msg),
            Self::ResponseParseError => write!(f, "Failed to parse Gemini response"),
            Self::SafetyBlocked(reason) => write!(f, "Gemini response blocked by safety filter: {}", reason),
            Self::QuotaExceeded => write!(f, "Gemini API quota exceeded"),
            Self::ModelNotFound => write!(f, "Gemini model not found"),
            Self::InvalidConfig => write!(f, "Invalid Gemini provider configuration"),
        }
    }
}

impl std::error::Error for GeminiError {}

// ─── Config ─────────────────────────────────────────────────────────────────

/// Extended configuration specific to the Gemini provider.
#[derive(Debug, Clone)]
pub struct GeminiConfig {
    pub api_key: String,
    pub api_url: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub safety_settings: Vec<SafetySetting>,
    pub system_instruction: Option<String>,
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            model: "gemini-2.5-pro".to_string(),
            temperature: 0.7,
            max_tokens: 8192,
            top_p: None,
            top_k: None,
            safety_settings: GeminiProvider::default_safety_settings(),
            system_instruction: None,
        }
    }
}

// ─── Provider ───────────────────────────────────────────────────────────────

/// Google Gemini provider implementing the `AIProvider` trait.
pub struct GeminiProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    display_name: String,
}

impl GeminiProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let display_name = format!("Gemini ({})", config.model);
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

    /// Build the API endpoint URL.
    pub fn build_url(&self, model: &str, stream: bool) -> String {
        let base = self
            .config
            .api_url
            .as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com/v1beta");
        let action = if stream {
            "streamGenerateContent"
        } else {
            "generateContent"
        };
        format!("{}/models/{}:{}", base, model, action)
    }

    /// Construct a `GeminiRequest` from message contents and optional system instruction.
    pub fn build_request(
        &self,
        messages: &[Content],
        system: Option<&str>,
    ) -> GeminiRequest {
        let system_instruction = system.map(|s| Content {
            role: "user".to_string(),
            parts: vec![Part::Text {
                text: s.to_string(),
            }],
        });

        GeminiRequest {
            model: self.config.model.clone(),
            contents: messages.to_vec(),
            system_instruction,
            generation_config: GenerationConfig {
                temperature: self.config.temperature,
                top_p: None,
                top_k: None,
                max_output_tokens: self.config.max_tokens.map(|t| t as u32),
                candidate_count: None,
                stop_sequences: None,
            },
            safety_settings: Self::default_safety_settings(),
            tools: None,
        }
    }

    /// Extract text from a successful JSON response body.
    pub fn parse_response(json: &str) -> Result<String, GeminiError> {
        let response: GeminiResponse =
            serde_json::from_str(json).map_err(|_| GeminiError::ResponseParseError)?;

        // Check for safety blocks first.
        if let Some(candidates) = &response.candidates {
            if let Some(candidate) = candidates.first() {
                for rating in &candidate.safety_ratings {
                    if rating.blocked {
                        return Err(GeminiError::SafetyBlocked(rating.category.clone()));
                    }
                }
            }
        }

        if let Some(candidates) = response.candidates {
            if let Some(candidate) = candidates.first() {
                if let Some(part) = candidate.content.parts.first() {
                    return Ok(part.text.clone());
                }
            }
        }

        Err(GeminiError::ResponseParseError)
    }

    /// Parse a single SSE-style streaming chunk, returning extracted text if any.
    pub fn parse_streaming_chunk(line: &str) -> Option<String> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "[" || trimmed == "]" || trimmed == "," {
            return None;
        }
        let cleaned = trimmed
            .trim_start_matches('[')
            .trim_start_matches(',')
            .trim_end_matches(']')
            .trim_end_matches(',')
            .trim();
        if cleaned.is_empty() {
            return None;
        }
        let response: GeminiResponse = serde_json::from_str(cleaned).ok()?;
        let candidates = response.candidates?;
        let candidate = candidates.first()?;
        let part = candidate.content.parts.first()?;
        Some(part.text.clone())
    }

    /// Create a `Part::FunctionCall` convenience helper.
    pub fn format_tool_call(name: &str, args: &str) -> Part {
        Part::FunctionCall {
            function_call: FunctionCallPayload {
                name: name.to_string(),
                args: args.to_string(),
            },
        }
    }

    /// Create a `Part::FunctionResponse` convenience helper.
    pub fn format_tool_response(name: &str, response: &str) -> Part {
        Part::FunctionResponse {
            function_response: FunctionResponsePayload {
                name: name.to_string(),
                response: response.to_string(),
            },
        }
    }

    /// Rough token estimation (~4 characters per token).
    pub fn estimate_tokens(text: &str) -> u32 {
        (text.len() as f64 / 4.0).ceil() as u32
    }

    /// Return `(context_window, max_output)` for a model variant.
    pub fn get_model_info(model: &GeminiModel) -> (usize, usize) {
        (model.context_window(), model.max_output())
    }

    /// Build a curl command string for debugging a request.
    pub fn build_curl_command(&self, request: &GeminiRequest) -> String {
        let url = self.build_url(&request.model, false);
        let api_key = self.config.api_key.as_deref().unwrap_or("YOUR_API_KEY");
        let body = serde_json::to_string_pretty(request).unwrap_or_default();
        format!(
            "curl -X POST '{}' \\\n  -H 'Content-Type: application/json' \\\n  -H 'x-goog-api-key: {}' \\\n  -d '{}'",
            url, api_key, body
        )
    }

    /// Default safety settings (medium threshold for all categories).
    pub fn default_safety_settings() -> Vec<SafetySetting> {
        vec![
            SafetySetting {
                category: HarmCategory::HateSpeech,
                threshold: HarmBlockThreshold::BlockMediumAndAbove,
            },
            SafetySetting {
                category: HarmCategory::Harassment,
                threshold: HarmBlockThreshold::BlockMediumAndAbove,
            },
            SafetySetting {
                category: HarmCategory::SexuallyExplicit,
                threshold: HarmBlockThreshold::BlockMediumAndAbove,
            },
            SafetySetting {
                category: HarmCategory::DangerousContent,
                threshold: HarmBlockThreshold::BlockMediumAndAbove,
            },
        ]
    }

    /// Validate provider configuration, returning an error if the config is invalid.
    pub fn validate_config(&self) -> Result<(), GeminiError> {
        if self.config.api_key.is_none() || self.config.api_key.as_deref() == Some("") {
            return Err(GeminiError::ApiKeyMissing);
        }
        if self.config.model.is_empty() {
            return Err(GeminiError::InvalidConfig);
        }
        Ok(())
    }

    // ── internal helpers ────────────────────────────────────────────────

    fn build_contents(&self, messages: &[Message], context: Option<String>) -> Vec<Content> {
        let mut gemini_contents = Vec::new();

        for m in messages {
            let role = match m.role {
                crate::provider::MessageRole::User => "user",
                crate::provider::MessageRole::Assistant => "model",
                crate::provider::MessageRole::System => "user",
            };

            gemini_contents.push(Content {
                role: role.to_string(),
                parts: vec![Part::Text {
                    text: m.content.clone(),
                }],
            });
        }

        if let Some(ctx) = context {
            if let Some(last_msg) = gemini_contents.last_mut() {
                if last_msg.role == "user" {
                    if let Some(Part::Text { text }) = last_msg.parts.first_mut() {
                        *text = format!("Context:\n{}\n\nUser: {}", ctx, text);
                    }
                }
            }
        }

        gemini_contents
    }
}

// ─── AIProvider impl ────────────────────────────────────────────────────────

#[async_trait]
impl AIProvider for GeminiProvider {
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

        let messages = vec![Message {
            role: crate::provider::MessageRole::User,
            content: prompt,
        }];

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

        let messages = vec![Message {
            role: crate::provider::MessageRole::User,
            content: prompt,
        }];

        self.stream_chat(&messages).await
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("Gemini API key not found")?;
        let contents = self.build_contents(messages, context);
        let request = self.build_request(&contents, None);
        let url = self.build_url(&self.config.model, false);

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", api_key)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini")?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await?;
            match status {
                429 => anyhow::bail!("Gemini API quota exceeded: {}", error_text),
                404 => anyhow::bail!("Gemini model not found: {}", error_text),
                _ => anyhow::bail!("Gemini API error ({}): {}", status, error_text),
            }
        }

        let body = response
            .text()
            .await
            .context("Failed to read Gemini response body")?;

        Self::parse_response(&body).map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("Gemini API key not found")?;
        let contents = self.build_contents(messages, None);
        let request = self.build_request(&contents, None);
        let url = self.build_url(&self.config.model, true);

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", api_key)
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to Gemini")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Gemini API error: {}", error_text);
        }

        // Gemini streamGenerateContent returns a JSON array: [{...},{...},...]
        // Raw bytes_stream() chunks can split mid-JSON object, causing parse
        // failures ("error decoding response body").  We buffer the incoming
        // bytes and extract complete JSON objects delimited by balanced braces
        // at the top-level array depth.
        let byte_stream = response.bytes_stream();

        let completion_stream = futures::stream::unfold(
            (byte_stream.boxed(), String::new()),
            |(mut stream, mut buf)| async move {
                loop {
                    // Try to extract a complete JSON object from the buffer.
                    if let Some((json_str, rest)) = extract_json_object(&buf) {
                        buf = rest;
                        if let Ok(resp) = serde_json::from_str::<GeminiResponse>(&json_str) {
                            if let Some(candidates) = resp.candidates {
                                if let Some(candidate) = candidates.first() {
                                    if let Some(part) = candidate.content.parts.first() {
                                        if !part.text.is_empty() {
                                            return Some((Ok(part.text.clone()), (stream, buf)));
                                        }
                                    }
                                }
                            }
                        }
                        // Parsed but no text (e.g. safety-only block) — continue
                        continue;
                    }

                    // Need more data from the network.
                    match stream.next().await {
                        Some(Ok(bytes)) => {
                            buf.push_str(&String::from_utf8_lossy(&bytes));
                        }
                        Some(Err(e)) => {
                            return Some((Err(anyhow::anyhow!("{}", e)), (stream, buf)));
                        }
                        None => {
                            // Stream ended — try to parse any remaining buffer.
                            let trimmed = buf.trim()
                                .trim_start_matches('[')
                                .trim_end_matches(']')
                                .trim_start_matches(',')
                                .trim();
                            if !trimmed.is_empty() {
                                if let Ok(resp) = serde_json::from_str::<GeminiResponse>(trimmed) {
                                    buf.clear();
                                    if let Some(candidates) = resp.candidates {
                                        if let Some(candidate) = candidates.first() {
                                            if let Some(part) = candidate.content.parts.first() {
                                                if !part.text.is_empty() {
                                                    return Some((Ok(part.text.clone()), (stream, buf)));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            return None; // truly done
                        }
                    }
                }
            },
        )
        .boxed();

        Ok(completion_stream)
    }

    fn supports_vision(&self) -> bool {
        // All Gemini 2.x models support multimodal (vision) input
        true
    }

    async fn chat_with_images(
        &self,
        messages: &[Message],
        images: &[crate::provider::ImageAttachment],
        context: Option<String>,
    ) -> Result<String> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("Gemini API key not found")?;

        // Build contents with images attached to the last user message
        let mut contents = self.build_contents(messages, context);

        // Attach images as inlineData parts to the last user content
        if let Some(last_user) = contents.iter_mut().rev().find(|c| c.role == "user") {
            for img in images {
                last_user.parts.push(Part::InlineData {
                    inline_data: InlineDataPayload {
                        mime_type: img.media_type.clone(),
                        data: img.base64.clone(),
                    },
                });
            }
        }

        let request = self.build_request(&contents, None);
        let url = self.build_url(&self.config.model, false);

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", api_key)
            .json(&request)
            .send()
            .await
            .context("Failed to send vision request to Gemini")?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await?;
            anyhow::bail!("Gemini vision API error ({}): {}", status, error_text);
        }

        let body = response
            .text()
            .await
            .context("Failed to read Gemini vision response body")?;

        Self::parse_response(&body).map_err(|e| anyhow::anyhow!("{}", e))
    }
}

// ─── Streaming helpers ─────────────────────────────────────────────────────

/// Extract the first complete JSON object from a buffer that may contain
/// fragments of the Gemini `[{...},{...},...]` streaming array.
///
/// Returns `Some((json_str, remaining))` when a balanced `{…}` is found,
/// skipping leading `[`, `,`, and whitespace.  Handles nested braces and
/// strings (including escaped quotes) correctly.
fn extract_json_object(buf: &str) -> Option<(String, String)> {
    // Skip leading array/separator chars
    let trimmed = buf.trim_start_matches(|c: char| c == '[' || c == ',' || c.is_whitespace());
    if !trimmed.starts_with('{') {
        return None;
    }

    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape_next = false;
    let mut end_idx = None;

    for (i, ch) in trimmed.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        if ch == '\\' && in_string {
            escape_next = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                end_idx = Some(i + ch.len_utf8());
                break;
            }
        }
    }

    let end = end_idx?;
    let json = trimmed[..end].to_string();
    // Calculate byte offset into original buf
    let skip = buf.len() - trimmed.len();
    let remaining = buf[skip + end..].to_string();
    Some((json, remaining))
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "gemini".into(),
            api_key: Some("AIza-test-key".into()),
            api_url: None,
            model: "gemini-2.5-pro".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    // ── config defaults ─────────────────────────────────────────────────

    #[test]
    fn gemini_config_defaults() {
        let cfg = GeminiConfig::default();
        assert_eq!(cfg.model, "gemini-2.5-pro");
        assert_eq!(
            cfg.api_url,
            "https://generativelanguage.googleapis.com/v1beta"
        );
        assert!((cfg.temperature - 0.7).abs() < f32::EPSILON);
        assert_eq!(cfg.max_tokens, 8192);
        assert!(cfg.top_p.is_none());
        assert!(cfg.top_k.is_none());
        assert_eq!(cfg.safety_settings.len(), 4);
        assert!(cfg.system_instruction.is_none());
    }

    // ── model info ──────────────────────────────────────────────────────

    #[test]
    fn model_as_str_gemini25_pro() {
        assert_eq!(GeminiModel::Gemini25Pro.as_str(), "gemini-2.5-pro");
    }

    #[test]
    fn model_as_str_gemini25_flash() {
        assert_eq!(GeminiModel::Gemini25Flash.as_str(), "gemini-2.5-flash");
    }

    #[test]
    fn model_as_str_gemini20_flash() {
        assert_eq!(GeminiModel::Gemini20Flash.as_str(), "gemini-2.0-flash");
    }

    #[test]
    fn model_as_str_gemini20_flash_lite() {
        assert_eq!(
            GeminiModel::Gemini20FlashLite.as_str(),
            "gemini-2.0-flash-lite"
        );
    }

    #[test]
    fn model_context_window_25pro() {
        assert_eq!(GeminiModel::Gemini25Pro.context_window(), 1_048_576);
    }

    #[test]
    fn model_max_output_25pro() {
        assert_eq!(GeminiModel::Gemini25Pro.max_output(), 65_536);
    }

    #[test]
    fn model_max_output_20flash() {
        assert_eq!(GeminiModel::Gemini20Flash.max_output(), 8_192);
    }

    #[test]
    fn get_model_info_returns_tuple() {
        let (ctx, out) = GeminiProvider::get_model_info(&GeminiModel::Gemini25Flash);
        assert_eq!(ctx, 1_048_576);
        assert_eq!(out, 65_536);
    }

    #[test]
    fn model_display_trait() {
        assert_eq!(format!("{}", GeminiModel::Gemini25Pro), "gemini-2.5-pro");
    }

    // ── URL building ────────────────────────────────────────────────────

    #[test]
    fn build_url_standard() {
        let p = GeminiProvider::new(test_config());
        let url = p.build_url("gemini-2.5-pro", false);
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-pro:generateContent"
        );
    }

    #[test]
    fn build_url_streaming() {
        let p = GeminiProvider::new(test_config());
        let url = p.build_url("gemini-2.5-pro", true);
        assert!(url.ends_with(":streamGenerateContent"));
    }

    #[test]
    fn build_url_custom_base() {
        let mut cfg = test_config();
        cfg.api_url = Some("https://custom.api.example.com/v1".into());
        let p = GeminiProvider::new(cfg);
        let url = p.build_url("gemini-2.0-flash", false);
        assert!(url.starts_with("https://custom.api.example.com/v1/models/"));
    }

    // ── request building ────────────────────────────────────────────────

    #[test]
    fn build_request_basic() {
        let p = GeminiProvider::new(test_config());
        let contents = vec![Content {
            role: "user".into(),
            parts: vec![Part::Text {
                text: "hello".into(),
            }],
        }];
        let req = p.build_request(&contents, None);
        assert_eq!(req.model, "gemini-2.5-pro");
        assert_eq!(req.contents.len(), 1);
        assert!(req.system_instruction.is_none());
        assert!(req.tools.is_none());
    }

    #[test]
    fn build_request_with_system_instruction() {
        let p = GeminiProvider::new(test_config());
        let contents = vec![Content {
            role: "user".into(),
            parts: vec![Part::Text {
                text: "hi".into(),
            }],
        }];
        let req = p.build_request(&contents, Some("You are a coding assistant"));
        assert!(req.system_instruction.is_some());
        let si = req.system_instruction.unwrap();
        match &si.parts[0] {
            Part::Text { text } => assert_eq!(text, "You are a coding assistant"),
            _ => panic!("Expected text part"),
        }
    }

    #[test]
    fn build_request_includes_safety_settings() {
        let p = GeminiProvider::new(test_config());
        let req = p.build_request(&[], None);
        assert_eq!(req.safety_settings.len(), 4);
    }

    // ── response parsing ────────────────────────────────────────────────

    #[test]
    fn parse_response_success() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"Hello!"}]},"finishReason":"STOP","safetyRatings":[]}]}"#;
        let result = GeminiProvider::parse_response(json);
        assert_eq!(result.unwrap(), "Hello!");
    }

    #[test]
    fn parse_response_error_invalid_json() {
        let result = GeminiProvider::parse_response("not json");
        assert!(matches!(result, Err(GeminiError::ResponseParseError)));
    }

    #[test]
    fn parse_response_empty_candidates() {
        let json = r#"{"candidates":[]}"#;
        let result = GeminiProvider::parse_response(json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_response_no_candidates_field() {
        let json = r#"{}"#;
        let result = GeminiProvider::parse_response(json);
        assert!(result.is_err());
    }

    #[test]
    fn parse_response_safety_blocked() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":""}]},"safetyRatings":[{"category":"HATE_SPEECH","probability":"HIGH","blocked":true}]}]}"#;
        let result = GeminiProvider::parse_response(json);
        assert!(matches!(result, Err(GeminiError::SafetyBlocked(_))));
    }

    #[test]
    fn parse_response_with_usage_metadata() {
        let json = r#"{"candidates":[{"content":{"parts":[{"text":"ok"}]},"safetyRatings":[]}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":5,"totalTokenCount":15}}"#;
        let result = GeminiProvider::parse_response(json);
        assert_eq!(result.unwrap(), "ok");
    }

    // ── streaming chunk parsing ─────────────────────────────────────────

    #[test]
    fn parse_streaming_chunk_valid() {
        let chunk = r#"{"candidates":[{"content":{"parts":[{"text":"stream part"}]},"safetyRatings":[]}]}"#;
        let result = GeminiProvider::parse_streaming_chunk(chunk);
        assert_eq!(result.unwrap(), "stream part");
    }

    #[test]
    fn parse_streaming_chunk_empty_line() {
        assert!(GeminiProvider::parse_streaming_chunk("").is_none());
    }

    #[test]
    fn parse_streaming_chunk_bracket() {
        assert!(GeminiProvider::parse_streaming_chunk("[").is_none());
        assert!(GeminiProvider::parse_streaming_chunk("]").is_none());
    }

    #[test]
    fn parse_streaming_chunk_comma_prefix() {
        let chunk = r#",{"candidates":[{"content":{"parts":[{"text":"chunk"}]},"safetyRatings":[]}]}"#;
        let result = GeminiProvider::parse_streaming_chunk(chunk);
        assert_eq!(result.unwrap(), "chunk");
    }

    // ── tool call formatting ────────────────────────────────────────────

    #[test]
    fn format_tool_call_creates_function_call_part() {
        let part = GeminiProvider::format_tool_call("get_weather", r#"{"city":"SF"}"#);
        match part {
            Part::FunctionCall { function_call } => {
                assert_eq!(function_call.name, "get_weather");
                assert_eq!(function_call.args, r#"{"city":"SF"}"#);
            }
            _ => panic!("Expected FunctionCall part"),
        }
    }

    #[test]
    fn format_tool_response_creates_function_response_part() {
        let part = GeminiProvider::format_tool_response("get_weather", r#"{"temp":72}"#);
        match part {
            Part::FunctionResponse { function_response } => {
                assert_eq!(function_response.name, "get_weather");
                assert_eq!(function_response.response, r#"{"temp":72}"#);
            }
            _ => panic!("Expected FunctionResponse part"),
        }
    }

    // ── token estimation ────────────────────────────────────────────────

    #[test]
    fn estimate_tokens_basic() {
        // 12 chars -> ceil(12/4) = 3
        assert_eq!(GeminiProvider::estimate_tokens("hello world!"), 3);
    }

    #[test]
    fn estimate_tokens_empty() {
        assert_eq!(GeminiProvider::estimate_tokens(""), 0);
    }

    #[test]
    fn estimate_tokens_single_char() {
        assert_eq!(GeminiProvider::estimate_tokens("a"), 1);
    }

    // ── safety settings ─────────────────────────────────────────────────

    #[test]
    fn default_safety_settings_four_categories() {
        let settings = GeminiProvider::default_safety_settings();
        assert_eq!(settings.len(), 4);
        assert_eq!(settings[0].category, HarmCategory::HateSpeech);
        assert_eq!(settings[1].category, HarmCategory::Harassment);
        assert_eq!(settings[2].category, HarmCategory::SexuallyExplicit);
        assert_eq!(settings[3].category, HarmCategory::DangerousContent);
        for s in &settings {
            assert_eq!(s.threshold, HarmBlockThreshold::BlockMediumAndAbove);
        }
    }

    #[test]
    fn safety_setting_serialization() {
        let setting = SafetySetting {
            category: HarmCategory::HateSpeech,
            threshold: HarmBlockThreshold::BlockNone,
        };
        let json = serde_json::to_value(&setting).unwrap();
        assert!(json["category"].as_str().is_some());
        assert!(json["threshold"].as_str().is_some());
    }

    // ── curl command generation ─────────────────────────────────────────

    #[test]
    fn build_curl_command_contains_url_and_key() {
        let p = GeminiProvider::new(test_config());
        let req = p.build_request(&[], None);
        let curl = p.build_curl_command(&req);
        assert!(curl.contains("curl -X POST"));
        assert!(curl.contains("generativelanguage.googleapis.com"));
        assert!(curl.contains("AIza-test-key"));
        assert!(curl.contains("x-goog-api-key"));
    }

    // ── config validation ───────────────────────────────────────────────

    #[test]
    fn validate_config_valid() {
        let p = GeminiProvider::new(test_config());
        assert!(p.validate_config().is_ok());
    }

    #[test]
    fn validate_config_missing_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = GeminiProvider::new(cfg);
        assert!(matches!(
            p.validate_config(),
            Err(GeminiError::ApiKeyMissing)
        ));
    }

    #[test]
    fn validate_config_empty_key() {
        let mut cfg = test_config();
        cfg.api_key = Some("".into());
        let p = GeminiProvider::new(cfg);
        assert!(matches!(
            p.validate_config(),
            Err(GeminiError::ApiKeyMissing)
        ));
    }

    #[test]
    fn validate_config_empty_model() {
        let mut cfg = test_config();
        cfg.model = "".into();
        let p = GeminiProvider::new(cfg);
        assert!(matches!(
            p.validate_config(),
            Err(GeminiError::InvalidConfig)
        ));
    }

    // ── error display ───────────────────────────────────────────────────

    #[test]
    fn error_display_messages() {
        assert_eq!(
            format!("{}", GeminiError::ApiKeyMissing),
            "Gemini API key is missing"
        );
        assert!(format!("{}", GeminiError::RequestFailed("timeout".into())).contains("timeout"));
        assert!(format!("{}", GeminiError::SafetyBlocked("HATE".into())).contains("HATE"));
        assert_eq!(
            format!("{}", GeminiError::QuotaExceeded),
            "Gemini API quota exceeded"
        );
        assert_eq!(
            format!("{}", GeminiError::ModelNotFound),
            "Gemini model not found"
        );
        assert!(format!("{}", GeminiError::ResponseParseError).contains("parse"));
        assert!(format!("{}", GeminiError::InvalidConfig).contains("Invalid"));
    }

    // ── AIProvider trait ────────────────────────────────────────────────

    #[test]
    fn name_is_gemini() {
        let p = GeminiProvider::new(test_config());
        assert!(p.name().starts_with("Gemini ("));
        assert!(p.name().contains("gemini-2.5-pro"));
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = GeminiProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = GeminiProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    // ── build_contents (internal) ───────────────────────────────────────

    #[test]
    fn build_contents_maps_roles_correctly() {
        use crate::provider::MessageRole;
        let p = GeminiProvider::new(test_config());
        let messages = vec![
            Message {
                role: MessageRole::User,
                content: "hi".into(),
            },
            Message {
                role: MessageRole::Assistant,
                content: "hello".into(),
            },
            Message {
                role: MessageRole::System,
                content: "sys".into(),
            },
        ];
        let contents = p.build_contents(&messages, None);
        assert_eq!(contents[0].role, "user");
        assert_eq!(contents[1].role, "model");
        assert_eq!(contents[2].role, "user"); // system -> user
    }

    #[test]
    fn build_contents_appends_context() {
        use crate::provider::MessageRole;
        let p = GeminiProvider::new(test_config());
        let messages = vec![Message {
            role: MessageRole::User,
            content: "question".into(),
        }];
        let contents = p.build_contents(&messages, Some("ctx data".into()));
        match &contents[0].parts[0] {
            Part::Text { text } => {
                assert!(text.contains("Context:"));
                assert!(text.contains("ctx data"));
                assert!(text.contains("question"));
            }
            _ => panic!("Expected text part"),
        }
    }

    #[test]
    fn build_contents_empty_messages() {
        let p = GeminiProvider::new(test_config());
        let result = p.build_contents(&[], None);
        assert!(result.is_empty());
    }

    // ── request serialization ───────────────────────────────────────────

    #[test]
    fn gemini_request_serde_roundtrip() {
        let p = GeminiProvider::new(test_config());
        let contents = vec![Content {
            role: "user".into(),
            parts: vec![Part::Text {
                text: "test".into(),
            }],
        }];
        let req = p.build_request(&contents, None);
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "gemini-2.5-pro");
        assert!(json.get("generationConfig").is_some());
    }

    #[test]
    fn generation_config_omits_none_fields() {
        let cfg = GenerationConfig {
            temperature: None,
            top_p: None,
            top_k: None,
            max_output_tokens: None,
            candidate_count: None,
            stop_sequences: None,
        };
        let json = serde_json::to_value(&cfg).unwrap();
        assert!(json.get("temperature").is_none());
        assert!(json.get("topP").is_none());
        assert!(json.get("topK").is_none());
        assert!(json.get("maxOutputTokens").is_none());
    }

    #[test]
    fn provider_preserves_model_config() {
        let mut cfg = test_config();
        cfg.model = "gemini-2.0-flash".into();
        cfg.temperature = Some(0.2);
        cfg.max_tokens = Some(1024);
        let p = GeminiProvider::new(cfg);
        assert_eq!(p.config.model, "gemini-2.0-flash");
        assert_eq!(p.config.temperature, Some(0.2));
        assert_eq!(p.config.max_tokens, Some(1024));
    }

    // ── harm category & threshold display ───────────────────────────────

    #[test]
    fn harm_category_display() {
        assert_eq!(format!("{}", HarmCategory::HateSpeech), "HARM_CATEGORY_HATE_SPEECH");
        assert_eq!(format!("{}", HarmCategory::Harassment), "HARM_CATEGORY_HARASSMENT");
        assert_eq!(
            format!("{}", HarmCategory::SexuallyExplicit),
            "HARM_CATEGORY_SEXUALLY_EXPLICIT"
        );
        assert_eq!(
            format!("{}", HarmCategory::DangerousContent),
            "HARM_CATEGORY_DANGEROUS_CONTENT"
        );
    }

    #[test]
    fn harm_block_threshold_display() {
        assert_eq!(
            format!("{}", HarmBlockThreshold::BlockNone),
            "BLOCK_NONE"
        );
        assert_eq!(
            format!("{}", HarmBlockThreshold::BlockLowAndAbove),
            "BLOCK_LOW_AND_ABOVE"
        );
        assert_eq!(
            format!("{}", HarmBlockThreshold::BlockMediumAndAbove),
            "BLOCK_MEDIUM_AND_ABOVE"
        );
        assert_eq!(
            format!("{}", HarmBlockThreshold::BlockHighAndAbove),
            "BLOCK_HIGH_AND_ABOVE"
        );
    }

    // ── extract_json_object tests ──────────────────────────────────────

    #[test]
    fn extract_single_object() {
        let buf = r#"[{"candidates":[{"content":{"parts":[{"text":"hi"}]}}]}]"#;
        let (json, rest) = extract_json_object(buf).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
        let _: GeminiResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(rest.trim(), "]");
    }

    #[test]
    fn extract_from_split_chunks() {
        // Simulate two objects in the array
        let buf = r#"[{"candidates":[{"content":{"parts":[{"text":"hello"}]}}]},{"candidates":[{"content":{"parts":[{"text":" world"}]}}]}]"#;
        let (json1, rest1) = extract_json_object(buf).unwrap();
        let r1: GeminiResponse = serde_json::from_str(&json1).unwrap();
        assert_eq!(r1.candidates.unwrap()[0].content.parts[0].text, "hello");

        let (json2, rest2) = extract_json_object(&rest1).unwrap();
        let r2: GeminiResponse = serde_json::from_str(&json2).unwrap();
        assert_eq!(r2.candidates.unwrap()[0].content.parts[0].text, " world");
        assert_eq!(rest2.trim(), "]");
    }

    #[test]
    fn extract_handles_incomplete() {
        let buf = r#"[{"candidates":[{"content":{"par"#;
        assert!(extract_json_object(buf).is_none());
    }

    #[test]
    fn extract_handles_escaped_braces_in_strings() {
        let buf = r#"[{"candidates":[{"content":{"parts":[{"text":"code: { x }"}]}}]}]"#;
        let (json, _) = extract_json_object(buf).unwrap();
        let r: GeminiResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(r.candidates.unwrap()[0].content.parts[0].text, "code: { x }");
    }

    #[test]
    fn extract_empty_buffer() {
        assert!(extract_json_object("").is_none());
        assert!(extract_json_object("[").is_none());
        assert!(extract_json_object("[,").is_none());
    }
}
