//! Azure OpenAI provider.
//!
//! Endpoint format: `https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}`
//!
//! Config:
//! ```toml
//! [azure_openai]
//! enabled = true
//! api_url = "https://myresource.openai.azure.com"
//! model = "gpt-4o"          # Azure deployment name
//! api_key = "..."
//! api_version = "2024-12-01-preview"   # optional
//! ```

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_API_VERSION: &str = "2024-12-01-preview";

#[derive(Debug, Serialize)]
struct AzRequest {
    messages: Vec<AzMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct AzMessage {
    role: String,
    content: Value,
}

#[derive(Debug, Deserialize)]
struct AzUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AzResponse {
    choices: Vec<AzChoice>,
    #[serde(default)]
    usage: Option<AzUsage>,
}

#[derive(Debug, Deserialize)]
struct AzChoice {
    message: AzMessage,
}

#[derive(Debug, Deserialize)]
struct AzStreamResponse {
    choices: Vec<AzStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct AzStreamChoice {
    delta: AzDelta,
}

#[derive(Debug, Deserialize)]
struct AzDelta {
    content: Option<String>,
}

/// Azure OpenAI provider.
pub struct AzureOpenAIProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    api_version: String,
}

impl AzureOpenAIProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self {
            api_version: DEFAULT_API_VERSION.to_string(),
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn with_api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = version.into();
        self
    }

    /// Build the full Azure endpoint URL for the deployment.
    fn endpoint_url(&self) -> String {
        let base = self.config.api_url.as_deref().unwrap_or("");
        let base = base.trim_end_matches('/');
        let deployment = &self.config.model;
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            base, deployment, self.api_version
        )
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<AzMessage> {
        let mut result: Vec<AzMessage> = messages.iter().map(|m| AzMessage {
            role: format!("{:?}", m.role).to_lowercase(),
            content: Value::String(m.content.clone()),
        }).collect();
        if let Some(ctx) = context {
            if let Some(last) = result.last_mut() {
                if last.role == "user" {
                    if let Value::String(ref s) = last.content.clone() {
                        last.content = Value::String(format!("Context:\n{}\n\nUser: {}", ctx, s));
                    }
                }
            }
        }
        result
    }
}

#[async_trait]
impl AIProvider for AzureOpenAIProvider {
    fn name(&self) -> &str { "AzureOpenAI" }

    async fn is_available(&self) -> bool {
        self.config.api_key.is_some() && self.config.api_url.is_some()
    }

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
        let api_key = self.config.api_key.as_ref().context("Azure OpenAI API key not set (AZURE_OPENAI_API_KEY)")?;
        let request = AzRequest {
            messages: self.build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };
        let resp = self.client.post(self.endpoint_url())
            .header("api-key", api_key.as_str())
            .json(&request)
            .send().await.context("Azure OpenAI request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Azure OpenAI error: {}", err);
        }
        let body: AzResponse = resp.json().await.context("Failed to parse Azure OpenAI response")?;
        let content = body.choices.first().context("No choices")?.message.content.clone();
        let text = match content { Value::String(s) => s, v => v.to_string() };
        let usage = body.usage.map(|u| TokenUsage { prompt_tokens: u.prompt_tokens, completion_tokens: u.completion_tokens });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Azure OpenAI API key not set")?;
        let request = AzRequest {
            messages: self.build_messages(messages, None),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: true,
        };
        let resp = self.client.post(self.endpoint_url())
            .header("api-key", api_key.as_str())
            .json(&request)
            .send().await.context("Azure OpenAI stream failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Azure OpenAI error: {}", err);
        }
        let stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(r) = serde_json::from_str::<AzStreamResponse>(data) {
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
