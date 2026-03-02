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

        // Skip files that are too large to scan efficiently.
        // Use the metadata already held by the WalkDir entry to avoid an
        // extra stat(2) syscall per file.
        if let Ok(meta) = entry.metadata() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("hello.rs"), "fn main() {\n    println!(\"hello\");\n}\n").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n").unwrap();
        fs::write(dir.path().join("readme.txt"), "This is a readme file.\n").unwrap();
        dir
    }

    #[test]
    fn search_finds_matching_lines() {
        let dir = setup_test_dir();
        let results = search_files(&dir.path().to_path_buf(), "fn main", true).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].path.contains("hello.rs"));
        assert_eq!(results[0].line_number, 1);
    }

    #[test]
    fn search_finds_multiple_files() {
        let dir = setup_test_dir();
        let results = search_files(&dir.path().to_path_buf(), r"fn\s+\w+", true).unwrap();
        assert!(results.len() >= 2, "should match in both .rs files, got {}", results.len());
    }

    #[test]
    fn search_case_insensitive() {
        let dir = setup_test_dir();
        let results = search_files(&dir.path().to_path_buf(), "FN MAIN", false).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_case_sensitive_no_match() {
        let dir = setup_test_dir();
        let results = search_files(&dir.path().to_path_buf(), "FN MAIN", true).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_no_results() {
        let dir = setup_test_dir();
        let results = search_files(&dir.path().to_path_buf(), "zzz_nonexistent_zzz", true).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn search_skips_hidden_files() {
        let dir = setup_test_dir();
        fs::create_dir_all(dir.path().join(".hidden")).unwrap();
        fs::write(dir.path().join(".hidden/secret.rs"), "fn main() {}\n").unwrap();
        let results = search_files(&dir.path().to_path_buf(), "fn main", true).unwrap();
        // Should only find hello.rs, not .hidden/secret.rs
        assert_eq!(results.len(), 1);
        assert!(results[0].path.contains("hello.rs"));
    }

    #[test]
    fn search_invalid_regex_returns_error() {
        let dir = setup_test_dir();
        let result = search_files(&dir.path().to_path_buf(), "[invalid", true);
        assert!(result.is_err());
    }

    #[test]
    fn search_result_line_content_is_trimmed() {
        let dir = setup_test_dir();
        let results = search_files(&dir.path().to_path_buf(), "println", true).unwrap();
        assert_eq!(results.len(), 1);
        // Line content should be trimmed
        assert!(!results[0].line_content.starts_with(' '));
    }
}
