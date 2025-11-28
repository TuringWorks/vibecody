//! VibeUI LSP - Language Server Protocol client implementation

pub mod client;
pub mod manager;
pub mod features;

pub use client::LspClient;
pub use manager::LspManager;
