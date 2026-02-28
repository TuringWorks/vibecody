//! AI chat interface

use crate::provider::{AIProvider, Message, MessageRole};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<Message>,
    pub created_at: std::time::SystemTime,
}

impl Conversation {
    pub fn new(id: String) -> Self {
        Self {
            id,
            messages: Vec::new(),
            created_at: std::time::SystemTime::now(),
        }
    }

    pub fn add_message(&mut self, role: MessageRole, content: String) {
        self.messages.push(Message { role, content });
    }

    pub fn add_user_message(&mut self, content: String) {
        self.add_message(MessageRole::User, content);
    }

    pub fn add_assistant_message(&mut self, content: String) {
        self.add_message(MessageRole::Assistant, content);
    }

    pub fn add_system_message(&mut self, content: String) {
        self.add_message(MessageRole::System, content);
    }
}

/// Chat engine for AI conversations
pub struct ChatEngine {
    providers: Vec<Arc<dyn AIProvider>>,
    active_provider_index: usize,
    conversations: Vec<Conversation>,
    active_conversation_index: Option<usize>,
}

impl ChatEngine {
    /// Create a new chat engine
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            active_provider_index: 0,
            conversations: Vec::new(),
            active_conversation_index: None,
        }
    }

    /// Add an AI provider
    pub fn add_provider(&mut self, provider: Arc<dyn AIProvider>) {
        self.providers.push(provider);
    }

    /// Get list of provider names
    pub fn get_provider_names(&self) -> Vec<String> {
        self.providers.iter().map(|p| p.name().to_string()).collect()
    }

    /// Set the active provider
    pub fn set_active_provider(&mut self, index: usize) -> Result<()> {
        if index >= self.providers.len() {
            anyhow::bail!("Provider index out of bounds");
        }
        self.active_provider_index = index;
        Ok(())
    }

    /// Get the active provider
    pub fn active_provider(&self) -> Option<&Arc<dyn AIProvider>> {
        self.providers.get(self.active_provider_index)
    }

    /// Remove all cloud providers (Claude, OpenAI, Gemini, Grok) so they can be re-added with new API keys.
    pub fn clear_cloud_providers(&mut self) {
        let cloud_prefixes = ["Claude", "OpenAI", "Gemini", "Grok"];
        self.providers.retain(|p| {
            let name = p.name();
            !cloud_prefixes.iter().any(|prefix| name.starts_with(prefix))
        });
        if self.providers.is_empty() {
            self.active_provider_index = 0;
        } else if self.active_provider_index >= self.providers.len() {
            self.active_provider_index = self.providers.len() - 1;
        }
    }

    /// Set the active provider by name
    pub fn set_provider_by_name(&mut self, name: &str) -> Result<()> {
        if let Some(index) = self.providers.iter().position(|p| p.name() == name) {
            self.active_provider_index = index;
            Ok(())
        } else {
            anyhow::bail!("Provider {} not found", name)
        }
    }

    /// Chat with the active provider using a list of messages
    pub async fn chat(&self, messages: &[Message], context: Option<String>) -> Result<String> {
        let provider = self
            .active_provider()
            .ok_or_else(|| anyhow::anyhow!("No active provider"))?
            .clone();

        if !provider.is_available().await {
            anyhow::bail!("Provider {} is not available", provider.name());
        }

        provider.chat(messages, context).await
    }

    /// Create a new conversation
    pub fn new_conversation(&mut self, id: String) -> usize {
        let conversation = Conversation::new(id);
        self.conversations.push(conversation);
        let index = self.conversations.len() - 1;
        self.active_conversation_index = Some(index);
        index
    }

    /// Get the active conversation
    pub fn active_conversation(&self) -> Option<&Conversation> {
        self.active_conversation_index
            .and_then(|i| self.conversations.get(i))
    }

    /// Get a mutable reference to the active conversation
    pub fn active_conversation_mut(&mut self) -> Option<&mut Conversation> {
        self.active_conversation_index
            .and_then(|i| self.conversations.get_mut(i))
    }

    /// Set the active conversation
    pub fn set_active_conversation(&mut self, index: usize) -> Result<()> {
        if index >= self.conversations.len() {
            anyhow::bail!("Conversation index out of bounds");
        }
        self.active_conversation_index = Some(index);
        Ok(())
    }

    /// Get all conversations
    pub fn conversations(&self) -> &[Conversation] {
        &self.conversations
    }

    /// Send a message and get a response
    pub async fn send_message(&mut self, content: String) -> Result<String> {
        let provider = self
            .active_provider()
            .ok_or_else(|| anyhow::anyhow!("No active provider"))?
            .clone();

        if !provider.is_available().await {
            anyhow::bail!("Provider {} is not available", provider.name());
        }

        let conversation = self
            .active_conversation_mut()
            .ok_or_else(|| anyhow::anyhow!("No active conversation"))?;

        // Add user message
        conversation.add_user_message(content);

        // Clone messages to avoid borrow checker issues
        let messages = conversation.messages.clone();

        // Get response from AI
        let response = provider.chat(&messages, None).await?;

        // Add assistant response
        let conversation = self.active_conversation_mut()
            .ok_or_else(|| anyhow::anyhow!("No active conversation after LLM response"))?;
        conversation.add_assistant_message(response.clone());

        Ok(response)
    }

    /// Send a message and stream the response
    pub async fn stream_message(
        &mut self,
        content: String,
    ) -> Result<impl futures::Stream<Item = Result<String>>> {
        let provider = self
            .active_provider()
            .ok_or_else(|| anyhow::anyhow!("No active provider"))?
            .clone();

        if !provider.is_available().await {
            anyhow::bail!("Provider {} is not available", provider.name());
        }

        let conversation = self
            .active_conversation_mut()
            .ok_or_else(|| anyhow::anyhow!("No active conversation"))?;

        // Add user message
        conversation.add_user_message(content);

        // Clone messages to avoid borrow checker issues
        let messages = conversation.messages.clone();

        // Stream response from AI
        provider.stream_chat(&messages).await
    }
}

impl Default for ChatEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_conversation() {
        let mut engine = ChatEngine::new();
        let index = engine.new_conversation("test-1".to_string());
        assert_eq!(index, 0);
        assert!(engine.active_conversation().is_some());
    }

    #[test]
    fn test_add_messages() {
        let mut conversation = Conversation::new("test".to_string());
        conversation.add_user_message("Hello".to_string());
        conversation.add_assistant_message("Hi there!".to_string());
        assert_eq!(conversation.messages.len(), 2);
    }
}
