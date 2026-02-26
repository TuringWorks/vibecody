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
    /// Groq ultra-fast inference (GROQ_API_KEY).
    pub groq: Option<ProviderConfig>,
    /// OpenRouter unified gateway — 300+ models (OPENROUTER_API_KEY).
    pub openrouter: Option<ProviderConfig>,
    /// Azure OpenAI service (AZURE_OPENAI_API_KEY + azure_openai.api_url).
    pub azure_openai: Option<ProviderConfig>,
    /// AWS Bedrock — run Claude/Titan/Llama via AWS.
    ///
    /// ```toml
    /// [bedrock]
    /// enabled = true
    /// region = "us-east-1"
    /// model = "anthropic.claude-3-5-sonnet-20241022-v2:0"
    /// # Credentials come from the standard AWS env vars:
    /// # AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, AWS_SESSION_TOKEN
    /// ```
    pub bedrock: Option<BedrockConfig>,
    /// GitHub Copilot — use the Copilot API for completions/chat.
    ///
    /// ```toml
    /// [copilot]
    /// enabled = true
    /// # Token is loaded from ~/.config/github-copilot/hosts.json automatically.
    /// # You may also set COPILOT_TOKEN env var.
    /// model = "gpt-4o"
    /// ```
    pub copilot: Option<CopilotConfig>,

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

    /// Auto memory recording.
    ///
    /// ```toml
    /// [memory]
    /// auto_record = true
    /// min_session_steps = 3
    /// ```
    #[serde(default)]
    pub memory: MemoryConfig,

    /// opusplan model routing.
    ///
    /// Separate provider/model for the planning step vs. the execution step.
    /// Falls back to `--provider`/`--model` flags when not set.
    ///
    /// ```toml
    /// [routing]
    /// planning_provider = "claude"
    /// planning_model = "claude-opus-4-6"
    /// execution_provider = "claude"
    /// execution_model = "claude-sonnet-4-6"
    /// ```
    #[serde(default)]
    pub routing: RoutingConfig,

    /// Messaging gateway configuration (Telegram, Discord, Slack bot mode).
    ///
    /// ```toml
    /// [gateway]
    /// platform = "telegram"
    /// telegram_token = "1234567:ABCDEF..."
    /// allowed_users = ["@alice"]
    /// ```
    #[serde(default)]
    pub gateway: GatewayConfig,

    /// Linear API key for issue tracking integration.
    /// Alternatively, set the LINEAR_API_KEY environment variable.
    ///
    /// ```toml
    /// linear_api_key = "lin_api_..."
    /// ```
    #[serde(default)]
    pub linear_api_key: Option<String>,

    /// Red team security scanning configuration.
    ///
    /// ```toml
    /// [redteam]
    /// max_depth = 3
    /// timeout_secs = 300
    /// parallel_agents = 3
    /// auto_report = true
    /// ```
    #[serde(default)]
    pub redteam: RedTeamCfg,
}

/// Configuration for the red team security scanning module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedTeamCfg {
    /// Maximum crawl depth for reconnaissance (default: 3).
    #[serde(default = "RedTeamCfg::default_max_depth")]
    pub max_depth: usize,
    /// Per-stage timeout in seconds (default: 300).
    #[serde(default = "RedTeamCfg::default_timeout")]
    pub timeout_secs: u64,
    /// Number of parallel exploitation agents (default: 3).
    #[serde(default = "RedTeamCfg::default_parallel")]
    pub parallel_agents: usize,
    /// URL patterns in scope (glob-style, default: ["*"]).
    #[serde(default = "RedTeamCfg::default_scope")]
    pub scope_patterns: Vec<String>,
    /// URL patterns to exclude from testing.
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    /// Path to auth configuration YAML file.
    #[serde(default)]
    pub auth_config: Option<String>,
    /// Automatically generate report after scan completion (default: true).
    #[serde(default = "default_true")]
    pub auto_report: bool,
}

impl RedTeamCfg {
    fn default_max_depth() -> usize { 3 }
    fn default_timeout() -> u64 { 300 }
    fn default_parallel() -> usize { 3 }
    fn default_scope() -> Vec<String> { vec!["*".to_string()] }
}

impl Default for RedTeamCfg {
    fn default() -> Self {
        Self {
            max_depth: 3,
            timeout_secs: 300,
            parallel_agents: 3,
            scope_patterns: vec!["*".to_string()],
            exclude_patterns: vec![],
            auth_config: None,
            auto_report: true,
        }
    }
}

/// Gateway configuration (inlined here to avoid circular dependency with gateway module).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GatewayConfig {
    /// Platform: "telegram" | "discord" | "slack"
    pub platform: Option<String>,
    pub telegram_token: Option<String>,
    pub discord_token: Option<String>,
    pub slack_bot_token: Option<String>,
    pub slack_app_token: Option<String>,
    /// Optional whitelist of usernames/user-ids allowed to use the bot.
    #[serde(default)]
    pub allowed_users: Vec<String>,
    /// Maximum characters to send back in a single message (default 4000).
    #[serde(default = "GatewayConfig::default_max_len")]
    pub max_response_length: usize,
    /// Discord channel ID to monitor.
    pub discord_channel_id: Option<String>,
    /// Slack channel ID to monitor.
    pub slack_channel_id: Option<String>,
}

impl GatewayConfig {
    fn default_max_len() -> usize { 4000 }

    pub fn resolve_telegram_token(&self) -> Option<String> {
        self.telegram_token.clone().or_else(|| std::env::var("TELEGRAM_BOT_TOKEN").ok())
    }
    pub fn resolve_discord_token(&self) -> Option<String> {
        self.discord_token.clone().or_else(|| std::env::var("DISCORD_BOT_TOKEN").ok())
    }
    pub fn resolve_slack_bot_token(&self) -> Option<String> {
        self.slack_bot_token.clone().or_else(|| std::env::var("SLACK_BOT_TOKEN").ok())
    }
}

/// Provider/model routing for planning vs. execution steps.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    /// Provider used for the planning / thinking step (e.g. "claude").
    pub planning_provider: Option<String>,
    /// Model used for the planning step (e.g. "claude-opus-4-6").
    pub planning_model: Option<String>,
    /// Provider used for tool-execution steps (e.g. "claude").
    pub execution_provider: Option<String>,
    /// Model used for tool-execution steps (e.g. "claude-sonnet-4-6").
    pub execution_model: Option<String>,
}

impl RoutingConfig {
    /// Effective planning provider: routing config → fallback.
    pub fn resolve_planning(&self, fallback_provider: &str, fallback_model: &str) -> (String, String) {
        (
            self.planning_provider.clone().unwrap_or_else(|| fallback_provider.to_string()),
            self.planning_model.clone().unwrap_or_else(|| fallback_model.to_string()),
        )
    }

    /// Effective execution provider: routing config → fallback.
    pub fn resolve_execution(&self, fallback_provider: &str, fallback_model: &str) -> (String, String) {
        (
            self.execution_provider.clone().unwrap_or_else(|| fallback_provider.to_string()),
            self.execution_model.clone().unwrap_or_else(|| fallback_model.to_string()),
        )
    }

    /// Returns true if any routing config is set (planning or execution differs from fallback).
    pub fn is_configured(&self) -> bool {
        self.planning_provider.is_some()
            || self.planning_model.is_some()
            || self.execution_provider.is_some()
            || self.execution_model.is_some()
    }
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

/// Web search configuration supporting DuckDuckGo (default), Tavily, and Brave Search.
///
/// ```toml
/// [tools.web_search]
/// enabled = true
/// engine = "tavily"          # "duckduckgo" | "tavily" | "brave"
/// max_results = 5
/// tavily_api_key = "tvly-..."     # or TAVILY_API_KEY env var
/// brave_api_key = "BSA..."        # or BRAVE_API_KEY env var
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// "duckduckgo" (default, no key) | "tavily" | "brave"
    #[serde(default = "default_engine")]
    pub engine: String,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    /// Tavily API key (https://app.tavily.com). Falls back to TAVILY_API_KEY env var.
    pub tavily_api_key: Option<String>,
    /// Brave Search API key (https://api.search.brave.com). Falls back to BRAVE_API_KEY env var.
    pub brave_api_key: Option<String>,
}

impl WebSearchConfig {
    /// Resolve Tavily API key: config field first, then TAVILY_API_KEY env var.
    pub fn resolve_tavily_key(&self) -> Option<String> {
        self.tavily_api_key.clone().or_else(|| std::env::var("TAVILY_API_KEY").ok())
    }
    /// Resolve Brave API key: config field first, then BRAVE_API_KEY env var.
    pub fn resolve_brave_key(&self) -> Option<String> {
        self.brave_api_key.clone().or_else(|| std::env::var("BRAVE_API_KEY").ok())
    }
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            engine: "duckduckgo".to_string(),
            max_results: 5,
            tavily_api_key: None,
            brave_api_key: None,
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
    /// On macOS: uses sandbox-exec (Seatbelt). On Linux: uses bwrap.
    #[serde(default)]
    pub sandbox: bool,
    /// Optional path to a custom sandbox profile (macOS .sb or Linux bwrap config).
    /// When unset, a built-in profile is used.
    #[serde(default)]
    pub sandbox_profile: Option<String>,
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
            sandbox_profile: None,
            shell_environment: ShellEnvironmentConfig::default(),
        }
    }
}

/// Auto memory recording configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// When true, summarize completed sessions and append key learnings to `~/.vibecli/memory.md`.
    #[serde(default)]
    pub auto_record: bool,
    /// Minimum number of tool-use steps before auto-recording triggers.
    #[serde(default = "default_min_steps")]
    pub min_session_steps: usize,
}

fn default_min_steps() -> usize { 3 }

impl Default for MemoryConfig {
    fn default() -> Self {
        Self { auto_record: false, min_session_steps: 3 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub api_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    /// Helper script path to fetch a rotating API key.
    /// E.g. `~/.vibecli/get-key.sh claude`
    #[serde(default)]
    pub api_key_helper: Option<String>,
    /// Extended thinking budget tokens (Claude only).
    #[serde(default)]
    pub thinking_budget_tokens: Option<u32>,
}

/// AWS Bedrock provider configuration.
///
/// Credentials are resolved via the standard AWS credential chain:
/// `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` (+ optional `AWS_SESSION_TOKEN`),
/// `~/.aws/credentials`, EC2/ECS instance roles, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BedrockConfig {
    /// Whether this provider is active.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// AWS region (default: `us-east-1`).
    #[serde(default = "BedrockConfig::default_region")]
    pub region: String,
    /// Bedrock model ID (default: `anthropic.claude-3-5-sonnet-20241022-v2:0`).
    #[serde(default = "BedrockConfig::default_model")]
    pub model: String,
    /// Optional cross-account IAM role ARN to assume before calling Bedrock.
    pub role_arn: Option<String>,
}

impl BedrockConfig {
    fn default_region() -> String { "us-east-1".to_string() }
    fn default_model() -> String { "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string() }
}

impl Default for BedrockConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            region: Self::default_region(),
            model: Self::default_model(),
            role_arn: None,
        }
    }
}

/// GitHub Copilot provider configuration.
///
/// The OAuth token is resolved from (in order):
/// 1. `COPILOT_TOKEN` environment variable
/// 2. `~/.config/github-copilot/hosts.json` (written by the official VS Code extension)
/// 3. This config's `token` field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotConfig {
    /// Whether this provider is active.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Model to request (default: `gpt-4o`).
    #[serde(default = "CopilotConfig::default_model")]
    pub model: String,
    /// Explicit OAuth token (prefer env var or hosts.json for security).
    pub token: Option<String>,
}

impl CopilotConfig {
    fn default_model() -> String { "gpt-4o".to_string() }

    /// Resolve the Copilot token from env → hosts.json → config field.
    #[allow(dead_code)]
    pub fn resolve_token(&self) -> Option<String> {
        // 1. Environment variable
        if let Ok(t) = std::env::var("COPILOT_TOKEN") {
            return Some(t);
        }
        // 2. VS Code Copilot hosts.json
        let hosts_path = std::env::var("HOME").ok()
            .map(|h| std::path::PathBuf::from(h).join(".config").join("github-copilot").join("hosts.json"));
        if let Some(path) = hosts_path {
            if let Ok(raw) = std::fs::read_to_string(&path) {
                // hosts.json structure: { "github.com": { "oauth_token": "..." } }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Some(tok) = v["github.com"]["oauth_token"].as_str() {
                        return Some(tok.to_string());
                    }
                }
            }
        }
        // 3. Config field
        self.token.clone()
    }
}

impl Default for CopilotConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model: Self::default_model(),
            token: None,
        }
    }
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
            "groq" => self.groq.as_ref(),
            "openrouter" => self.openrouter.as_ref(),
            "azure_openai" | "azure" => self.azure_openai.as_ref(),
            _ => None,
        }
    }

}
