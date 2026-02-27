//! CollabServer: manages rooms via a concurrent DashMap registry.

use dashmap::DashMap;
use std::sync::Arc;

use crate::room::CollabRoom;

/// Central collaboration server that manages room lifecycle.
pub struct CollabServer {
    /// Concurrent room registry keyed by room_id.
    rooms: DashMap<String, Arc<CollabRoom>>,
    /// Maximum peers per room.
    max_peers_per_room: usize,
}

impl CollabServer {
    /// Create a new CollabServer.
    pub fn new(max_peers_per_room: usize) -> Self {
        Self {
            rooms: DashMap::new(),
            max_peers_per_room,
        }
    }

    /// Get an existing room or create a new one.
    pub fn get_or_create_room(&self, room_id: &str) -> Arc<CollabRoom> {
        self.rooms
            .entry(room_id.to_string())
            .or_insert_with(|| {
                tracing::info!(room_id, "creating new collab room");
                Arc::new(CollabRoom::new(
                    room_id.to_string(),
                    self.max_peers_per_room,
                ))
            })
            .value()
            .clone()
    }

    /// Get a room by ID, returning None if it doesn't exist.
    pub fn get_room(&self, room_id: &str) -> Option<Arc<CollabRoom>> {
        self.rooms.get(room_id).map(|r| r.value().clone())
    }

    /// List all active room IDs.
    pub fn list_rooms(&self) -> Vec<String> {
        self.rooms.iter().map(|r| r.key().clone()).collect()
    }

    /// Remove empty rooms. Returns the number of rooms cleaned up.
    pub async fn cleanup_empty_rooms(&self) -> usize {
        let mut to_remove = Vec::new();
        for entry in self.rooms.iter() {
            if entry.value().peer_count().await == 0 {
                to_remove.push(entry.key().clone());
            }
        }
        let count = to_remove.len();
        for room_id in to_remove {
            tracing::info!(room_id, "cleaning up empty room");
            self.rooms.remove(&room_id);
        }
        count
    }

    /// Remove a specific room.
    pub fn remove_room(&self, room_id: &str) -> bool {
        self.rooms.remove(room_id).is_some()
    }

    /// Get the total number of active rooms.
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_or_create_room() {
        let server = CollabServer::new(10);

        let room1 = server.get_or_create_room("room-1");
        assert_eq!(room1.id, "room-1");
        assert_eq!(server.room_count(), 1);

        // Getting the same room returns the same instance
        let room1_again = server.get_or_create_room("room-1");
        assert_eq!(room1_again.id, "room-1");
        assert_eq!(server.room_count(), 1);

        // Different room
        let _room2 = server.get_or_create_room("room-2");
        assert_eq!(server.room_count(), 2);
    }

    #[tokio::test]
    async fn test_list_rooms() {
        let server = CollabServer::new(10);
        server.get_or_create_room("alpha");
        server.get_or_create_room("beta");

        let rooms = server.list_rooms();
        assert_eq!(rooms.len(), 2);
        assert!(rooms.contains(&"alpha".to_string()));
        assert!(rooms.contains(&"beta".to_string()));
    }

    #[tokio::test]
    async fn test_cleanup_empty_rooms() {
        let server = CollabServer::new(10);
        let _room = server.get_or_create_room("empty-room");
        assert_eq!(server.room_count(), 1);

        // Room starts empty
        let cleaned = server.cleanup_empty_rooms().await;
        assert_eq!(cleaned, 1);
        assert_eq!(server.room_count(), 0);
    }

    #[tokio::test]
    async fn test_cleanup_preserves_occupied_rooms() {
        let server = CollabServer::new(10);
        let room = server.get_or_create_room("occupied");
        room.add_peer("p1".to_string(), "Alice".to_string())
            .await
            .unwrap();

        let cleaned = server.cleanup_empty_rooms().await;
        assert_eq!(cleaned, 0);
        assert_eq!(server.room_count(), 1);
    }

    #[tokio::test]
    async fn test_remove_room() {
        let server = CollabServer::new(10);
        server.get_or_create_room("to-remove");
        assert_eq!(server.room_count(), 1);

        let removed = server.remove_room("to-remove");
        assert!(removed);
        assert_eq!(server.room_count(), 0);

        let removed_again = server.remove_room("to-remove");
        assert!(!removed_again);
    }
}
