//! VibeCLI Core - AI-powered coding assistant library

pub mod llm;
pub mod diff;
pub mod fs;
pub mod git;
pub mod executor;
pub mod config;
pub mod syntax;

pub use syntect;

pub use llm::{LLMProvider, Message, MessageRole};
pub use diff::{DiffEngine, DiffHunk};
pub use config::Config;
pub use syntax::{SyntaxHighlighter, highlight_code_blocks};
