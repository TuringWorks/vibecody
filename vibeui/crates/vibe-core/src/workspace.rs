//! Workspace management for multi-folder projects

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::buffer::TextBuffer;
use crate::file_system::FileSystem;

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub name: String,
    pub folders: Vec<PathBuf>,
    pub settings: HashMap<String, serde_json::Value>,
}

/// Workspace manager
pub struct Workspace {
    config: WorkspaceConfig,
    file_system: FileSystem,
    open_buffers: HashMap<PathBuf, TextBuffer>,
}

impl Workspace {
    /// Create a new workspace
    pub fn new(name: String) -> Self {
        Self {
            config: WorkspaceConfig {
                name,
                folders: Vec::new(),
                settings: HashMap::new(),
            },
            file_system: FileSystem::new(),
            open_buffers: HashMap::new(),
        }
    }

    /// Create a workspace from a configuration
    pub fn from_config(config: WorkspaceConfig) -> Self {
        Self {
            config,
            file_system: FileSystem::new(),
            open_buffers: HashMap::new(),
        }
    }

    /// Get the workspace name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get workspace folders
    pub fn folders(&self) -> &[PathBuf] {
        &self.config.folders
    }

    /// Add a folder to the workspace
    pub fn add_folder(&mut self, path: PathBuf) -> Result<()> {
        if !self.config.folders.contains(&path) {
            self.config.folders.push(path.clone());
            self.file_system.watch_directory(&path)?;
        }
        Ok(())
    }

    /// Remove a folder from the workspace
    pub fn remove_folder(&mut self, path: &PathBuf) {
        self.config.folders.retain(|p| p != path);
    }

    /// Get a setting value
    pub fn get_setting(&self, key: &str) -> Option<&serde_json::Value> {
        self.config.settings.get(key)
    }

    /// Set a setting value
    pub fn set_setting(&mut self, key: String, value: serde_json::Value) {
        self.config.settings.insert(key, value);
    }

    /// Open a file in the workspace
    pub async fn open_file(&mut self, path: PathBuf) -> Result<&TextBuffer> {
        if !self.open_buffers.contains_key(&path) {
            let buffer = TextBuffer::from_file(path.clone())?;
            self.open_buffers.insert(path.clone(), buffer);
        }
        self.open_buffers.get(&path)
            .ok_or_else(|| anyhow::anyhow!("Buffer for '{}' missing after insertion", path.display()))
    }

    /// Get an open buffer
    pub fn get_buffer(&self, path: &PathBuf) -> Option<&TextBuffer> {
        self.open_buffers.get(path)
    }

    /// Get a mutable reference to an open buffer
    pub fn get_buffer_mut(&mut self, path: &PathBuf) -> Option<&mut TextBuffer> {
        self.open_buffers.get_mut(path)
    }

    /// Close a file
    pub fn close_file(&mut self, path: &PathBuf) -> Option<TextBuffer> {
        self.open_buffers.remove(path)
    }

    /// Get all open file paths
    pub fn open_files(&self) -> Vec<PathBuf> {
        self.open_buffers.keys().cloned().collect()
    }

    /// Save all modified buffers
    pub async fn save_all(&mut self) -> Result<()> {
        for buffer in self.open_buffers.values_mut() {
            if buffer.is_modified() {
                buffer.save()?;
            }
        }
        Ok(())
    }

    /// Get the file system
    pub fn file_system(&self) -> &FileSystem {
        &self.file_system
    }

    /// Get a mutable reference to the file system
    pub fn file_system_mut(&mut self) -> &mut FileSystem {
        &mut self.file_system
    }

    /// Save workspace configuration to a file
    pub async fn save_config(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.config)?;
        self.file_system.write_file(path, &json).await?;
        Ok(())
    }

    /// Load workspace configuration from a file
    pub async fn load_config(path: &Path) -> Result<WorkspaceConfig> {
        let fs = FileSystem::new();
        let json = fs.read_file(path).await?;
        let config: WorkspaceConfig = serde_json::from_str(&json)?;
        Ok(config)
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new("Untitled Workspace".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_workspace() {
        let workspace = Workspace::new("Test Workspace".to_string());
        assert_eq!(workspace.name(), "Test Workspace");
        assert_eq!(workspace.folders().len(), 0);
    }

    #[test]
    fn test_add_remove_folder() {
        let mut workspace = Workspace::new("Test".to_string());
        let path = PathBuf::from("/test/path");
        
        workspace.add_folder(path.clone()).ok();
        assert_eq!(workspace.folders().len(), 1);
        
        workspace.remove_folder(&path);
        assert_eq!(workspace.folders().len(), 0);
    }

    #[test]
    fn test_settings() {
        let mut workspace = Workspace::new("Test".to_string());

        workspace.set_setting(
            "theme".to_string(),
            serde_json::json!("dark")
        );

        assert_eq!(
            workspace.get_setting("theme"),
            Some(&serde_json::json!("dark"))
        );
    }

    #[test]
    fn workspace_name() {
        let ws = Workspace::new("My Project".to_string());
        assert_eq!(ws.name(), "My Project");
    }

    #[test]
    fn workspace_default() {
        let ws = Workspace::default();
        assert_eq!(ws.name(), "Untitled Workspace");
        assert!(ws.folders().is_empty());
    }

    #[test]
    fn workspace_from_config() {
        let config = WorkspaceConfig {
            name: "FromConfig".to_string(),
            folders: vec![PathBuf::from("/a"), PathBuf::from("/b")],
            settings: HashMap::new(),
        };
        let ws = Workspace::from_config(config);
        assert_eq!(ws.name(), "FromConfig");
        assert_eq!(ws.folders().len(), 2);
    }

    #[test]
    fn workspace_add_folder_deduplication() {
        let mut ws = Workspace::new("Test".to_string());
        let path = PathBuf::from("/test");
        ws.add_folder(path.clone()).ok();
        ws.add_folder(path.clone()).ok();
        // Adding the same folder twice should not duplicate
        assert_eq!(ws.folders().len(), 1);
    }

    #[test]
    fn workspace_folders_empty_initially() {
        let ws = Workspace::new("Test".to_string());
        assert!(ws.folders().is_empty());
    }

    #[test]
    fn workspace_setting_overwrite() {
        let mut ws = Workspace::new("Test".to_string());
        ws.set_setting("key".to_string(), serde_json::json!(1));
        ws.set_setting("key".to_string(), serde_json::json!(2));
        assert_eq!(ws.get_setting("key"), Some(&serde_json::json!(2)));
    }

    #[test]
    fn workspace_get_setting_missing() {
        let ws = Workspace::new("Test".to_string());
        assert!(ws.get_setting("nonexistent").is_none());
    }

    #[test]
    fn workspace_setting_types() {
        let mut ws = Workspace::new("Test".to_string());
        ws.set_setting("string".to_string(), serde_json::json!("hello"));
        ws.set_setting("number".to_string(), serde_json::json!(42));
        ws.set_setting("bool".to_string(), serde_json::json!(true));
        ws.set_setting("array".to_string(), serde_json::json!([1, 2, 3]));
        assert_eq!(ws.get_setting("string"), Some(&serde_json::json!("hello")));
        assert_eq!(ws.get_setting("number"), Some(&serde_json::json!(42)));
        assert_eq!(ws.get_setting("bool"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn workspace_open_files_initially_empty() {
        let ws = Workspace::new("Test".to_string());
        assert!(ws.open_files().is_empty());
    }

    #[test]
    fn workspace_get_buffer_missing() {
        let ws = Workspace::new("Test".to_string());
        assert!(ws.get_buffer(&PathBuf::from("/nonexistent")).is_none());
    }

    #[test]
    fn workspace_close_file_not_open() {
        let mut ws = Workspace::new("Test".to_string());
        let result = ws.close_file(&PathBuf::from("/not_open"));
        assert!(result.is_none());
    }

    #[test]
    fn workspace_config_serialization() {
        let config = WorkspaceConfig {
            name: "Serde Test".to_string(),
            folders: vec![PathBuf::from("/a")],
            settings: {
                let mut m = HashMap::new();
                m.insert("theme".to_string(), serde_json::json!("dark"));
                m
            },
        };
        let json = serde_json::to_string(&config).unwrap();
        let deser: WorkspaceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.name, "Serde Test");
        assert_eq!(deser.folders.len(), 1);
        assert_eq!(deser.settings.get("theme"), Some(&serde_json::json!("dark")));
    }

    #[test]
    fn workspace_remove_folder_not_present_is_noop() {
        let mut ws = Workspace::new("Test".to_string());
        ws.add_folder(PathBuf::from("/a")).ok();
        ws.remove_folder(&PathBuf::from("/nonexistent"));
        assert_eq!(ws.folders().len(), 1, "removing absent folder should not change list");
    }

    #[test]
    fn workspace_multiple_folders_maintain_order() {
        let mut ws = Workspace::new("Test".to_string());
        ws.add_folder(PathBuf::from("/z")).ok();
        ws.add_folder(PathBuf::from("/a")).ok();
        ws.add_folder(PathBuf::from("/m")).ok();
        assert_eq!(ws.folders(), &[PathBuf::from("/z"), PathBuf::from("/a"), PathBuf::from("/m")]);
    }

    #[test]
    fn workspace_remove_middle_folder() {
        let mut ws = Workspace::new("Test".to_string());
        ws.add_folder(PathBuf::from("/a")).ok();
        ws.add_folder(PathBuf::from("/b")).ok();
        ws.add_folder(PathBuf::from("/c")).ok();
        ws.remove_folder(&PathBuf::from("/b"));
        assert_eq!(ws.folders(), &[PathBuf::from("/a"), PathBuf::from("/c")]);
    }

    #[test]
    fn workspace_settings_multiple_keys() {
        let mut ws = Workspace::new("Test".to_string());
        ws.set_setting("a".to_string(), serde_json::json!(1));
        ws.set_setting("b".to_string(), serde_json::json!(2));
        ws.set_setting("c".to_string(), serde_json::json!(3));
        assert_eq!(ws.get_setting("a"), Some(&serde_json::json!(1)));
        assert_eq!(ws.get_setting("b"), Some(&serde_json::json!(2)));
        assert_eq!(ws.get_setting("c"), Some(&serde_json::json!(3)));
    }

    #[test]
    fn workspace_config_empty_settings_serialization() {
        let config = WorkspaceConfig {
            name: "Empty".to_string(),
            folders: vec![],
            settings: HashMap::new(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let back: WorkspaceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Empty");
        assert!(back.folders.is_empty());
        assert!(back.settings.is_empty());
    }

    #[test]
    fn workspace_config_clone() {
        let config = WorkspaceConfig {
            name: "Clone".to_string(),
            folders: vec![PathBuf::from("/x")],
            settings: HashMap::new(),
        };
        let cloned = config.clone();
        assert_eq!(cloned.name, "Clone");
        assert_eq!(cloned.folders.len(), 1);
    }

    #[test]
    fn workspace_config_debug_format() {
        let config = WorkspaceConfig {
            name: "Debug".to_string(),
            folders: vec![],
            settings: HashMap::new(),
        };
        let debug = format!("{:?}", config);
        assert!(debug.contains("Debug"), "Debug output should contain the name");
    }

    #[test]
    fn workspace_get_buffer_mut_missing() {
        let mut ws = Workspace::new("Test".to_string());
        assert!(ws.get_buffer_mut(&PathBuf::from("/missing")).is_none());
    }
}
