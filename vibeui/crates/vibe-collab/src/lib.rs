//! `vibe-collab` — CRDT-based multiplayer collaboration for VibeCody.
//!
//! Provides real-time collaborative editing powered by [`yrs`] (the Rust port of Yjs).
//! The server manages rooms via [`CollabServer`], each room holds a shared [`yrs::Doc`],
//! and peers sync via the Yjs binary protocol over WebSocket.

pub mod awareness;
pub mod error;
pub mod protocol;
pub mod room;
pub mod server;

// Re-exports for convenience
pub use awareness::{AwarenessState, CursorState, PeerInfo};
pub use error::CollabError;
pub use protocol::CollabMessage;
pub use room::{CollabRoom, SyncBroadcast};
pub use server::CollabServer;
