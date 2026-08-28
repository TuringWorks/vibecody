//! Apple's IWA block framing around raw Snappy.
//!
//! An `.iwa` file is a sequence of blocks. Each block is a 4-byte header — a
//! type byte (always 0) followed by a 24-bit little-endian compressed length —
//! and then that many bytes of *raw* Snappy (no stream identifier, no CRC), each
//! block decompressing to at most 64 KiB. This is not the Snappy framing format,
//! which is why the `snap` frame decoder cannot read these files.

use crate::error::DocError;

/// Largest uncompressed block Apple's writer emits, and what this one emits.
const BLOCK_SIZE: usize = 65_536;

/// Decompress an `.iwa` byte stream.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, DocError> {
    let mut out = Vec::with_capacity(data.len() * 4);
    let mut offset = 0;
    let mut decoder = snap::raw::Decoder::new();

    while offset < data.len() {
        if offset + 4 > data.len() {
            return Err(DocError::Parse(format!(
                "IWA stream ends mid-header at byte {offset}"
            )));
        }
        let kind = data[offset];
        if kind != 0x00 {
            return Err(DocError::Parse(format!(
                "unsupported IWA block type 0x{kind:02x} at byte {offset}"
            )));
        }
        let len =
            u32::from_le_bytes([data[offset + 1], data[offset + 2], data[offset + 3], 0]) as usize;
        let start = offset + 4;
        let end = start
            .checked_add(len)
            .ok_or_else(|| DocError::Parse("IWA block length overflows the stream".to_string()))?;
        if end > data.len() {
            return Err(DocError::Parse(format!(
                "IWA block at byte {offset} claims {len} bytes but only {} remain",
                data.len() - start
            )));
        }
        let block = decoder
            .decompress_vec(&data[start..end])
            .map_err(|e| DocError::Parse(format!("snappy block at byte {offset}: {e}")))?;
        out.extend_from_slice(&block);
        offset = end;
    }
    Ok(out)
}

/// Compress bytes back into the same block framing.
pub fn compress(data: &[u8]) -> Result<Vec<u8>, DocError> {
    let mut out = Vec::with_capacity(data.len() / 2 + 16);
    let mut encoder = snap::raw::Encoder::new();
    for chunk in data.chunks(BLOCK_SIZE) {
        let block = encoder
            .compress_vec(chunk)
            .map_err(|e| DocError::Container(format!("snappy compress: {e}")))?;
        if block.len() > 0x00FF_FFFF {
            return Err(DocError::Container(
                "compressed IWA block exceeds the 24-bit length field".to_string(),
            ));
        }
        let len = block.len() as u32;
        out.push(0x00);
        out.extend_from_slice(&len.to_le_bytes()[..3]);
        out.extend_from_slice(&block);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_multi_block_payload() {
        // Two blocks' worth, with enough structure that Snappy actually
        // compresses rather than storing verbatim.
        let payload: Vec<u8> = (0..BLOCK_SIZE * 2 + 5)
            .map(|i| ((i / 7) % 251) as u8)
            .collect();
        let compressed = compress(&payload).expect("compress");
        assert_eq!(compressed[0], 0x00, "block header type byte");
        assert_eq!(decompress(&compressed).expect("decompress"), payload);
    }

    #[test]
    fn rejects_a_truncated_block_instead_of_returning_partial_text() {
        let compressed = compress(b"hello iwa").expect("compress");
        let truncated = &compressed[..compressed.len() - 2];
        let err = decompress(truncated).expect_err("truncation is an error");
        assert_eq!(err.kind(), "parse");
    }

    #[test]
    fn rejects_an_unknown_block_type() {
        let err = decompress(&[0x01, 0x02, 0x00, 0x00, 0x00, 0x00]).expect_err("bad type");
        assert!(err.to_string().contains("block type"), "{err}");
    }
}
