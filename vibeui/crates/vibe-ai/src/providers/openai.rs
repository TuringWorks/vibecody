//! OpenAI provider implementation (ChatGPT, Codex)

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, ImageAttachment, Message, ProviderConfig, TokenUsage};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: Value,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    #[serde(default)]
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamResponse {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
}

#[derive(Debug, Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
}

/// OpenAI provider
pub struct OpenAIProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl OpenAIProvider {
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

    const DEFAULT_API_URL: &'static str = "https://api.openai.com/v1/chat/completions";

    fn api_url(&self) -> &str {
        self.config.api_url.as_deref().unwrap_or(Self::DEFAULT_API_URL)
    }

    /// Translate a raw OpenAI API error response into a user-friendly message.
    fn translate_api_error(status: u16, body: &str) -> String {
        if let Ok(v) = serde_json::from_str::<Value>(body) {
            let msg = v.pointer("/error/message")
                .and_then(|m| m.as_str())
                .unwrap_or(body);
            return match status {
                401 => format!("Authentication failed: {}. Check your OPENAI_API_KEY or api_key_helper in config.", msg),
                403 => format!("Access denied: {}. Your API key may lack permissions for this model.", msg),
                429 => format!("Rate limited: {}. Wait a moment or check your OpenAI plan limits.", msg),
                404 => format!("Model not found: {}. Check your model name in config.", msg),
                503 => format!("OpenAI is temporarily overloaded: {}. Retry in a few seconds.", msg),
                _ => format!("OpenAI API error (HTTP {}): {}", status, msg),
            };
        }
        format!("OpenAI API error (HTTP {}): {}", status, body)
    }

    fn build_messages(&self, messages: &[Message], context: Option<String>) -> Vec<OpenAIMessage> {
        let mut openai_messages: Vec<OpenAIMessage> = messages
            .iter()
            .map(|m| OpenAIMessage {
                role: m.role.as_str().to_string(),
                content: Value::String(m.content.clone()),
            })
            .collect();

        if let Some(ctx) = context {
            if let Some(last_msg) = openai_messages.last_mut() {
                if last_msg.role == "user" {
                    if let Value::String(ref s) = last_msg.content.clone() {
                        last_msg.content = Value::String(
                            format!("Context:\n{}\n\nUser: {}", ctx, s)
                        );
                    }
                }
            }
        }
        openai_messages
    }

    /// Build messages with image content blocks for the last user message.
    fn build_vision_messages(
        &self,
        messages: &[Message],
        images: &[ImageAttachment],
        context: Option<String>,
    ) -> Vec<OpenAIMessage> {
        let mut openai_messages = self.build_messages(messages, context);

        if images.is_empty() {
            return openai_messages;
        }

        if let Some(last) = openai_messages.last_mut() {
            if last.role == "user" {
                let text = match &last.content {
                    Value::String(s) => s.clone(),
                    _ => String::new(),
                };

                let mut parts: Vec<Value> = images
                    .iter()
                    .map(|img| {
                        json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};base64,{}", img.media_type, img.base64)
                            }
                        })
                    })
                    .collect();

                parts.push(json!({ "type": "text", "text": text }));
                last.content = Value::Array(parts);
            }
        }

        openai_messages
    }
}

#[async_trait]
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "OpenAI"
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
        let api_key = self.config.api_key.as_ref().context("OpenAI API key not found")?;
        let request = OpenAIRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };

        let response = self.client
            .post(self.api_url())
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await?;
            anyhow::bail!("{}", Self::translate_api_error(status, &error_text));
        }

        let openai_response: OpenAIResponse = response.json().await.context("Failed to parse OpenAI response")?;

        let content = openai_response.choices.first()
            .context("No choices in OpenAI response")?
            .message.content.clone();
        let text = match content {
            Value::String(s) => s,
            other => other.to_string(),
        };

        let usage = openai_response.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
        });

        Ok(CompletionResponse {
            text,
            model: self.config.model.clone(),
            usage,
        })
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        Ok(self.chat_response(messages, context).await?.text)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let api_key = self.config.api_key.as_ref().context("OpenAI API key not found")?;
        let request = OpenAIRequest {
            model: self.config.model.clone(),
            messages: self.build_messages(messages, None), // Context handled in build_messages
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: true,
        };

        let response = self.client
            .post(self.api_url())
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await?;
            anyhow::bail!("{}", Self::translate_api_error(status, &error_text));
        }

        let stream = response.bytes_stream();
        
        let completion_stream = stream
            .map(|chunk| {
                let chunk = chunk?;
                let chunk_str = String::from_utf8_lossy(&chunk);
                // OpenAI stream format is "data: {json}\n\n"
                // We need to parse multiple lines
                let mut content = String::new();
                
                for line in chunk_str.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            continue;
                        }
                        if let Ok(response) = serde_json::from_str::<OpenAIStreamResponse>(data) {
                            if let Some(choice) = response.choices.first() {
                                if let Some(delta_content) = &choice.delta.content {
                                    content.push_str(delta_content);
                                }
                            }
                        }
                    }
                }
                Ok(content)
            })
            .boxed();

        Ok(completion_stream)
    }

    fn supports_vision(&self) -> bool {
        // GPT-4 Vision, GPT-4o, and GPT-4-turbo models support images
        let m = &self.config.model;
        m.contains("gpt-4o") || m.contains("gpt-4-vision") || m.contains("gpt-4-turbo")
            || m == "gpt-4" || m.contains("o1")
    }

    async fn chat_with_images(
        &self,
        messages: &[Message],
        images: &[ImageAttachment],
        context: Option<String>,
    ) -> Result<String> {
        if images.is_empty() || !self.supports_vision() {
            return self.chat(messages, context).await;
        }

        let api_key = self.config.api_key.as_ref().context("OpenAI API key not found")?;
        let request = OpenAIRequest {
            model: self.config.model.clone(),
            messages: self.build_vision_messages(messages, images, context),
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            stream: false,
        };

        let response = self.client
            .post(self.api_url())
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send vision request to OpenAI")?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("OpenAI vision API error: {}", error_text);
        }

        let openai_response: OpenAIResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI vision response")?;

        let content = openai_response.choices.first()
            .context("No choices in OpenAI vision response")?
            .message.content.clone();
        match content {
            Value::String(s) => Ok(s),
            other => Ok(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::MessageRole;

    fn test_config() -> ProviderConfig {
        ProviderConfig {
            provider_type: "openai".into(),
            api_key: Some("sk-test".into()),
            api_url: Some("https://api.openai.com".into()),
            model: "gpt-4o".into(),
            temperature: None,
            max_tokens: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        }
    }

    #[test]
    fn name_is_openai() {
        let p = OpenAIProvider::new(test_config());
        assert_eq!(p.name(), "OpenAI");
    }

    #[tokio::test]
    async fn is_available_with_key() {
        let p = OpenAIProvider::new(test_config());
        assert!(p.is_available().await);
    }

    #[tokio::test]
    async fn not_available_without_key() {
        let mut cfg = test_config();
        cfg.api_key = None;
        let p = OpenAIProvider::new(cfg);
        assert!(!p.is_available().await);
    }

    #[test]
    fn supports_vision() {
        let p = OpenAIProvider::new(test_config());
        assert!(p.supports_vision());
    }

    #[test]
    fn build_messages_basic() {
        let p = OpenAIProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::User, content: "hello".into() },
        ];
        let result = p.build_messages(&msgs, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].role, "user");
    }

    #[test]
    fn build_messages_with_context() {
        let p = OpenAIProvider::new(test_config());
        let msgs = vec![
            Message { role: MessageRole::User, content: "hello".into() },
        ];
        let result = p.build_messages(&msgs, Some("ctx".into()));
        assert_eq!(result.len(), 1);
        let content = result[0].content.as_str().unwrap();
        assert!(content.contains("ctx"));
        assert!(content.contains("hello"));
    }

    #[test]
    fn openai_response_deser() {
        let json = r#"{"choices":[{"message":{"role":"assistant","content":"hi"}}],"usage":{"prompt_tokens":3,"completion_tokens":1}}"#;
        let resp: OpenAIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.usage.unwrap().completion_tokens, 1);
    }
}
