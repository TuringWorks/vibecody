//! Ollama AI provider implementation

use crate::provider::{AIProvider, CodeContext, CompletionResponse, CompletionStream, Message, ProviderConfig};
use anyhow::{Context, Result};
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaChatMessage,
    #[allow(dead_code)]
    done: bool,
}

/// Ollama AI provider
pub struct OllamaProvider {
    config: ProviderConfig,
    client: reqwest::Client,
    base_url: String,
    display_name: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider
    pub fn new(config: ProviderConfig) -> Self {
        let base_url = config
            .api_url
            .clone()
            .unwrap_or_else(|| "http://localhost:11434".to_string());
        
        let display_name = format!("Ollama ({})", config.model);

        Self {
            config,
            client: reqwest::Client::new(),
            base_url,
            display_name,
        }
    }

    fn build_prompt(&self, context: &CodeContext) -> String {
        format!(
            "Complete the following {} code:\n\n{}<CURSOR>{}",
            context.language, context.prefix, context.suffix
        )
    }

    fn build_options(&self) -> Option<OllamaOptions> {
        if self.config.temperature.is_some() || self.config.max_tokens.is_some() {
            Some(OllamaOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens,
            })
        } else {
            None
        }
    }

    /// List available Ollama models
    pub async fn list_models(base_url: Option<String>) -> Result<Vec<String>> {
        let base_url = base_url.unwrap_or_else(|| "http://localhost:11434".to_string());
        let client = reqwest::Client::new();
        
        let response = client
            .get(format!("{}/api/tags", base_url))
            .send()
            .await
            .context("Failed to connect to Ollama")?;

        #[derive(Deserialize)]
        struct ModelListResponse {
            models: Vec<ModelInfo>,
        }

        #[derive(Deserialize)]
        struct ModelInfo {
            name: String,
        }

        let list: ModelListResponse = response
            .json()
            .await
            .context("Failed to parse model list")?;

        Ok(list.models.into_iter().map(|m| m.name).collect())
    }
}

#[async_trait]
impl AIProvider for OllamaProvider {
    fn name(&self) -> &str {
        &self.display_name
    }

    async fn is_available(&self) -> bool {
        // Try to ping the Ollama API
        self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .is_ok()
    }

    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse> {
        let prompt = self.build_prompt(context);
        
        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt,
            stream: false,
            options: self.build_options(),
        };

        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        Ok(CompletionResponse {
            text: ollama_response.response,
            model: self.config.model.clone(),
            usage: None,
        })
    }

    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream> {
        let prompt = self.build_prompt(context);
        
        let request = OllamaRequest {
            model: self.config.model.clone(),
            prompt,
            stream: true,
            options: self.build_options(),
        };

        let response = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        let stream = response.bytes_stream();
        
        let completion_stream = stream
            .map(|chunk| {
                let chunk = chunk?;
                let response: OllamaResponse = serde_json::from_slice(&chunk)?;
                Ok(response.response)
            })
            .boxed();

        Ok(completion_stream)
    }

    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let mut ollama_messages: Vec<OllamaChatMessage> = messages
            .iter()
            .map(|m| OllamaChatMessage {
                role: format!("{:?}", m.role).to_lowercase(),
                content: m.content.clone(),
            })
            .collect();

        // Inject context into the last user message if available
        if let Some(ctx) = context {
            if let Some(last_msg) = ollama_messages.last_mut() {
                if last_msg.role == "user" {
                    last_msg.content = format!("Context:\n{}\n\nUser: {}", ctx, last_msg.content);
                }
            }
        }

        let request = OllamaChatRequest {
            model: self.config.model.clone(),
            messages: ollama_messages,
            stream: false,
            options: self.build_options(),
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send chat request to Ollama")?;

        let status = response.status();
        let body_text = response.text().await.context("Failed to read response body")?;
        


        if !status.is_success() {
            anyhow::bail!("Ollama API error: {}", body_text);
        }

        let ollama_response: OllamaChatResponse = serde_json::from_str(&body_text)
            .context(format!("Failed to parse Ollama chat response: {}", body_text))?;

        Ok(ollama_response.message.content)
    }

    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream> {
        let ollama_messages: Vec<OllamaChatMessage> = messages
            .iter()
            .map(|m| OllamaChatMessage {
                role: format!("{:?}", m.role).to_lowercase(),
                content: m.content.clone(),
            })
            .collect();

        let request = OllamaChatRequest {
            model: self.config.model.clone(),
            messages: ollama_messages,
            stream: true,
            options: self.build_options(),
        };

        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send chat request to Ollama")?;

        let stream = response.bytes_stream();
        
        let completion_stream = stream
            .map(|chunk| {
                let chunk = chunk?;
                let response: OllamaChatResponse = serde_json::from_slice(&chunk)?;
                Ok(response.message.content)
            })
            .boxed();

        Ok(completion_stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_prompt() {
        let config = ProviderConfig::new("ollama".to_string(), "codellama".to_string());
        let provider = OllamaProvider::new(config);
        
        let context = CodeContext {
            language: "rust".to_string(),
            file_path: None,
            prefix: "fn main() {\n    ".to_string(),
            suffix: "\n}".to_string(),
            additional_context: vec![],
        };
        
        let prompt = provider.build_prompt(&context);
        assert!(prompt.contains("rust"));
        assert!(prompt.contains("fn main()"));
    }
}
