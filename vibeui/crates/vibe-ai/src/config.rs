use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AIConfig {
    pub ollama: Option<ProviderConfigFile>,
    pub openai: Option<ProviderConfigFile>,
    pub claude: Option<ProviderConfigFile>,
    pub gemini: Option<ProviderConfigFile>,
    pub grok: Option<ProviderConfigFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfigFile {
    pub enabled: bool,
    pub api_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
}

impl AIConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: AIConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
