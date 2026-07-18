//! VibeCoder Core - Text buffer, file system, workspace, indexing, and context management

pub mod buffer;
pub mod context;
pub mod diff;
pub mod executor;
pub mod file_system;
pub mod git;
pub mod index;
pub mod path_guard;
pub mod search;
pub mod terminal;
pub mod workspace;
// SonarQube-compatible rule engine — promoted from vibecoder/src-tauri
// 2026-05-19 so the Security Posture sonar adapter (in vibecli-cli)
// can import it. SQLite-backed rule store is included; vibe-core
// already depends on git2 so the rusqlite add is small.
pub mod sonar_rules;

pub use buffer::TextBuffer as Buffer;
pub use context::ContextBuilder;
pub use diff::{DiffEngine, DiffHunk};
pub use executor::CommandExecutor;
pub use file_system::FileSystem;
pub use index::{
    CodebaseIndex, EmbeddingDoc, EmbeddingIndex, EmbeddingProvider, IndexStats, Language,
    SearchHit, SymbolInfo, SymbolKind,
};
pub use workspace::Workspace;
