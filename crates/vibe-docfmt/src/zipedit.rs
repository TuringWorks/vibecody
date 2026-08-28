//! Minimal read/rewrite helpers for the ZIP containers behind DOCX, EPUB and
//! Pages.
//!
//! Every writer here follows the same rule: entries the writer did not touch
//! are copied back byte-for-byte, in their original order, with their original
//! compression method. EPUB depends on that (`mimetype` must stay first and
//! stored), and for Pages it is the difference between "edited the text" and
//! "re-encoded a proprietary archive we do not fully understand".

use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::DocError;

/// One entry of a ZIP container, held in memory.
#[derive(Debug, Clone)]
pub struct ZipEntry {
    pub name: String,
    pub data: Vec<u8>,
    pub compression: CompressionMethod,
    pub is_dir: bool,
}

/// Read every entry of a ZIP archive into memory, preserving order.
pub fn read_entries(bytes: &[u8]) -> Result<Vec<ZipEntry>, DocError> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|e| DocError::Container(e.to_string()))?;
    (0..archive.len())
        .map(|i| {
            let mut file = archive
                .by_index(i)
                .map_err(|e| DocError::Container(e.to_string()))?;
            let name = file.name().to_string();
            let compression = file.compression();
            let is_dir = file.is_dir();
            let mut data = Vec::with_capacity(file.size() as usize);
            if !is_dir {
                file.read_to_end(&mut data)
                    .map_err(|e| DocError::Container(e.to_string()))?;
            }
            Ok(ZipEntry {
                name,
                data,
                compression,
                is_dir,
            })
        })
        .collect()
}

/// Write entries back out as a ZIP archive, in the order given.
pub fn write_entries(entries: &[ZipEntry]) -> Result<Vec<u8>, DocError> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::<u8>::new()));
    for entry in entries {
        if entry.is_dir {
            writer
                .add_directory(entry.name.clone(), SimpleFileOptions::default())
                .map_err(|e| DocError::Container(e.to_string()))?;
            continue;
        }
        // Unsupported methods (a container compressed with something this build
        // of `zip` cannot re-encode) would otherwise fail at `start_file`; fall
        // back to deflate rather than losing the entry.
        let method = match entry.compression {
            CompressionMethod::Stored => CompressionMethod::Stored,
            CompressionMethod::Deflated => CompressionMethod::Deflated,
            _ => CompressionMethod::Deflated,
        };
        writer
            .start_file(
                entry.name.clone(),
                SimpleFileOptions::default().compression_method(method),
            )
            .map_err(|e| DocError::Container(e.to_string()))?;
        writer
            .write_all(&entry.data)
            .map_err(|e| DocError::Container(e.to_string()))?;
    }
    let cursor = writer
        .finish()
        .map_err(|e| DocError::Container(e.to_string()))?;
    Ok(cursor.into_inner())
}

/// Find an entry by exact name.
pub fn find<'a>(entries: &'a [ZipEntry], name: &str) -> Option<&'a ZipEntry> {
    entries.iter().find(|e| e.name == name)
}

/// Replace the data of a named entry, keeping its position and compression.
///
/// Returns `false` when the entry does not exist — callers decide whether a
/// missing part is an error or an insert.
pub fn replace(entries: &mut [ZipEntry], name: &str, data: Vec<u8>) -> bool {
    match entries.iter_mut().find(|e| e.name == name) {
        Some(entry) => {
            entry.data = data;
            true
        }
        None => false,
    }
}

/// Decode an entry as UTF-8 text.
pub fn text(entry: &ZipEntry) -> Result<String, DocError> {
    String::from_utf8(entry.data.clone())
        .map_err(|e| DocError::Parse(format!("{} is not valid UTF-8: {e}", entry.name)))
}
