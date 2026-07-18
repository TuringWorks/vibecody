//! VibeCoder LSP - Language Server Protocol client implementation

pub mod client;
pub mod features;
pub mod manager;

pub use client::LspClient;
pub use manager::LspManager;
