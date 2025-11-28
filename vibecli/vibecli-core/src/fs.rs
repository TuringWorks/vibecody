//! File system operations

use anyhow::Result;
use std::fs;
use std::path::Path;

pub struct FileSystem;

impl FileSystem {
    pub fn read_file(path: &Path) -> Result<String> {
        Ok(fs::read_to_string(path)?)
    }

    pub fn write_file(path: &Path, content: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    pub fn list_files(dir: &Path) -> Result<Vec<String>> {
        let mut files = Vec::new();
        
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_file() {
                    if let Some(path_str) = path.to_str() {
                        files.push(path_str.to_string());
                    }
                } else if path.is_dir() {
                    // Recursively list subdirectories
                    files.extend(Self::list_files(&path)?);
                }
            }
        }
        
        Ok(files)
    }

    pub fn exists(path: &Path) -> bool {
        path.exists()
    }
}
