//! Error types for the vibe-collab crate.

use thiserror::Error;

/// Errors that can occur during collaboration operations.
#[derive(Debug, Error)]
pub enum CollabError {
    #[error("Room not found: {0}")]
    RoomNotFound(String),

    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    #[error("Room is full (max {0} peers)")]
    RoomFull(usize),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Y.Doc error: {0}")]
    YrsError(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<CollabError> for axum::http::StatusCode {
    fn from(err: CollabError) -> Self {
        match err {
            CollabError::RoomNotFound(_) | CollabError::PeerNotFound(_) => {
                axum::http::StatusCode::NOT_FOUND
            }
            CollabError::RoomFull(_) => axum::http::StatusCode::CONFLICT,
            CollabError::AuthFailed(_) => axum::http::StatusCode::UNAUTHORIZED,
            CollabError::InvalidMessage(_) => axum::http::StatusCode::BAD_REQUEST,
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
