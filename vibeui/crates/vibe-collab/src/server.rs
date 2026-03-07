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

    #[tokio::test]
    async fn test_server_creation_defaults() {
        let server = CollabServer::new(5);
        assert_eq!(server.room_count(), 0);
        assert!(server.list_rooms().is_empty());
    }

    #[tokio::test]
    async fn test_get_room_returns_none_for_missing() {
        let server = CollabServer::new(10);
        assert!(server.get_room("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_get_room_returns_some_for_existing() {
        let server = CollabServer::new(10);
        server.get_or_create_room("existing");
        let room = server.get_room("existing");
        assert!(room.is_some());
        assert_eq!(room.unwrap().id, "existing");
    }

    #[tokio::test]
    async fn test_get_or_create_room_returns_same_arc() {
        let server = CollabServer::new(10);
        let room1 = server.get_or_create_room("shared");
        let room2 = server.get_or_create_room("shared");
        // Both should point to the same underlying allocation
        assert!(Arc::ptr_eq(&room1, &room2));
    }

    #[tokio::test]
    async fn test_room_inherits_max_peers_from_server() {
        let server = CollabServer::new(3);
        let room = server.get_or_create_room("limited");
        assert_eq!(room.max_peers, 3);

        // Verify the limit is enforced
        room.add_peer("p1".to_string(), "A".to_string()).await.unwrap();
        room.add_peer("p2".to_string(), "B".to_string()).await.unwrap();
        room.add_peer("p3".to_string(), "C".to_string()).await.unwrap();
        let result = room.add_peer("p4".to_string(), "D".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_mixed_rooms() {
        let server = CollabServer::new(10);

        // Create an occupied room
        let occupied = server.get_or_create_room("occupied");
        occupied.add_peer("p1".to_string(), "Alice".to_string()).await.unwrap();

        // Create two empty rooms
        server.get_or_create_room("empty1");
        server.get_or_create_room("empty2");
        assert_eq!(server.room_count(), 3);

        let cleaned = server.cleanup_empty_rooms().await;
        assert_eq!(cleaned, 2);
        assert_eq!(server.room_count(), 1);

        // The occupied room should still be accessible
        assert!(server.get_room("occupied").is_some());
        assert!(server.get_room("empty1").is_none());
        assert!(server.get_room("empty2").is_none());
    }

    #[tokio::test]
    async fn test_remove_room_then_recreate() {
        let server = CollabServer::new(10);
        let room1 = server.get_or_create_room("recycled");
        room1.add_peer("p1".to_string(), "Alice".to_string()).await.unwrap();
        assert_eq!(room1.peer_count().await, 1);

        server.remove_room("recycled");
        assert_eq!(server.room_count(), 0);

        // Re-create — should be a fresh room with no peers
        let room2 = server.get_or_create_room("recycled");
        assert_eq!(room2.peer_count().await, 0);
        assert!(!Arc::ptr_eq(&room1, &room2));
    }

    #[tokio::test]
    async fn test_many_rooms() {
        let server = CollabServer::new(10);
        for i in 0..50 {
            server.get_or_create_room(&format!("room-{i}"));
        }
        assert_eq!(server.room_count(), 50);
        assert_eq!(server.list_rooms().len(), 50);
    }

    #[tokio::test]
    async fn test_remove_nonexistent_room_returns_false() {
        let server = CollabServer::new(10);
        assert!(!server.remove_room("ghost"));
    }

    #[tokio::test]
    async fn test_cleanup_empty_on_empty_server() {
        let server = CollabServer::new(10);
        let cleaned = server.cleanup_empty_rooms().await;
        assert_eq!(cleaned, 0);
    }

    #[tokio::test]
    async fn test_room_lifecycle_create_use_cleanup() {
        let server = CollabServer::new(10);

        // Create room and add peers
        let room = server.get_or_create_room("lifecycle");
        room.add_peer("p1".to_string(), "Alice".to_string()).await.unwrap();
        room.add_peer("p2".to_string(), "Bob".to_string()).await.unwrap();

        // Cleanup should not remove it
        let cleaned = server.cleanup_empty_rooms().await;
        assert_eq!(cleaned, 0);

        // Remove peers
        room.remove_peer("p1").await;
        room.remove_peer("p2").await;

        // Now cleanup should remove it
        let cleaned = server.cleanup_empty_rooms().await;
        assert_eq!(cleaned, 1);
        assert_eq!(server.room_count(), 0);
    }
}
