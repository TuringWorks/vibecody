//! LLM provider abstraction and implementations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use futures::stream::Stream;
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat(&self, messages: &[Message]) -> Result<String>;
    async fn stream_chat(&self, messages: &[Message]) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>>;
    fn name(&self) -> &str;
}

pub mod ollama;
pub mod openai;
pub mod anthropic;
pub mod gemini;
pub mod grok;

pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use grok::GrokProvider;
