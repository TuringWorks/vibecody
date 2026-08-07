//! VibeCoder LSP - Language Server Protocol client implementation

pub mod client;
pub mod discovery;
pub mod features;
pub mod manager;

pub use client::{path_to_uri, LspClient};
pub use manager::{LanguageStatus, LspManager};
