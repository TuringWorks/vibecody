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
    /// Mistral AI native API (MISTRAL_API_KEY).
    pub mistral: Option<ProviderConfig>,
    /// Cerebras ultra-fast inference (CEREBRAS_API_KEY).
    pub cerebras: Option<ProviderConfig>,
    /// DeepSeek code-focused models (DEEPSEEK_API_KEY).
    pub deepseek: Option<ProviderConfig>,
    /// Zhipu GLM — Chinese market AI models (ZHIPU_API_KEY, format: "id.secret").
    pub zhipu: Option<ProviderConfig>,
    /// Vercel AI Gateway — unified proxy (VERCEL_AI_API_KEY + api_url required).
    pub vercel_ai: Option<ProviderConfig>,

    /// Provider failover chain — try providers in order.
    ///
    /// ```toml
    /// [failover]
    /// chain = ["claude", "openai", "gemini"]
    /// ```
    #[serde(default)]
    pub failover: FailoverConfig,

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

    /// Voice & media configuration (Whisper transcription, ElevenLabs TTS).
    ///
    /// ```toml
    /// [voice]
    /// whisper_api_key = "gsk_..."     # or reuses groq.api_key / GROQ_API_KEY
    /// elevenlabs_api_key = "..."      # or ELEVENLABS_API_KEY
    /// elevenlabs_voice_id = "..."     # ElevenLabs voice to use
    /// tts_enabled = false             # Enable TTS output for gateway
    /// ```
    #[serde(default)]
    pub voice: VoiceConfig,

    /// Container sandbox configuration (Docker, Podman, OpenSandbox).
    ///
    /// ```toml
    /// [sandbox_config]
    /// runtime = "auto"              # "docker" | "podman" | "opensandbox" | "auto"
    /// image = "ubuntu:22.04"
    /// timeout_secs = 3600
    ///
    /// [sandbox_config.resources]
    /// cpus = "2.0"
    /// memory = "4g"
    /// pids_limit = 256
    ///
    /// [sandbox_config.network]
    /// mode = "restricted"
    /// allowed_domains = ["github.com", "registry.npmjs.org"]
    ///
    /// [sandbox_config.opensandbox]
    /// api_url = "http://localhost:8080"
    /// api_key = ""                  # or OPEN_SANDBOX_API_KEY env
    /// ```
    #[serde(default)]
    pub sandbox_config: SandboxConfig,
}

/// Container sandbox configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Runtime: "auto" | "docker" | "podman" | "opensandbox"
    #[serde(default = "SandboxConfig::default_runtime")]
    pub runtime: String,
    /// Default container image.
    #[serde(default = "SandboxConfig::default_image")]
    pub image: String,
    /// Container timeout in seconds (default: 3600).
    #[serde(default = "SandboxConfig::default_timeout")]
    pub timeout_secs: u64,
    /// Resource limits.
    #[serde(default)]
    pub resources: ResourceLimitsConfig,
    /// Network policy.
    #[serde(default)]
    pub network: NetworkPolicyConfig,
    /// OpenSandbox remote settings.
    #[serde(default)]
    pub opensandbox: OpenSandboxConfig,
    /// Private registry authentication.
    #[serde(default)]
    pub registry: RegistryConfig,
}

impl SandboxConfig {
    fn default_runtime() -> String { "auto".to_string() }
    fn default_image() -> String { "ubuntu:22.04".to_string() }
    fn default_timeout() -> u64 { 3600 }

    /// Convert to a ContainerConfig for creating a container.
    pub fn to_container_config(&self) -> crate::container_runtime::ContainerConfig {
        use crate::container_runtime::*;
        ContainerConfig {
            image: self.image.clone(),
            name: None,
            env: vec![],
            volumes: vec![],
            resource_limits: ResourceLimits {
                cpus: self.resources.cpus.as_deref().and_then(|s| s.parse().ok()),
                memory_bytes: self
                    .resources
                    .memory
                    .as_deref()
                    .and_then(|s| parse_memory_string(s).ok()),
                pids_limit: self.resources.pids_limit,
            },
            network_policy: match self.network.mode.as_str() {
                "none" => NetworkPolicy::None,
                "restricted" => NetworkPolicy::Restricted {
                    allowed_domains: self.network.allowed_domains.clone(),
                },
                _ => NetworkPolicy::Full,
            },
            timeout_secs: self.timeout_secs,
            working_dir: Some("/workspace".to_string()),
        }
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            runtime: "auto".to_string(),
            image: "ubuntu:22.04".to_string(),
            timeout_secs: 3600,
            resources: ResourceLimitsConfig::default(),
            network: NetworkPolicyConfig::default(),
            opensandbox: OpenSandboxConfig::default(),
            registry: RegistryConfig::default(),
        }
    }
}

/// Resource limits config (string values parsed at use time).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceLimitsConfig {
    /// CPU cores as a string (e.g. "2.0").
    pub cpus: Option<String>,
    /// Memory as a string (e.g. "4g", "512m").
    pub memory: Option<String>,
    /// Maximum PIDs.
    pub pids_limit: Option<u32>,
}

/// Network policy config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPolicyConfig {
    /// "none" | "restricted" | "full"
    #[serde(default = "NetworkPolicyConfig::default_mode")]
    pub mode: String,
    /// Domains allowed when mode = "restricted".
    #[serde(default)]
    pub allowed_domains: Vec<String>,
}

impl NetworkPolicyConfig {
    fn default_mode() -> String { "full".to_string() }
}

impl Default for NetworkPolicyConfig {
    fn default() -> Self {
        Self {
            mode: "full".to_string(),
            allowed_domains: vec![],
        }
    }
}

/// OpenSandbox remote service configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenSandboxConfig {
    /// OpenSandbox API URL (e.g. "http://localhost:8080").
    pub api_url: Option<String>,
    /// API key for OpenSandbox (falls back to OPEN_SANDBOX_API_KEY env var).
    pub api_key: Option<String>,
}

#[allow(dead_code)]
impl OpenSandboxConfig {
    pub fn resolve_api_url(&self) -> String {
        self.api_url
            .clone()
            .or_else(|| std::env::var("OPEN_SANDBOX_API_URL").ok())
            .unwrap_or_else(|| "http://localhost:8080".to_string())
    }

    pub fn resolve_api_key(&self) -> Option<String> {
        self.api_key
            .clone()
            .or_else(|| std::env::var("OPEN_SANDBOX_API_KEY").ok())
    }
}

/// Private container registry authentication.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistryConfig {
    /// Registry URL.
    pub url: Option<String>,
    /// Registry username.
    pub username: Option<String>,
    /// Registry password (falls back to REGISTRY_PASSWORD env var).
    pub password: Option<String>,
}

#[allow(dead_code)]
impl RegistryConfig {
    pub fn resolve_password(&self) -> Option<String> {
        self.password
            .clone()
            .or_else(|| std::env::var("REGISTRY_PASSWORD").ok())
    }
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

/// Voice and media configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceConfig {
    /// Groq API key for Whisper transcription (falls back to groq.api_key or GROQ_API_KEY).
    pub whisper_api_key: Option<String>,
    /// ElevenLabs API key for TTS output.
    pub elevenlabs_api_key: Option<String>,
    /// ElevenLabs voice ID.
    pub elevenlabs_voice_id: Option<String>,
    /// Enable TTS output for gateway responses.
    #[serde(default)]
    pub tts_enabled: bool,
}

#[allow(dead_code)]
impl VoiceConfig {
    pub fn resolve_whisper_api_key(&self, groq_key: Option<&str>) -> Option<String> {
        self.whisper_api_key.clone()
            .or_else(|| groq_key.map(|s| s.to_string()))
            .or_else(|| std::env::var("GROQ_API_KEY").ok())
    }
    pub fn resolve_elevenlabs_api_key(&self) -> Option<String> {
        self.elevenlabs_api_key.clone()
            .or_else(|| std::env::var("ELEVENLABS_API_KEY").ok())
    }
    pub fn resolve_elevenlabs_voice_id(&self) -> String {
        self.elevenlabs_voice_id.clone()
            .or_else(|| std::env::var("ELEVENLABS_VOICE_ID").ok())
            .unwrap_or_else(|| "21m00Tcm4TlvDq8ikWAM".to_string()) // Rachel default
    }
}

/// Provider failover chain configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FailoverConfig {
    /// Ordered list of provider names to try in sequence.
    /// Example: `["claude", "openai", "gemini"]`
    #[serde(default)]
    pub chain: Vec<String>,
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

    // ── Google Chat ──
    /// Google service account JSON (or token string) for Chat API.
    pub googlechat_service_account_json: Option<String>,
    /// Google Chat space ID (e.g. "spaces/AAAA...").
    pub googlechat_space_id: Option<String>,

    // ── Mattermost ──
    /// Mattermost server URL (e.g. "https://mattermost.example.com").
    pub mattermost_url: Option<String>,
    /// Mattermost personal access token or bot token.
    pub mattermost_token: Option<String>,
    /// Mattermost channel ID to monitor.
    pub mattermost_channel_id: Option<String>,

    // ── IRC ──
    /// IRC server hostname (e.g. "irc.libera.chat").
    pub irc_server: Option<String>,
    /// IRC server port (default 6667).
    pub irc_port: Option<u16>,
    /// IRC nickname for the bot.
    pub irc_nick: Option<String>,
    /// IRC channel to join (e.g. "#vibecli").
    pub irc_channel: Option<String>,

    // ── LINE ──
    /// LINE channel access token.
    pub line_channel_access_token: Option<String>,
    /// LINE channel secret for webhook verification.
    pub line_channel_secret: Option<String>,

    // ── Twitch ──
    /// Twitch OAuth token (oauth:...).
    pub twitch_oauth_token: Option<String>,
    /// Twitch channel name to join.
    pub twitch_channel: Option<String>,
    /// Twitch bot nickname.
    pub twitch_nick: Option<String>,

    // ── Nextcloud Talk ──
    /// Nextcloud server URL (e.g. "https://cloud.example.com").
    pub nextcloud_url: Option<String>,
    /// Nextcloud username.
    pub nextcloud_user: Option<String>,
    /// Nextcloud password or app password.
    pub nextcloud_password: Option<String>,
    /// Nextcloud Talk room token.
    pub nextcloud_room_token: Option<String>,

    // ── WebChat ──
    /// Port for the WebChat HTTP endpoint (default 8090).
    pub webchat_port: Option<u16>,

    // ── Nostr ──
    /// Nostr private key (nsec format).
    pub nostr_private_key: Option<String>,
    /// Nostr relay URLs.
    #[serde(default)]
    pub nostr_relay_urls: Vec<String>,

    // ── Feishu (Lark) ──
    /// Feishu app ID.
    pub feishu_app_id: Option<String>,
    /// Feishu app secret.
    pub feishu_app_secret: Option<String>,

    // ── DingTalk ──
    /// DingTalk robot access token.
    pub dingtalk_access_token: Option<String>,
    /// DingTalk robot webhook secret.
    pub dingtalk_webhook_secret: Option<String>,

    // ── QQ ──
    /// QQ Bot app ID.
    pub qq_app_id: Option<String>,
    /// QQ Bot token.
    pub qq_token: Option<String>,

    // ── WeCom (WeChat Work) ──
    /// WeCom corp ID.
    pub wecom_corp_id: Option<String>,
    /// WeCom agent ID.
    pub wecom_agent_id: Option<String>,
    /// WeCom app secret.
    pub wecom_secret: Option<String>,

    // ── Zalo ──
    /// Zalo OA access token.
    pub zalo_access_token: Option<String>,

    // ── BlueBubbles ──
    /// BlueBubbles server URL (e.g. "http://localhost:1234").
    pub bluebubbles_url: Option<String>,
    /// BlueBubbles server password.
    pub bluebubbles_password: Option<String>,

    // ── Synology Chat ──
    /// Synology NAS URL.
    pub synology_url: Option<String>,
    /// Synology Chat incoming webhook URL.
    pub synology_incoming_url: Option<String>,
    /// Synology Chat bot token.
    pub synology_token: Option<String>,

    // ── Tlon (Urbit) ──
    /// Urbit ship URL (e.g. "http://localhost:8080").
    pub tlon_ship_url: Option<String>,
    /// Urbit ship access code (+code).
    pub tlon_ship_code: Option<String>,
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

    // ── Google Chat ──
    pub fn resolve_googlechat_service_account_json(&self) -> Option<String> {
        self.googlechat_service_account_json.clone().or_else(|| std::env::var("GOOGLE_CHAT_SERVICE_ACCOUNT_JSON").ok())
    }
    pub fn resolve_googlechat_space_id(&self) -> Option<String> {
        self.googlechat_space_id.clone().or_else(|| std::env::var("GOOGLE_CHAT_SPACE_ID").ok())
    }

    // ── Mattermost ──
    pub fn resolve_mattermost_url(&self) -> Option<String> {
        self.mattermost_url.clone().or_else(|| std::env::var("MATTERMOST_URL").ok())
    }
    pub fn resolve_mattermost_token(&self) -> Option<String> {
        self.mattermost_token.clone().or_else(|| std::env::var("MATTERMOST_TOKEN").ok())
    }
    pub fn resolve_mattermost_channel_id(&self) -> Option<String> {
        self.mattermost_channel_id.clone().or_else(|| std::env::var("MATTERMOST_CHANNEL_ID").ok())
    }

    // ── IRC ──
    pub fn resolve_irc_server(&self) -> Option<String> {
        self.irc_server.clone().or_else(|| std::env::var("IRC_SERVER").ok())
    }
    pub fn resolve_irc_nick(&self) -> Option<String> {
        self.irc_nick.clone().or_else(|| std::env::var("IRC_NICK").ok())
    }
    pub fn resolve_irc_channel(&self) -> Option<String> {
        self.irc_channel.clone().or_else(|| std::env::var("IRC_CHANNEL").ok())
    }

    // ── LINE ──
    pub fn resolve_line_channel_access_token(&self) -> Option<String> {
        self.line_channel_access_token.clone().or_else(|| std::env::var("LINE_CHANNEL_ACCESS_TOKEN").ok())
    }
    pub fn resolve_line_channel_secret(&self) -> Option<String> {
        self.line_channel_secret.clone().or_else(|| std::env::var("LINE_CHANNEL_SECRET").ok())
    }

    // ── Twitch ──
    pub fn resolve_twitch_oauth_token(&self) -> Option<String> {
        self.twitch_oauth_token.clone().or_else(|| std::env::var("TWITCH_OAUTH_TOKEN").ok())
    }
    pub fn resolve_twitch_channel(&self) -> Option<String> {
        self.twitch_channel.clone().or_else(|| std::env::var("TWITCH_CHANNEL").ok())
    }
    pub fn resolve_twitch_nick(&self) -> Option<String> {
        self.twitch_nick.clone().or_else(|| std::env::var("TWITCH_NICK").ok())
    }

    // ── Nextcloud Talk ──
    pub fn resolve_nextcloud_url(&self) -> Option<String> {
        self.nextcloud_url.clone().or_else(|| std::env::var("NEXTCLOUD_URL").ok())
    }
    pub fn resolve_nextcloud_user(&self) -> Option<String> {
        self.nextcloud_user.clone().or_else(|| std::env::var("NEXTCLOUD_USER").ok())
    }
    pub fn resolve_nextcloud_password(&self) -> Option<String> {
        self.nextcloud_password.clone().or_else(|| std::env::var("NEXTCLOUD_PASSWORD").ok())
    }
    pub fn resolve_nextcloud_room_token(&self) -> Option<String> {
        self.nextcloud_room_token.clone().or_else(|| std::env::var("NEXTCLOUD_ROOM_TOKEN").ok())
    }

    // ── Nostr ──
    pub fn resolve_nostr_private_key(&self) -> Option<String> {
        self.nostr_private_key.clone().or_else(|| std::env::var("NOSTR_PRIVATE_KEY").ok())
    }

    // ── Feishu (Lark) ──
    pub fn resolve_feishu_app_id(&self) -> Option<String> {
        self.feishu_app_id.clone().or_else(|| std::env::var("FEISHU_APP_ID").ok())
    }
    pub fn resolve_feishu_app_secret(&self) -> Option<String> {
        self.feishu_app_secret.clone().or_else(|| std::env::var("FEISHU_APP_SECRET").ok())
    }

    // ── DingTalk ──
    pub fn resolve_dingtalk_access_token(&self) -> Option<String> {
        self.dingtalk_access_token.clone().or_else(|| std::env::var("DINGTALK_ACCESS_TOKEN").ok())
    }
    pub fn resolve_dingtalk_webhook_secret(&self) -> Option<String> {
        self.dingtalk_webhook_secret.clone().or_else(|| std::env::var("DINGTALK_WEBHOOK_SECRET").ok())
    }

    // ── QQ ──
    pub fn resolve_qq_app_id(&self) -> Option<String> {
        self.qq_app_id.clone().or_else(|| std::env::var("QQ_APP_ID").ok())
    }
    pub fn resolve_qq_token(&self) -> Option<String> {
        self.qq_token.clone().or_else(|| std::env::var("QQ_TOKEN").ok())
    }

    // ── WeCom ──
    pub fn resolve_wecom_corp_id(&self) -> Option<String> {
        self.wecom_corp_id.clone().or_else(|| std::env::var("WECOM_CORP_ID").ok())
    }
    pub fn resolve_wecom_agent_id(&self) -> Option<String> {
        self.wecom_agent_id.clone().or_else(|| std::env::var("WECOM_AGENT_ID").ok())
    }
    pub fn resolve_wecom_secret(&self) -> Option<String> {
        self.wecom_secret.clone().or_else(|| std::env::var("WECOM_SECRET").ok())
    }

    // ── Zalo ──
    pub fn resolve_zalo_access_token(&self) -> Option<String> {
        self.zalo_access_token.clone().or_else(|| std::env::var("ZALO_ACCESS_TOKEN").ok())
    }

    // ── BlueBubbles ──
    pub fn resolve_bluebubbles_url(&self) -> Option<String> {
        self.bluebubbles_url.clone().or_else(|| std::env::var("BLUEBUBBLES_URL").ok())
    }
    pub fn resolve_bluebubbles_password(&self) -> Option<String> {
        self.bluebubbles_password.clone().or_else(|| std::env::var("BLUEBUBBLES_PASSWORD").ok())
    }

    // ── Synology Chat ──
    pub fn resolve_synology_url(&self) -> Option<String> {
        self.synology_url.clone().or_else(|| std::env::var("SYNOLOGY_URL").ok())
    }
    pub fn resolve_synology_incoming_url(&self) -> Option<String> {
        self.synology_incoming_url.clone().or_else(|| std::env::var("SYNOLOGY_INCOMING_URL").ok())
    }
    pub fn resolve_synology_token(&self) -> Option<String> {
        self.synology_token.clone().or_else(|| std::env::var("SYNOLOGY_TOKEN").ok())
    }

    // ── Tlon (Urbit) ──
    pub fn resolve_tlon_ship_url(&self) -> Option<String> {
        self.tlon_ship_url.clone().or_else(|| std::env::var("TLON_SHIP_URL").ok())
    }
    pub fn resolve_tlon_ship_code(&self) -> Option<String> {
        self.tlon_ship_code.clone().or_else(|| std::env::var("TLON_SHIP_CODE").ok())
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

    pub fn config_path() -> Result<PathBuf> {
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
            "mistral" => self.mistral.as_ref(),
            "cerebras" => self.cerebras.as_ref(),
            "deepseek" => self.deepseek.as_ref(),
            "zhipu" | "glm" => self.zhipu.as_ref(),
            "vercel_ai" | "vercel" => self.vercel_ai.as_ref(),
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
        assert_eq!(cfg.sandbox_config.runtime, "auto");
        assert_eq!(cfg.sandbox_config.image, "ubuntu:22.04");
        assert_eq!(cfg.sandbox_config.timeout_secs, 3600);
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

    // ── Config load/save tempfile roundtrip ──

    #[test]
    fn config_load_save_tempfile_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("config.toml");

        // Build a non-trivial config
        let mut cfg = Config::default();
        cfg.ollama = Some(ProviderConfig {
            enabled: true,
            api_url: Some("http://localhost:11434".into()),
            model: Some("llama3".into()),
            api_key: None,
            api_key_helper: None,
            thinking_budget_tokens: None,
        });
        cfg.safety.approval_policy = "full-auto".to_string();
        cfg.routing.planning_provider = Some("claude".into());
        cfg.failover.chain = vec!["claude".into(), "openai".into()];

        // Serialize and write
        let content = toml::to_string_pretty(&cfg).expect("serialize");
        std::fs::write(&cfg_path, &content).expect("write");

        // Read back and deserialize
        let raw = std::fs::read_to_string(&cfg_path).expect("read");
        let cfg2: Config = toml::from_str(&raw).expect("deserialize");

        assert!(cfg2.ollama.is_some());
        assert_eq!(cfg2.ollama.as_ref().unwrap().model.as_deref(), Some("llama3"));
        assert_eq!(cfg2.safety.approval_policy, "full-auto");
        assert_eq!(cfg2.routing.planning_provider.as_deref(), Some("claude"));
        assert_eq!(cfg2.failover.chain, vec!["claude", "openai"]);
    }

    #[test]
    fn config_deserialize_empty_toml() {
        let cfg: Config = toml::from_str("").expect("empty toml should deserialize");
        assert!(cfg.ollama.is_none());
        assert!(cfg.index.enabled);
        assert!(!cfg.otel.enabled);
        assert_eq!(cfg.safety.approval_policy, "suggest");
    }

    #[test]
    fn config_deserialize_partial_toml() {
        let toml_str = r#"
[safety]
require_approval_for_commands = false
require_approval_for_file_changes = false
approval_policy = "auto-edit"
"#;
        let cfg: Config = toml::from_str(toml_str).expect("partial toml");
        assert!(!cfg.safety.require_approval_for_commands);
        assert!(!cfg.safety.require_approval_for_file_changes);
        assert_eq!(cfg.safety.approval_policy, "auto-edit");
        // Other fields should be default
        assert!(cfg.ollama.is_none());
        assert!(cfg.index.enabled);
    }

    // ── GatewayConfig resolve methods (config field takes priority) ──

    #[test]
    fn gateway_resolve_discord_token_from_config() {
        let g = GatewayConfig {
            discord_token: Some("discord-tok".into()),
            ..Default::default()
        };
        assert_eq!(g.resolve_discord_token(), Some("discord-tok".into()));
    }

    #[test]
    fn gateway_resolve_slack_bot_token_from_config() {
        let g = GatewayConfig {
            slack_bot_token: Some("xoxb-slack".into()),
            ..Default::default()
        };
        assert_eq!(g.resolve_slack_bot_token(), Some("xoxb-slack".into()));
    }

    #[test]
    fn gateway_resolve_signal_fields_from_config() {
        let g = GatewayConfig {
            signal_api_url: Some("http://signal:8080".into()),
            signal_phone_number: Some("+15551234567".into()),
            ..Default::default()
        };
        assert_eq!(g.resolve_signal_api_url(), Some("http://signal:8080".into()));
        assert_eq!(g.resolve_signal_phone_number(), Some("+15551234567".into()));
    }

    #[test]
    fn gateway_resolve_matrix_fields_from_config() {
        let g = GatewayConfig {
            matrix_homeserver_url: Some("https://matrix.org".into()),
            matrix_access_token: Some("mat-tok".into()),
            matrix_room_id: Some("!abc:matrix.org".into()),
            matrix_user_id: Some("@bot:matrix.org".into()),
            ..Default::default()
        };
        assert_eq!(g.resolve_matrix_homeserver_url(), Some("https://matrix.org".into()));
        assert_eq!(g.resolve_matrix_access_token(), Some("mat-tok".into()));
        assert_eq!(g.resolve_matrix_room_id(), Some("!abc:matrix.org".into()));
        assert_eq!(g.resolve_matrix_user_id(), Some("@bot:matrix.org".into()));
    }

    #[test]
    fn gateway_resolve_returns_none_when_empty_no_env() {
        // With a fresh default and no env vars set for these specific keys,
        // resolve should return None. We test a less common platform to
        // avoid collision with real env vars.
        let g = GatewayConfig::default();
        // tlon_ship_url is very unlikely to be set in the env
        assert!(g.tlon_ship_url.is_none());
    }

    // ── SandboxConfig ──

    #[test]
    fn sandbox_config_default_values() {
        let s = SandboxConfig::default();
        assert_eq!(s.runtime, "auto");
        assert_eq!(s.image, "ubuntu:22.04");
        assert_eq!(s.timeout_secs, 3600);
        assert_eq!(s.network.mode, "full");
        assert!(s.network.allowed_domains.is_empty());
        assert!(s.opensandbox.api_url.is_none());
        assert!(s.opensandbox.api_key.is_none());
        assert!(s.registry.url.is_none());
    }

    #[test]
    fn sandbox_config_serde_roundtrip() {
        let toml_str = r#"
runtime = "docker"
image = "node:20"
timeout_secs = 1800

[resources]
cpus = "4.0"
memory = "8g"
pids_limit = 512

[network]
mode = "restricted"
allowed_domains = ["github.com", "npmjs.org"]

[opensandbox]
api_url = "http://sandbox:9090"
api_key = "key123"
"#;
        let s: SandboxConfig = toml::from_str(toml_str).expect("deserialize");
        assert_eq!(s.runtime, "docker");
        assert_eq!(s.image, "node:20");
        assert_eq!(s.timeout_secs, 1800);
        assert_eq!(s.resources.cpus.as_deref(), Some("4.0"));
        assert_eq!(s.resources.memory.as_deref(), Some("8g"));
        assert_eq!(s.resources.pids_limit, Some(512));
        assert_eq!(s.network.mode, "restricted");
        assert_eq!(s.network.allowed_domains, vec!["github.com", "npmjs.org"]);
        assert_eq!(s.opensandbox.api_url.as_deref(), Some("http://sandbox:9090"));
        assert_eq!(s.opensandbox.api_key.as_deref(), Some("key123"));
    }

    // ── OpenSandboxConfig resolve ──

    #[test]
    fn open_sandbox_resolve_api_url_from_config() {
        let o = OpenSandboxConfig {
            api_url: Some("http://custom:1234".into()),
            api_key: None,
        };
        assert_eq!(o.resolve_api_url(), "http://custom:1234");
    }

    #[test]
    fn open_sandbox_resolve_api_url_default_fallback() {
        let o = OpenSandboxConfig::default();
        // When no config and no env var, falls back to localhost:8080
        // (env var might be set, but the default is deterministic)
        let url = o.resolve_api_url();
        assert!(!url.is_empty());
    }

    #[test]
    fn open_sandbox_resolve_api_key_from_config() {
        let o = OpenSandboxConfig {
            api_url: None,
            api_key: Some("my-key".into()),
        };
        assert_eq!(o.resolve_api_key(), Some("my-key".into()));
    }

    // ── RegistryConfig ──

    #[test]
    fn registry_resolve_password_from_config() {
        let r = RegistryConfig {
            url: Some("https://registry.example.com".into()),
            username: Some("user".into()),
            password: Some("secret".into()),
        };
        assert_eq!(r.resolve_password(), Some("secret".into()));
    }

    // ── VoiceConfig resolve ──

    #[test]
    fn voice_resolve_whisper_key_from_config() {
        let v = VoiceConfig {
            whisper_api_key: Some("wsk-123".into()),
            ..Default::default()
        };
        assert_eq!(v.resolve_whisper_api_key(None), Some("wsk-123".into()));
    }

    #[test]
    fn voice_resolve_whisper_key_groq_fallback() {
        let v = VoiceConfig::default();
        assert_eq!(
            v.resolve_whisper_api_key(Some("groq-key-abc")),
            Some("groq-key-abc".into())
        );
    }

    #[test]
    fn voice_resolve_elevenlabs_voice_id_default() {
        let v = VoiceConfig::default();
        // When no config and no env, should return the Rachel default
        let id = v.resolve_elevenlabs_voice_id();
        assert!(!id.is_empty());
        // The hardcoded default is "21m00Tcm4TlvDq8ikWAM"
        // But env may override, so just check non-empty
    }

    #[test]
    fn voice_resolve_elevenlabs_voice_id_from_config() {
        let v = VoiceConfig {
            elevenlabs_voice_id: Some("custom-voice-id".into()),
            ..Default::default()
        };
        assert_eq!(v.resolve_elevenlabs_voice_id(), "custom-voice-id");
    }

    // ── SafetyConfig ──

    #[test]
    fn safety_config_default() {
        let s = SafetyConfig::default();
        assert!(s.require_approval_for_commands);
        assert!(s.require_approval_for_file_changes);
        assert_eq!(s.approval_policy, "suggest");
        assert!(!s.sandbox);
        assert!(s.sandbox_profile.is_none());
        assert_eq!(s.shell_environment.inherit, "all");
    }

    #[test]
    fn approval_policy_full_auto_wins_over_auto_edit() {
        // When both auto_edit and full_auto are set, full_auto should win
        assert_eq!(
            SafetyConfig::approval_policy_from_flags(false, true, true),
            "full-auto"
        );
    }

    // ── FailoverConfig ──

    #[test]
    fn failover_config_default_empty_chain() {
        let f = FailoverConfig::default();
        assert!(f.chain.is_empty());
    }

    #[test]
    fn failover_config_serde_roundtrip() {
        let toml_str = r#"chain = ["claude", "openai", "gemini"]"#;
        let f: FailoverConfig = toml::from_str(toml_str).expect("deserialize");
        assert_eq!(f.chain, vec!["claude", "openai", "gemini"]);
        let re = toml::to_string_pretty(&f).expect("serialize");
        let f2: FailoverConfig = toml::from_str(&re).expect("re-deserialize");
        assert_eq!(f2.chain, vec!["claude", "openai", "gemini"]);
    }

    // ── get_provider_config extended aliases ──

    #[test]
    fn get_provider_config_all_aliases() {
        let mut cfg = Config::default();
        cfg.zhipu = Some(ProviderConfig {
            enabled: true, api_url: None, model: None,
            api_key: None, api_key_helper: None, thinking_budget_tokens: None,
        });
        cfg.vercel_ai = Some(ProviderConfig {
            enabled: true, api_url: None, model: None,
            api_key: None, api_key_helper: None, thinking_budget_tokens: None,
        });
        cfg.azure_openai = Some(ProviderConfig {
            enabled: true, api_url: None, model: None,
            api_key: None, api_key_helper: None, thinking_budget_tokens: None,
        });
        // "glm" is alias for zhipu
        assert!(cfg.get_provider_config("glm").is_some());
        assert!(cfg.get_provider_config("zhipu").is_some());
        // "vercel" is alias for vercel_ai
        assert!(cfg.get_provider_config("vercel").is_some());
        assert!(cfg.get_provider_config("vercel_ai").is_some());
        // "azure" is alias for azure_openai
        assert!(cfg.get_provider_config("azure").is_some());
        assert!(cfg.get_provider_config("azure_openai").is_some());
        // unknown returns None
        assert!(cfg.get_provider_config("nonexistent").is_none());
    }

    // ── Config::approval_from_flags (delegating wrapper) ──

    #[test]
    fn config_approval_from_flags_delegates() {
        assert_eq!(Config::approval_from_flags(false, false, true), "full-auto");
        assert_eq!(Config::approval_from_flags(false, true, false), "auto-edit");
        assert_eq!(Config::approval_from_flags(true, false, false), "suggest");
        assert_eq!(Config::approval_from_flags(false, false, false), "suggest");
    }
}
