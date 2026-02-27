//! Collaboration protocol messages and Yjs sync helpers.

use serde::{Deserialize, Serialize};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{ReadTxn, Transact};
use crate::awareness::{AwarenessState, PeerInfo};

/// JSON messages sent over text WebSocket frames for session coordination.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CollabMessage {
    /// Server → Client: Welcome with room info and peer list.
    #[serde(rename = "welcome")]
    Welcome {
        room_id: String,
        peer_id: String,
        peers: Vec<PeerInfo>,
    },
    /// Server → Client: A new peer joined.
    #[serde(rename = "peer_joined")]
    PeerJoined { peer: PeerInfo },
    /// Server → Client: A peer left.
    #[serde(rename = "peer_left")]
    PeerLeft { peer_id: String },
    /// Client → Server: Open a file for collaborative editing.
    #[serde(rename = "file_opened")]
    FileOpened { file_path: String },
    /// Bidirectional: Awareness state (cursor, selection) update.
    #[serde(rename = "awareness")]
    Awareness(AwarenessState),
    /// Server → Client: Error message.
    #[serde(rename = "error")]
    Error { message: String },
}

// ── Yjs binary sync protocol helpers ────────────────────────────────────────

/// Message type tags for the Yjs binary sync protocol.
pub mod sync {
    /// SyncStep1: Client sends state vector to server.
    pub const SYNC_STEP1: u8 = 0;
    /// SyncStep2: Server responds with missing updates.
    pub const SYNC_STEP2: u8 = 1;
    /// Update: Incremental document update.
    pub const UPDATE: u8 = 2;
}

/// Encode a state vector from a Y.Doc into a SyncStep1 binary message.
pub fn encode_sync_step1(doc: &yrs::Doc) -> Vec<u8> {
    let txn = doc.transact();
    let sv = txn.state_vector().encode_v1();
    let mut buf = Vec::with_capacity(1 + sv.len());
    buf.push(sync::SYNC_STEP1);
    buf.extend_from_slice(&sv);
    buf
}

/// Encode a SyncStep2 response: compute diff from remote state vector.
pub fn encode_sync_step2(doc: &yrs::Doc, remote_sv: &[u8]) -> Result<Vec<u8>, String> {
    use yrs::StateVector;
    let sv = StateVector::decode_v1(remote_sv).map_err(|e| format!("decode state vector: {e}"))?;
    let txn = doc.transact();
    let update = txn.encode_diff_v1(&sv);
    let mut buf = Vec::with_capacity(1 + update.len());
    buf.push(sync::SYNC_STEP2);
    buf.extend_from_slice(&update);
    Ok(buf)
}

/// Encode an incremental update message.
pub fn encode_update(update: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(1 + update.len());
    buf.push(sync::UPDATE);
    buf.extend_from_slice(update);
    buf
}

/// Apply a received binary sync message to a Y.Doc.
/// Returns an optional update to broadcast (for SyncStep1 → SyncStep2 replies).
pub fn apply_sync_message(doc: &yrs::Doc, msg: &[u8]) -> Result<Option<Vec<u8>>, String> {
    if msg.is_empty() {
        return Err("empty sync message".to_string());
    }

    let msg_type = msg[0];
    let payload = &msg[1..];

    match msg_type {
        sync::SYNC_STEP1 => {
            // Remote sent their state vector — reply with our diff
            let reply = encode_sync_step2(doc, payload)?;
            Ok(Some(reply))
        }
        sync::SYNC_STEP2 | sync::UPDATE => {
            // Apply remote update to our doc
            let update = yrs::Update::decode_v1(payload).map_err(|e| format!("decode update: {e}"))?;
            let mut txn = doc.transact_mut();
            txn.apply_update(update).map_err(|e| format!("apply update: {e}"))?;
            Ok(None)
        }
        _ => Err(format!("unknown sync message type: {msg_type}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yrs::{GetString, Text, WriteTxn};

    #[test]
    fn test_collab_message_serialization() {
        let msg = CollabMessage::Welcome {
            room_id: "test-room".to_string(),
            peer_id: "peer-1".to_string(),
            peers: vec![],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"welcome\""));
        let deserialized: CollabMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            CollabMessage::Welcome { room_id, .. } => assert_eq!(room_id, "test-room"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_sync_step1_encode() {
        let doc = yrs::Doc::new();
        let msg = encode_sync_step1(&doc);
        assert_eq!(msg[0], sync::SYNC_STEP1);
        assert!(msg.len() > 1);
    }

    #[test]
    fn test_sync_roundtrip() {
        // Create two docs and sync them
        let doc_a = yrs::Doc::new();
        let doc_b = yrs::Doc::new();

        // Insert text in doc_a
        {
            let mut txn = doc_a.transact_mut();
            let text = txn.get_or_insert_text("test-file");
            text.insert(&mut txn, 0, "hello world");
        }

        // SyncStep1: doc_b sends its state vector
        let step1 = encode_sync_step1(&doc_b);

        // SyncStep2: doc_a processes step1 and sends back diff
        let step2 = apply_sync_message(&doc_a, &step1).unwrap().unwrap();
        assert_eq!(step2[0], sync::SYNC_STEP2);

        // doc_b applies the diff
        let result = apply_sync_message(&doc_b, &step2).unwrap();
        assert!(result.is_none()); // SyncStep2 doesn't produce a reply

        // Verify doc_b now has the text
        let txn = doc_b.transact();
        let text = txn.get_text("test-file").unwrap();
        assert_eq!(text.get_string(&txn), "hello world");
    }

    #[test]
    fn test_incremental_update() {
        let doc_a = yrs::Doc::new();
        let doc_b = yrs::Doc::new();

        // First sync both docs
        let step1 = encode_sync_step1(&doc_b);
        let step2 = apply_sync_message(&doc_a, &step1).unwrap();
        if let Some(s2) = step2 {
            let _ = apply_sync_message(&doc_b, &s2);
        }

        // Now make an edit in doc_a and capture the update
        let update = {
            let mut txn = doc_a.transact_mut();
            let text = txn.get_or_insert_text("file.rs");
            text.insert(&mut txn, 0, "fn main() {}");
            txn.encode_update_v1()
        };

        // Send as incremental update
        let update_msg = encode_update(&update);
        assert_eq!(update_msg[0], sync::UPDATE);

        let result = apply_sync_message(&doc_b, &update_msg).unwrap();
        assert!(result.is_none());

        // Verify
        let txn = doc_b.transact();
        let text = txn.get_text("file.rs").unwrap();
        assert_eq!(text.get_string(&txn), "fn main() {}");
    }
}
