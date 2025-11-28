//! VibeUI Core - Text buffer, file system, and workspace management

pub mod buffer;
pub mod file_system;
pub mod workspace;
pub mod search;
pub mod git;
pub mod terminal;

pub use buffer::TextBuffer as Buffer;
pub use file_system::FileSystem;
pub use workspace::Workspace;
