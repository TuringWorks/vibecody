use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use regex::RegexBuilder;
use std::fs;

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
    for entry in WalkDir::new(root_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok()) 
    {
        let path = entry.path();
        
        // Skip directories and hidden files/git
        if path.is_dir() {
            continue;
        }
        
        if path.to_string_lossy().contains("/.") || path.to_string_lossy().contains("\\.") {
            continue;
        }

        // Read file content
        // TODO: Use memory mapping or buffered reading for large files
        if let Ok(content) = fs::read_to_string(path) {
            for (line_idx, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    results.push(SearchResult {
                        path: path.to_string_lossy().to_string(),
                        line_number: line_idx + 1,
                        line_content: line.trim().to_string(),
                    });
                    
                    // Limit results per file/total to avoid freezing?
                    // For now, let's keep it simple.
                }
            }
        }
    }

    Ok(results)
}
