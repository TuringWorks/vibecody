use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use regex::RegexBuilder;
use std::fs;
use std::io::{BufRead, BufReader};

/// Skip files larger than this to avoid reading multi-MB binaries into memory.
const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
/// Cap total matches to keep the UI responsive.
const MAX_TOTAL_RESULTS: usize = 500;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub path: String,
    pub line_number: usize,
    pub line_content: String,
}

pub fn search_files(root_path: &PathBuf, query: &str, case_sensitive: bool) -> Result<Vec<SearchResult>, anyhow::Error> {
    let mut results = Vec::new();

    // Compile regex
    let re = RegexBuilder::new(query)
        .case_insensitive(!case_sensitive)
        .build()
        .map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;

    // Walk directory
    'outer: for entry in WalkDir::new(root_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip directories and hidden files/git
        if path.is_dir() {
            continue;
        }

        // Skip hidden files/directories (any component *relative to root* starting with '.')
        // We strip the root prefix so that root paths like /tmp/.tmpXXX are not penalised.
        let rel = path.strip_prefix(root_path).unwrap_or(path);
        if rel.components().any(|c| {
            c.as_os_str().to_string_lossy().starts_with('.')
        }) {
            continue;
        }

        // Skip files that are too large to scan efficiently
        if let Ok(meta) = fs::metadata(path) {
            if meta.len() > MAX_FILE_BYTES {
                continue;
            }
        }

        // Buffered line-by-line reading — avoids loading the whole file at once
        if let Ok(file) = fs::File::open(path) {
            let reader = BufReader::new(file);
            for (line_idx, line_result) in reader.lines().enumerate() {
                if let Ok(line) = line_result {
                    if re.is_match(&line) {
                        results.push(SearchResult {
                            path: path.to_string_lossy().to_string(),
                            line_number: line_idx + 1,
                            line_content: line.trim().to_string(),
                        });
                        if results.len() >= MAX_TOTAL_RESULTS {
                            break 'outer;
                        }
                    }
                }
            }
        }
    }

    Ok(results)
}
