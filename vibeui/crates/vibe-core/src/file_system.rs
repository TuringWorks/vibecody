//! File system operations and file watching

use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use tokio::sync::broadcast;

/// Represents a file or directory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub modified: Option<std::time::SystemTime>,
}

/// File system change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileChangeEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

/// File system manager
pub struct FileSystem {
    watchers: Vec<RecommendedWatcher>,
    event_tx: broadcast::Sender<FileChangeEvent>,
}

impl FileSystem {
    /// Create a new file system manager
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(100);
        Self {
            watchers: Vec::new(),
            event_tx,
        }
    }

    /// Subscribe to file system change events
    pub fn subscribe(&self) -> broadcast::Receiver<FileChangeEvent> {
        self.event_tx.subscribe()
    }

    /// Read a file's contents
    pub async fn read_file(&self, path: &Path) -> Result<String> {
        let content = tokio::fs::read_to_string(path).await?;
        Ok(content)
    }

    /// Write content to a file
    pub async fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    /// Delete a file
    pub async fn delete_file(&self, path: &Path) -> Result<()> {
        tokio::fs::remove_file(path).await?;
        Ok(())
    }

    /// Create a directory
    pub async fn create_directory(&self, path: &Path) -> Result<()> {
        tokio::fs::create_dir_all(path).await?;
        Ok(())
    }

    /// Delete a directory
    pub async fn delete_directory(&self, path: &Path) -> Result<()> {
        tokio::fs::remove_dir_all(path).await?;
        Ok(())
    }

    /// Rename a file or directory
    pub async fn rename_item(&self, from: &Path, to: &Path) -> Result<()> {
        tokio::fs::rename(from, to).await?;
        Ok(())
    }

    /// List directory contents
    pub async fn list_directory(&self, path: &Path) -> Result<Vec<FileEntry>> {
        let mut entries = Vec::new();
        let mut read_dir = tokio::fs::read_dir(path).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;
            
            entries.push(FileEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                path: path.clone(),
                is_directory: metadata.is_dir(),
                size: if metadata.is_file() {
                    Some(metadata.len())
                } else {
                    None
                },
                modified: metadata.modified().ok(),
            });
        }

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| {
            match (a.is_directory, b.is_directory) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(entries)
    }

    /// Watch a directory for changes
    pub fn watch_directory(&mut self, path: &Path) -> Result<()> {
        let (tx, rx) = channel();
        let event_tx = self.event_tx.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })?;

        watcher.watch(path, RecursiveMode::Recursive)?;
        self.watchers.push(watcher);

        // Spawn a task to forward events
        tokio::spawn(async move {
            while let Ok(event) = rx.recv() {
                use notify::EventKind;
                
                match event.kind {
                    EventKind::Create(_) => {
                        for path in event.paths {
                            let _ = event_tx.send(FileChangeEvent::Created(path));
                        }
                    }
                    EventKind::Modify(_) => {
                        for path in event.paths {
                            let _ = event_tx.send(FileChangeEvent::Modified(path));
                        }
                    }
                    EventKind::Remove(_) => {
                        for path in event.paths {
                            let _ = event_tx.send(FileChangeEvent::Deleted(path));
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Check if a path exists
    pub async fn exists(&self, path: &Path) -> bool {
        tokio::fs::try_exists(path).await.unwrap_or(false)
    }

    /// Check if a path is a directory
    pub async fn is_directory(&self, path: &Path) -> bool {
        tokio::fs::metadata(path)
            .await
            .map(|m| m.is_dir())
            .unwrap_or(false)
    }

    /// Check if a path is a file
    pub async fn is_file(&self, path: &Path) -> bool {
        tokio::fs::metadata(path)
            .await
            .map(|m| m.is_file())
            .unwrap_or(false)
    }

    /// Get file metadata
    pub async fn metadata(&self, path: &Path) -> Result<FileEntry> {
        let metadata = tokio::fs::metadata(path).await?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        Ok(FileEntry {
            path: path.to_path_buf(),
            name,
            is_directory: metadata.is_dir(),
            size: if metadata.is_file() {
                Some(metadata.len())
            } else {
                None
            },
            modified: metadata.modified().ok(),
        })
    }
}

impl Default for FileSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_read_write_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        
        let fs = FileSystem::new();
        fs.write_file(&file_path, "Hello, World!").await.unwrap();
        
        let content = fs.read_file(&file_path).await.unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_list_directory() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        
        // Create some test files
        fs.write_file(&dir.path().join("file1.txt"), "test").await.unwrap();
        fs.write_file(&dir.path().join("file2.txt"), "test").await.unwrap();
        fs.create_directory(&dir.path().join("subdir")).await.unwrap();
        
        let entries = fs.list_directory(dir.path()).await.unwrap();
        assert_eq!(entries.len(), 3);
        
        // Directory should be first
        assert!(entries[0].is_directory);
        assert_eq!(entries[0].name, "subdir");
    }
}
