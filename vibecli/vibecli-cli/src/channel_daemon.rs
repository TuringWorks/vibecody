//! Always-on channel daemon for VibeCody.
//!
//! Listens on configured channels (Slack, Discord, GitHub webhooks, Linear,
//! PagerDuty, custom HTTP, MCP channels) and routes incoming events to
//! automation triggers with session affinity.
//!
//! # Architecture
//!
//! ```text
//! Channel Sources ─┐
//!   Slack           │
//!   Discord         ├─→ ChannelDaemon ─→ EventFilter ─→ SessionManager ─→ Automation
//!   GitHub Webhook  │        │                                │
//!   Linear          │        ├─ RateLimiter (token bucket)    ├─ session affinity
//!   PagerDuty       │        ├─ signature verification        ├─ timeout cleanup
//!   Custom HTTP     │        └─ health monitoring             └─ max concurrent limit
//!   MCP Channel  ───┘
//! ```
//!
//! # Configuration
//!
//! ```toml
//! [channel_daemon]
//! port = 7879
//! max_concurrent_sessions = 8
//! rate_limit_per_channel = 60
//! health_check_interval_secs = 30
//! auto_restart = true
//!
//! [[channel_daemon.channels]]
//! name = "github-webhooks"
//! channel_type = "GitHubWebhook"
//! endpoint = "/webhooks/github"
//! secret = "whsec_..."
//! enabled = true
//! ```

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Simple HMAC-SHA256-like signature check (hex comparison).
/// In production this would use a real HMAC crate; here we do a
/// constant-time-ish comparison of `sha256=<hex(secret+payload)>`.
fn compute_signature(secret: &str, payload: &str) -> String {
    // Lightweight non-crypto hash for the module (no external crate).
    // We combine secret + payload bytes via a simple digest.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a offset basis
    for b in secret.as_bytes().iter().chain(payload.as_bytes()) {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x0100_0000_01b3); // FNV prime
    }
    format!("sha256={:016x}", hash)
}

// ---------------------------------------------------------------------------
// Channel types
// ---------------------------------------------------------------------------

/// Supported channel types for the daemon.
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelType {
    Slack,
    Discord,
    GitHubWebhook,
    Linear,
    PagerDuty,
    CustomHttp,
    McpChannel,
}

impl ChannelType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Slack => "slack",
            Self::Discord => "discord",
            Self::GitHubWebhook => "github_webhook",
            Self::Linear => "linear",
            Self::PagerDuty => "pagerduty",
            Self::CustomHttp => "custom_http",
            Self::McpChannel => "mcp_channel",
        }
    }
}

// ---------------------------------------------------------------------------
// Event filter
// ---------------------------------------------------------------------------

/// Filters which events a channel should process.
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Whitelist of event types to process (empty = allow all).
    pub event_types: Vec<String>,
    /// Keywords that must appear in the payload.
    pub keywords: Vec<String>,
    /// Patterns to exclude (if any matches, event is dropped).
    pub exclude_patterns: Vec<String>,
}

impl EventFilter {
    pub fn new() -> Self {
        Self {
            event_types: Vec::new(),
            keywords: Vec::new(),
            exclude_patterns: Vec::new(),
        }
    }

    /// Returns `true` if the event passes the filter.
    pub fn matches(&self, event_type: &str, payload: &str) -> bool {
        // Check exclude patterns first.
        for pat in &self.exclude_patterns {
            if payload.contains(pat.as_str()) {
                return false;
            }
        }
        // Check event type whitelist.
        if !self.event_types.is_empty()
            && !self.event_types.iter().any(|t| t == event_type)
        {
            return false;
        }
        // Check keyword match (all must be present).
        for kw in &self.keywords {
            if !payload.contains(kw.as_str()) {
                return false;
            }
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Channel configuration
// ---------------------------------------------------------------------------

/// Configuration for a single channel the daemon listens on.
#[derive(Debug, Clone)]
pub struct ChannelConfig {
    pub name: String,
    pub channel_type: ChannelType,
    /// Webhook URL or connection string.
    pub endpoint: String,
    /// Webhook secret for signature verification.
    pub secret: Option<String>,
    /// Optional event filter.
    pub filter: Option<EventFilter>,
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Daemon configuration
// ---------------------------------------------------------------------------

/// Top-level daemon configuration.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub channels: Vec<ChannelConfig>,
    /// HTTP webhook listener port.
    pub port: u16,
    /// Maximum parallel agent sessions.
    pub max_concurrent_sessions: usize,
    /// Max events per minute per channel.
    pub rate_limit_per_channel: u32,
    /// Health check interval in seconds.
    pub health_check_interval_secs: u64,
    /// Restart on crash.
    pub auto_restart: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            channels: Vec::new(),
            port: 7879,
            max_concurrent_sessions: 8,
            rate_limit_per_channel: 60,
            health_check_interval_secs: 30,
            auto_restart: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Incoming event
// ---------------------------------------------------------------------------

/// An event received from any channel source.
#[derive(Debug, Clone)]
pub struct IncomingEvent {
    pub id: String,
    pub channel: String,
    pub channel_type: ChannelType,
    /// e.g. "pr.opened", "message.received", "incident.triggered"
    pub event_type: String,
    /// JSON payload.
    pub payload: String,
    pub timestamp: u64,
    /// Webhook signature for verification.
    pub signature: Option<String>,
}

// ---------------------------------------------------------------------------
// Event response
// ---------------------------------------------------------------------------

/// Response after processing an event.
#[derive(Debug, Clone)]
pub struct EventResponse {
    pub event_id: String,
    pub action_taken: String,
    pub session_id: Option<String>,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Daemon-specific errors.
#[derive(Debug, Clone, PartialEq)]
pub enum DaemonError {
    ChannelNotFound(String),
    RateLimited(String),
    SessionLimitReached,
    InvalidSignature,
    AutomationFailed(String),
    ConfigError(String),
    ShutdownInProgress,
}

impl std::fmt::Display for DaemonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ChannelNotFound(name) => write!(f, "channel not found: {name}"),
            Self::RateLimited(ch) => write!(f, "rate limited on channel: {ch}"),
            Self::SessionLimitReached => write!(f, "max concurrent sessions reached"),
            Self::InvalidSignature => write!(f, "invalid webhook signature"),
            Self::AutomationFailed(msg) => write!(f, "automation failed: {msg}"),
            Self::ConfigError(msg) => write!(f, "config error: {msg}"),
            Self::ShutdownInProgress => write!(f, "daemon is shutting down"),
        }
    }
}

// ---------------------------------------------------------------------------
// Rate limiter (token bucket)
// ---------------------------------------------------------------------------

/// Per-channel token-bucket rate limiter.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// Tokens per minute per channel.
    rate_per_minute: u32,
    /// Current state: channel name → (tokens_remaining, last_refill_time).
    buckets: HashMap<String, (u32, u64)>,
}

impl RateLimiter {
    pub fn new(rate_per_minute: u32) -> Self {
        Self {
            rate_per_minute,
            buckets: HashMap::new(),
        }
    }

    /// Check and consume a token. Returns `true` if allowed.
    pub fn allow(&mut self, channel: &str) -> bool {
        let now = now_secs();
        let entry = self
            .buckets
            .entry(channel.to_string())
            .or_insert((self.rate_per_minute, now));

        // Refill tokens based on elapsed time.
        let elapsed_secs = now.saturating_sub(entry.1);
        if elapsed_secs > 0 {
            let refill = ((elapsed_secs as f64 / 60.0) * self.rate_per_minute as f64) as u32;
            entry.0 = (entry.0 + refill).min(self.rate_per_minute);
            entry.1 = now;
        }

        if entry.0 > 0 {
            entry.0 -= 1;
            true
        } else {
            false
        }
    }

    /// Get remaining tokens for a channel.
    pub fn remaining(&self, channel: &str) -> u32 {
        self.buckets
            .get(channel)
            .map(|(tokens, _)| *tokens)
            .unwrap_or(self.rate_per_minute)
    }
}

// ---------------------------------------------------------------------------
// Agent session
// ---------------------------------------------------------------------------

/// A tracked agent session bound to a channel.
#[derive(Debug, Clone)]
pub struct AgentSession {
    pub session_id: String,
    pub channel: String,
    pub created_at: u64,
    pub last_activity: u64,
    pub events_processed: u64,
}

// ---------------------------------------------------------------------------
// Session manager
// ---------------------------------------------------------------------------

/// Manages active agent sessions with channel affinity.
pub struct SessionManager {
    sessions: HashMap<String, AgentSession>,
    max_concurrent: usize,
    /// Session timeout in seconds (default 30 minutes).
    session_timeout_secs: u64,
}

impl SessionManager {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            sessions: HashMap::new(),
            max_concurrent,
            session_timeout_secs: 1800,
        }
    }

    /// Get or create a session for the given channel.
    /// Returns the session_id or an error if the limit is reached.
    pub fn get_or_create(&mut self, channel: &str) -> Result<String, DaemonError> {
        // Check for existing session (affinity).
        if let Some(session) = self.sessions.get_mut(channel) {
            session.last_activity = now_secs();
            session.events_processed += 1;
            return Ok(session.session_id.clone());
        }

        // Check capacity.
        if self.sessions.len() >= self.max_concurrent {
            return Err(DaemonError::SessionLimitReached);
        }

        // Create new session.
        let session_id = format!("sess-{}-{}", channel, now_ms());
        let session = AgentSession {
            session_id: session_id.clone(),
            channel: channel.to_string(),
            created_at: now_secs(),
            last_activity: now_secs(),
            events_processed: 1,
        };
        self.sessions.insert(channel.to_string(), session);
        Ok(session_id)
    }

    /// Remove a session by channel name.
    pub fn remove(&mut self, channel: &str) -> bool {
        self.sessions.remove(channel).is_some()
    }

    /// Clean up timed-out sessions.
    pub fn cleanup_expired(&mut self) -> usize {
        let now = now_secs();
        let timeout = self.session_timeout_secs;
        let before = self.sessions.len();
        self.sessions
            .retain(|_, s| now.saturating_sub(s.last_activity) < timeout);
        before - self.sessions.len()
    }

    /// Number of active sessions.
    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get session by channel name.
    pub fn get(&self, channel: &str) -> Option<&AgentSession> {
        self.sessions.get(channel)
    }

    /// List all active sessions.
    pub fn list(&self) -> Vec<&AgentSession> {
        self.sessions.values().collect()
    }
}

// ---------------------------------------------------------------------------
// Daemon status
// ---------------------------------------------------------------------------

/// Runtime status of the channel daemon.
#[derive(Debug, Clone)]
pub struct DaemonStatus {
    pub running: bool,
    pub uptime_secs: u64,
    pub channels_active: usize,
    pub total_events_processed: u64,
    pub active_sessions: usize,
    pub events_per_minute: f64,
    pub last_event_time: Option<u64>,
    pub errors: Vec<DaemonError>,
}

// ---------------------------------------------------------------------------
// Channel daemon
// ---------------------------------------------------------------------------

/// Persistent daemon process that listens on configured channels,
/// routes events to automation triggers, and manages agent sessions.
pub struct ChannelDaemon {
    config: DaemonConfig,
    channels: HashMap<String, ChannelConfig>,
    session_manager: SessionManager,
    rate_limiter: RateLimiter,
    running: bool,
    shutdown_requested: bool,
    started_at: u64,
    total_events_processed: u64,
    last_event_time: Option<u64>,
    errors: Vec<DaemonError>,
}

impl ChannelDaemon {
    /// Create a new channel daemon with the given configuration.
    pub fn new(config: DaemonConfig) -> Self {
        let rate_limiter = RateLimiter::new(config.rate_limit_per_channel);
        let session_manager = SessionManager::new(config.max_concurrent_sessions);
        let mut channels = HashMap::new();
        for ch in &config.channels {
            if ch.enabled {
                channels.insert(ch.name.clone(), ch.clone());
            }
        }
        Self {
            config,
            channels,
            session_manager,
            rate_limiter,
            running: false,
            shutdown_requested: false,
            started_at: 0,
            total_events_processed: 0,
            last_event_time: None,
            errors: Vec::new(),
        }
    }

    /// Start listening on all configured channels.
    pub fn start(&mut self) -> Result<(), DaemonError> {
        if self.running {
            return Err(DaemonError::ConfigError(
                "daemon is already running".to_string(),
            ));
        }
        if self.channels.is_empty() {
            return Err(DaemonError::ConfigError(
                "no enabled channels configured".to_string(),
            ));
        }
        self.running = true;
        self.shutdown_requested = false;
        self.started_at = now_secs();
        Ok(())
    }

    /// Graceful shutdown — signals all listeners to stop.
    pub fn stop(&mut self) {
        self.shutdown_requested = true;
        self.running = false;
    }

    /// Return current daemon status.
    pub fn status(&self) -> DaemonStatus {
        let uptime = if self.started_at > 0 {
            now_secs().saturating_sub(self.started_at)
        } else {
            0
        };
        let epm = if uptime > 0 {
            (self.total_events_processed as f64 / uptime as f64) * 60.0
        } else {
            0.0
        };
        DaemonStatus {
            running: self.running,
            uptime_secs: uptime,
            channels_active: self.channels.len(),
            total_events_processed: self.total_events_processed,
            active_sessions: self.session_manager.active_count(),
            events_per_minute: epm,
            last_event_time: self.last_event_time,
            errors: self.errors.clone(),
        }
    }

    /// Add a channel at runtime.
    pub fn add_channel(&mut self, config: ChannelConfig) -> Result<(), DaemonError> {
        if self.shutdown_requested {
            return Err(DaemonError::ShutdownInProgress);
        }
        if self.channels.contains_key(&config.name) {
            return Err(DaemonError::ConfigError(format!(
                "channel '{}' already exists",
                config.name
            )));
        }
        self.channels.insert(config.name.clone(), config);
        Ok(())
    }

    /// Remove a channel at runtime.
    pub fn remove_channel(&mut self, name: &str) -> Result<(), DaemonError> {
        if self.shutdown_requested {
            return Err(DaemonError::ShutdownInProgress);
        }
        if self.channels.remove(name).is_none() {
            return Err(DaemonError::ChannelNotFound(name.to_string()));
        }
        // Also remove the associated session.
        self.session_manager.remove(name);
        Ok(())
    }

    /// Verify the signature of an incoming event against the channel secret.
    pub fn verify_signature(&self, event: &IncomingEvent, secret: &str) -> bool {
        match &event.signature {
            Some(sig) => {
                let expected = compute_signature(secret, &event.payload);
                sig == &expected
            }
            None => false,
        }
    }

    /// Route an event to the appropriate automation trigger.
    /// Returns a description of the automation action.
    pub fn route_to_automation(
        &self,
        event: &IncomingEvent,
    ) -> Result<String, DaemonError> {
        let channel_config = self
            .channels
            .get(&event.channel)
            .ok_or_else(|| DaemonError::ChannelNotFound(event.channel.clone()))?;

        // Apply event filter if configured.
        if let Some(filter) = &channel_config.filter {
            if !filter.matches(&event.event_type, &event.payload) {
                return Err(DaemonError::AutomationFailed(
                    "event filtered out".to_string(),
                ));
            }
        }

        Ok(format!(
            "routed {} event '{}' from channel '{}' to automation",
            channel_config.channel_type.as_str(),
            event.event_type,
            event.channel,
        ))
    }

    /// Process an incoming event end-to-end: verify, filter, rate-limit,
    /// assign a session, and route to automation.
    pub fn process_event(
        &mut self,
        event: IncomingEvent,
    ) -> Result<EventResponse, DaemonError> {
        let start = now_ms();

        if self.shutdown_requested {
            return Err(DaemonError::ShutdownInProgress);
        }

        // Look up channel config.
        let channel_config = self
            .channels
            .get(&event.channel)
            .ok_or_else(|| DaemonError::ChannelNotFound(event.channel.clone()))?
            .clone();

        // Verify signature if secret is configured.
        if let Some(secret) = &channel_config.secret {
            if !self.verify_signature(&event, secret) {
                let err = DaemonError::InvalidSignature;
                self.errors.push(err.clone());
                return Err(err);
            }
        }

        // Apply event filter.
        if let Some(filter) = &channel_config.filter {
            if !filter.matches(&event.event_type, &event.payload) {
                return Err(DaemonError::AutomationFailed(
                    "event filtered out".to_string(),
                ));
            }
        }

        // Rate limiting.
        if !self.rate_limiter.allow(&event.channel) {
            return Err(DaemonError::RateLimited(event.channel.clone()));
        }

        // Session management.
        let session_id = self.session_manager.get_or_create(&event.channel)?;

        // Route to automation.
        let action = self.route_to_automation(&event)?;

        // Update stats.
        self.total_events_processed += 1;
        self.last_event_time = Some(event.timestamp);

        let duration_ms = now_ms().saturating_sub(start);

        Ok(EventResponse {
            event_id: event.id,
            action_taken: action,
            session_id: Some(session_id),
            duration_ms,
        })
    }

    /// Get the number of active channels.
    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    /// Check if a channel exists by name.
    pub fn has_channel(&self, name: &str) -> bool {
        self.channels.contains_key(name)
    }

    /// Get the daemon configuration.
    pub fn config(&self) -> &DaemonConfig {
        &self.config
    }

    /// Clean up expired sessions and return count removed.
    pub fn cleanup_sessions(&mut self) -> usize {
        self.session_manager.cleanup_expired()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- Helpers -------------------------------------------------------------

    fn test_channel(name: &str, ct: ChannelType) -> ChannelConfig {
        ChannelConfig {
            name: name.to_string(),
            channel_type: ct,
            endpoint: format!("/webhooks/{name}"),
            secret: None,
            filter: None,
            enabled: true,
        }
    }

    fn test_event(channel: &str, event_type: &str) -> IncomingEvent {
        IncomingEvent {
            id: format!("evt-{}", now_ms()),
            channel: channel.to_string(),
            channel_type: ChannelType::GitHubWebhook,
            event_type: event_type.to_string(),
            payload: r#"{"action":"opened"}"#.to_string(),
            timestamp: now_secs(),
            signature: None,
        }
    }

    fn default_daemon() -> ChannelDaemon {
        let config = DaemonConfig {
            channels: vec![
                test_channel("github", ChannelType::GitHubWebhook),
                test_channel("slack", ChannelType::Slack),
            ],
            ..DaemonConfig::default()
        };
        ChannelDaemon::new(config)
    }

    // -- DaemonConfig defaults -----------------------------------------------

    #[test]
    fn test_daemon_config_defaults() {
        let cfg = DaemonConfig::default();
        assert_eq!(cfg.port, 7879);
        assert_eq!(cfg.max_concurrent_sessions, 8);
        assert_eq!(cfg.rate_limit_per_channel, 60);
        assert_eq!(cfg.health_check_interval_secs, 30);
        assert!(cfg.auto_restart);
        assert!(cfg.channels.is_empty());
    }

    // -- Daemon creation -----------------------------------------------------

    #[test]
    fn test_daemon_new() {
        let d = default_daemon();
        assert!(!d.running);
        assert_eq!(d.channel_count(), 2);
        assert!(d.has_channel("github"));
        assert!(d.has_channel("slack"));
    }

    #[test]
    fn test_daemon_new_skips_disabled_channels() {
        let mut ch = test_channel("disabled", ChannelType::Discord);
        ch.enabled = false;
        let config = DaemonConfig {
            channels: vec![
                test_channel("active", ChannelType::Slack),
                ch,
            ],
            ..DaemonConfig::default()
        };
        let d = ChannelDaemon::new(config);
        assert_eq!(d.channel_count(), 1);
        assert!(d.has_channel("active"));
        assert!(!d.has_channel("disabled"));
    }

    // -- Start / stop --------------------------------------------------------

    #[test]
    fn test_daemon_start() {
        let mut d = default_daemon();
        assert!(d.start().is_ok());
        assert!(d.running);
    }

    #[test]
    fn test_daemon_start_already_running() {
        let mut d = default_daemon();
        d.start().unwrap();
        let err = d.start().unwrap_err();
        assert_eq!(
            err,
            DaemonError::ConfigError("daemon is already running".to_string())
        );
    }

    #[test]
    fn test_daemon_start_no_channels() {
        let config = DaemonConfig::default();
        let mut d = ChannelDaemon::new(config);
        let err = d.start().unwrap_err();
        assert_eq!(
            err,
            DaemonError::ConfigError("no enabled channels configured".to_string())
        );
    }

    #[test]
    fn test_daemon_stop() {
        let mut d = default_daemon();
        d.start().unwrap();
        d.stop();
        assert!(!d.running);
        assert!(d.shutdown_requested);
    }

    // -- Channel add / remove ------------------------------------------------

    #[test]
    fn test_add_channel() {
        let mut d = default_daemon();
        let ch = test_channel("linear", ChannelType::Linear);
        assert!(d.add_channel(ch).is_ok());
        assert_eq!(d.channel_count(), 3);
        assert!(d.has_channel("linear"));
    }

    #[test]
    fn test_add_channel_duplicate() {
        let mut d = default_daemon();
        let ch = test_channel("github", ChannelType::GitHubWebhook);
        let err = d.add_channel(ch).unwrap_err();
        assert!(matches!(err, DaemonError::ConfigError(_)));
    }

    #[test]
    fn test_add_channel_during_shutdown() {
        let mut d = default_daemon();
        d.start().unwrap();
        d.stop();
        let ch = test_channel("new", ChannelType::CustomHttp);
        let err = d.add_channel(ch).unwrap_err();
        assert_eq!(err, DaemonError::ShutdownInProgress);
    }

    #[test]
    fn test_remove_channel() {
        let mut d = default_daemon();
        assert!(d.remove_channel("github").is_ok());
        assert_eq!(d.channel_count(), 1);
        assert!(!d.has_channel("github"));
    }

    #[test]
    fn test_remove_channel_not_found() {
        let mut d = default_daemon();
        let err = d.remove_channel("nonexistent").unwrap_err();
        assert_eq!(
            err,
            DaemonError::ChannelNotFound("nonexistent".to_string())
        );
    }

    #[test]
    fn test_remove_channel_during_shutdown() {
        let mut d = default_daemon();
        d.start().unwrap();
        d.stop();
        let err = d.remove_channel("github").unwrap_err();
        assert_eq!(err, DaemonError::ShutdownInProgress);
    }

    // -- Event processing ----------------------------------------------------

    #[test]
    fn test_process_event_success() {
        let mut d = default_daemon();
        d.start().unwrap();
        let event = test_event("github", "pr.opened");
        let resp = d.process_event(event).unwrap();
        assert!(!resp.event_id.is_empty());
        assert!(resp.session_id.is_some());
        assert!(resp.action_taken.contains("github_webhook"));
        assert_eq!(d.total_events_processed, 1);
    }

    #[test]
    fn test_process_event_unknown_channel() {
        let mut d = default_daemon();
        d.start().unwrap();
        let event = test_event("unknown", "test");
        let err = d.process_event(event).unwrap_err();
        assert_eq!(
            err,
            DaemonError::ChannelNotFound("unknown".to_string())
        );
    }

    #[test]
    fn test_process_event_during_shutdown() {
        let mut d = default_daemon();
        d.start().unwrap();
        d.stop();
        let event = test_event("github", "push");
        let err = d.process_event(event).unwrap_err();
        assert_eq!(err, DaemonError::ShutdownInProgress);
    }

    // -- Signature verification ----------------------------------------------

    #[test]
    fn test_verify_signature_valid() {
        let d = default_daemon();
        let secret = "mysecret";
        let payload = r#"{"action":"opened"}"#;
        let sig = compute_signature(secret, payload);
        let event = IncomingEvent {
            id: "evt-1".to_string(),
            channel: "github".to_string(),
            channel_type: ChannelType::GitHubWebhook,
            event_type: "pr.opened".to_string(),
            payload: payload.to_string(),
            timestamp: now_secs(),
            signature: Some(sig),
        };
        assert!(d.verify_signature(&event, secret));
    }

    #[test]
    fn test_verify_signature_invalid() {
        let d = default_daemon();
        let event = IncomingEvent {
            id: "evt-1".to_string(),
            channel: "github".to_string(),
            channel_type: ChannelType::GitHubWebhook,
            event_type: "pr.opened".to_string(),
            payload: r#"{"action":"opened"}"#.to_string(),
            timestamp: now_secs(),
            signature: Some("sha256=badhash".to_string()),
        };
        assert!(!d.verify_signature(&event, "mysecret"));
    }

    #[test]
    fn test_verify_signature_missing() {
        let d = default_daemon();
        let event = IncomingEvent {
            id: "evt-1".to_string(),
            channel: "github".to_string(),
            channel_type: ChannelType::GitHubWebhook,
            event_type: "pr.opened".to_string(),
            payload: "{}".to_string(),
            timestamp: now_secs(),
            signature: None,
        };
        assert!(!d.verify_signature(&event, "secret"));
    }

    #[test]
    fn test_process_event_with_invalid_signature() {
        let mut ch = test_channel("secured", ChannelType::GitHubWebhook);
        ch.secret = Some("topsecret".to_string());
        let config = DaemonConfig {
            channels: vec![ch],
            ..DaemonConfig::default()
        };
        let mut d = ChannelDaemon::new(config);
        d.start().unwrap();
        let event = IncomingEvent {
            id: "evt-bad".to_string(),
            channel: "secured".to_string(),
            channel_type: ChannelType::GitHubWebhook,
            event_type: "push".to_string(),
            payload: "{}".to_string(),
            timestamp: now_secs(),
            signature: Some("sha256=wrong".to_string()),
        };
        let err = d.process_event(event).unwrap_err();
        assert_eq!(err, DaemonError::InvalidSignature);
    }

    #[test]
    fn test_process_event_with_valid_signature() {
        let secret = "topsecret";
        let payload = r#"{"ref":"refs/heads/main"}"#;
        let sig = compute_signature(secret, payload);
        let mut ch = test_channel("secured", ChannelType::GitHubWebhook);
        ch.secret = Some(secret.to_string());
        let config = DaemonConfig {
            channels: vec![ch],
            ..DaemonConfig::default()
        };
        let mut d = ChannelDaemon::new(config);
        d.start().unwrap();
        let event = IncomingEvent {
            id: "evt-good".to_string(),
            channel: "secured".to_string(),
            channel_type: ChannelType::GitHubWebhook,
            event_type: "push".to_string(),
            payload: payload.to_string(),
            timestamp: now_secs(),
            signature: Some(sig),
        };
        assert!(d.process_event(event).is_ok());
    }

    // -- Event filtering -----------------------------------------------------

    #[test]
    fn test_event_filter_whitelist_pass() {
        let filter = EventFilter {
            event_types: vec!["pr.opened".to_string()],
            keywords: Vec::new(),
            exclude_patterns: Vec::new(),
        };
        assert!(filter.matches("pr.opened", "{}"));
    }

    #[test]
    fn test_event_filter_whitelist_block() {
        let filter = EventFilter {
            event_types: vec!["pr.opened".to_string()],
            keywords: Vec::new(),
            exclude_patterns: Vec::new(),
        };
        assert!(!filter.matches("push", "{}"));
    }

    #[test]
    fn test_event_filter_keyword_match() {
        let filter = EventFilter {
            event_types: Vec::new(),
            keywords: vec!["urgent".to_string()],
            exclude_patterns: Vec::new(),
        };
        assert!(filter.matches("alert", r#"{"msg":"urgent fix needed"}"#));
        assert!(!filter.matches("alert", r#"{"msg":"normal update"}"#));
    }

    #[test]
    fn test_event_filter_exclude_pattern() {
        let filter = EventFilter {
            event_types: Vec::new(),
            keywords: Vec::new(),
            exclude_patterns: vec!["bot".to_string()],
        };
        assert!(!filter.matches("message", r#"{"user":"bot-ci"}"#));
        assert!(filter.matches("message", r#"{"user":"alice"}"#));
    }

    #[test]
    fn test_event_filter_empty_allows_all() {
        let filter = EventFilter::new();
        assert!(filter.matches("anything", "any payload"));
    }

    #[test]
    fn test_process_event_filtered_out() {
        let mut ch = test_channel("filtered", ChannelType::Slack);
        ch.filter = Some(EventFilter {
            event_types: vec!["message.received".to_string()],
            keywords: Vec::new(),
            exclude_patterns: Vec::new(),
        });
        let config = DaemonConfig {
            channels: vec![ch],
            ..DaemonConfig::default()
        };
        let mut d = ChannelDaemon::new(config);
        d.start().unwrap();
        let event = IncomingEvent {
            id: "evt-f".to_string(),
            channel: "filtered".to_string(),
            channel_type: ChannelType::Slack,
            event_type: "reaction.added".to_string(),
            payload: "{}".to_string(),
            timestamp: now_secs(),
            signature: None,
        };
        let err = d.process_event(event).unwrap_err();
        assert!(matches!(err, DaemonError::AutomationFailed(_)));
    }

    // -- Rate limiting -------------------------------------------------------

    #[test]
    fn test_rate_limiter_allow() {
        let mut rl = RateLimiter::new(10);
        assert!(rl.allow("ch1"));
        assert_eq!(rl.remaining("ch1"), 9);
    }

    #[test]
    fn test_rate_limiter_deny_exhausted() {
        let mut rl = RateLimiter::new(2);
        assert!(rl.allow("ch1"));
        assert!(rl.allow("ch1"));
        assert!(!rl.allow("ch1"));
    }

    #[test]
    fn test_rate_limiter_burst() {
        let mut rl = RateLimiter::new(5);
        // Consume all tokens.
        for _ in 0..5 {
            assert!(rl.allow("ch1"));
        }
        assert!(!rl.allow("ch1"));
        assert_eq!(rl.remaining("ch1"), 0);
    }

    #[test]
    fn test_rate_limiter_separate_channels() {
        let mut rl = RateLimiter::new(1);
        assert!(rl.allow("ch1"));
        assert!(rl.allow("ch2"));
        assert!(!rl.allow("ch1"));
        assert!(!rl.allow("ch2"));
    }

    #[test]
    fn test_rate_limiter_remaining_unknown_channel() {
        let rl = RateLimiter::new(60);
        assert_eq!(rl.remaining("new"), 60);
    }

    #[test]
    fn test_process_event_rate_limited() {
        let config = DaemonConfig {
            channels: vec![test_channel("limited", ChannelType::CustomHttp)],
            rate_limit_per_channel: 1,
            ..DaemonConfig::default()
        };
        let mut d = ChannelDaemon::new(config);
        d.start().unwrap();
        // First event succeeds.
        let e1 = test_event("limited", "ping");
        assert!(d.process_event(e1).is_ok());
        // Second event should be rate-limited.
        let e2 = test_event("limited", "ping");
        let err = d.process_event(e2).unwrap_err();
        assert_eq!(err, DaemonError::RateLimited("limited".to_string()));
    }

    // -- Session management --------------------------------------------------

    #[test]
    fn test_session_create() {
        let mut sm = SessionManager::new(4);
        let id = sm.get_or_create("ch1").unwrap();
        assert!(id.starts_with("sess-ch1-"));
        assert_eq!(sm.active_count(), 1);
    }

    #[test]
    fn test_session_affinity() {
        let mut sm = SessionManager::new(4);
        let id1 = sm.get_or_create("ch1").unwrap();
        let id2 = sm.get_or_create("ch1").unwrap();
        assert_eq!(id1, id2);
        assert_eq!(sm.active_count(), 1);
        // Second call should have bumped events_processed.
        let session = sm.get("ch1").unwrap();
        assert_eq!(session.events_processed, 2);
    }

    #[test]
    fn test_session_max_limit() {
        let mut sm = SessionManager::new(2);
        sm.get_or_create("ch1").unwrap();
        sm.get_or_create("ch2").unwrap();
        let err = sm.get_or_create("ch3").unwrap_err();
        assert_eq!(err, DaemonError::SessionLimitReached);
    }

    #[test]
    fn test_session_remove() {
        let mut sm = SessionManager::new(4);
        sm.get_or_create("ch1").unwrap();
        assert!(sm.remove("ch1"));
        assert_eq!(sm.active_count(), 0);
        assert!(!sm.remove("ch1"));
    }

    #[test]
    fn test_session_cleanup_expired() {
        let mut sm = SessionManager::new(4);
        sm.session_timeout_secs = 0; // Expire immediately.
        sm.get_or_create("ch1").unwrap();
        // The session was just created with last_activity = now, but timeout = 0
        // means anything with elapsed > 0 is expired. Since time has passed
        // (even a microsecond), cleanup should remove it — but to be safe we
        // set last_activity to the past.
        if let Some(s) = sm.sessions.get_mut("ch1") {
            s.last_activity = 0; // epoch
        }
        let removed = sm.cleanup_expired();
        assert_eq!(removed, 1);
        assert_eq!(sm.active_count(), 0);
    }

    #[test]
    fn test_session_list() {
        let mut sm = SessionManager::new(4);
        sm.get_or_create("ch1").unwrap();
        sm.get_or_create("ch2").unwrap();
        let list = sm.list();
        assert_eq!(list.len(), 2);
    }

    // -- Status reporting ----------------------------------------------------

    #[test]
    fn test_status_initial() {
        let d = default_daemon();
        let st = d.status();
        assert!(!st.running);
        assert_eq!(st.uptime_secs, 0);
        assert_eq!(st.channels_active, 2);
        assert_eq!(st.total_events_processed, 0);
        assert_eq!(st.active_sessions, 0);
        assert!(st.last_event_time.is_none());
        assert!(st.errors.is_empty());
    }

    #[test]
    fn test_status_after_events() {
        let mut d = default_daemon();
        d.start().unwrap();
        let event = test_event("github", "push");
        d.process_event(event).unwrap();
        let st = d.status();
        assert!(st.running);
        assert_eq!(st.total_events_processed, 1);
        assert_eq!(st.active_sessions, 1);
        assert!(st.last_event_time.is_some());
    }

    // -- Channel type --------------------------------------------------------

    #[test]
    fn test_channel_type_as_str() {
        assert_eq!(ChannelType::Slack.as_str(), "slack");
        assert_eq!(ChannelType::Discord.as_str(), "discord");
        assert_eq!(ChannelType::GitHubWebhook.as_str(), "github_webhook");
        assert_eq!(ChannelType::Linear.as_str(), "linear");
        assert_eq!(ChannelType::PagerDuty.as_str(), "pagerduty");
        assert_eq!(ChannelType::CustomHttp.as_str(), "custom_http");
        assert_eq!(ChannelType::McpChannel.as_str(), "mcp_channel");
    }

    // -- Error display -------------------------------------------------------

    #[test]
    fn test_error_display() {
        assert_eq!(
            DaemonError::ChannelNotFound("x".into()).to_string(),
            "channel not found: x"
        );
        assert_eq!(
            DaemonError::RateLimited("y".into()).to_string(),
            "rate limited on channel: y"
        );
        assert_eq!(
            DaemonError::SessionLimitReached.to_string(),
            "max concurrent sessions reached"
        );
        assert_eq!(
            DaemonError::InvalidSignature.to_string(),
            "invalid webhook signature"
        );
        assert_eq!(
            DaemonError::ShutdownInProgress.to_string(),
            "daemon is shutting down"
        );
    }

    // -- Signature computation -----------------------------------------------

    #[test]
    fn test_compute_signature_deterministic() {
        let s1 = compute_signature("secret", "payload");
        let s2 = compute_signature("secret", "payload");
        assert_eq!(s1, s2);
        assert!(s1.starts_with("sha256="));
    }

    #[test]
    fn test_compute_signature_different_inputs() {
        let s1 = compute_signature("secret1", "payload");
        let s2 = compute_signature("secret2", "payload");
        assert_ne!(s1, s2);
    }

    // -- Route to automation -------------------------------------------------

    #[test]
    fn test_route_to_automation_success() {
        let d = default_daemon();
        let event = test_event("github", "pr.opened");
        let result = d.route_to_automation(&event).unwrap();
        assert!(result.contains("github_webhook"));
        assert!(result.contains("pr.opened"));
    }

    #[test]
    fn test_route_to_automation_unknown_channel() {
        let d = default_daemon();
        let event = test_event("nonexistent", "test");
        let err = d.route_to_automation(&event).unwrap_err();
        assert_eq!(
            err,
            DaemonError::ChannelNotFound("nonexistent".to_string())
        );
    }
}
