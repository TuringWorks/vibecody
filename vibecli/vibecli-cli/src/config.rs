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

    /// GitHub App CI/CD review bot configuration.
    ///
    /// ```toml
    /// [github_app]
    /// app_id = 12345
    /// private_key_path = "path/to/key.pem"
    /// webhook_secret = "your-webhook-secret"
    /// auto_fix = false
    /// severity_threshold = "high"
    /// ```
    #[serde(default)]
    pub github_app: crate::github_app::GithubAppConfig,
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
    /// Platform: "telegram" | "discord" | "slack" | "teams" | "twilio" | "whatsapp" | "signal" | "imessage" | "matrix"
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

    // ── Signal ──
    /// Base URL of the signal-cli REST API (e.g. "http://localhost:8080").
    pub signal_api_url: Option<String>,
    /// Registered phone number for signal-cli (e.g. "+15551234567").
    pub signal_phone_number: Option<String>,

    // ── Matrix ──
    /// Matrix homeserver URL (e.g. "https://matrix.org").
    pub matrix_homeserver_url: Option<String>,
    /// Matrix access token for the bot account.
    pub matrix_access_token: Option<String>,
    /// Matrix room ID to monitor (e.g. "!abc123:matrix.org").
    pub matrix_room_id: Option<String>,
    /// Matrix user ID of the bot (e.g. "@vibecli:matrix.org") — used to skip own messages.
    pub matrix_user_id: Option<String>,

    // ── Twilio SMS ──
    /// Twilio Account SID (starts with "AC").
    pub twilio_account_sid: Option<String>,
    /// Twilio Auth Token.
    pub twilio_auth_token: Option<String>,
    /// Twilio sender phone number (e.g. "+15559876543").
    pub twilio_from_number: Option<String>,

    // ── WhatsApp (Meta Cloud API) ──
    /// WhatsApp Business permanent access token.
    pub whatsapp_access_token: Option<String>,
    /// WhatsApp Phone Number ID from Meta Business dashboard.
    pub whatsapp_phone_number_id: Option<String>,
    /// Verify token for webhook registration.
    pub whatsapp_verify_token: Option<String>,
    /// Port for the WhatsApp webhook receiver (default 8443).
    pub whatsapp_webhook_port: Option<u16>,

    // ── iMessage (macOS only) ──
    /// Path to the Messages chat.db (default: ~/Library/Messages/chat.db).
    pub imessage_db_path: Option<String>,

    // ── Microsoft Teams ──
    /// Azure AD Tenant ID.
    pub teams_tenant_id: Option<String>,
    /// Azure Bot Client ID.
    pub teams_client_id: Option<String>,
    /// Azure Bot Client Secret.
    pub teams_client_secret: Option<String>,
    /// Port for the Teams webhook receiver (default 3978).
    pub teams_webhook_port: Option<u16>,
}

#[allow(dead_code)]
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

    // ── Signal ──
    pub fn resolve_signal_api_url(&self) -> Option<String> {
        self.signal_api_url.clone().or_else(|| std::env::var("SIGNAL_API_URL").ok())
    }
    pub fn resolve_signal_phone_number(&self) -> Option<String> {
        self.signal_phone_number.clone().or_else(|| std::env::var("SIGNAL_PHONE_NUMBER").ok())
    }

    // ── Matrix ──
    pub fn resolve_matrix_homeserver_url(&self) -> Option<String> {
        self.matrix_homeserver_url.clone().or_else(|| std::env::var("MATRIX_HOMESERVER_URL").ok())
    }
    pub fn resolve_matrix_access_token(&self) -> Option<String> {
        self.matrix_access_token.clone().or_else(|| std::env::var("MATRIX_ACCESS_TOKEN").ok())
    }
    pub fn resolve_matrix_room_id(&self) -> Option<String> {
        self.matrix_room_id.clone().or_else(|| std::env::var("MATRIX_ROOM_ID").ok())
    }
    pub fn resolve_matrix_user_id(&self) -> Option<String> {
        self.matrix_user_id.clone().or_else(|| std::env::var("MATRIX_USER_ID").ok())
    }

    // ── Twilio SMS ──
    pub fn resolve_twilio_account_sid(&self) -> Option<String> {
        self.twilio_account_sid.clone().or_else(|| std::env::var("TWILIO_ACCOUNT_SID").ok())
    }
    pub fn resolve_twilio_auth_token(&self) -> Option<String> {
        self.twilio_auth_token.clone().or_else(|| std::env::var("TWILIO_AUTH_TOKEN").ok())
    }
    pub fn resolve_twilio_from_number(&self) -> Option<String> {
        self.twilio_from_number.clone().or_else(|| std::env::var("TWILIO_FROM_NUMBER").ok())
    }

    // ── WhatsApp ──
    pub fn resolve_whatsapp_access_token(&self) -> Option<String> {
        self.whatsapp_access_token.clone().or_else(|| std::env::var("WHATSAPP_ACCESS_TOKEN").ok())
    }
    pub fn resolve_whatsapp_phone_number_id(&self) -> Option<String> {
        self.whatsapp_phone_number_id.clone().or_else(|| std::env::var("WHATSAPP_PHONE_NUMBER_ID").ok())
    }
    pub fn resolve_whatsapp_verify_token(&self) -> Option<String> {
        self.whatsapp_verify_token.clone().or_else(|| std::env::var("WHATSAPP_VERIFY_TOKEN").ok())
    }

    // ── iMessage ──
    pub fn resolve_imessage_db_path(&self) -> Option<String> {
        self.imessage_db_path.clone().or_else(|| std::env::var("IMESSAGE_DB_PATH").ok())
    }

    // ── Teams ──
    pub fn resolve_teams_tenant_id(&self) -> Option<String> {
        self.teams_tenant_id.clone().or_else(|| std::env::var("TEAMS_TENANT_ID").ok())
    }
    pub fn resolve_teams_client_id(&self) -> Option<String> {
        self.teams_client_id.clone().or_else(|| std::env::var("TEAMS_CLIENT_ID").ok())
    }
    pub fn resolve_teams_client_secret(&self) -> Option<String> {
        self.teams_client_secret.clone().or_else(|| std::env::var("TEAMS_CLIENT_SECRET").ok())
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

    pub fn approval_policy_from_flags(_suggest: bool, auto_edit: bool, full_auto: bool) -> String {
        if full_auto {
            "full-auto".to_string()
        } else if auto_edit {
            "auto-edit".to_string()
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
            // Restrict directory to owner-only on Unix (may contain API keys)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
            }
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(&config_path, &content)?;

        // Restrict config file to owner-only on Unix (contains API keys)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&config_path, fs::Permissions::from_mode(0o600))?;
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default_has_expected_sub_defaults() {
        let cfg = Config::default();
        assert!(cfg.index.enabled);
        assert!(!cfg.otel.enabled);
        assert!(cfg.ollama.is_none());
        assert!(cfg.claude.is_none());
        assert!(cfg.mcp_servers.is_empty());
        assert!(cfg.hooks.is_empty());
        assert_eq!(cfg.safety.approval_policy, "suggest");
        assert!(!cfg.memory.auto_record);
        assert!(!cfg.routing.is_configured());
        assert!(cfg.gateway.platform.is_none());
        assert!(cfg.linear_api_key.is_none());
        assert_eq!(cfg.redteam.max_depth, 3);
    }

    #[test]
    fn approval_policy_full_auto() {
        assert_eq!(SafetyConfig::approval_policy_from_flags(false, false, true), "full-auto");
    }

    #[test]
    fn approval_policy_auto_edit() {
        assert_eq!(SafetyConfig::approval_policy_from_flags(false, true, false), "auto-edit");
    }

    #[test]
    fn approval_policy_suggest_explicit() {
        assert_eq!(SafetyConfig::approval_policy_from_flags(true, false, false), "suggest");
    }

    #[test]
    fn approval_policy_none_defaults_to_suggest() {
        assert_eq!(SafetyConfig::approval_policy_from_flags(false, false, false), "suggest");
    }

    #[test]
    fn routing_resolve_planning_custom() {
        let r = RoutingConfig {
            planning_provider: Some("claude".into()),
            planning_model: Some("opus".into()),
            ..Default::default()
        };
        assert_eq!(r.resolve_planning("ollama", "llama3"), ("claude".into(), "opus".into()));
    }

    #[test]
    fn routing_resolve_planning_fallback() {
        let r = RoutingConfig::default();
        assert_eq!(r.resolve_planning("ollama", "llama3"), ("ollama".into(), "llama3".into()));
    }

    #[test]
    fn routing_resolve_execution_custom() {
        let r = RoutingConfig {
            execution_provider: Some("openai".into()),
            execution_model: Some("gpt-4o".into()),
            ..Default::default()
        };
        assert_eq!(r.resolve_execution("ollama", "llama3"), ("openai".into(), "gpt-4o".into()));
    }

    #[test]
    fn routing_is_configured_none() {
        assert!(!RoutingConfig::default().is_configured());
    }

    #[test]
    fn routing_is_configured_partial() {
        let r = RoutingConfig { planning_provider: Some("claude".into()), ..Default::default() };
        assert!(r.is_configured());
    }

    #[test]
    fn routing_is_configured_all() {
        let r = RoutingConfig {
            planning_provider: Some("claude".into()),
            planning_model: Some("opus".into()),
            execution_provider: Some("openai".into()),
            execution_model: Some("gpt-4o".into()),
        };
        assert!(r.is_configured());
    }

    #[test]
    fn gateway_resolve_telegram_token_from_config() {
        let g = GatewayConfig { telegram_token: Some("tok123".into()), ..Default::default() };
        assert_eq!(g.resolve_telegram_token(), Some("tok123".into()));
    }

    #[test]
    fn gateway_default_max_len_is_4000() {
        assert_eq!(GatewayConfig::default_max_len(), 4000);
        // The serde default function returns 4000; verify via TOML deserialization.
        let g: GatewayConfig = toml::from_str("").expect("empty toml");
        assert_eq!(g.max_response_length, 4000);
    }

    #[test]
    fn copilot_resolve_token_from_config_field() {
        let c = CopilotConfig { token: Some("ghp_abc".into()), ..Default::default() };
        // When env var and hosts.json are absent, config field is used.
        // We cannot guarantee env is clean, so just check it returns Some.
        assert!(c.resolve_token().is_some());
    }

    #[test]
    fn copilot_default() {
        let c = CopilotConfig::default();
        assert!(c.enabled);
        assert_eq!(c.model, "gpt-4o");
        assert!(c.token.is_none());
    }

    #[test]
    fn bedrock_default() {
        let b = BedrockConfig::default();
        assert!(b.enabled);
        assert_eq!(b.region, "us-east-1");
        assert_eq!(b.model, "anthropic.claude-3-5-sonnet-20241022-v2:0");
        assert!(b.role_arn.is_none());
    }

    #[test]
    fn index_config_default() {
        let i = IndexConfig::default();
        assert!(i.enabled);
        assert_eq!(i.embedding_provider, "ollama");
        assert_eq!(i.embedding_model, "nomic-embed-text");
        assert!(!i.rebuild_on_startup);
        assert_eq!(i.max_file_size_kb, 500);
    }

    #[test]
    fn otel_config_default() {
        let o = OtelConfig::default();
        assert!(!o.enabled);
        assert_eq!(o.endpoint, "http://localhost:4318");
        assert_eq!(o.service_name, "vibecli");
    }

    #[test]
    fn web_search_config_default_and_resolve_keys() {
        let w = WebSearchConfig::default();
        assert!(w.enabled);
        assert_eq!(w.engine, "duckduckgo");
        assert_eq!(w.max_results, 5);
        assert!(w.tavily_api_key.is_none());
        assert!(w.brave_api_key.is_none());

        let w2 = WebSearchConfig { tavily_api_key: Some("tvly-key".into()), ..Default::default() };
        assert_eq!(w2.resolve_tavily_key(), Some("tvly-key".into()));

        let w3 = WebSearchConfig { brave_api_key: Some("BSA-key".into()), ..Default::default() };
        assert_eq!(w3.resolve_brave_key(), Some("BSA-key".into()));
    }

    #[test]
    fn config_toml_serde_roundtrip() {
        let mut cfg = Config::default();
        cfg.ollama = Some(ProviderConfig {
            enabled: true,
            api_url: Some("http://localhost:11434".into()),
            model: Some("llama3".into()),
            api_key: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        });
        cfg.routing = RoutingConfig {
            planning_provider: Some("claude".into()),
            planning_model: Some("opus".into()),
            execution_provider: None,
            execution_model: None,
        };
        let toml_str = toml::to_string_pretty(&cfg).expect("serialize");
        let cfg2: Config = toml::from_str(&toml_str).expect("deserialize");
        assert!(cfg2.ollama.is_some());
        let ollama = cfg2.ollama.unwrap();
        assert!(ollama.enabled);
        assert_eq!(ollama.model.as_deref(), Some("llama3"));
        assert_eq!(cfg2.routing.planning_provider.as_deref(), Some("claude"));
        assert!(cfg2.routing.execution_provider.is_none());
    }

    #[test]
    fn provider_config_serde_roundtrip() {
        let pc = ProviderConfig {
            enabled: true,
            api_url: Some("http://example.com".into()),
            model: Some("test-model".into()),
            api_key: Some("sk-test".into()),
            api_key_helper: Some("./get-key.sh".into()),
            thinking_budget_tokens: Some(8000),
        };
        let toml_str = toml::to_string_pretty(&pc).expect("serialize");
        let pc2: ProviderConfig = toml::from_str(&toml_str).expect("deserialize");
        assert!(pc2.enabled);
        assert_eq!(pc2.api_url.as_deref(), Some("http://example.com"));
        assert_eq!(pc2.model.as_deref(), Some("test-model"));
        assert_eq!(pc2.api_key.as_deref(), Some("sk-test"));
        assert_eq!(pc2.api_key_helper.as_deref(), Some("./get-key.sh"));
        assert_eq!(pc2.thinking_budget_tokens, Some(8000));
    }

    #[test]
    fn redteam_cfg_default() {
        let r = RedTeamCfg::default();
        assert_eq!(r.max_depth, 3);
        assert_eq!(r.timeout_secs, 300);
        assert_eq!(r.parallel_agents, 3);
        assert_eq!(r.scope_patterns, vec!["*".to_string()]);
        assert!(r.exclude_patterns.is_empty());
        assert!(r.auth_config.is_none());
        assert!(r.auto_report);
    }

    #[test]
    fn shell_environment_config_default() {
        let s = ShellEnvironmentConfig::default();
        assert_eq!(s.inherit, "all");
        assert!(s.include.is_empty());
        assert!(s.exclude.is_empty());
        assert!(s.set.is_empty());
    }

    #[test]
    fn memory_config_default() {
        let m = MemoryConfig::default();
        assert!(!m.auto_record);
        assert_eq!(m.min_session_steps, 3);
    }

    #[test]
    fn get_provider_config_various_names() {
        let mut cfg = Config::default();
        cfg.claude = Some(ProviderConfig {
            enabled: true, api_url: None, model: None,
            api_key: None, api_key_helper: None, thinking_budget_tokens: None,
        });
        assert!(cfg.get_provider_config("claude").is_some());
        assert!(cfg.get_provider_config("anthropic").is_some());
        assert!(cfg.get_provider_config("Claude").is_some());
        assert!(cfg.get_provider_config("CLAUDE").is_some());
        assert!(cfg.get_provider_config("ollama").is_none());
        assert!(cfg.get_provider_config("unknown").is_none());
        assert!(cfg.get_provider_config("azure").is_none());
        assert!(cfg.get_provider_config("azure_openai").is_none());
    }

    #[test]
    fn ui_config_default_theme_is_dark() {
        let u = UiConfig::default();
        assert_eq!(u.theme.as_deref(), Some("dark"));
    }
}
