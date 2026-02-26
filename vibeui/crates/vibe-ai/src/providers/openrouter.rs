//! OpenRouter provider — 300+ models via unified OpenAI-compatible API.
//!
//! Set OPENROUTER_API_KEY. Model format: "anthropic/claude-3.5-sonnet",
//! "google/gemini-flash-1.5", "meta-llama/llama-3.3-70b-instruct", etc.

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Serialize)]
struct ORRequest {
    model: String,
    messages: Vec<ORMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ORMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ORUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ORResponse {
    choices: Vec<ORChoice>,
    #[serde(default)]
    usage: Option<ORUsage>,
}

#[derive(Debug, Deserialize)]
struct ORChoice {
    message: ORMessage,
}

#[derive(Debug, Deserialize)]
struct ORStreamResponse {
    choices: Vec<ORStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct ORStreamChoice {
    delta: ORDelta,
}

#[derive(Debug, Deserialize)]
struct ORDelta {
    content: Option<String>,
}

/// OpenRouter provider — access 300+ models through a single API key.
pub struct OpenRouterProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    /// Site URL for OpenRouter attribution (optional).
    site_url: String,
    /// App name for OpenRouter attribution (optional).
    app_name: String,
}

impl OpenRouterProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            site_url: "https://github.com/vibecody/vibecody".to_string(),
            app_name: "VibeCody".to_string(),
        }
    }

    fn base_url(&self) -> String {
        self.config.api_url.clone().unwrap_or_else(|| OPENROUTER_BASE_URL.to_string())
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<ORMessage> {
        let mut result: Vec<ORMessage> = messages.iter().map(|m| ORMessage {
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

    fn client_with_headers(&self, api_key: &str) -> reqwest::RequestBuilder {
        self.client.post(format!("{}/chat/completions", self.base_url()))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("HTTP-Referer", &self.site_url)
            .header("X-Title", &self.app_name)
    }
}

#[async_trait]
impl AIProvider for OpenRouterProvider {
    fn name(&self) -> &str { "OpenRouter" }

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
        let api_key = self.config.api_key.as_ref().context("OpenRouter API key not set (OPENROUTER_API_KEY)")?;
        let request = ORRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };
        let resp = self.client_with_headers(api_key)
            .json(&request).send().await.context("OpenRouter request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("OpenRouter API error: {}", err);
        }
        let body: ORResponse = resp.json().await.context("Failed to parse OpenRouter response")?;
        let text = body.choices.first().context("No choices")?.message.content.clone();
        let usage = body.usage.map(|u| TokenUsage { prompt_tokens: u.prompt_tokens, completion_tokens: u.completion_tokens });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("OpenRouter API key not set")?;
        let request = ORRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, None),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: true,
        };
        let resp = self.client_with_headers(api_key)
            .json(&request).send().await.context("OpenRouter stream failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("OpenRouter API error: {}", err);
        }
        let stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(r) = serde_json::from_str::<ORStreamResponse>(data) {
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
        // OpenRouter passes through vision to underlying providers that support it
        self.chat(messages, context).await
    }
}
