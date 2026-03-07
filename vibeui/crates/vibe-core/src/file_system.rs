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

    #[tokio::test]
    async fn test_delete_file() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let path = dir.path().join("to_delete.txt");

        fs.write_file(&path, "goodbye").await.unwrap();
        assert!(fs.exists(&path).await);

        fs.delete_file(&path).await.unwrap();
        assert!(!fs.exists(&path).await);
    }

    #[tokio::test]
    async fn test_rename_item() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let src = dir.path().join("original.txt");
        let dst = dir.path().join("renamed.txt");

        fs.write_file(&src, "content").await.unwrap();
        assert!(fs.exists(&src).await);

        fs.rename_item(&src, &dst).await.unwrap();
        assert!(!fs.exists(&src).await);
        assert!(fs.exists(&dst).await);

        let content = fs.read_file(&dst).await.unwrap();
        assert_eq!(content, "content");
    }

    #[tokio::test]
    async fn test_exists_and_is_directory() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();

        // Non-existent path
        assert!(!fs.exists(&dir.path().join("nope")).await);

        // Create a directory and verify
        let sub = dir.path().join("mydir");
        fs.create_directory(&sub).await.unwrap();
        assert!(fs.exists(&sub).await);
        assert!(fs.is_directory(&sub).await);
        assert!(!fs.is_file(&sub).await);

        // Create a file and verify
        let file = dir.path().join("myfile.txt");
        fs.write_file(&file, "data").await.unwrap();
        assert!(fs.exists(&file).await);
        assert!(fs.is_file(&file).await);
        assert!(!fs.is_directory(&file).await);
    }

    #[tokio::test]
    async fn test_metadata_reading() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let path = dir.path().join("meta_test.txt");

        fs.write_file(&path, "hello world").await.unwrap();

        let entry = fs.metadata(&path).await.unwrap();
        assert_eq!(entry.name, "meta_test.txt");
        assert!(!entry.is_directory);
        // "hello world" is 11 bytes
        assert_eq!(entry.size, Some(11));
        assert!(entry.modified.is_some());
        assert_eq!(entry.path, path);
    }

    #[tokio::test]
    async fn test_delete_directory() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let sub = dir.path().join("nested");
        let file_in_sub = sub.join("inner.txt");

        fs.create_directory(&sub).await.unwrap();
        fs.write_file(&file_in_sub, "inner content").await.unwrap();
        assert!(fs.exists(&file_in_sub).await);

        fs.delete_directory(&sub).await.unwrap();
        assert!(!fs.exists(&sub).await);
        assert!(!fs.exists(&file_in_sub).await);
    }

    #[tokio::test]
    async fn test_write_file_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let deeply_nested = dir.path().join("a").join("b").join("c").join("file.txt");

        fs.write_file(&deeply_nested, "deep").await.unwrap();
        let content = fs.read_file(&deeply_nested).await.unwrap();
        assert_eq!(content, "deep");
    }

    #[tokio::test]
    async fn test_read_nonexistent_file_returns_error() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let missing = dir.path().join("does_not_exist.txt");

        let result = fs.read_file(&missing).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_file_returns_error() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let missing = dir.path().join("ghost.txt");

        let result = fs.delete_file(&missing).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_nonexistent_directory_returns_error() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let missing = dir.path().join("no_such_dir");

        let result = fs.list_directory(&missing).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_metadata_nonexistent_path_returns_error() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let missing = dir.path().join("vanished.txt");

        let result = fs.metadata(&missing).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_empty_content() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let path = dir.path().join("empty.txt");

        fs.write_file(&path, "").await.unwrap();
        let content = fs.read_file(&path).await.unwrap();
        assert_eq!(content, "");

        let entry = fs.metadata(&path).await.unwrap();
        assert_eq!(entry.size, Some(0));
    }

    #[tokio::test]
    async fn test_overwrite_existing_file() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let path = dir.path().join("overwrite.txt");

        fs.write_file(&path, "first").await.unwrap();
        assert_eq!(fs.read_file(&path).await.unwrap(), "first");

        fs.write_file(&path, "second").await.unwrap();
        assert_eq!(fs.read_file(&path).await.unwrap(), "second");
    }

    #[tokio::test]
    async fn test_list_directory_sorts_dirs_first_then_alpha() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();

        // Create files and dirs with names that test alphabetical sorting
        fs.write_file(&dir.path().join("zebra.txt"), "z").await.unwrap();
        fs.write_file(&dir.path().join("alpha.txt"), "a").await.unwrap();
        fs.create_directory(&dir.path().join("beta_dir")).await.unwrap();
        fs.create_directory(&dir.path().join("aaa_dir")).await.unwrap();
        fs.write_file(&dir.path().join("middle.txt"), "m").await.unwrap();

        let entries = fs.list_directory(dir.path()).await.unwrap();
        assert_eq!(entries.len(), 5);

        // Directories come first, sorted alphabetically
        assert!(entries[0].is_directory);
        assert_eq!(entries[0].name, "aaa_dir");
        assert!(entries[1].is_directory);
        assert_eq!(entries[1].name, "beta_dir");

        // Then files, sorted alphabetically
        assert!(!entries[2].is_directory);
        assert_eq!(entries[2].name, "alpha.txt");
        assert!(!entries[3].is_directory);
        assert_eq!(entries[3].name, "middle.txt");
        assert!(!entries[4].is_directory);
        assert_eq!(entries[4].name, "zebra.txt");
    }

    #[tokio::test]
    async fn test_list_empty_directory() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();

        let entries = fs.list_directory(dir.path()).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_metadata_for_directory() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let sub = dir.path().join("mydir");
        fs.create_directory(&sub).await.unwrap();

        let entry = fs.metadata(&sub).await.unwrap();
        assert_eq!(entry.name, "mydir");
        assert!(entry.is_directory);
        // Directories should have no size
        assert_eq!(entry.size, None);
        assert!(entry.modified.is_some());
    }

    #[test]
    fn test_default_trait() {
        let fs = FileSystem::default();
        // Verify it constructs without panic and has no watchers
        assert!(fs.watchers.is_empty());
    }

    #[test]
    fn test_subscribe_returns_receiver() {
        let fs = FileSystem::new();
        let _rx = fs.subscribe();
        // Getting a second subscriber should also work
        let _rx2 = fs.subscribe();
    }

    #[test]
    fn test_file_entry_clone_and_debug() {
        let entry = FileEntry {
            path: PathBuf::from("/tmp/test.txt"),
            name: "test.txt".to_string(),
            is_directory: false,
            size: Some(42),
            modified: None,
        };
        let cloned = entry.clone();
        assert_eq!(cloned.name, "test.txt");
        assert_eq!(cloned.size, Some(42));
        assert!(!cloned.is_directory);
        // Debug trait should work
        let debug_str = format!("{:?}", entry);
        assert!(debug_str.contains("test.txt"));
    }

    #[test]
    fn test_file_change_event_variants() {
        let created = FileChangeEvent::Created(PathBuf::from("/a"));
        let modified = FileChangeEvent::Modified(PathBuf::from("/b"));
        let deleted = FileChangeEvent::Deleted(PathBuf::from("/c"));
        let renamed = FileChangeEvent::Renamed {
            from: PathBuf::from("/d"),
            to: PathBuf::from("/e"),
        };

        // Verify Debug formatting works for all variants
        assert!(format!("{:?}", created).contains("Created"));
        assert!(format!("{:?}", modified).contains("Modified"));
        assert!(format!("{:?}", deleted).contains("Deleted"));
        assert!(format!("{:?}", renamed).contains("Renamed"));

        // Verify Clone works
        let cloned = created.clone();
        assert!(format!("{:?}", cloned).contains("Created"));
    }

    #[tokio::test]
    async fn test_is_directory_and_is_file_for_nonexistent() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let missing = dir.path().join("nonexistent");

        assert!(!fs.is_directory(&missing).await);
        assert!(!fs.is_file(&missing).await);
    }

    #[tokio::test]
    async fn test_rename_nonexistent_returns_error() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let src = dir.path().join("no_src.txt");
        let dst = dir.path().join("no_dst.txt");

        let result = fs.rename_item(&src, &dst).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_nonempty_directory() {
        let dir = tempdir().unwrap();
        let fs = FileSystem::new();
        let sub = dir.path().join("parent");
        let nested = sub.join("child").join("grandchild");

        fs.create_directory(&nested).await.unwrap();
        fs.write_file(&nested.join("file.txt"), "data").await.unwrap();

        // delete_directory (remove_dir_all) should remove everything recursively
        fs.delete_directory(&sub).await.unwrap();
        assert!(!fs.exists(&sub).await);
    }
}
