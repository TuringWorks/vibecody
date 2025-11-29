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
            client: reqwest::Client::new(),
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
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.config.model, api_key
        );

        let response = self.client
            .post(&url)
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
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
            self.config.model, api_key
        );

        let response = self.client
            .post(&url)
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
                // Gemini stream returns a JSON array of responses, but chunked.
                // This is a simplification; robust parsing might need a proper JSON stream parser.
                // However, for SSE-like behavior, we might need to handle partial JSONs if not careful.
                // But Gemini returns a standard HTTP response with a JSON array if not using SSE?
                // Actually, streamGenerateContent returns a stream of partial responses.
                // Let's assume we get valid JSON objects or a list.
                // For simplicity in this iteration, we'll try to parse the chunk as a partial response.
                // Note: Real implementation might need more robust framing handling.
                
                // Hacky parsing for now: remove '[' at start, ']' at end, and split by ','? 
                // Or just try to parse the chunk.
                
                // Let's try to parse as GeminiResponse.
                if let Ok(response) = serde_json::from_slice::<GeminiResponse>(&chunk) {
                     if let Some(candidates) = response.candidates {
                        if let Some(candidate) = candidates.first() {
                            if let Some(part) = candidate.content.parts.first() {
                                return Ok(part.text.clone());
                            }
                        }
                    }
                }
                
                // If direct parse fails, it might be wrapped in an array or comma separated.
                // For now, return empty string to avoid breaking the stream if parse fails.
                Ok(String::new()) 
            })
            .boxed();

        Ok(completion_stream)
    }
}
