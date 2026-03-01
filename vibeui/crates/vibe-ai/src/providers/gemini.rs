//! Google Gemini provider implementation

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiConfig>,
}

#[derive(Debug, Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContentResponse,
}

#[derive(Debug, Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Debug, Deserialize)]
struct GeminiPartResponse {
    text: String,
}

/// Gemini provider
pub struct GeminiProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl GeminiProvider {
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

    fn build_contents(&self, messages: &[Message], context: Option<String>) -> Vec<GeminiContent> {
        let mut gemini_contents = Vec::new();

        for m in messages {
            let role = match m.role {
                crate::provider::MessageRole::User => "user",
                crate::provider::MessageRole::Assistant => "model",
                crate::provider::MessageRole::System => "user", // Gemini doesn't have system role in v1beta, usually mapped to user
            };
            
            gemini_contents.push(GeminiContent {
                role: role.to_string(),
                parts: vec![GeminiPart { text: m.content.clone() }],
            });
        }

        if let Some(ctx) = context {
            if let Some(last_msg) = gemini_contents.last_mut() {
                if last_msg.role == "user" {
                    last_msg.parts[0].text = format!("Context:\n{}\n\nUser: {}", ctx, last_msg.parts[0].text);
                }
            }
        }

        gemini_contents
    }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    fn name(&self) -> &str {
        "Gemini"
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
            Message { role: crate::provider::MessageRole::User, content: prompt },
        ];

        self.stream_chat(&messages).await
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let api_key = self.config.api_key.as_ref().context("Gemini API key not found")?;
        let request = GeminiRequest {
            contents: self.build_contents(messages, context),
            generation_config: Some(GeminiConfig {
                temperature: self.config.temperature,
                max_output_tokens: self.config.max_tokens,
            }),
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.config.model
        );

        let response = self.client
            .post(&url)
            .header("x-goog-api-key", api_key)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Gemini API error: {}", error_text);
        }

        let gemini_response: GeminiResponse = response.json().await.context("Failed to parse Gemini response")?;
        
        if let Some(candidates) = gemini_response.candidates {
            if let Some(candidate) = candidates.first() {
                if let Some(part) = candidate.content.parts.first() {
                    return Ok(part.text.clone());
                }
            }
        }
        
        anyhow::bail!("No content in Gemini response")
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Gemini API key not found")?;
        let request = GeminiRequest {
            contents: self.build_contents(messages, None),
            generation_config: Some(GeminiConfig {
                temperature: self.config.temperature,
                max_output_tokens: self.config.max_tokens,
            }),
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent",
            self.config.model
        );

        let response = self.client
            .post(&url)
            .header("x-goog-api-key", api_key)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Gemini API error: {}", error_text);
        }

        let stream = response.bytes_stream();
        
        let completion_stream = stream
            .map(|chunk| {
                let chunk = chunk?;
                // Gemini streamGenerateContent returns a JSON array of response objects.
                // Chunks may contain full or partial JSON. Try multiple parse strategies.

                // Strategy 1: parse as a single GeminiResponse object.
                if let Ok(response) = serde_json::from_slice::<GeminiResponse>(&chunk) {
                    if let Some(candidates) = response.candidates {
                        if let Some(candidate) = candidates.first() {
                            if let Some(part) = candidate.content.parts.first() {
                                return Ok(part.text.clone());
                            }
                        }
                    }
                }

                // Strategy 2: parse as a JSON array of GeminiResponse objects.
                if let Ok(responses) = serde_json::from_slice::<Vec<GeminiResponse>>(&chunk) {
                    let mut content = String::new();
                    for response in responses {
                        if let Some(candidates) = response.candidates {
                            if let Some(candidate) = candidates.first() {
                                if let Some(part) = candidate.content.parts.first() {
                                    content.push_str(&part.text);
                                }
                            }
                        }
                    }
                    if !content.is_empty() {
                        return Ok(content);
                    }
                }

                // Strategy 3: strip leading/trailing array punctuation and try
                // to extract individual JSON objects separated by commas.
                let text = String::from_utf8_lossy(&chunk);
                let trimmed = text.trim().trim_start_matches('[').trim_start_matches(',')
                    .trim_end_matches(']').trim_end_matches(',').trim();
                if !trimmed.is_empty() {
                    if let Ok(response) = serde_json::from_str::<GeminiResponse>(trimmed) {
                        if let Some(candidates) = response.candidates {
                            if let Some(candidate) = candidates.first() {
                                if let Some(part) = candidate.content.parts.first() {
                                    return Ok(part.text.clone());
                                }
                            }
                        }
                    }
                }

                // Chunk could not be parsed — return empty to keep the stream alive.
                Ok(String::new())
            })
            .boxed();

        Ok(completion_stream)
    }
}
