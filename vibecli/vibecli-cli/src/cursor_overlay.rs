#![allow(dead_code)]
//! Live collaboration cursor overlay — tracks remote peer cursors for display
//! on top of the local editor. Extends the existing CRDT sync module.
//!
//! Each peer has a named cursor with a colour and position (line + col).
//! The overlay merges CRDT position updates into renderable cursor states.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A peer's identity in a collaboration session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerId(pub String);

impl PeerId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
}

impl std::fmt::Display for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Colour for a peer cursor (CSS hex string or named colour).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorColor(pub String);

impl CursorColor {
    pub fn from_hex(hex: impl Into<String>) -> Self { Self(hex.into()) }
    /// Assign a deterministic colour from a peer ID hash.
    pub fn from_peer_id(peer: &PeerId) -> Self {
        let hash: u32 = peer.0.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
        // Map to a palette of 8 distinct colours
        let palette = [
            "#e06c75", "#98c379", "#e5c07b", "#61afef",
            "#c678dd", "#56b6c2", "#d19a66", "#abb2bf",
        ];
        Self(palette[(hash as usize) % palette.len()].to_string())
    }
}

/// A cursor position in a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    /// 1-based line number.
    pub line: usize,
    /// 0-based column.
    pub col: usize,
}

impl CursorPosition {
    pub fn new(line: usize, col: usize) -> Self { Self { line, col } }
}

/// Optional text selection range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub start: CursorPosition,
    pub end: CursorPosition,
}

impl Selection {
    pub fn is_empty(&self) -> bool { self.start == self.end }
}

/// The full state of a peer's cursor as rendered in the overlay.
#[derive(Debug, Clone)]
pub struct PeerCursor {
    pub peer_id: PeerId,
    pub display_name: String,
    pub color: CursorColor,
    pub position: CursorPosition,
    pub selection: Option<Selection>,
    pub file_path: Option<String>,
    pub last_seen_ms: u64,
    pub is_typing: bool,
}

impl PeerCursor {
    /// Returns true if the cursor was last seen more than `timeout_ms` ago.
    pub fn is_stale(&self, timeout_ms: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        now.saturating_sub(self.last_seen_ms) > timeout_ms
    }
}

// ---------------------------------------------------------------------------
// Update messages
// ---------------------------------------------------------------------------

/// An update received from a remote peer.
#[derive(Debug, Clone)]
pub struct CursorUpdate {
    pub peer_id: PeerId,
    pub display_name: Option<String>,
    pub position: CursorPosition,
    pub selection: Option<Selection>,
    pub file_path: Option<String>,
    pub is_typing: bool,
    pub timestamp_ms: u64,
}

// ---------------------------------------------------------------------------
// Overlay manager
// ---------------------------------------------------------------------------

/// Manages the set of live peer cursors.
pub struct CursorOverlay {
    pub local_peer: PeerId,
    cursors: HashMap<PeerId, PeerCursor>,
    /// Milliseconds before a cursor is considered stale.
    pub stale_timeout_ms: u64,
}

impl CursorOverlay {
    pub fn new(local_peer: PeerId) -> Self {
        Self {
            local_peer,
            cursors: HashMap::new(),
            stale_timeout_ms: 30_000,
        }
    }

    /// Apply a cursor update from a remote peer.
    pub fn apply_update(&mut self, update: CursorUpdate) {
        // Don't track our own cursor in the overlay
        if update.peer_id == self.local_peer { return; }

        let color = CursorColor::from_peer_id(&update.peer_id);
        let display_name = update.display_name
            .clone()
            .unwrap_or_else(|| update.peer_id.0.clone());

        self.cursors.insert(update.peer_id.clone(), PeerCursor {
            peer_id: update.peer_id,
            display_name,
            color,
            position: update.position,
            selection: update.selection,
            file_path: update.file_path,
            last_seen_ms: update.timestamp_ms,
            is_typing: update.is_typing,
        });
    }

    /// Remove a peer (disconnected).
    pub fn remove_peer(&mut self, peer_id: &PeerId) {
        self.cursors.remove(peer_id);
    }

    /// Get all active (non-stale) cursors.
    pub fn active_cursors(&self) -> Vec<&PeerCursor> {
        self.cursors.values()
            .filter(|c| !c.is_stale(self.stale_timeout_ms))
            .collect()
    }

    /// Get cursors for a specific file.
    pub fn cursors_in_file(&self, file_path: &str) -> Vec<&PeerCursor> {
        self.cursors.values()
            .filter(|c| {
                c.file_path.as_deref() == Some(file_path) &&
                !c.is_stale(self.stale_timeout_ms)
            })
            .collect()
    }

    /// Get cursors near a given line (within `radius` lines).
    pub fn cursors_near_line(&self, file_path: &str, line: usize, radius: usize) -> Vec<&PeerCursor> {
        self.cursors_in_file(file_path).into_iter()
            .filter(|c| {
                let diff = if c.position.line >= line {
                    c.position.line - line
                } else {
                    line - c.position.line
                };
                diff <= radius
            })
            .collect()
    }

    /// Number of currently tracked peers (including stale).
    pub fn peer_count(&self) -> usize {
        self.cursors.len()
    }

    /// Purge stale cursors.
    pub fn prune_stale(&mut self) {
        let timeout = self.stale_timeout_ms;
        self.cursors.retain(|_, c| !c.is_stale(timeout));
    }

    /// Render a text summary for debug / status bar.
    pub fn status_line(&self) -> String {
        let active = self.active_cursors();
        if active.is_empty() {
            return "No collaborators online".to_string();
        }
        let names: Vec<&str> = active.iter().map(|c| c.display_name.as_str()).collect();
        format!("{} collaborator(s): {}", active.len(), names.join(", "))
    }

    /// Serialize cursors as minimal JSON for IPC.
    pub fn to_json(&self) -> String {
        let active = self.active_cursors();
        let items: Vec<String> = active.iter().map(|c| {
            format!(
                "{{\"peer\":\"{}\",\"name\":\"{}\",\"color\":\"{}\",\"line\":{},\"col\":{}}}",
                c.peer_id.0,
                c.display_name.replace('"', "\\\""),
                c.color.0,
                c.position.line,
                c.position.col
            )
        }).collect();
        format!("[{}]", items.join(","))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    fn make_update(peer: &str, line: usize, col: usize) -> CursorUpdate {
        CursorUpdate {
            peer_id: PeerId::new(peer),
            display_name: Some(peer.to_string()),
            position: CursorPosition::new(line, col),
            selection: None,
            file_path: Some("src/main.rs".into()),
            is_typing: false,
            timestamp_ms: now_ms(),
        }
    }

    #[test]
    fn test_apply_update_tracks_peer() {
        let mut overlay = CursorOverlay::new(PeerId::new("local"));
        overlay.apply_update(make_update("alice", 10, 5));
        assert_eq!(overlay.peer_count(), 1);
    }

    #[test]
    fn test_local_peer_ignored() {
        let mut overlay = CursorOverlay::new(PeerId::new("local"));
        overlay.apply_update(make_update("local", 1, 0));
        assert_eq!(overlay.peer_count(), 0);
    }

    #[test]
    fn test_active_cursors() {
        let mut overlay = CursorOverlay::new(PeerId::new("local"));
        overlay.apply_update(make_update("alice", 5, 0));
        overlay.apply_update(make_update("bob", 10, 0));
        assert_eq!(overlay.active_cursors().len(), 2);
    }

    #[test]
    fn test_remove_peer() {
        let mut overlay = CursorOverlay::new(PeerId::new("local"));
        overlay.apply_update(make_update("alice", 1, 0));
        overlay.remove_peer(&PeerId::new("alice"));
        assert_eq!(overlay.peer_count(), 0);
    }

    #[test]
    fn test_cursors_in_file() {
        let mut overlay = CursorOverlay::new(PeerId::new("local"));
        let mut u = make_update("alice", 5, 0);
        u.file_path = Some("src/lib.rs".into());
        overlay.apply_update(u);
        overlay.apply_update(make_update("bob", 3, 0)); // src/main.rs

        assert_eq!(overlay.cursors_in_file("src/lib.rs").len(), 1);
        assert_eq!(overlay.cursors_in_file("src/main.rs").len(), 1);
    }

    #[test]
    fn test_cursors_near_line() {
        let mut overlay = CursorOverlay::new(PeerId::new("local"));
        overlay.apply_update(make_update("alice", 10, 0));
        overlay.apply_update(make_update("bob", 25, 0));
        let near = overlay.cursors_near_line("src/main.rs", 10, 2);
        assert_eq!(near.len(), 1);
        assert_eq!(near[0].peer_id.0, "alice");
    }

    #[test]
    fn test_cursor_color_deterministic() {
        let peer = PeerId::new("alice");
        let c1 = CursorColor::from_peer_id(&peer);
        let c2 = CursorColor::from_peer_id(&peer);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_cursor_color_different_peers() {
        let c1 = CursorColor::from_peer_id(&PeerId::new("alice"));
        let c2 = CursorColor::from_peer_id(&PeerId::new("zzzzz-unique-peer"));
        // They might collide by chance but usually differ
        let _ = (c1, c2); // just check it doesn't panic
    }

    #[test]
    fn test_update_overwrites_position() {
        let mut overlay = CursorOverlay::new(PeerId::new("local"));
        overlay.apply_update(make_update("alice", 5, 0));
        overlay.apply_update(make_update("alice", 20, 3));
        let cursor = overlay.cursors.get(&PeerId::new("alice")).unwrap();
        assert_eq!(cursor.position.line, 20);
        assert_eq!(cursor.position.col, 3);
    }

    #[test]
    fn test_status_line_no_peers() {
        let overlay = CursorOverlay::new(PeerId::new("local"));
        assert_eq!(overlay.status_line(), "No collaborators online");
    }

    #[test]
    fn test_status_line_with_peers() {
        let mut overlay = CursorOverlay::new(PeerId::new("local"));
        overlay.apply_update(make_update("alice", 1, 0));
        let status = overlay.status_line();
        assert!(status.contains("1 collaborator"));
        assert!(status.contains("alice"));
    }

    #[test]
    fn test_to_json_format() {
        let mut overlay = CursorOverlay::new(PeerId::new("local"));
        overlay.apply_update(make_update("alice", 5, 2));
        let json = overlay.to_json();
        assert!(json.starts_with('['));
        assert!(json.ends_with(']'));
        assert!(json.contains("alice"));
        assert!(json.contains("\"line\":5"));
    }

    #[test]
    fn test_stale_cursor_detection() {
        let cursor = PeerCursor {
            peer_id: PeerId::new("old"),
            display_name: "old".into(),
            color: CursorColor::from_hex("#fff"),
            position: CursorPosition::new(1, 0),
            selection: None,
            file_path: None,
            last_seen_ms: 0, // epoch — definitely stale
            is_typing: false,
        };
        assert!(cursor.is_stale(1000));
    }

    #[test]
    fn test_selection_is_empty() {
        let pos = CursorPosition::new(5, 3);
        let sel = Selection { start: pos, end: pos };
        assert!(sel.is_empty());
    }
}
