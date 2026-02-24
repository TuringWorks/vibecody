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

pub use buffer::TextBuffer as Buffer;
pub use file_system::FileSystem;
pub use workspace::Workspace;
pub use diff::{DiffEngine, DiffHunk};
pub use executor::CommandExecutor;
pub use index::{CodebaseIndex, IndexStats, SymbolInfo, SymbolKind, Language};
pub use context::ContextBuilder;
