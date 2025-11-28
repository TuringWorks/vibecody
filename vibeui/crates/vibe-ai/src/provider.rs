//! AI provider abstraction layer

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures::Stream;
use anyhow::Result;

/// Code context for AI completions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeContext {
    /// The programming language
    pub language: String,
    /// File path
    pub file_path: Option<String>,
    /// Code before the cursor
    pub prefix: String,
    /// Code after the cursor
    pub suffix: String,
    /// Additional context (e.g., imports, related files)
    pub additional_context: Vec<String>,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub text: String,
    pub model: String,
}

/// Stream of completion chunks
pub type CompletionStream = Pin<Box<dyn Stream<Item = Result<String>> + Send>>;

/// AI provider trait
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Check if the provider is available/configured
    async fn is_available(&self) -> bool;

    /// Generate a code completion
    async fn complete(&self, context: &CodeContext) -> Result<CompletionResponse>;

    /// Generate a streaming code completion
    async fn stream_complete(&self, context: &CodeContext) -> Result<CompletionStream>;

    /// Chat with the provider
    async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String>;

    /// Stream chat response
    async fn stream_chat(&self, messages: &[Message]) -> Result<CompletionStream>;
}

/// AI provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: String,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub model: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
}

impl ProviderConfig {
    pub fn new(provider_type: String, model: String) -> Self {
        Self {
            provider_type,
            api_key: None,
            api_url: None,
            model,
            max_tokens: None,
            temperature: None,
        }
    }

    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = Some(api_key);
        self
    }

    pub fn with_api_url(mut self, api_url: String) -> Self {
        self.api_url = Some(api_url);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
}
