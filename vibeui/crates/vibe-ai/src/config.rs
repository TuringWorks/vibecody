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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_all_none() {
        let cfg = AIConfig::default();
        assert!(cfg.ollama.is_none());
        assert!(cfg.openai.is_none());
        assert!(cfg.claude.is_none());
        assert!(cfg.gemini.is_none());
        assert!(cfg.grok.is_none());
    }

    #[test]
    fn load_from_file_success() {
        let dir = std::env::temp_dir().join("vibecody_ai_config_test");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("ai.toml");
        std::fs::write(&file, r#"
[ollama]
enabled = true
model = "codellama"
"#).unwrap();

        let cfg = AIConfig::load_from_file(&file).unwrap();
        assert!(cfg.ollama.is_some());
        let ollama = cfg.ollama.unwrap();
        assert!(ollama.enabled);
        assert_eq!(ollama.model.as_deref(), Some("codellama"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_from_file_nonexistent_returns_error() {
        let result = AIConfig::load_from_file("/nonexistent/ai.toml");
        assert!(result.is_err());
    }

    #[test]
    fn load_from_file_invalid_toml_returns_error() {
        let dir = std::env::temp_dir().join("vibecody_ai_config_bad");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("bad.toml");
        std::fs::write(&file, "this is not valid { toml").unwrap();

        let result = AIConfig::load_from_file(&file);
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn provider_config_file_serde_roundtrip() {
        let pcf = ProviderConfigFile {
            enabled: true,
            api_url: Some("http://localhost:11434".to_string()),
            model: Some("llama3".to_string()),
            api_key: None,
            max_tokens: Some(4096),
            temperature: Some(0.7),
        };
        let toml_str = toml::to_string(&pcf).unwrap();
        let back: ProviderConfigFile = toml::from_str(&toml_str).unwrap();
        assert!(back.enabled);
        assert_eq!(back.model.as_deref(), Some("llama3"));
        assert_eq!(back.max_tokens, Some(4096));
        assert!((back.temperature.unwrap() - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn ai_config_serde_roundtrip() {
        let cfg = AIConfig {
            ollama: Some(ProviderConfigFile {
                enabled: true,
                api_url: None,
                model: Some("codellama".to_string()),
                api_key: None,
                max_tokens: None,
                temperature: None,
            }),
            openai: None,
            claude: None,
            gemini: None,
            grok: None,
        };
        let toml_str = toml::to_string(&cfg).unwrap();
        let back: AIConfig = toml::from_str(&toml_str).unwrap();
        assert!(back.ollama.is_some());
        assert!(back.openai.is_none());
    }

    #[test]
    fn empty_toml_loads_as_default() {
        let dir = std::env::temp_dir().join("vibecody_ai_config_empty");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("empty.toml");
        std::fs::write(&file, "").unwrap();

        let cfg = AIConfig::load_from_file(&file).unwrap();
        assert!(cfg.ollama.is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
