//! The `.iwa` archive stream inside an iWork document.
//!
//! A stream is a run of archives. Each archive is a length-prefixed
//! `TSP.ArchiveInfo` followed by one payload per message the info lists:
//!
//! ```text
//! varint(len)  ArchiveInfo[len]  payload[info.messages[0].length]  payload[…]
//! ```
//!
//! `ArchiveInfo` is read with the generic walker, so the only two things this
//! module claims to know about Apple's schema are the field numbers for an
//! archive's identifier and for a message's type and payload length.

use super::protobuf::{read_varint, write_varint, Message, Value};
use crate::error::DocError;

/// `TSP.ArchiveInfo.identifier`
const INFO_IDENTIFIER: u32 = 1;
/// `TSP.ArchiveInfo.message_infos`
const INFO_MESSAGES: u32 = 2;
/// `TSP.MessageInfo.type`
const MESSAGE_TYPE: u32 = 1;
/// `TSP.MessageInfo.length`
const MESSAGE_LENGTH: u32 = 3;

/// One message inside an archive.
#[derive(Debug, Clone)]
pub struct ArchivedMessage {
    /// Apple's numeric message type, e.g. 2001 for `TSWP.StorageArchive`.
    pub type_id: u64,
    /// The `MessageInfo` describing this payload, kept so unknown fields survive.
    pub info: Message,
    /// The payload's wire bytes.
    pub payload: Vec<u8>,
}

/// One archive in a stream.
#[derive(Debug, Clone)]
pub struct Archive {
    pub identifier: u64,
    /// The `ArchiveInfo`, minus its message infos (held on the messages).
    pub info: Message,
    pub messages: Vec<ArchivedMessage>,
}

/// Parse an uncompressed `.iwa` stream.
pub fn parse_stream(data: &[u8]) -> Result<Vec<Archive>, DocError> {
    let mut archives = Vec::new();
    let mut offset = 0;

    while offset < data.len() {
        let (info_len, consumed) = read_varint(&data[offset..])?;
        offset += consumed;
        let info_end = offset.checked_add(info_len as usize).ok_or_else(|| {
            DocError::Parse("IWA archive info length overflows the stream".to_string())
        })?;
        if info_end > data.len() {
            return Err(DocError::Parse(format!(
                "IWA archive info wants {info_len} bytes but only {} remain",
                data.len() - offset
            )));
        }
        let info = Message::parse(&data[offset..info_end])?;
        offset = info_end;

        let identifier = info.varint(INFO_IDENTIFIER).unwrap_or(0);
        let message_infos: Vec<Message> = info
            .bytes_values(INFO_MESSAGES)
            .into_iter()
            .map(|bytes| Message::parse(bytes))
            .collect::<Result<_, _>>()?;

        let mut messages = Vec::with_capacity(message_infos.len());
        for message_info in message_infos {
            let length = message_info.varint(MESSAGE_LENGTH).ok_or_else(|| {
                DocError::Parse("IWA message info carries no payload length".to_string())
            })? as usize;
            let end = offset.checked_add(length).ok_or_else(|| {
                DocError::Parse("IWA payload length overflows the stream".to_string())
            })?;
            if end > data.len() {
                return Err(DocError::Parse(format!(
                    "IWA payload wants {length} bytes but only {} remain",
                    data.len() - offset
                )));
            }
            messages.push(ArchivedMessage {
                type_id: message_info.varint(MESSAGE_TYPE).unwrap_or(0),
                info: message_info,
                payload: data[offset..end].to_vec(),
            });
            offset = end;
        }

        // Strip the message infos: they are rebuilt from `messages` on the way
        // out, so a payload whose length changed cannot disagree with its info.
        let mut trimmed = info;
        trimmed.fields.retain(|(n, v)| !(*n == INFO_MESSAGES && matches!(v, Value::Bytes(_))));
        archives.push(Archive { identifier, info: trimmed, messages });
    }
    Ok(archives)
}

/// Serialize archives back into an uncompressed `.iwa` stream.
pub fn serialize_stream(archives: &[Archive]) -> Vec<u8> {
    let mut out = Vec::new();
    for archive in archives {
        let mut info = archive.info.clone();
        // Message infos go back where they came from: after the identifier and
        // before any trailing fields the walker preserved.
        let insert_at = info
            .fields
            .iter()
            .position(|(n, _)| *n > INFO_IDENTIFIER)
            .unwrap_or(info.fields.len());
        for (k, message) in archive.messages.iter().enumerate() {
            let mut message_info = message.info.clone();
            message_info.set_varint(MESSAGE_LENGTH, message.payload.len() as u64);
            info.fields
                .insert(insert_at + k, (INFO_MESSAGES, Value::Bytes(message_info.serialize())));
        }
        let info_bytes = info.serialize();
        write_varint(&mut out, info_bytes.len() as u64);
        out.extend_from_slice(&info_bytes);
        for message in &archive.messages {
            out.extend_from_slice(&message.payload);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a stream the way Apple's writer lays one out.
    fn stream(payloads: &[(u64, &[u8])]) -> Vec<u8> {
        let messages: Vec<ArchivedMessage> = payloads
            .iter()
            .map(|(type_id, payload)| {
                let mut info = Message::default();
                info.set_varint(MESSAGE_TYPE, *type_id);
                info.set_varint(MESSAGE_LENGTH, payload.len() as u64);
                ArchivedMessage { type_id: *type_id, info, payload: payload.to_vec() }
            })
            .collect();
        let mut info = Message::default();
        info.set_varint(INFO_IDENTIFIER, 42);
        serialize_stream(&[Archive { identifier: 42, info, messages }])
    }

    #[test]
    fn round_trips_a_stream() {
        let bytes = stream(&[(2001, b"first-payload"), (2002, b"second")]);
        let archives = parse_stream(&bytes).expect("parse");
        assert_eq!(archives.len(), 1);
        assert_eq!(archives[0].identifier, 42);
        assert_eq!(archives[0].messages.len(), 2);
        assert_eq!(archives[0].messages[0].type_id, 2001);
        assert_eq!(archives[0].messages[1].payload, b"second");
        assert_eq!(serialize_stream(&archives), bytes, "re-serialization is byte-identical");
    }

    #[test]
    fn a_longer_payload_updates_its_declared_length() {
        let bytes = stream(&[(2001, b"short")]);
        let mut archives = parse_stream(&bytes).expect("parse");
        archives[0].messages[0].payload = b"much longer payload".to_vec();

        let rewritten = serialize_stream(&archives);
        let reparsed = parse_stream(&rewritten).expect("re-parse");
        assert_eq!(reparsed[0].messages[0].payload, b"much longer payload");
    }

    #[test]
    fn rejects_a_stream_that_claims_more_than_it_holds() {
        let mut bytes = stream(&[(2001, b"payload")]);
        bytes.truncate(bytes.len() - 3);
        let err = parse_stream(&bytes).expect_err("truncated stream");
        assert_eq!(err.kind(), "parse");
    }
}
