//! Groq provider — OpenAI-compatible ultra-fast inference.
//!
//! Supported models: llama-3.3-70b-versatile, llama-3.1-8b-instant,
//! mixtral-8x7b-32768, gemma2-9b-it, deepseek-r1-distill-llama-70b

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

const GROQ_BASE_URL: &str = "https://api.groq.com/openai/v1";

#[derive(Debug, Serialize)]
struct GroqRequest {
    model: String,
    messages: Vec<GroqMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct GroqMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GroqUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
    #[serde(default)]
    usage: Option<GroqUsage>,
}

#[derive(Debug, Deserialize)]
struct GroqChoice {
    message: GroqMessage,
}

#[derive(Debug, Deserialize)]
struct GroqStreamResponse {
    choices: Vec<GroqStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct GroqStreamChoice {
    delta: GroqDelta,
}

#[derive(Debug, Deserialize)]
struct GroqDelta {
    content: Option<String>,
}

/// Groq provider — OpenAI-compatible endpoint, ultra-fast inference.
pub struct GroqProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl GroqProvider {
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
        self.config.api_url.clone().unwrap_or_else(|| GROQ_BASE_URL.to_string())
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<GroqMessage> {
        let mut result: Vec<GroqMessage> = messages.iter().map(|m| GroqMessage {
            role: format!("{:?}", m.role).to_lowercase(),
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

#[async_trait]
impl AIProvider for GroqProvider {
    fn name(&self) -> &str { "Groq" }

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
        let api_key = self.config.api_key.as_ref().context("Groq API key not set (GROQ_API_KEY)")?;
        let request = GroqRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };
        let url = format!("{}/chat/completions", self.base_url());
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send().await.context("Groq request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Groq API error: {}", err);
        }
        let body: GroqResponse = resp.json().await.context("Failed to parse Groq response")?;
        let text = body.choices.first().context("No choices")?.message.content.clone();
        let usage = body.usage.map(|u| TokenUsage { prompt_tokens: u.prompt_tokens, completion_tokens: u.completion_tokens });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Groq API key not set")?;
        let request = GroqRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, None),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: true,
        };
        let url = format!("{}/chat/completions", self.base_url());
        let resp = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send().await.context("Groq stream request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Groq API error: {}", err);
        }
        let stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(r) = serde_json::from_str::<GroqStreamResponse>(data) {
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
        // Groq doesn't support vision for most models; fall back to text
        self.chat(messages, context).await
    }
}
