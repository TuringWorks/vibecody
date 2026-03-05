//! Mistral AI provider — native API, OpenAI-compatible.
//!
//! Supported models: mistral-large-latest, mistral-medium-latest,
//! mistral-small-latest, codestral-latest, open-mistral-nemo

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

const MISTRAL_BASE_URL: &str = "https://api.mistral.ai/v1";

#[derive(Debug, Serialize)]
struct MistralRequest {
    model: String,
    messages: Vec<MistralMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct MistralMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct MistralUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct MistralResponse {
    choices: Vec<MistralChoice>,
    #[serde(default)]
    usage: Option<MistralUsage>,
}

#[derive(Debug, Deserialize)]
struct MistralChoice {
    message: MistralMessage,
}

#[derive(Debug, Deserialize)]
struct MistralStreamResponse {
    choices: Vec<MistralStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct MistralStreamChoice {
    delta: MistralDelta,
}

#[derive(Debug, Deserialize)]
struct MistralDelta {
    content: Option<String>,
}

/// Mistral AI provider — native API endpoint.
pub struct MistralProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl MistralProvider {
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
        self.config.api_url.clone().unwrap_or_else(|| MISTRAL_BASE_URL.to_string())
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<MistralMessage> {
        let mut result: Vec<MistralMessage> = messages.iter().map(|m| MistralMessage {
            role: m.role.as_str().to_string(),
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
impl AIProvider for MistralProvider {
    fn name(&self) -> &str { "Mistral" }

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
        let api_key = self.config.api_key.as_ref().context("Mistral API key not set (MISTRAL_API_KEY)")?;
        let request = MistralRequest {
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
            .send().await.context("Mistral request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Mistral API error: {}", err);
        }
        let body: MistralResponse = resp.json().await.context("Failed to parse Mistral response")?;
        let text = body.choices.first().context("No choices")?.message.content.clone();
        let usage = body.usage.map(|u| TokenUsage { prompt_tokens: u.prompt_tokens, completion_tokens: u.completion_tokens });
        Ok(CompletionResponse { text, model: self.config.model.clone(), usage })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("Mistral API key not set")?;
        let request = MistralRequest {
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
            .send().await.context("Mistral stream request failed")?;

        if !resp.status().is_success() {
            let err = resp.text().await?;
            anyhow::bail!("Mistral API error: {}", err);
        }
        let stream = resp.bytes_stream().map(|chunk| {
            let chunk = chunk?;
            let text = String::from_utf8_lossy(&chunk);
            let mut content = String::new();
            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" { continue; }
                    if let Ok(r) = serde_json::from_str::<MistralStreamResponse>(data) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "mistral".into(),
            api_key: Some("test_key".into()),
            api_url: None,
            model: "mistral-large-latest".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_mistral() {
        let p = MistralProvider::new(test_config());
        assert_eq!(p.name(), "Mistral");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = MistralProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = MistralProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn base_url_constant() {
        assert_eq!(MISTRAL_BASE_URL, "https://api.mistral.ai/v1");
    }

    #[test]
    fn mistral_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"hello"}}],"usage":{"prompt_tokens":5,"completion_tokens":1}}"#;
        let resp: MistralResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices[0].message.content, "hello");
        assert_eq!(resp.usage.unwrap().completion_tokens, 1);
    }
}
