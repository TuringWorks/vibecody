//! Configuration management

use anyhow::{Result, Context};
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
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            require_approval_for_commands: true,
            require_approval_for_file_changes: true,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.ollama.is_none());
        assert_eq!(config.ui.theme.as_deref(), Some("dark"));
        assert!(config.safety.require_approval_for_commands);
    }

    #[test]
    fn test_parse_config() {
        let toml_str = r#"
            [ollama]
            enabled = true
            model = "llama3"

            [ui]
            theme = "light"
        "#;

        let config: Config = toml::from_str(toml_str).expect("Failed to parse config");
        
        assert!(config.ollama.is_some());
        assert_eq!(config.ollama.unwrap().model.as_deref(), Some("llama3"));
        assert_eq!(config.ui.theme.as_deref(), Some("light"));
    }

    #[test]
    fn test_get_provider_config() {
        let mut config = Config::default();
        config.openai = Some(ProviderConfig {
            enabled: true,
            api_key: Some("sk-test".to_string()),
            model: Some("gpt-4".to_string()),
            api_url: None,
        });

        let provider = config.get_provider_config("openai");
        assert!(provider.is_some());
        assert_eq!(provider.unwrap().model.as_deref(), Some("gpt-4"));

        let unknown = config.get_provider_config("unknown");
        assert!(unknown.is_none());
    }
}
