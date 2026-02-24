//! Configuration management

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {

    pub ollama: Option<ProviderConfig>,
    pub openai: Option<ProviderConfig>,
    pub claude: Option<ProviderConfig>,
    pub gemini: Option<ProviderConfig>,
    pub grok: Option<ProviderConfig>,
    
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub safety: SafetyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: Option<String>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: Some("dark".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    pub require_approval_for_commands: bool,
    pub require_approval_for_file_changes: bool,
    /// Agent approval policy: "suggest" | "auto-edit" | "full-auto"
    #[serde(default = "SafetyConfig::default_approval_policy")]
    pub approval_policy: String,
    /// Wrap agent command execution in an OS-level sandbox when available.
    #[serde(default)]
    pub sandbox: bool,
}

impl SafetyConfig {
    fn default_approval_policy() -> String {
        "suggest".to_string()
    }

    pub fn approval_policy_from_flags(suggest: bool, auto_edit: bool, full_auto: bool) -> String {
        if full_auto {
            "full-auto".to_string()
        } else if auto_edit {
            "auto-edit".to_string()
        } else if suggest {
            "suggest".to_string()
        } else {
            "suggest".to_string()
        }
    }
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            require_approval_for_commands: true,
            require_approval_for_file_changes: true,
            approval_policy: "suggest".to_string(),
            sandbox: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub api_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, content)?;
        
        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        Ok(home.join(".vibecli").join("config.toml"))
    }
    /// Derive approval policy string from boolean CLI flags.
    pub fn approval_from_flags(suggest: bool, auto_edit: bool, full_auto: bool) -> String {
        SafetyConfig::approval_policy_from_flags(suggest, auto_edit, full_auto)
    }

    pub fn get_provider_config(&self, name: &str) -> Option<&ProviderConfig> {
        match name.to_lowercase().as_str() {
            "ollama" => self.ollama.as_ref(),
            "openai" => self.openai.as_ref(),
            "claude" | "anthropic" => self.claude.as_ref(),
            "gemini" => self.gemini.as_ref(),
            "grok" => self.grok.as_ref(),
            _ => None,
        }
    }
}
