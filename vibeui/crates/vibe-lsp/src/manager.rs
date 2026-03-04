//! LSP manager for handling multiple language servers

use crate::client::LspClient;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

/// LSP manager
pub struct LspManager {
    clients: HashMap<String, LspClient>, // language_id -> client
    server_configs: HashMap<String, (String, Vec<String>)>, // language_id -> (cmd, args)
}

impl LspManager {
    pub fn new() -> Self {
        let mut server_configs = HashMap::new();
        
        // Default configurations (assumes binaries are in PATH)
        server_configs.insert("rust".to_string(), ("rust-analyzer".to_string(), vec![]));
        server_configs.insert("typescript".to_string(), ("typescript-language-server".to_string(), vec!["--stdio".to_string()]));
        server_configs.insert("javascript".to_string(), ("typescript-language-server".to_string(), vec!["--stdio".to_string()]));
        server_configs.insert("python".to_string(), ("pylsp".to_string(), vec![]));

        Self {
            clients: HashMap::new(),
            server_configs,
        }
    }

    /// Get or create a client for the given language
    pub async fn get_client_for_language(&mut self, language: &str, root_path: &Path) -> Result<&mut LspClient> {
        if !self.clients.contains_key(language) {
            if let Some((cmd, args)) = self.server_configs.get(language) {
                let mut client = LspClient::new(cmd.clone(), args.clone());
                client.initialize(root_path.to_path_buf()).await?;
                self.clients.insert(language.to_string(), client);
            } else {
                return Err(anyhow::anyhow!("No LSP server configured for language: {}", language));
            }
        }
        
        self.clients.get_mut(language)
            .ok_or_else(|| anyhow::anyhow!("LSP client for '{}' missing after initialization", language))
    }

    pub fn add_client(&mut self, language: String, client: LspClient) {
        self.clients.insert(language, client);
    }

    pub fn get_client(&self, language: &str) -> Option<&LspClient> {
        self.clients.get(language)
    }

    pub fn get_client_mut(&mut self, language: &str) -> Option<&mut LspClient> {
        self.clients.get_mut(language)
    }
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_default_configs() {
        let mgr = LspManager::new();
        assert!(mgr.server_configs.contains_key("rust"));
        assert!(mgr.server_configs.contains_key("typescript"));
        assert!(mgr.server_configs.contains_key("javascript"));
        assert!(mgr.server_configs.contains_key("python"));
    }

    #[test]
    fn new_has_four_default_configs() {
        let mgr = LspManager::new();
        assert_eq!(mgr.server_configs.len(), 4);
    }

    #[test]
    fn new_has_no_clients_initially() {
        let mgr = LspManager::new();
        assert!(mgr.clients.is_empty());
    }

    #[test]
    fn default_rust_config_is_rust_analyzer() {
        let mgr = LspManager::new();
        let (cmd, args) = mgr.server_configs.get("rust").unwrap();
        assert_eq!(cmd, "rust-analyzer");
        assert!(args.is_empty());
    }

    #[test]
    fn default_typescript_config() {
        let mgr = LspManager::new();
        let (cmd, args) = mgr.server_configs.get("typescript").unwrap();
        assert_eq!(cmd, "typescript-language-server");
        assert_eq!(args, &["--stdio"]);
    }

    #[test]
    fn default_python_config_is_pylsp() {
        let mgr = LspManager::new();
        let (cmd, _) = mgr.server_configs.get("python").unwrap();
        assert_eq!(cmd, "pylsp");
    }

    #[test]
    fn get_client_for_unknown_language_returns_none() {
        let mgr = LspManager::new();
        assert!(mgr.get_client("haskell").is_none());
    }

    #[test]
    fn get_client_mut_for_unknown_returns_none() {
        let mut mgr = LspManager::new();
        assert!(mgr.get_client_mut("cobol").is_none());
    }

    #[test]
    fn default_is_same_as_new() {
        let mgr = LspManager::default();
        assert_eq!(mgr.server_configs.len(), 4);
    }
}
