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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[test]
    fn display_room_not_found() {
        let e = CollabError::RoomNotFound("abc".into());
        assert_eq!(e.to_string(), "Room not found: abc");
    }

    #[test]
    fn display_peer_not_found() {
        let e = CollabError::PeerNotFound("peer1".into());
        assert_eq!(e.to_string(), "Peer not found: peer1");
    }

    #[test]
    fn display_room_full() {
        let e = CollabError::RoomFull(10);
        assert_eq!(e.to_string(), "Room is full (max 10 peers)");
    }

    #[test]
    fn display_auth_failed() {
        let e = CollabError::AuthFailed("bad token".into());
        assert_eq!(e.to_string(), "Authentication failed: bad token");
    }

    #[test]
    fn display_invalid_message() {
        let e = CollabError::InvalidMessage("malformed".into());
        assert_eq!(e.to_string(), "Invalid message: malformed");
    }

    #[test]
    fn display_yrs_error() {
        let e = CollabError::YrsError("corrupt doc".into());
        assert_eq!(e.to_string(), "Y.Doc error: corrupt doc");
    }

    #[test]
    fn display_websocket_error() {
        let e = CollabError::WebSocketError("closed".into());
        assert_eq!(e.to_string(), "WebSocket error: closed");
    }

    #[test]
    fn display_internal() {
        let e = CollabError::Internal("oops".into());
        assert_eq!(e.to_string(), "Internal error: oops");
    }

    // ── StatusCode conversion ────────────────────────────────────────────

    #[test]
    fn status_code_not_found() {
        assert_eq!(StatusCode::from(CollabError::RoomNotFound("x".into())), StatusCode::NOT_FOUND);
        assert_eq!(StatusCode::from(CollabError::PeerNotFound("x".into())), StatusCode::NOT_FOUND);
    }

    #[test]
    fn status_code_conflict() {
        assert_eq!(StatusCode::from(CollabError::RoomFull(5)), StatusCode::CONFLICT);
    }

    #[test]
    fn status_code_unauthorized() {
        assert_eq!(StatusCode::from(CollabError::AuthFailed("x".into())), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn status_code_bad_request() {
        assert_eq!(StatusCode::from(CollabError::InvalidMessage("x".into())), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn status_code_internal() {
        assert_eq!(StatusCode::from(CollabError::YrsError("x".into())), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(StatusCode::from(CollabError::WebSocketError("x".into())), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(StatusCode::from(CollabError::Internal("x".into())), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
