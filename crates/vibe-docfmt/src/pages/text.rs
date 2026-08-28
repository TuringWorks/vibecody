//! Finding and replacing the text inside iWork archives.
//!
//! Apple publishes no schema, so nothing here is asserted from a table of field
//! names. A text storage is recognised by shape: a message of the text-storage
//! type holding a length-delimited field whose every value is human text. The
//! field number that was used is reported back so it appears in the UI rather
//! than living as a hidden assumption.

use similar::{ChangeTag, TextDiff};

use super::iwa::Archive;
use super::protobuf::{Message, Value};
use crate::error::DocError;

/// `TSWP.StorageArchive` — the message type that holds word-processing text in
/// Pages, Keynote and Numbers alike.
pub const TEXT_STORAGE_TYPE: u64 = 2001;

/// How deep to look for character-index tables inside a storage.
const MAX_DEPTH: u8 = 12;

/// A run of text found in an archive, and where it came from.
#[derive(Debug, Clone)]
pub struct Storage {
    /// Entry name of the `.iwa` this came from.
    pub iwa: String,
    /// Index of the archive within that file's stream.
    pub archive_index: usize,
    /// Index of the message within that archive.
    pub message_index: usize,
    /// Apple's archive identifier, used to build a stable id.
    pub archive_id: u64,
    /// The field number the text was found in.
    pub field: u32,
    /// The text, one entry per wire value.
    pub chunks: Vec<String>,
    /// True when the field was picked by shape rather than by the known type.
    pub guessed: bool,
}

impl Storage {
    /// Stable identifier used in the buffer's storage markers.
    pub fn id(&self) -> String {
        format!("{}:{}:{}", self.iwa, self.archive_id, self.message_index)
    }

    pub fn text(&self) -> String {
        self.chunks.concat()
    }
}

/// Find every text storage in one `.iwa` stream.
pub fn find_storages(iwa: &str, archives: &[Archive]) -> Vec<Storage> {
    let typed: Vec<Storage> = collect(
        iwa,
        archives,
        &|type_id| type_id == TEXT_STORAGE_TYPE,
        false,
    );
    if !typed.is_empty() {
        return typed;
    }
    // No message of the known type: fall back to shape alone, and say so.
    collect(iwa, archives, &|_| true, true)
}

fn collect(
    iwa: &str,
    archives: &[Archive],
    type_matches: &dyn Fn(u64) -> bool,
    guessed: bool,
) -> Vec<Storage> {
    archives
        .iter()
        .enumerate()
        .flat_map(|(archive_index, archive)| {
            archive
                .messages
                .iter()
                .enumerate()
                .filter_map(move |(message_index, message)| {
                    if !type_matches(message.type_id) {
                        return None;
                    }
                    let parsed = Message::parse(&message.payload).ok()?;
                    let (field, chunks) = text_field(&parsed, guessed)?;
                    Some(Storage {
                        iwa: iwa.to_string(),
                        archive_index,
                        message_index,
                        archive_id: archive.identifier,
                        field,
                        chunks,
                        guessed,
                    })
                })
        })
        .collect()
}

/// Pick the length-delimited field that holds the message's text.
///
/// `strict` raises the bar for the fallback scan: without a known message type,
/// a short string is as likely to be a style name as a sentence.
fn text_field(message: &Message, strict: bool) -> Option<(u32, Vec<String>)> {
    let min_len = if strict { 24 } else { 1 };
    message
        .bytes_field_numbers()
        .into_iter()
        .filter_map(|number| {
            let values = message.bytes_values(number);
            let chunks: Vec<String> = values
                .iter()
                .map(|bytes| std::str::from_utf8(bytes).map(str::to_string))
                .collect::<Result<_, _>>()
                .ok()?;
            let total: usize = chunks.iter().map(String::len).sum();
            (total >= min_len && chunks.iter().all(|c| is_texty(c)))
                .then_some((number, chunks, total))
        })
        .max_by_key(|(_, _, total)| *total)
        .map(|(number, chunks, _)| (number, chunks))
}

/// Whether a string reads as document text rather than as packed binary that
/// happens to decode as UTF-8.
fn is_texty(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    text.chars()
        .all(|c| matches!(c, '\n' | '\r' | '\t' | '\u{2028}' | '\u{2029}') || !c.is_control())
}

/// Report of one text substitution.
#[derive(Debug, Clone, Copy, Default)]
pub struct EditReport {
    /// How many character-index entries were shifted to follow the new text.
    pub remapped_indices: usize,
    /// True when the text length changed, so style ranges had to move.
    pub length_changed: bool,
}

/// Replace a storage's text, keeping every other field of the message.
pub fn set_text(
    archives: &mut [Archive],
    storage: &Storage,
    new_text: &str,
) -> Result<EditReport, DocError> {
    let archive = archives.get_mut(storage.archive_index).ok_or_else(|| {
        DocError::Structure(format!(
            "archive {} vanished mid-write",
            storage.archive_index
        ))
    })?;
    let message = archive
        .messages
        .get_mut(storage.message_index)
        .ok_or_else(|| {
            DocError::Structure(format!(
                "message {} vanished mid-write",
                storage.message_index
            ))
        })?;
    let mut parsed = Message::parse(&message.payload)?;

    let old_text = storage.text();
    let mut report = EditReport {
        remapped_indices: 0,
        length_changed: old_text != new_text,
    };

    // Keep the original chunking: the boundaries move with the text.
    let chunks = rechunk(&old_text, new_text, &storage.chunks);
    parsed.replace_bytes(
        storage.field,
        chunks.iter().map(|c| c.as_bytes().to_vec()).collect(),
    );

    if old_text.chars().count() != new_text.chars().count() {
        let map = offset_map(&old_text, new_text);
        let old_len = old_text.chars().count();
        report.remapped_indices = remap_indices(&mut parsed, storage.field, &map, old_len, 0);
    }

    message.payload = parsed.serialize();
    Ok(report)
}

/// Split the new text at the boundaries the old chunks had, so a document that
/// stored its text in several pieces keeps that shape.
fn rechunk(old_text: &str, new_text: &str, old_chunks: &[String]) -> Vec<String> {
    if old_chunks.len() <= 1 {
        return vec![new_text.to_string()];
    }
    let map = offset_map(old_text, new_text);
    let new_chars: Vec<char> = new_text.chars().collect();

    let mut out = Vec::with_capacity(old_chunks.len());
    let mut old_pos = 0usize;
    let mut new_pos = 0usize;
    for (i, chunk) in old_chunks.iter().enumerate() {
        old_pos += chunk.chars().count();
        let end = if i + 1 == old_chunks.len() {
            new_chars.len()
        } else {
            map.get(old_pos)
                .copied()
                .unwrap_or(new_chars.len())
                .clamp(new_pos, new_chars.len())
        };
        out.push(new_chars[new_pos..end].iter().collect());
        new_pos = end;
    }
    out
}

/// Map each character index in the old text to its index in the new text.
///
/// The map has `old_len + 1` entries so an end-of-text index maps too.
fn offset_map(old_text: &str, new_text: &str) -> Vec<usize> {
    let old_len = old_text.chars().count();
    let mut map = Vec::with_capacity(old_len + 1);
    let diff = TextDiff::from_chars(old_text, new_text);
    let mut new_index = 0usize;
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                map.push(new_index);
                new_index += 1;
            }
            // A deleted character collapses onto the position that replaced it.
            ChangeTag::Delete => map.push(new_index),
            ChangeTag::Insert => new_index += 1,
        }
    }
    map.push(new_index);
    map.truncate(old_len + 1);
    while map.len() < old_len + 1 {
        map.push(new_index);
    }
    map
}

/// Shift character-index tables so style, list and attachment ranges still
/// point at the text they described.
///
/// A table is only rewritten when its shape leaves little room for doubt: two
/// or more entries, each a message whose first field is a varint, strictly
/// increasing, and none past the end of the old text. Anything else is left
/// alone — a stale range is recoverable, a corrupted archive is not.
fn remap_indices(
    message: &mut Message,
    skip_field: u32,
    map: &[usize],
    old_len: usize,
    depth: u8,
) -> usize {
    if depth >= MAX_DEPTH {
        return 0;
    }
    let mut remapped = 0;

    for number in message.bytes_field_numbers() {
        if number == skip_field && depth == 0 {
            continue;
        }
        let values: Vec<Vec<u8>> = message.bytes_values(number).into_iter().cloned().collect();
        let parsed: Option<Vec<Message>> = values.iter().map(|b| Message::parse(b).ok()).collect();
        let Some(mut children) = parsed else { continue };

        let mut changed = false;
        if is_index_table(&children, old_len) {
            for child in children.iter_mut() {
                if let Some(index) = child.varint(1) {
                    let mapped = map
                        .get(index as usize)
                        .copied()
                        .unwrap_or(*map.last().unwrap_or(&0));
                    if mapped as u64 != index {
                        child.set_varint(1, mapped as u64);
                        changed = true;
                        remapped += 1;
                    }
                }
            }
        }
        for child in children.iter_mut() {
            let nested = remap_indices(child, skip_field, map, old_len, depth + 1);
            if nested > 0 {
                changed = true;
                remapped += nested;
            }
        }
        if changed {
            message.replace_bytes(number, children.iter().map(Message::serialize).collect());
        }
    }
    remapped
}

/// Whether a repeated submessage looks like a character-index table.
fn is_index_table(children: &[Message], old_len: usize) -> bool {
    if children.len() < 2 {
        return false;
    }
    let indices: Option<Vec<u64>> = children.iter().map(|c| c.varint(1)).collect();
    let Some(indices) = indices else { return false };
    if indices.iter().any(|i| *i as usize > old_len) {
        return false;
    }
    if indices.windows(2).any(|w| w[0] >= w[1]) {
        return false;
    }
    // A table starts at the beginning of the text; a list of unrelated varints
    // usually does not.
    indices.first() == Some(&0)
}

/// Whether a message parses as protobuf at all — used by tests and by the
/// storage scan to avoid treating packed binary as text.
pub fn parses_as_message(bytes: &[u8]) -> bool {
    Message::parse(bytes).is_ok()
}

/// Build a text-storage payload, for tests and fixtures.
pub fn build_storage_payload(field: u32, chunks: &[&str]) -> Vec<u8> {
    let mut message = Message::default();
    for chunk in chunks {
        message
            .fields
            .push((field, Value::Bytes(chunk.as_bytes().to_vec())));
    }
    message.serialize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_follow_an_insertion() {
        let map = offset_map("abcd", "abXcd");
        assert_eq!(map[0], 0);
        assert_eq!(map[2], 3, "index of 'c' moved past the inserted character");
        assert_eq!(map[4], 5, "end of text maps to the new end");
    }

    #[test]
    fn offsets_follow_a_deletion() {
        let map = offset_map("abcd", "acd");
        assert_eq!(map[2], 1, "'c' moved back one");
        assert_eq!(map.len(), 5);
    }

    #[test]
    fn an_index_table_is_recognised_only_when_its_shape_is_unambiguous() {
        let entry = |index: u64| {
            let mut m = Message::default();
            m.set_varint(1, index);
            m
        };
        assert!(is_index_table(&[entry(0), entry(4), entry(9)], 20));
        assert!(
            !is_index_table(&[entry(0)], 20),
            "a single entry is not enough"
        );
        assert!(
            !is_index_table(&[entry(3), entry(9)], 20),
            "a table starts at 0"
        );
        assert!(
            !is_index_table(&[entry(0), entry(4)], 3),
            "indices past the text are not a table"
        );
        assert!(
            !is_index_table(&[entry(0), entry(0)], 20),
            "indices must increase"
        );
    }

    #[test]
    fn text_is_replaced_and_style_ranges_follow() {
        // A storage holding "hello world" plus a style table at 0 and 6.
        let mut payload = Message::default();
        payload
            .fields
            .push((3, Value::Bytes(b"hello world".to_vec())));
        let mut table = Message::default();
        for index in [0u64, 6] {
            let mut entry = Message::default();
            entry.set_varint(1, index);
            entry.set_varint(2, 77);
            table.fields.push((1, Value::Bytes(entry.serialize())));
        }
        payload.fields.push((5, Value::Bytes(table.serialize())));

        let mut archives = vec![Archive {
            identifier: 7,
            info: Message::default(),
            messages: vec![super::super::iwa::ArchivedMessage {
                type_id: TEXT_STORAGE_TYPE,
                info: Message::default(),
                payload: payload.serialize(),
            }],
        }];
        let storages = find_storages("Index/Document.iwa", &archives);
        assert_eq!(storages.len(), 1);
        assert_eq!(storages[0].text(), "hello world");
        assert_eq!(storages[0].field, 3);

        let report = set_text(&mut archives, &storages[0], "hello there world").expect("set");
        assert!(report.length_changed);
        assert_eq!(report.remapped_indices, 1, "the second range moved");

        let after = find_storages("Index/Document.iwa", &archives);
        assert_eq!(after[0].text(), "hello there world");

        let parsed = Message::parse(&archives[0].messages[0].payload).expect("parse");
        let table_bytes = parsed.bytes_values(5)[0].clone();
        let table = Message::parse(&table_bytes).expect("table");
        let indices: Vec<u64> = table
            .bytes_values(1)
            .into_iter()
            .filter_map(|b| Message::parse(b).ok().and_then(|m| m.varint(1)))
            .collect();
        assert_eq!(indices, vec![0, 12], "'world' moved from 6 to 12");
    }

    #[test]
    fn unrelated_fields_survive_a_text_edit() {
        let mut payload = Message::default();
        payload.set_varint(2, 9);
        payload.fields.push((3, Value::Bytes(b"text".to_vec())));
        payload.fields.push((9, Value::Fixed32([4, 3, 2, 1])));

        let mut archives = vec![Archive {
            identifier: 1,
            info: Message::default(),
            messages: vec![super::super::iwa::ArchivedMessage {
                type_id: TEXT_STORAGE_TYPE,
                info: Message::default(),
                payload: payload.serialize(),
            }],
        }];
        let storages = find_storages("Index/Document.iwa", &archives);
        set_text(&mut archives, &storages[0], "different text").expect("set");

        let parsed = Message::parse(&archives[0].messages[0].payload).expect("parse");
        assert_eq!(parsed.varint(2), Some(9));
        assert!(parsed
            .fields
            .iter()
            .any(|(n, v)| *n == 9 && matches!(v, Value::Fixed32(_))));
    }

    #[test]
    fn chunk_boundaries_are_kept() {
        let chunks = vec!["hello ".to_string(), "world".to_string()];
        let out = rechunk("hello world", "hello brave world", &chunks);
        assert_eq!(out.len(), 2);
        assert_eq!(out.concat(), "hello brave world");
    }
}
