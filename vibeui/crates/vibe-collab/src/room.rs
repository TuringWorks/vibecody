//! Collaboration room: Y.Doc per room, peer list, broadcast fan-out.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, RwLock};
use yrs::{Doc, GetString, ReadTxn, Transact, WriteTxn};

use crate::awareness::{color_for_peer, PeerInfo};
use crate::error::CollabError;
use crate::protocol;

/// A collaborative editing room.
///
/// Each room has a single `Y.Doc` that holds one `Y.Text` per file path.
/// Peers join the room, receive the current document state, and then
/// exchange incremental updates via the broadcast channel.
pub struct CollabRoom {
    /// Unique room identifier.
    pub id: String,
    /// The shared CRDT document.
    pub doc: Arc<RwLock<Doc>>,
    /// Connected peers keyed by peer_id.
    pub peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    /// Total number of peers that have ever joined (used for color assignment).
    peer_counter: Arc<std::sync::atomic::AtomicUsize>,
    /// Broadcast channel for binary Yjs sync messages.
    pub sync_tx: broadcast::Sender<SyncBroadcast>,
    /// Maximum allowed peers in this room.
    pub max_peers: usize,
}

/// A broadcast payload: the update bytes + the sender peer_id (so the sender can skip it).
#[derive(Debug, Clone)]
pub struct SyncBroadcast {
    pub sender_peer_id: String,
    pub data: Vec<u8>,
}

impl CollabRoom {
    /// Create a new room with the given ID and max peer limit.
    pub fn new(id: String, max_peers: usize) -> Self {
        let (sync_tx, _) = broadcast::channel(256);
        Self {
            id,
            doc: Arc::new(RwLock::new(Doc::new())),
            peers: Arc::new(RwLock::new(HashMap::new())),
            peer_counter: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            sync_tx,
            max_peers,
        }
    }

    /// Add a peer to the room. Returns the assigned PeerInfo.
    pub async fn add_peer(&self, peer_id: String, name: String) -> Result<PeerInfo, CollabError> {
        let mut peers = self.peers.write().await;
        if peers.len() >= self.max_peers {
            return Err(CollabError::RoomFull(self.max_peers));
        }

        let index = self
            .peer_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let color = color_for_peer(index).to_string();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let peer = PeerInfo {
            peer_id: peer_id.clone(),
            name,
            color,
            cursor: None,
            last_active: now,
        };

        peers.insert(peer_id, peer.clone());
        Ok(peer)
    }

    /// Remove a peer from the room. Returns true if the room is now empty.
    pub async fn remove_peer(&self, peer_id: &str) -> bool {
        let mut peers = self.peers.write().await;
        peers.remove(peer_id);
        peers.is_empty()
    }

    /// Get a snapshot of all connected peers.
    pub async fn list_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    /// Get or create a Y.Text for the given file path.
    pub async fn get_or_create_text(&self, file_path: &str) -> String {
        let doc = self.doc.read().await;
        let txn = doc.transact();
        match txn.get_text(file_path) {
            Some(text) => text.get_string(&txn),
            None => {
                drop(txn);
                drop(doc);
                // Need write access
                let doc = self.doc.write().await;
                let mut txn = doc.transact_mut();
                let text = txn.get_or_insert_text(file_path);
                text.get_string(&txn)
            }
        }
    }

    /// Encode the current document state as a SyncStep1 message.
    pub async fn encode_state(&self) -> Vec<u8> {
        let doc = self.doc.read().await;
        protocol::encode_sync_step1(&doc)
    }

    /// Apply a binary sync message from a peer. Returns an optional reply message.
    pub async fn apply_message(&self, msg: &[u8]) -> Result<Option<Vec<u8>>, CollabError> {
        let doc = self.doc.write().await;
        protocol::apply_sync_message(&doc, msg).map_err(|e| CollabError::YrsError(e))
    }

    /// Get the number of connected peers.
    pub async fn peer_count(&self) -> usize {
        let peers = self.peers.read().await;
        peers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yrs::Text;

    #[tokio::test]
    async fn test_room_lifecycle() {
        let room = CollabRoom::new("test-room".to_string(), 10);

        // Add peers
        let peer1 = room.add_peer("p1".to_string(), "Alice".to_string()).await.unwrap();
        assert_eq!(peer1.name, "Alice");
        assert!(!peer1.color.is_empty());

        let peer2 = room.add_peer("p2".to_string(), "Bob".to_string()).await.unwrap();
        assert_ne!(peer1.color, peer2.color);

        assert_eq!(room.peer_count().await, 2);

        // List peers
        let peers = room.list_peers().await;
        assert_eq!(peers.len(), 2);

        // Remove peer — room not empty
        let empty = room.remove_peer("p1").await;
        assert!(!empty);
        assert_eq!(room.peer_count().await, 1);

        // Remove last peer — room empty
        let empty = room.remove_peer("p2").await;
        assert!(empty);
    }

    #[tokio::test]
    async fn test_room_full() {
        let room = CollabRoom::new("small-room".to_string(), 2);
        room.add_peer("p1".to_string(), "A".to_string()).await.unwrap();
        room.add_peer("p2".to_string(), "B".to_string()).await.unwrap();

        let result = room.add_peer("p3".to_string(), "C".to_string()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            CollabError::RoomFull(max) => assert_eq!(max, 2),
            other => panic!("expected RoomFull, got: {other}"),
        }
    }

    #[tokio::test]
    async fn test_doc_sync() {
        let room = CollabRoom::new("sync-room".to_string(), 10);

        // Get or create text for a file
        let content = room.get_or_create_text("main.rs").await;
        assert_eq!(content, "");

        // Insert text directly
        {
            let doc = room.doc.write().await;
            let mut txn = doc.transact_mut();
            let text = txn.get_or_insert_text("main.rs");
            text.insert(&mut txn, 0, "fn main() {}");
        }

        // Verify
        let content = room.get_or_create_text("main.rs").await;
        assert_eq!(content, "fn main() {}");
    }

    #[tokio::test]
    async fn test_state_encode_and_apply() {
        let room_a = CollabRoom::new("a".to_string(), 10);
        let room_b = CollabRoom::new("b".to_string(), 10);

        // Insert text in room_a
        {
            let doc = room_a.doc.write().await;
            let mut txn = doc.transact_mut();
            let text = txn.get_or_insert_text("file.rs");
            text.insert(&mut txn, 0, "hello");
        }

        // room_b sends SyncStep1 to room_a
        let step1 = room_b.encode_state().await;
        let reply = room_a.apply_message(&step1).await.unwrap();
        assert!(reply.is_some());

        // room_b applies the reply (SyncStep2)
        let result = room_b.apply_message(&reply.unwrap()).await.unwrap();
        assert!(result.is_none());

        // Verify room_b has the text
        let content = room_b.get_or_create_text("file.rs").await;
        assert_eq!(content, "hello");
    }
}
