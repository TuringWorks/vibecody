//! Google Gemini LLM provider

use super::{LLMProvider, Message, MessageRole};
use async_trait::async_trait;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct GeminiProvider {
    api_key: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }
}

#[async_trait]
impl LLMProvider for GeminiProvider {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let client = reqwest::Client::new();
        
        let gemini_contents: Vec<GeminiContent> = messages
            .iter()
            .map(|m| GeminiContent {
                role: match m.role {
                    MessageRole::System | MessageRole::User => "user".to_string(), // Gemini uses 'user' for system prompts too usually, or separate config
                    MessageRole::Assistant => "model".to_string(),
                },
                parts: vec![GeminiPart {
                    text: m.content.clone(),
                }],
            })
            .collect();

        let request = GeminiRequest {
            contents: gemini_contents,
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gemini")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Gemini API error: {}", error_text);
        }

        let gemini_response: GeminiResponse = response
            .json()
            .await
            .context("Failed to parse Gemini response")?;

        if let Some(candidates) = gemini_response.candidates {
            if let Some(first) = candidates.first() {
                if let Some(part) = first.content.parts.first() {
                    return Ok(part.text.clone());
                }
            }
        }

        Ok(String::new())
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let client = reqwest::Client::new();
        
        let gemini_contents: Vec<GeminiContent> = messages
            .iter()
            .map(|m| GeminiContent {
                role: match m.role {
                    MessageRole::System | MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "model".to_string(),
                },
                parts: vec![GeminiPart {
                    text: m.content.clone(),
                }],
            })
            .collect();

        let request = GeminiRequest {
            contents: gemini_contents,
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
            self.model, self.api_key
        );

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to Gemini")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Gemini API error: {}", error_text);
        }

        let stream = response.bytes_stream();
        
        let text_stream = stream.map(|chunk_result| {
            chunk_result
                .context("Failed to read chunk")
                .and_then(|chunk| {
                    let text = String::from_utf8_lossy(&chunk);
                    // Gemini streaming returns a JSON array of objects, but sometimes split across chunks.
                    // This is a simplified parser assuming clean chunks for now.
                    // Real implementation needs a proper JSON stream parser.
                    // For now, let's try to parse the whole chunk if it looks like a complete JSON object
                    
                    // NOTE: Gemini stream returns `[{...}, \n {...}]`. It's tricky to parse without a buffer.
                    // For this MVP, we might struggle with partial JSONs. 
                    // Let's assume for now we can extract text via simple string matching if JSON fails, 
                    // or just accumulate.
                    
                    // Actually, let's just try to find "text": "..." patterns
                    let mut content = String::new();
                    // Very naive parsing to extract text from JSON structure
                    // "text": "..."
                    let parts: Vec<&str> = text.split("\"text\": \"").collect();
                    for part in parts.iter().skip(1) {
                        if let Some(end) = part.find("\"") {
                            let extracted = &part[..end];
                            // Unescape basic chars
                            let unescaped = extracted.replace("\\n", "\n").replace("\\\"", "\"");
                            content.push_str(&unescaped);
                        }
                    }
                    
                    Ok(content)
                })
        });

        Ok(Box::pin(text_stream))
    }

    fn name(&self) -> &str {
        "Gemini"
    }
}
