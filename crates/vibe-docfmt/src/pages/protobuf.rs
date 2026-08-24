//! A protobuf wire-format walker that keeps what it does not understand.
//!
//! Apple ships no schema for the messages inside a Pages document, and this
//! crate deliberately does not embed a reverse-engineered one: guessing a field
//! *name* wrong is a silent corruption. Instead every field is kept as its raw
//! wire value, nested messages stay as opaque bytes until something needs to
//! look inside, and re-serialization emits exactly the fields that were read,
//! in order. Only a message this crate actually edits is ever rebuilt.

use crate::error::DocError;

/// A protobuf wire value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Varint(u64),
    Fixed64([u8; 8]),
    Bytes(Vec<u8>),
    Fixed32([u8; 4]),
}

impl Value {
    fn wire_type(&self) -> u8 {
        match self {
            Value::Varint(_) => 0,
            Value::Fixed64(_) => 1,
            Value::Bytes(_) => 2,
            Value::Fixed32(_) => 5,
        }
    }
}

/// A protobuf message: an ordered list of (field number, value).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Message {
    pub fields: Vec<(u32, Value)>,
}

impl Message {
    /// Parse a message from wire bytes.
    pub fn parse(data: &[u8]) -> Result<Message, DocError> {
        let mut fields = Vec::new();
        let mut cursor = Cursor::new(data);
        while !cursor.done() {
            let key = cursor.varint()?;
            let number = (key >> 3) as u32;
            let wire = (key & 0x07) as u8;
            if number == 0 {
                return Err(DocError::Parse("protobuf field number 0".into()));
            }
            let value = match wire {
                0 => Value::Varint(cursor.varint()?),
                1 => Value::Fixed64(cursor.fixed::<8>()?),
                2 => {
                    let len = cursor.varint()? as usize;
                    Value::Bytes(cursor.take(len)?.to_vec())
                }
                5 => Value::Fixed32(cursor.fixed::<4>()?),
                // Groups (3/4) were removed from proto3 and do not appear in
                // iWork archives; refusing beats mis-parsing the rest.
                other => {
                    return Err(DocError::Parse(format!("unsupported protobuf wire type {other}")))
                }
            };
            fields.push((number, value));
        }
        Ok(Message { fields })
    }

    /// Serialize back to wire bytes.
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (number, value) in &self.fields {
            write_varint(&mut out, (u64::from(*number) << 3) | u64::from(value.wire_type()));
            match value {
                Value::Varint(v) => write_varint(&mut out, *v),
                Value::Fixed64(b) => out.extend_from_slice(b),
                Value::Fixed32(b) => out.extend_from_slice(b),
                Value::Bytes(b) => {
                    write_varint(&mut out, b.len() as u64);
                    out.extend_from_slice(b);
                }
            }
        }
        out
    }

    /// First varint value for a field number.
    pub fn varint(&self, number: u32) -> Option<u64> {
        self.fields.iter().find_map(|(n, v)| match v {
            Value::Varint(value) if *n == number => Some(*value),
            _ => None,
        })
    }

    /// All length-delimited values for a field number, in order.
    pub fn bytes_values(&self, number: u32) -> Vec<&Vec<u8>> {
        self.fields
            .iter()
            .filter_map(|(n, v)| match v {
                Value::Bytes(b) if *n == number => Some(b),
                _ => None,
            })
            .collect()
    }

    /// Field numbers that carry length-delimited values, in first-seen order.
    pub fn bytes_field_numbers(&self) -> Vec<u32> {
        let mut seen = Vec::new();
        for (number, value) in &self.fields {
            if matches!(value, Value::Bytes(_)) && !seen.contains(number) {
                seen.push(*number);
            }
        }
        seen
    }

    /// Replace every length-delimited value of `number` with `values`, keeping
    /// the position of the first occurrence.
    pub fn replace_bytes(&mut self, number: u32, values: Vec<Vec<u8>>) {
        let at = self
            .fields
            .iter()
            .position(|(n, v)| *n == number && matches!(v, Value::Bytes(_)))
            .unwrap_or(self.fields.len());
        self.fields.retain(|(n, v)| !(*n == number && matches!(v, Value::Bytes(_))));
        let at = at.min(self.fields.len());
        for (k, value) in values.into_iter().enumerate() {
            self.fields.insert(at + k, (number, Value::Bytes(value)));
        }
    }

    /// Set the first varint value of a field, appending if absent.
    pub fn set_varint(&mut self, number: u32, value: u64) {
        match self.fields.iter_mut().find(|(n, v)| *n == number && matches!(v, Value::Varint(_))) {
            Some(slot) => slot.1 = Value::Varint(value),
            None => self.fields.push((number, Value::Varint(value))),
        }
    }
}

struct Cursor<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Cursor { data, offset: 0 }
    }

    fn done(&self) -> bool {
        self.offset >= self.data.len()
    }

    fn varint(&mut self) -> Result<u64, DocError> {
        let mut value = 0u64;
        let mut shift = 0;
        loop {
            let byte = *self
                .data
                .get(self.offset)
                .ok_or_else(|| DocError::Parse("protobuf varint runs past the end".into()))?;
            self.offset += 1;
            value |= u64::from(byte & 0x7F) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
            if shift >= 64 {
                return Err(DocError::Parse("protobuf varint is longer than 64 bits".into()));
            }
        }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], DocError> {
        let end = self.offset.checked_add(len).ok_or_else(|| {
            DocError::Parse("protobuf length-delimited field overflows".to_string())
        })?;
        if end > self.data.len() {
            return Err(DocError::Parse(format!(
                "protobuf field wants {len} bytes but only {} remain",
                self.data.len() - self.offset
            )));
        }
        let slice = &self.data[self.offset..end];
        self.offset = end;
        Ok(slice)
    }

    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], DocError> {
        let slice = self.take(N)?;
        let mut out = [0u8; N];
        out.copy_from_slice(slice);
        Ok(out)
    }
}

/// Append a base-128 varint.
pub fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// Read a varint from the front of `data`, returning it and the bytes consumed.
pub fn read_varint(data: &[u8]) -> Result<(u64, usize), DocError> {
    let mut cursor = Cursor::new(data);
    let value = cursor.varint()?;
    Ok((value, cursor.offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<u8> {
        let mut message = Message::default();
        message.fields.push((1, Value::Varint(150)));
        message.fields.push((3, Value::Bytes(b"hello".to_vec())));
        message.fields.push((3, Value::Bytes(b"world".to_vec())));
        message.fields.push((7, Value::Fixed32([1, 2, 3, 4])));
        message.serialize()
    }

    #[test]
    fn round_trips_bytes_exactly() {
        let bytes = sample();
        let parsed = Message::parse(&bytes).expect("parse");
        assert_eq!(parsed.serialize(), bytes, "re-serialization is byte-identical");
    }

    #[test]
    fn keeps_unknown_fields_when_one_field_is_replaced() {
        let bytes = sample();
        let mut parsed = Message::parse(&bytes).expect("parse");
        parsed.replace_bytes(3, vec![b"edited".to_vec()]);
        let again = Message::parse(&parsed.serialize()).expect("re-parse");

        assert_eq!(again.varint(1), Some(150), "unrelated varint survived");
        assert_eq!(again.bytes_values(3), vec![&b"edited".to_vec()]);
        assert!(
            again.fields.iter().any(|(n, v)| *n == 7 && matches!(v, Value::Fixed32([1, 2, 3, 4]))),
            "unrelated fixed32 survived"
        );
    }

    #[test]
    fn replacement_keeps_field_order() {
        let bytes = sample();
        let mut parsed = Message::parse(&bytes).expect("parse");
        parsed.replace_bytes(3, vec![b"a".to_vec(), b"b".to_vec()]);
        let numbers: Vec<u32> = parsed.fields.iter().map(|(n, _)| *n).collect();
        assert_eq!(numbers, vec![1, 3, 3, 7]);
    }

    #[test]
    fn rejects_truncated_input() {
        let bytes = sample();
        let err = Message::parse(&bytes[..bytes.len() - 2]).expect_err("truncated");
        assert_eq!(err.kind(), "parse");
    }
}
