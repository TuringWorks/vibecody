//! Mobile/web remote control — control CLI sessions from phone/browser via QR code.
//!
//! Code stays local, only chat flows through an encrypted bridge.

use std::fmt;
use std::time::{Duration, SystemTime};

// ─── Enums ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum RemoteSessionStatus {
    Waiting,
    Connected,
    Active,
    Disconnected,
    Expired,
}

impl fmt::Display for RemoteSessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Waiting => write!(f, "waiting"),
            Self::Connected => write!(f, "connected"),
            Self::Active => write!(f, "active"),
            Self::Disconnected => write!(f, "disconnected"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClientType {
    Mobile,
    Browser,
    Tablet,
    Desktop,
}

impl fmt::Display for ClientType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mobile => write!(f, "mobile"),
            Self::Browser => write!(f, "browser"),
            Self::Tablet => write!(f, "tablet"),
            Self::Desktop => write!(f, "desktop"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageDirection {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RemoteMessageType {
    Chat,
    Command,
    FileRequest,
    FileResponse,
    Status,
    Heartbeat,
}

impl fmt::Display for RemoteMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Chat => write!(f, "chat"),
            Self::Command => write!(f, "command"),
            Self::FileRequest => write!(f, "file_request"),
            Self::FileResponse => write!(f, "file_response"),
            Self::Status => write!(f, "status"),
            Self::Heartbeat => write!(f, "heartbeat"),
        }
    }
}

// ─── Structs ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RemoteSession {
    pub id: String,
    pub session_token: String,
    pub qr_code_data: String,
    pub bridge_url: String,
    pub encryption_key: String,
    pub status: RemoteSessionStatus,
    pub client_type: Option<ClientType>,
    pub client_ip: Option<String>,
    pub created_at: SystemTime,
    pub expires_at: SystemTime,
    pub last_heartbeat: Option<SystemTime>,
    pub messages_sent: usize,
    pub messages_received: usize,
}

#[derive(Debug, Clone)]
pub struct RemoteMessage {
    pub id: String,
    pub direction: MessageDirection,
    pub content: String,
    pub encrypted: bool,
    pub timestamp: SystemTime,
    pub message_type: RemoteMessageType,
}

#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub bridge_host: String,
    pub bridge_port: u16,
    pub use_tls: bool,
    pub session_ttl_minutes: u64,
    pub heartbeat_interval_secs: u64,
    pub max_message_size_bytes: usize,
    pub allow_file_transfer: bool,
    pub allowed_commands: Vec<String>,
}

#[derive(Debug)]
pub struct RemoteControlManager {
    pub sessions: Vec<RemoteSession>,
    pub config: BridgeConfig,
    pub active_session: Option<String>,
    next_id: u64,
    next_msg_id: u64,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Simple deterministic ID generator (no external crate needed).
fn generate_id(prefix: &str, counter: u64) -> String {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    format!("{}-{:x}-{:x}", prefix, ts, counter)
}

/// Pseudo-random hex string derived from system time and a salt.
fn pseudo_random_hex(len: usize, salt: u64) -> String {
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos()
        .wrapping_add(salt as u128);
    let mut out = String::with_capacity(len);
    let mut state = ts;
    while out.len() < len {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        out.push_str(&format!("{:016x}", state));
    }
    out.truncate(len);
    out
}

// ─── RemoteSession impl ─────────────────────────────────────────────────────

impl RemoteSession {
    /// Creates a new session with a random token, encryption key, QR data, and 8-hour expiry.
    pub fn new(bridge_url: &str) -> Self {
        let now = SystemTime::now();
        let session_token = pseudo_random_hex(32, 1);
        let encryption_key = pseudo_random_hex(64, 2); // 256-bit key in hex
        let id = generate_id("rs", 0);
        let qr_code_data = format!(
            "{}?session={}&token={}",
            bridge_url, id, session_token
        );

        Self {
            id,
            session_token,
            qr_code_data,
            bridge_url: bridge_url.to_string(),
            encryption_key,
            status: RemoteSessionStatus::Waiting,
            client_type: None,
            client_ip: None,
            created_at: now,
            expires_at: now + Duration::from_secs(8 * 60 * 60),
            last_heartbeat: None,
            messages_sent: 0,
            messages_received: 0,
        }
    }

    /// Returns the full URL suitable for encoding into a QR code.
    pub fn qr_url(&self) -> String {
        format!(
            "{}?session={}&token={}",
            self.bridge_url, self.id, self.session_token
        )
    }

    /// Whether the session has passed its expiry time.
    pub fn is_expired(&self) -> bool {
        SystemTime::now() >= self.expires_at
    }

    /// Mark the session as connected from a specific client.
    pub fn connect(&mut self, client_type: ClientType, ip: &str) {
        self.status = RemoteSessionStatus::Connected;
        self.client_type = Some(client_type);
        self.client_ip = Some(ip.to_string());
        self.last_heartbeat = Some(SystemTime::now());
    }

    /// Disconnect the session, preserving client info for audit.
    pub fn disconnect(&mut self) {
        self.status = RemoteSessionStatus::Disconnected;
    }

    /// Record a heartbeat from the remote client.
    pub fn record_heartbeat(&mut self) {
        self.last_heartbeat = Some(SystemTime::now());
        if self.status == RemoteSessionStatus::Connected {
            self.status = RemoteSessionStatus::Active;
        }
    }

    /// Create an outbound message.
    pub fn send_message(&mut self, content: &str, msg_type: RemoteMessageType) -> RemoteMessage {
        self.messages_sent += 1;
        RemoteMessage {
            id: generate_id("msg", self.messages_sent as u64),
            direction: MessageDirection::Outbound,
            content: content.to_string(),
            encrypted: true,
            timestamp: SystemTime::now(),
            message_type: msg_type,
        }
    }

    /// Record an inbound message.
    pub fn receive_message(
        &mut self,
        content: &str,
        msg_type: RemoteMessageType,
    ) -> RemoteMessage {
        self.messages_received += 1;
        RemoteMessage {
            id: generate_id(
                "msg",
                (self.messages_sent + self.messages_received) as u64,
            ),
            direction: MessageDirection::Inbound,
            content: content.to_string(),
            encrypted: true,
            timestamp: SystemTime::now(),
            message_type: msg_type,
        }
    }

    /// Time elapsed since session creation.
    pub fn elapsed(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or(Duration::ZERO)
    }
}

// ─── BridgeConfig impl ──────────────────────────────────────────────────────

impl BridgeConfig {
    /// Production defaults: bridge.vibecody.dev:443, TLS enabled, 8-hour TTL.
    pub fn default_config() -> Self {
        Self {
            bridge_host: "bridge.vibecody.dev".to_string(),
            bridge_port: 443,
            use_tls: true,
            session_ttl_minutes: 480,
            heartbeat_interval_secs: 30,
            max_message_size_bytes: 1_048_576,
            allow_file_transfer: false,
            allowed_commands: vec![
                "chat".to_string(),
                "status".to_string(),
                "help".to_string(),
            ],
        }
    }

    /// Local development: localhost:8080, no TLS.
    pub fn local_dev() -> Self {
        Self {
            bridge_host: "localhost".to_string(),
            bridge_port: 8080,
            use_tls: false,
            session_ttl_minutes: 480,
            heartbeat_interval_secs: 30,
            max_message_size_bytes: 1_048_576,
            allow_file_transfer: false,
            allowed_commands: vec![
                "chat".to_string(),
                "status".to_string(),
                "help".to_string(),
            ],
        }
    }

    /// Whether a command string is in the whitelist.
    pub fn is_command_allowed(&self, cmd: &str) -> bool {
        self.allowed_commands.iter().any(|c| c == cmd)
    }

    /// The full bridge URL (scheme + host + port).
    pub fn bridge_url(&self) -> String {
        let scheme = if self.use_tls { "wss" } else { "ws" };
        format!("{}://{}:{}", scheme, self.bridge_host, self.bridge_port)
    }
}

// ─── RemoteControlManager impl ──────────────────────────────────────────────

impl RemoteControlManager {
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            config: BridgeConfig::default_config(),
            active_session: None,
            next_id: 1,
            next_msg_id: 1,
        }
    }

    /// Create a new session using the manager's bridge config.
    pub fn create_session(&mut self) -> &RemoteSession {
        let url = self.config.bridge_url();
        let session = RemoteSession::new(&url);
        self.sessions.push(session);
        self.sessions.last().expect("just pushed a session")
    }

    /// Connect a session by ID to a remote client.
    pub fn connect_session(
        &mut self,
        id: &str,
        client_type: ClientType,
        ip: &str,
    ) -> Result<(), String> {
        let session = self
            .sessions
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| format!("session not found: {}", id))?;

        if session.is_expired() {
            session.status = RemoteSessionStatus::Expired;
            return Err("session is expired".to_string());
        }

        session.connect(client_type, ip);
        self.active_session = Some(id.to_string());
        Ok(())
    }

    /// Disconnect a session by ID.
    pub fn disconnect_session(&mut self, id: &str) -> Result<(), String> {
        let session = self
            .sessions
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or_else(|| format!("session not found: {}", id))?;

        session.disconnect();

        if self.active_session.as_deref() == Some(id) {
            self.active_session = None;
        }
        Ok(())
    }

    /// Look up a session by ID.
    pub fn get_session(&self, id: &str) -> Option<&RemoteSession> {
        self.sessions.iter().find(|s| s.id == id)
    }

    /// Get a mutable reference to a session by ID.
    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut RemoteSession> {
        self.sessions.iter_mut().find(|s| s.id == id)
    }

    /// Returns the currently active session, if any.
    pub fn active_session(&self) -> Option<&RemoteSession> {
        self.active_session
            .as_ref()
            .and_then(|id| self.sessions.iter().find(|s| &s.id == id))
    }

    /// Remove all expired sessions.
    pub fn cleanup_expired(&mut self) {
        let expired_ids: Vec<String> = self
            .sessions
            .iter()
            .filter(|s| s.is_expired())
            .map(|s| s.id.clone())
            .collect();

        if let Some(ref active) = self.active_session {
            if expired_ids.contains(active) {
                self.active_session = None;
            }
        }

        self.sessions.retain(|s| !expired_ids.contains(&s.id));
    }

    /// Send a message on the active session.
    pub fn send(
        &mut self,
        content: &str,
        msg_type: RemoteMessageType,
    ) -> Result<RemoteMessage, String> {
        let active_id = self
            .active_session
            .clone()
            .ok_or("no active session")?;

        let session = self
            .sessions
            .iter_mut()
            .find(|s| s.id == active_id)
            .ok_or("active session not found")?;

        if session.status == RemoteSessionStatus::Disconnected
            || session.status == RemoteSessionStatus::Expired
        {
            return Err("session is not connected".to_string());
        }

        Ok(session.send_message(content, msg_type))
    }

    /// Total (sent, received) across all sessions.
    pub fn total_messages(&self) -> (usize, usize) {
        self.sessions.iter().fold((0, 0), |(s, r), sess| {
            (s + sess.messages_sent, r + sess.messages_received)
        })
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── RemoteSession tests ──

    #[test]
    fn test_session_new_sets_waiting_status() {
        let s = RemoteSession::new("wss://bridge.test:443");
        assert_eq!(s.status, RemoteSessionStatus::Waiting);
    }

    #[test]
    fn test_session_new_generates_token() {
        let s = RemoteSession::new("wss://bridge.test:443");
        assert_eq!(s.session_token.len(), 32);
    }

    #[test]
    fn test_session_new_generates_encryption_key() {
        let s = RemoteSession::new("wss://bridge.test:443");
        assert_eq!(s.encryption_key.len(), 64);
    }

    #[test]
    fn test_session_new_qr_code_data_contains_url() {
        let s = RemoteSession::new("wss://bridge.test:443");
        assert!(s.qr_code_data.starts_with("wss://bridge.test:443"));
    }

    #[test]
    fn test_session_qr_url_contains_id_and_token() {
        let s = RemoteSession::new("wss://bridge.test:443");
        let url = s.qr_url();
        assert!(url.contains(&s.id));
        assert!(url.contains(&s.session_token));
    }

    #[test]
    fn test_session_not_expired_initially() {
        let s = RemoteSession::new("wss://bridge.test:443");
        assert!(!s.is_expired());
    }

    #[test]
    fn test_session_expired_when_expires_at_in_past() {
        let mut s = RemoteSession::new("wss://bridge.test:443");
        s.expires_at = SystemTime::now() - Duration::from_secs(1);
        assert!(s.is_expired());
    }

    #[test]
    fn test_session_connect_sets_status() {
        let mut s = RemoteSession::new("wss://bridge.test:443");
        s.connect(ClientType::Mobile, "192.168.1.10");
        assert_eq!(s.status, RemoteSessionStatus::Connected);
    }

    #[test]
    fn test_session_connect_sets_client_type() {
        let mut s = RemoteSession::new("wss://bridge.test:443");
        s.connect(ClientType::Mobile, "192.168.1.10");
        assert_eq!(s.client_type, Some(ClientType::Mobile));
    }

    #[test]
    fn test_session_connect_sets_client_ip() {
        let mut s = RemoteSession::new("wss://bridge.test:443");
        s.connect(ClientType::Browser, "10.0.0.1");
        assert_eq!(s.client_ip.as_deref(), Some("10.0.0.1"));
    }

    #[test]
    fn test_session_disconnect_preserves_client_info() {
        let mut s = RemoteSession::new("wss://bridge.test:443");
        s.connect(ClientType::Browser, "10.0.0.1");
        s.disconnect();
        assert_eq!(s.status, RemoteSessionStatus::Disconnected);
        assert_eq!(s.client_type, Some(ClientType::Browser));
    }

    #[test]
    fn test_session_record_heartbeat_sets_time() {
        let mut s = RemoteSession::new("wss://bridge.test:443");
        assert!(s.last_heartbeat.is_none());
        s.connect(ClientType::Tablet, "10.0.0.2");
        s.record_heartbeat();
        assert!(s.last_heartbeat.is_some());
    }

    #[test]
    fn test_session_heartbeat_transitions_connected_to_active() {
        let mut s = RemoteSession::new("wss://bridge.test:443");
        s.connect(ClientType::Desktop, "10.0.0.3");
        assert_eq!(s.status, RemoteSessionStatus::Connected);
        s.record_heartbeat();
        assert_eq!(s.status, RemoteSessionStatus::Active);
    }

    #[test]
    fn test_session_heartbeat_does_not_transition_from_active() {
        let mut s = RemoteSession::new("wss://bridge.test:443");
        s.connect(ClientType::Desktop, "10.0.0.3");
        s.record_heartbeat(); // -> Active
        s.record_heartbeat(); // stays Active
        assert_eq!(s.status, RemoteSessionStatus::Active);
    }

    #[test]
    fn test_session_send_message_outbound() {
        let mut s = RemoteSession::new("wss://bridge.test:443");
        let msg = s.send_message("hello", RemoteMessageType::Chat);
        assert_eq!(msg.direction, MessageDirection::Outbound);
        assert_eq!(msg.content, "hello");
        assert!(msg.encrypted);
        assert_eq!(s.messages_sent, 1);
    }

    #[test]
    fn test_session_receive_message_inbound() {
        let mut s = RemoteSession::new("wss://bridge.test:443");
        let msg = s.receive_message("world", RemoteMessageType::Chat);
        assert_eq!(msg.direction, MessageDirection::Inbound);
        assert_eq!(msg.content, "world");
        assert_eq!(s.messages_received, 1);
    }

    #[test]
    fn test_session_message_counts_increment() {
        let mut s = RemoteSession::new("wss://bridge.test:443");
        s.send_message("a", RemoteMessageType::Chat);
        s.send_message("b", RemoteMessageType::Command);
        s.receive_message("c", RemoteMessageType::Status);
        assert_eq!(s.messages_sent, 2);
        assert_eq!(s.messages_received, 1);
    }

    #[test]
    fn test_session_elapsed_is_small() {
        let s = RemoteSession::new("wss://bridge.test:443");
        assert!(s.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn test_session_bridge_url_preserved() {
        let s = RemoteSession::new("wss://custom.host:9999");
        assert_eq!(s.bridge_url, "wss://custom.host:9999");
    }

    #[test]
    fn test_session_initial_no_client() {
        let s = RemoteSession::new("wss://bridge.test:443");
        assert!(s.client_type.is_none());
        assert!(s.client_ip.is_none());
    }

    #[test]
    fn test_session_expiry_is_8_hours() {
        let s = RemoteSession::new("wss://bridge.test:443");
        let diff = s
            .expires_at
            .duration_since(s.created_at)
            .expect("expires_at after created_at");
        let eight_hours = Duration::from_secs(8 * 60 * 60);
        assert!(diff >= eight_hours - Duration::from_secs(1));
        assert!(diff <= eight_hours + Duration::from_secs(1));
    }

    #[test]
    fn test_session_messages_start_at_zero() {
        let s = RemoteSession::new("wss://test:443");
        assert_eq!(s.messages_sent, 0);
        assert_eq!(s.messages_received, 0);
    }

    // ── BridgeConfig tests ──

    #[test]
    fn test_default_config_host_and_port() {
        let cfg = BridgeConfig::default_config();
        assert_eq!(cfg.bridge_host, "bridge.vibecody.dev");
        assert_eq!(cfg.bridge_port, 443);
        assert!(cfg.use_tls);
    }

    #[test]
    fn test_default_config_ttl() {
        let cfg = BridgeConfig::default_config();
        assert_eq!(cfg.session_ttl_minutes, 480);
    }

    #[test]
    fn test_default_config_no_file_transfer() {
        let cfg = BridgeConfig::default_config();
        assert!(!cfg.allow_file_transfer);
    }

    #[test]
    fn test_local_dev_config() {
        let cfg = BridgeConfig::local_dev();
        assert_eq!(cfg.bridge_host, "localhost");
        assert_eq!(cfg.bridge_port, 8080);
        assert!(!cfg.use_tls);
    }

    #[test]
    fn test_is_command_allowed_positive() {
        let cfg = BridgeConfig::default_config();
        assert!(cfg.is_command_allowed("chat"));
        assert!(cfg.is_command_allowed("status"));
        assert!(cfg.is_command_allowed("help"));
    }

    #[test]
    fn test_is_command_allowed_negative() {
        let cfg = BridgeConfig::default_config();
        assert!(!cfg.is_command_allowed("rm"));
        assert!(!cfg.is_command_allowed("sudo"));
    }

    #[test]
    fn test_bridge_url_tls() {
        let cfg = BridgeConfig::default_config();
        assert_eq!(cfg.bridge_url(), "wss://bridge.vibecody.dev:443");
    }

    #[test]
    fn test_bridge_url_no_tls() {
        let cfg = BridgeConfig::local_dev();
        assert_eq!(cfg.bridge_url(), "ws://localhost:8080");
    }

    #[test]
    fn test_default_max_message_size() {
        let cfg = BridgeConfig::default_config();
        assert_eq!(cfg.max_message_size_bytes, 1_048_576);
    }

    #[test]
    fn test_default_heartbeat_interval() {
        let cfg = BridgeConfig::default_config();
        assert_eq!(cfg.heartbeat_interval_secs, 30);
    }

    // ── RemoteControlManager tests ──

    #[test]
    fn test_manager_new_empty() {
        let mgr = RemoteControlManager::new();
        assert!(mgr.sessions.is_empty());
        assert!(mgr.active_session.is_none());
    }

    #[test]
    fn test_manager_create_session() {
        let mut mgr = RemoteControlManager::new();
        let id = mgr.create_session().id.clone();
        assert_eq!(mgr.sessions.len(), 1);
        assert!(mgr.get_session(&id).is_some());
    }

    #[test]
    fn test_manager_connect_session() {
        let mut mgr = RemoteControlManager::new();
        let id = mgr.create_session().id.clone();
        mgr.connect_session(&id, ClientType::Mobile, "10.0.0.1")
            .unwrap();
        assert_eq!(
            mgr.active_session().unwrap().status,
            RemoteSessionStatus::Connected
        );
    }

    #[test]
    fn test_manager_connect_nonexistent() {
        let mut mgr = RemoteControlManager::new();
        let result = mgr.connect_session("nope", ClientType::Browser, "10.0.0.1");
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_connect_expired_session() {
        let mut mgr = RemoteControlManager::new();
        let id = mgr.create_session().id.clone();
        mgr.get_session_mut(&id).unwrap().expires_at =
            SystemTime::now() - Duration::from_secs(1);
        let result = mgr.connect_session(&id, ClientType::Browser, "10.0.0.1");
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_disconnect_session() {
        let mut mgr = RemoteControlManager::new();
        let id = mgr.create_session().id.clone();
        mgr.connect_session(&id, ClientType::Desktop, "10.0.0.1")
            .unwrap();
        mgr.disconnect_session(&id).unwrap();
        assert!(mgr.active_session().is_none());
        assert_eq!(
            mgr.get_session(&id).unwrap().status,
            RemoteSessionStatus::Disconnected
        );
    }

    #[test]
    fn test_manager_disconnect_nonexistent() {
        let mut mgr = RemoteControlManager::new();
        assert!(mgr.disconnect_session("nope").is_err());
    }

    #[test]
    fn test_manager_active_session_none_initially() {
        let mgr = RemoteControlManager::new();
        assert!(mgr.active_session().is_none());
    }

    #[test]
    fn test_manager_cleanup_expired_removes_sessions() {
        let mut mgr = RemoteControlManager::new();
        let id1 = mgr.create_session().id.clone();
        let _id2 = mgr.create_session().id.clone();

        mgr.get_session_mut(&id1).unwrap().expires_at =
            SystemTime::now() - Duration::from_secs(1);

        mgr.cleanup_expired();
        assert_eq!(mgr.sessions.len(), 1);
        assert!(mgr.get_session(&id1).is_none());
    }

    #[test]
    fn test_manager_cleanup_clears_active_if_expired() {
        let mut mgr = RemoteControlManager::new();
        let id = mgr.create_session().id.clone();
        mgr.connect_session(&id, ClientType::Mobile, "10.0.0.1")
            .unwrap();
        mgr.get_session_mut(&id).unwrap().expires_at =
            SystemTime::now() - Duration::from_secs(1);
        mgr.cleanup_expired();
        assert!(mgr.active_session().is_none());
    }

    #[test]
    fn test_manager_send_on_active() {
        let mut mgr = RemoteControlManager::new();
        let id = mgr.create_session().id.clone();
        mgr.connect_session(&id, ClientType::Browser, "10.0.0.1")
            .unwrap();
        let msg = mgr.send("hi", RemoteMessageType::Chat).unwrap();
        assert_eq!(msg.content, "hi");
        assert_eq!(msg.direction, MessageDirection::Outbound);
    }

    #[test]
    fn test_manager_send_no_active() {
        let mut mgr = RemoteControlManager::new();
        let result = mgr.send("hi", RemoteMessageType::Chat);
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_send_on_disconnected_fails() {
        let mut mgr = RemoteControlManager::new();
        let id = mgr.create_session().id.clone();
        mgr.connect_session(&id, ClientType::Browser, "10.0.0.1")
            .unwrap();
        mgr.get_session_mut(&id).unwrap().disconnect();
        let result = mgr.send("hi", RemoteMessageType::Chat);
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_total_messages() {
        let mut mgr = RemoteControlManager::new();
        let id = mgr.create_session().id.clone();
        mgr.connect_session(&id, ClientType::Mobile, "10.0.0.1")
            .unwrap();
        mgr.send("a", RemoteMessageType::Chat).unwrap();
        mgr.send("b", RemoteMessageType::Chat).unwrap();
        mgr.get_session_mut(&id)
            .unwrap()
            .receive_message("c", RemoteMessageType::Chat);
        assert_eq!(mgr.total_messages(), (2, 1));
    }

    #[test]
    fn test_manager_multiple_sessions() {
        let mut mgr = RemoteControlManager::new();
        mgr.create_session();
        mgr.create_session();
        mgr.create_session();
        assert_eq!(mgr.sessions.len(), 3);
    }

    // ── Display / formatting tests ──

    #[test]
    fn test_session_status_display() {
        assert_eq!(format!("{}", RemoteSessionStatus::Waiting), "waiting");
        assert_eq!(format!("{}", RemoteSessionStatus::Active), "active");
        assert_eq!(format!("{}", RemoteSessionStatus::Expired), "expired");
        assert_eq!(
            format!("{}", RemoteSessionStatus::Disconnected),
            "disconnected"
        );
        assert_eq!(format!("{}", RemoteSessionStatus::Connected), "connected");
    }

    #[test]
    fn test_client_type_display() {
        assert_eq!(format!("{}", ClientType::Mobile), "mobile");
        assert_eq!(format!("{}", ClientType::Browser), "browser");
        assert_eq!(format!("{}", ClientType::Tablet), "tablet");
        assert_eq!(format!("{}", ClientType::Desktop), "desktop");
    }

    #[test]
    fn test_message_type_display() {
        assert_eq!(format!("{}", RemoteMessageType::Chat), "chat");
        assert_eq!(format!("{}", RemoteMessageType::Heartbeat), "heartbeat");
        assert_eq!(format!("{}", RemoteMessageType::Command), "command");
        assert_eq!(
            format!("{}", RemoteMessageType::FileRequest),
            "file_request"
        );
        assert_eq!(
            format!("{}", RemoteMessageType::FileResponse),
            "file_response"
        );
        assert_eq!(format!("{}", RemoteMessageType::Status), "status");
    }

    #[test]
    fn test_send_message_type_preserved() {
        let mut s = RemoteSession::new("wss://test:443");
        let msg = s.send_message("cmd", RemoteMessageType::Command);
        assert_eq!(msg.message_type, RemoteMessageType::Command);
    }

    #[test]
    fn test_receive_message_type_file_request() {
        let mut s = RemoteSession::new("wss://test:443");
        let msg = s.receive_message("get file.txt", RemoteMessageType::FileRequest);
        assert_eq!(msg.message_type, RemoteMessageType::FileRequest);
    }

    #[test]
    fn test_manager_total_messages_empty() {
        let mgr = RemoteControlManager::new();
        assert_eq!(mgr.total_messages(), (0, 0));
    }

    #[test]
    fn test_session_qr_url_matches_qr_code_data() {
        let s = RemoteSession::new("wss://bridge.test:443");
        assert_eq!(s.qr_url(), s.qr_code_data);
    }

    #[test]
    fn test_manager_get_session_mut() {
        let mut mgr = RemoteControlManager::new();
        let id = mgr.create_session().id.clone();
        let session = mgr.get_session_mut(&id).unwrap();
        session.status = RemoteSessionStatus::Active;
        assert_eq!(mgr.get_session(&id).unwrap().status, RemoteSessionStatus::Active);
    }
}
