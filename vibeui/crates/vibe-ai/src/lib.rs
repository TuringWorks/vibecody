//! VibeUI AI - AI provider abstraction and integrations

pub mod provider;
pub mod completion;
pub mod chat;
pub mod providers;
pub mod config;

pub use chat::ChatEngine;
pub use provider::{CodeContext, Message, MessageRole};
pub use config::AIConfig;
pub use completion::CompletionEngine;
