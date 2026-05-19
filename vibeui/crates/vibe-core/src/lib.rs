//! VibeUI Core - Text buffer, file system, workspace, indexing, and context management

pub mod buffer;
pub mod file_system;
pub mod workspace;
pub mod search;
pub mod git;
pub mod terminal;
pub mod diff;
pub mod executor;
pub mod index;
pub mod context;
pub mod path_guard;
// SonarQube-compatible rule engine — promoted from vibeui/src-tauri
// 2026-05-19 so the Security Posture sonar adapter (in vibecli-cli)
// can import it. SQLite-backed rule store is included; vibe-core
// already depends on git2 so the rusqlite add is small.
pub mod sonar_rules;

pub use buffer::TextBuffer as Buffer;
pub use file_system::FileSystem;
pub use workspace::Workspace;
pub use diff::{DiffEngine, DiffHunk};
pub use executor::CommandExecutor;
pub use index::{CodebaseIndex, IndexStats, SymbolInfo, SymbolKind, Language,
               EmbeddingIndex, EmbeddingProvider, EmbeddingDoc, SearchHit};
pub use context::ContextBuilder;
