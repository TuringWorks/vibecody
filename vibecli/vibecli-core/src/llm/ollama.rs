//! Ollama LLM provider with streaming support

use super::{LLMProvider, Message, MessageRole};
use async_trait::async_trait;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use futures::stream::{Stream, StreamExt};
use std::pin::Pin;

#[derive(Debug, Clone)]
pub struct OllamaProvider {
    base_url: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaError {
    error: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
    #[allow(dead_code)]
    done: bool,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamResponse {
    message: Option<OllamaMessage>,
    #[allow(dead_code)]
    done: bool,
}

impl OllamaProvider {
    pub fn new(base_url: String, model: String) -> Self {
        Self { base_url, model }
    }

    pub fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: "qwen3-coder:480b-cloud".to_string(),
        }
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn chat(&self, messages: &[Message]) -> Result<String> {
        let client = reqwest::Client::new();
        
        let ollama_messages: Vec<OllamaMessage> = messages
            .iter()
            .map(|m| OllamaMessage {
                role: match m.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: m.content.clone(),
            })
            .collect();

        let request = OllamaRequest {
            model: self.model.clone(),
            messages: ollama_messages,
            stream: false,
        };

        let response = client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            if let Ok(ollama_error) = serde_json::from_str::<OllamaError>(&error_text) {
                return Err(anyhow::anyhow!("Ollama error: {}", ollama_error.error));
            }
            return Err(anyhow::anyhow!("Ollama request failed: {}", error_text));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        Ok(ollama_response.message.content)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let client = reqwest::Client::new();
        
        let ollama_messages: Vec<OllamaMessage> = messages
            .iter()
            .map(|m| OllamaMessage {
                role: match m.role {
                    MessageRole::System => "system".to_string(),
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                },
                content: m.content.clone(),
            })
            .collect();

        let request = OllamaRequest {
            model: self.model.clone(),
            messages: ollama_messages,
            stream: true,
        };

        let response = client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send streaming request to Ollama")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            if let Ok(ollama_error) = serde_json::from_str::<OllamaError>(&error_text) {
                return Err(anyhow::anyhow!("Ollama error: {}", ollama_error.error));
            }
            return Err(anyhow::anyhow!("Ollama request failed: {}", error_text));
        }

        let stream = response.bytes_stream();
        
        let text_stream = stream.map(|chunk_result| {
            chunk_result
                .context("Failed to read chunk")
                .and_then(|chunk| {
                    let text = String::from_utf8_lossy(&chunk);
                    
                    // Parse each line as a separate JSON object
                    for line in text.lines() {
                        if line.trim().is_empty() {
                            continue;
                        }
                        
                        if let Ok(response) = serde_json::from_str::<OllamaStreamResponse>(line) {
                            if let Some(msg) = response.message {
                                return Ok(msg.content);
                            }
                        }
                    }
                    
                    Ok(String::new())
                })
        });

        Ok(Box::pin(text_stream))
    }

    fn name(&self) -> &str {
        "Ollama"
    }
}
