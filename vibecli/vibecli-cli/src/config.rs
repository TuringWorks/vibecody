//! Configuration management

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use vibe_ai::hooks::HookConfig;
use vibe_ai::mcp::McpServerConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub index: IndexConfig,

    /// OpenTelemetry tracing configuration.
    ///
    /// ```toml
    /// [otel]
    /// enabled = true
    /// endpoint = "http://localhost:4318"  # OTLP/HTTP
    /// service_name = "vibecli"
    /// ```
    #[serde(default)]
    pub otel: OtelConfig,

    pub ollama: Option<ProviderConfig>,
    pub openai: Option<ProviderConfig>,
    pub claude: Option<ProviderConfig>,
    pub gemini: Option<ProviderConfig>,
    pub grok: Option<ProviderConfig>,

    /// MCP server definitions.  Example:
    /// ```toml
    /// [[mcp_servers]]
    /// name = "github"
    /// command = "npx @modelcontextprotocol/server-github"
    /// ```
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,

    /// Hook definitions.  Example:
    /// ```toml
    /// [[hooks]]
    /// event = "PostToolUse"
    /// tools = ["write_file"]
    /// handler = { command = "sh .vibecli/hooks/format.sh" }
    /// ```
    #[serde(default)]
    pub hooks: Vec<HookConfig>,

    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub safety: SafetyConfig,

    /// Web search tool configuration.
    #[serde(default)]
    pub tools: ToolsConfig,
}

/// Embedding index configuration.
///
/// ```toml
/// [index]
/// enabled = true
/// embedding_provider = "ollama"
/// embedding_model = "nomic-embed-text"
/// rebuild_on_startup = false
/// max_file_size_kb = 500
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// "ollama" or "openai"
    #[serde(default = "IndexConfig::default_provider")]
    pub embedding_provider: String,
    #[serde(default = "IndexConfig::default_model")]
    pub embedding_model: String,
    /// Rebuild the full index every time the agent starts.
    #[serde(default)]
    pub rebuild_on_startup: bool,
    #[serde(default = "IndexConfig::default_max_file_size_kb")]
    pub max_file_size_kb: u64,
}

impl IndexConfig {
    fn default_provider() -> String { "ollama".to_string() }
    fn default_model() -> String { "nomic-embed-text".to_string() }
    fn default_max_file_size_kb() -> u64 { 500 }
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            embedding_provider: "ollama".to_string(),
            embedding_model: "nomic-embed-text".to_string(),
            rebuild_on_startup: false,
            max_file_size_kb: 500,
        }
    }
}

/// Configuration for agent tools.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolsConfig {
    #[serde(default)]
    pub web_search: WebSearchConfig,
}

/// DuckDuckGo (default) or Google CSE web search configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// "duckduckgo" (default, no key) or "google"
    #[serde(default = "default_engine")]
    pub engine: String,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            engine: "duckduckgo".to_string(),
            max_results: 5,
        }
    }
}

/// OpenTelemetry tracing configuration.
///
/// ```toml
/// [otel]
/// enabled = false
/// endpoint = "http://localhost:4318"
/// service_name = "vibecli"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelConfig {
    /// Whether to enable OTLP export. Defaults to `false`.
    #[serde(default)]
    pub enabled: bool,
    /// OTLP/HTTP endpoint. Defaults to `http://localhost:4318`.
    #[serde(default = "OtelConfig::default_endpoint")]
    pub endpoint: String,
    /// Service name reported in spans. Defaults to `"vibecli"`.
    #[serde(default = "OtelConfig::default_service_name")]
    pub service_name: String,
}

impl OtelConfig {
    fn default_endpoint() -> String { "http://localhost:4318".to_string() }
    fn default_service_name() -> String { "vibecli".to_string() }
}

impl Default for OtelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: Self::default_endpoint(),
            service_name: Self::default_service_name(),
        }
    }
}

fn default_true() -> bool { true }
fn default_engine() -> String { "duckduckgo".to_string() }
fn default_max_results() -> usize { 5 }

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
    /// Shell environment policy for subprocess tool calls.
    #[serde(default)]
    pub shell_environment: ShellEnvironmentConfig,
}

/// Fine-grained control over what environment variables subprocess tool calls inherit.
///
/// ```toml
/// [safety.shell_environment]
/// inherit = "core"
/// include = ["CARGO_HOME", "RUSTUP_HOME"]
/// exclude = ["AWS_SECRET_*", "*_API_KEY"]
/// [safety.shell_environment.set]
/// VIBECLI_AGENT = "1"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellEnvironmentConfig {
    /// Base inheritance: "all" (default) | "core" | "none"
    #[serde(default = "ShellEnvironmentConfig::default_inherit")]
    pub inherit: String,
    /// Extra variable names / patterns to include.
    #[serde(default)]
    pub include: Vec<String>,
    /// Variable names / patterns to exclude.
    #[serde(default)]
    pub exclude: Vec<String>,
    /// Variables to forcibly set.
    #[serde(default)]
    pub set: HashMap<String, String>,
}

impl ShellEnvironmentConfig {
    fn default_inherit() -> String { "all".to_string() }

    /// Convert to the ToolExecutor's ShellEnvPolicy.
    pub fn to_policy(&self) -> crate::tool_executor::ShellEnvPolicy {
        crate::tool_executor::ShellEnvPolicy {
            inherit: self.inherit.clone(),
            include: self.include.clone(),
            exclude: self.exclude.clone(),
            set: self.set.clone(),
        }
    }
}

impl Default for ShellEnvironmentConfig {
    fn default() -> Self {
        Self {
            inherit: "all".to_string(),
            include: vec![],
            exclude: vec![],
            set: HashMap::new(),
        }
    }
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
            shell_environment: ShellEnvironmentConfig::default(),
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
