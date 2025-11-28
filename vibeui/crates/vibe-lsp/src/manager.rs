//! LSP manager for handling multiple language servers

use crate::client::LspClient;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

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
    pub async fn get_client_for_language(&mut self, language: &str, root_path: &PathBuf) -> Result<&mut LspClient> {
        if !self.clients.contains_key(language) {
            if let Some((cmd, args)) = self.server_configs.get(language) {
                let mut client = LspClient::new(cmd.clone(), args.clone());
                client.initialize(root_path.clone()).await?;
                self.clients.insert(language.to_string(), client);
            } else {
                return Err(anyhow::anyhow!("No LSP server configured for language: {}", language));
            }
        }
        
        Ok(self.clients.get_mut(language).unwrap())
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
