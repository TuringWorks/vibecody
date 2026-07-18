//! Awareness state for peer cursors and presence.

use serde::{Deserialize, Serialize};

/// Color palette for peer cursors (8 distinct colors).
const PEER_COLORS: &[&str] = &[
    "#e06c75", // red
    "#61afef", // blue
    "#98c379", // green
    "#e5c07b", // yellow
    "#c678dd", // purple
    "#56b6c2", // cyan
    "#d19a66", // orange
    "#be5046", // dark red
];

/// Assign a color to a peer based on their index in the room.
pub fn color_for_peer(index: usize) -> &'static str {
    PEER_COLORS[index % PEER_COLORS.len()]
}

/// Cursor position within a file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CursorState {
    /// Relative file path within the workspace.
    pub file: String,
    /// Line number (0-indexed).
    pub line: u32,
    /// Column number (0-indexed).
    pub column: u32,
    /// Optional selection end (line, column).
    pub selection_end: Option<(u32, u32)>,
}

/// Information about a connected peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Unique peer identifier.
    pub peer_id: String,
    /// Display name.
    pub name: String,
    /// Assigned color hex code.
    pub color: String,
    /// Current cursor position (if any).
    pub cursor: Option<CursorState>,
    /// Timestamp of last activity (unix millis).
    pub last_active: u64,
}

/// Awareness state broadcast between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwarenessState {
    /// Peer who generated this update.
    pub peer_id: String,
    /// Updated cursor state.
    pub cursor: Option<CursorState>,
    /// Timestamp (unix millis).
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_palette_cycling() {
        assert_eq!(color_for_peer(0), "#e06c75");
        assert_eq!(color_for_peer(8), "#e06c75"); // wraps around
        assert_eq!(color_for_peer(1), "#61afef");
    }

    #[test]
    fn test_peer_info_serialization() {
        let peer = PeerInfo {
            peer_id: "abc-123".to_string(),
            name: "Alice".to_string(),
            color: "#e06c75".to_string(),
            cursor: Some(CursorState {
                file: "src/main.rs".to_string(),
                line: 10,
                column: 5,
                selection_end: None,
            }),
            last_active: 1700000000000,
        };
        let json = serde_json::to_string(&peer).unwrap();
        let deserialized: PeerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Alice");
        assert_eq!(deserialized.cursor.unwrap().line, 10);
    }
}
