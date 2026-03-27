#![allow(dead_code)]
//! Mobile Gateway — Machine registration & dispatch for iOS/Android remote management.
//!
//! Inspired by Claude's dispatch feature and OpenClaw gateway. Allows mobile apps
//! to register, discover, and remotely manage VibeCody CLI/UI sessions running on
//! any machine.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐        ┌──────────────────┐        ┌─────────────────┐
//! │  iOS/Android App │◄──────►│  Bridge Relay     │◄──────►│ VibeCody Daemon │
//! │  (mobile client) │  WSS   │  (cloud or self-  │  WSS   │ (machine agent) │
//! │                  │        │   hosted)         │        │                 │
//! └─────────────────┘        └──────────────────┘        └─────────────────┘
//! ```
//!
//! # Features
//!
//! - **Machine Registration**: Daemon registers itself with a machine ID, name, OS,
//!   workspace info, and capabilities
//! - **Device Pairing**: QR code or 6-digit PIN pairing between mobile and machine
//! - **Session Dispatch**: Send tasks from mobile to any registered machine
//! - **Live Streaming**: SSE/WebSocket relay of agent events to mobile
//! - **Push Notifications**: FCM/APNs token management for background alerts
//! - **Multi-Machine**: Manage multiple machines from a single mobile app
//! - **Heartbeat & Presence**: Online/offline status with configurable intervals

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ─── Enums ───────────────────────────────────────────────────────────────────

/// Operating system of the registered machine.
#[derive(Debug, Clone, PartialEq)]
pub enum MachineOS {
    MacOS,
    Linux,
    Windows,
    Docker,
    WSL,
    Unknown,
}

impl fmt::Display for MachineOS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MacOS => write!(f, "macOS"),
            Self::Linux => write!(f, "Linux"),
            Self::Windows => write!(f, "Windows"),
            Self::Docker => write!(f, "Docker"),
            Self::WSL => write!(f, "WSL"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Current status of a registered machine.
#[derive(Debug, Clone, PartialEq)]
pub enum MachineStatus {
    Online,
    Busy,
    Idle,
    Offline,
    Unreachable,
}

impl fmt::Display for MachineStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Online => write!(f, "online"),
            Self::Busy => write!(f, "busy"),
            Self::Idle => write!(f, "idle"),
            Self::Offline => write!(f, "offline"),
            Self::Unreachable => write!(f, "unreachable"),
        }
    }
}

/// Type of dispatch task sent from mobile.
#[derive(Debug, Clone, PartialEq)]
pub enum DispatchType {
    /// Free-form chat message to the agent.
    Chat,
    /// Execute a specific agent task.
    AgentTask,
    /// Run a CLI command.
    Command,
    /// Run a REPL slash-command.
    ReplCommand,
    /// File operation (read, list, search).
    FileOp,
    /// Git operation (status, commit, push).
    GitOp,
    /// Cancel a running task.
    Cancel,
}

impl fmt::Display for DispatchType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Chat => write!(f, "chat"),
            Self::AgentTask => write!(f, "agent_task"),
            Self::Command => write!(f, "command"),
            Self::ReplCommand => write!(f, "repl_command"),
            Self::FileOp => write!(f, "file_op"),
            Self::GitOp => write!(f, "git_op"),
            Self::Cancel => write!(f, "cancel"),
        }
    }
}

/// Status of a dispatched task.
#[derive(Debug, Clone, PartialEq)]
pub enum DispatchStatus {
    Queued,
    Sent,
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl fmt::Display for DispatchStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Queued => write!(f, "queued"),
            Self::Sent => write!(f, "sent"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::TimedOut => write!(f, "timed_out"),
        }
    }
}

/// Push notification platform.
#[derive(Debug, Clone, PartialEq)]
pub enum PushPlatform {
    APNs,
    FCM,
    WebPush,
}

impl fmt::Display for PushPlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::APNs => write!(f, "apns"),
            Self::FCM => write!(f, "fcm"),
            Self::WebPush => write!(f, "webpush"),
        }
    }
}

/// Pairing method for connecting mobile to machine.
#[derive(Debug, Clone, PartialEq)]
pub enum PairingMethod {
    /// QR code scanned by mobile camera.
    QrCode,
    /// 6-digit numeric PIN entered manually.
    Pin,
    /// Tailscale mesh — pre-authenticated via tailnet.
    Tailscale,
    /// Cloud relay — both sides connect to bridge.vibecody.dev.
    CloudRelay,
}

impl fmt::Display for PairingMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QrCode => write!(f, "qr_code"),
            Self::Pin => write!(f, "pin"),
            Self::Tailscale => write!(f, "tailscale"),
            Self::CloudRelay => write!(f, "cloud_relay"),
        }
    }
}

// ─── Structs ─────────────────────────────────────────────────────────────────

/// A registered machine running VibeCody daemon.
#[derive(Debug, Clone)]
pub struct RegisteredMachine {
    pub machine_id: String,
    pub name: String,
    pub hostname: String,
    pub os: MachineOS,
    pub arch: String,
    pub status: MachineStatus,
    pub daemon_port: u16,
    pub daemon_version: String,
    pub workspace_root: String,
    pub capabilities: Vec<String>,
    pub active_sessions: usize,
    pub max_sessions: usize,
    pub cpu_cores: usize,
    pub memory_gb: f64,
    pub disk_free_gb: f64,
    pub registered_at: u64,
    pub last_heartbeat: u64,
    pub heartbeat_interval_secs: u64,
    pub api_token_hash: String,
    pub tailscale_ip: Option<String>,
    pub public_url: Option<String>,
    pub tags: Vec<String>,
}

/// A mobile device paired with one or more machines.
#[derive(Debug, Clone)]
pub struct PairedDevice {
    pub device_id: String,
    pub device_name: String,
    pub platform: PushPlatform,
    pub push_token: Option<String>,
    pub paired_machines: Vec<String>,
    pub paired_at: u64,
    pub last_seen: u64,
    pub app_version: String,
    pub os_version: String,
}

/// A pairing request (pending or completed).
#[derive(Debug, Clone)]
pub struct PairingRequest {
    pub id: String,
    pub machine_id: String,
    pub method: PairingMethod,
    pub pin: Option<String>,
    pub qr_data: Option<String>,
    pub status: PairingStatus,
    pub created_at: u64,
    pub expires_at: u64,
    pub device_id: Option<String>,
}

/// Status of a pairing request.
#[derive(Debug, Clone, PartialEq)]
pub enum PairingStatus {
    Pending,
    Accepted,
    Rejected,
    Expired,
}

impl fmt::Display for PairingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Accepted => write!(f, "accepted"),
            Self::Rejected => write!(f, "rejected"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

/// A task dispatched from mobile to a machine.
#[derive(Debug, Clone)]
pub struct DispatchedTask {
    pub task_id: String,
    pub machine_id: String,
    pub device_id: String,
    pub dispatch_type: DispatchType,
    pub payload: String,
    pub status: DispatchStatus,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub result: Option<String>,
    pub error: Option<String>,
    pub session_id: Option<String>,
    pub notify_on_complete: bool,
    pub timeout_secs: u64,
}

/// A notification to be sent to a mobile device.
#[derive(Debug, Clone)]
pub struct PushNotification {
    pub id: String,
    pub device_id: String,
    pub title: String,
    pub body: String,
    pub category: NotificationCategory,
    pub data: HashMap<String, String>,
    pub created_at: u64,
    pub sent: bool,
    pub sent_at: Option<u64>,
}

/// Category of push notification.
#[derive(Debug, Clone, PartialEq)]
pub enum NotificationCategory {
    TaskComplete,
    TaskFailed,
    ApprovalRequired,
    MachineOffline,
    MachineOnline,
    SessionEvent,
    SecurityAlert,
}

impl fmt::Display for NotificationCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TaskComplete => write!(f, "task_complete"),
            Self::TaskFailed => write!(f, "task_failed"),
            Self::ApprovalRequired => write!(f, "approval_required"),
            Self::MachineOffline => write!(f, "machine_offline"),
            Self::MachineOnline => write!(f, "machine_online"),
            Self::SessionEvent => write!(f, "session_event"),
            Self::SecurityAlert => write!(f, "security_alert"),
        }
    }
}

/// Gateway configuration.
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub bridge_url: String,
    pub heartbeat_interval_secs: u64,
    pub session_ttl_hours: u64,
    pub pairing_ttl_minutes: u64,
    pub max_machines_per_device: usize,
    pub max_devices_per_machine: usize,
    pub max_pending_dispatches: usize,
    pub require_pin_confirmation: bool,
    pub enable_push_notifications: bool,
    pub allowed_dispatch_types: Vec<DispatchType>,
}

impl GatewayConfig {
    pub fn default_config() -> Self {
        Self {
            bridge_url: "wss://bridge.vibecody.dev".to_string(),
            heartbeat_interval_secs: 30,
            session_ttl_hours: 24,
            pairing_ttl_minutes: 10,
            max_machines_per_device: 10,
            max_devices_per_machine: 5,
            max_pending_dispatches: 50,
            require_pin_confirmation: true,
            enable_push_notifications: true,
            allowed_dispatch_types: vec![
                DispatchType::Chat,
                DispatchType::AgentTask,
                DispatchType::Command,
                DispatchType::ReplCommand,
                DispatchType::FileOp,
                DispatchType::GitOp,
                DispatchType::Cancel,
            ],
        }
    }

    pub fn local_dev() -> Self {
        Self {
            bridge_url: "ws://localhost:7879".to_string(),
            heartbeat_interval_secs: 10,
            session_ttl_hours: 8,
            pairing_ttl_minutes: 30,
            max_machines_per_device: 5,
            max_devices_per_machine: 3,
            max_pending_dispatches: 20,
            require_pin_confirmation: false,
            enable_push_notifications: false,
            allowed_dispatch_types: vec![
                DispatchType::Chat,
                DispatchType::AgentTask,
                DispatchType::Command,
                DispatchType::ReplCommand,
                DispatchType::FileOp,
                DispatchType::GitOp,
                DispatchType::Cancel,
            ],
        }
    }
}

/// Machine system info snapshot sent with heartbeats.
#[derive(Debug, Clone)]
pub struct MachineMetrics {
    pub machine_id: String,
    pub timestamp: u64,
    pub cpu_usage_pct: f64,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    pub active_agent_sessions: usize,
    pub queued_tasks: usize,
    pub uptime_secs: u64,
    pub provider_name: String,
    pub provider_healthy: bool,
}

/// Summary of a machine's active sessions for the mobile overview.
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub session_id: String,
    pub machine_id: String,
    pub task: String,
    pub status: String,
    pub provider: String,
    pub started_at: u64,
    pub last_event_at: u64,
    pub steps_completed: usize,
    pub has_pending_approval: bool,
}

/// A file listing entry returned for FileOp dispatches.
#[derive(Debug, Clone)]
pub struct RemoteFileEntry {
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub modified_at: u64,
}

/// Git status returned for GitOp dispatches.
#[derive(Debug, Clone)]
pub struct RemoteGitStatus {
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub staged: Vec<String>,
    pub modified: Vec<String>,
    pub untracked: Vec<String>,
    pub has_conflicts: bool,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn generate_id(prefix: &str, counter: u64) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    format!("{}-{:x}-{:x}", prefix, ts, counter)
}

fn generate_pin() -> String {
    // 6-digit PIN from timestamp entropy
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos();
    let pin = (ts % 1_000_000) as u32;
    format!("{:06}", pin)
}

fn simple_hash(input: &str) -> String {
    let mut hash: u64 = 5381;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    format!("{:016x}", hash)
}

fn detect_os() -> MachineOS {
    if cfg!(target_os = "macos") {
        MachineOS::MacOS
    } else if cfg!(target_os = "linux") {
        // Check for Docker or WSL
        if std::path::Path::new("/.dockerenv").exists() {
            MachineOS::Docker
        } else if std::fs::read_to_string("/proc/version")
            .unwrap_or_default()
            .to_lowercase()
            .contains("microsoft")
        {
            MachineOS::WSL
        } else {
            MachineOS::Linux
        }
    } else if cfg!(target_os = "windows") {
        MachineOS::Windows
    } else {
        MachineOS::Unknown
    }
}

fn detect_arch() -> String {
    if cfg!(target_arch = "x86_64") {
        "x86_64".to_string()
    } else if cfg!(target_arch = "aarch64") {
        "aarch64".to_string()
    } else {
        std::env::consts::ARCH.to_string()
    }
}

// ─── MobileGateway ───────────────────────────────────────────────────────────

/// Central manager for machine registration, device pairing, and task dispatch.
pub struct MobileGateway {
    pub machines: HashMap<String, RegisteredMachine>,
    pub devices: HashMap<String, PairedDevice>,
    pub pairing_requests: Vec<PairingRequest>,
    pub dispatched_tasks: Vec<DispatchedTask>,
    pub notifications: Vec<PushNotification>,
    pub config: GatewayConfig,
    next_id: u64,
}

impl MobileGateway {
    /// Create a new gateway with default configuration.
    pub fn new() -> Self {
        Self {
            machines: HashMap::new(),
            devices: HashMap::new(),
            pairing_requests: Vec::new(),
            dispatched_tasks: Vec::new(),
            notifications: Vec::new(),
            config: GatewayConfig::default_config(),
            next_id: 0,
        }
    }

    /// Create a gateway with custom configuration.
    pub fn with_config(config: GatewayConfig) -> Self {
        Self {
            machines: HashMap::new(),
            devices: HashMap::new(),
            pairing_requests: Vec::new(),
            dispatched_tasks: Vec::new(),
            notifications: Vec::new(),
            config,
            next_id: 0,
        }
    }

    fn next_id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        generate_id(prefix, self.next_id)
    }

    // ── Machine Registration ─────────────────────────────────────────────

    /// Register this machine as a VibeCody daemon endpoint.
    pub fn register_machine(
        &mut self,
        name: &str,
        hostname: &str,
        daemon_port: u16,
        workspace_root: &str,
        api_token: &str,
    ) -> &RegisteredMachine {
        let machine_id = self.next_id("mach");
        let now = now_epoch_secs();

        let machine = RegisteredMachine {
            machine_id: machine_id.clone(),
            name: name.to_string(),
            hostname: hostname.to_string(),
            os: detect_os(),
            arch: detect_arch(),
            status: MachineStatus::Online,
            daemon_port,
            daemon_version: env!("CARGO_PKG_VERSION").to_string(),
            workspace_root: workspace_root.to_string(),
            capabilities: vec![
                "chat".to_string(),
                "agent".to_string(),
                "file_ops".to_string(),
                "git_ops".to_string(),
                "repl".to_string(),
                "streaming".to_string(),
            ],
            active_sessions: 0,
            max_sessions: 8,
            cpu_cores: num_cpus(),
            memory_gb: 0.0, // filled by metrics
            disk_free_gb: 0.0,
            registered_at: now,
            last_heartbeat: now,
            heartbeat_interval_secs: self.config.heartbeat_interval_secs,
            api_token_hash: simple_hash(api_token),
            tailscale_ip: None,
            public_url: None,
            tags: Vec::new(),
        };

        self.machines.insert(machine_id.clone(), machine);
        self.machines.get(&machine_id).expect("just inserted")
    }

    /// Register with auto-detected system info.
    pub fn register_self(
        &mut self,
        daemon_port: u16,
        workspace_root: &str,
        api_token: &str,
    ) -> &RegisteredMachine {
        let hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "localhost".to_string());
        let name = format!("{} ({})", hostname, detect_os());
        self.register_machine(&name, &hostname, daemon_port, workspace_root, api_token)
    }

    /// Update machine heartbeat and status.
    pub fn heartbeat(&mut self, machine_id: &str, metrics: Option<MachineMetrics>) -> Result<(), String> {
        let machine = self.machines.get_mut(machine_id)
            .ok_or_else(|| format!("Machine {} not found", machine_id))?;

        machine.last_heartbeat = now_epoch_secs();

        if let Some(m) = metrics {
            machine.memory_gb = m.memory_total_gb;
            machine.disk_free_gb = m.disk_total_gb - m.disk_used_gb;
            machine.active_sessions = m.active_agent_sessions;

            if m.active_agent_sessions > 0 {
                machine.status = MachineStatus::Busy;
            } else {
                machine.status = MachineStatus::Idle;
            }
        } else {
            if machine.status == MachineStatus::Offline || machine.status == MachineStatus::Unreachable {
                machine.status = MachineStatus::Online;
            }
        }

        Ok(())
    }

    /// Unregister a machine.
    pub fn unregister_machine(&mut self, machine_id: &str) -> Result<RegisteredMachine, String> {
        self.machines.remove(machine_id)
            .ok_or_else(|| format!("Machine {} not found", machine_id))
    }

    /// Get a registered machine by ID.
    pub fn get_machine(&self, machine_id: &str) -> Option<&RegisteredMachine> {
        self.machines.get(machine_id)
    }

    /// List all registered machines.
    pub fn list_machines(&self) -> Vec<&RegisteredMachine> {
        self.machines.values().collect()
    }

    /// List machines that are currently online or idle.
    pub fn list_available_machines(&self) -> Vec<&RegisteredMachine> {
        self.machines.values()
            .filter(|m| matches!(m.status, MachineStatus::Online | MachineStatus::Idle))
            .collect()
    }

    /// Mark stale machines (no heartbeat for 3x interval) as unreachable.
    pub fn check_stale_machines(&mut self) -> Vec<String> {
        let now = now_epoch_secs();
        let mut stale = Vec::new();

        for machine in self.machines.values_mut() {
            let threshold = machine.heartbeat_interval_secs * 3;
            if now.saturating_sub(machine.last_heartbeat) > threshold
                && machine.status != MachineStatus::Offline
                && machine.status != MachineStatus::Unreachable
            {
                machine.status = MachineStatus::Unreachable;
                stale.push(machine.machine_id.clone());
            }
        }

        stale
    }

    /// Set machine tags for organization.
    pub fn tag_machine(&mut self, machine_id: &str, tags: Vec<String>) -> Result<(), String> {
        let machine = self.machines.get_mut(machine_id)
            .ok_or_else(|| format!("Machine {} not found", machine_id))?;
        machine.tags = tags;
        Ok(())
    }

    // ── Device Pairing ───────────────────────────────────────────────────

    /// Create a pairing request for a machine (generates QR + PIN).
    pub fn create_pairing(
        &mut self,
        machine_id: &str,
        method: PairingMethod,
    ) -> Result<&PairingRequest, String> {
        if !self.machines.contains_key(machine_id) {
            return Err(format!("Machine {} not found", machine_id));
        }

        let now = now_epoch_secs();
        let ttl = self.config.pairing_ttl_minutes * 60;
        let id = self.next_id("pair");

        let pin = if matches!(method, PairingMethod::Pin | PairingMethod::QrCode) {
            Some(generate_pin())
        } else {
            None
        };

        let qr_data = if matches!(method, PairingMethod::QrCode) {
            let machine = self.machines.get(machine_id).expect("checked above");
            Some(format!(
                "vibecody://pair?machine={}&host={}&port={}&pin={}&bridge={}",
                machine_id,
                machine.hostname,
                machine.daemon_port,
                pin.as_deref().unwrap_or(""),
                self.config.bridge_url,
            ))
        } else {
            None
        };

        let request = PairingRequest {
            id: id.clone(),
            machine_id: machine_id.to_string(),
            method,
            pin,
            qr_data,
            status: PairingStatus::Pending,
            created_at: now,
            expires_at: now + ttl,
            device_id: None,
        };

        self.pairing_requests.push(request);
        Ok(self.pairing_requests.last().expect("just pushed"))
    }

    /// Accept a pairing request from a mobile device.
    pub fn accept_pairing(
        &mut self,
        pairing_id: &str,
        device_id: &str,
        device_name: &str,
        platform: PushPlatform,
        push_token: Option<String>,
        app_version: &str,
        os_version: &str,
    ) -> Result<(), String> {
        let now = now_epoch_secs();

        // Find and validate pairing request.
        let req = self.pairing_requests.iter_mut()
            .find(|r| r.id == pairing_id)
            .ok_or_else(|| format!("Pairing request {} not found", pairing_id))?;

        if req.status != PairingStatus::Pending {
            return Err(format!("Pairing {} is {}, not pending", pairing_id, req.status));
        }
        if now > req.expires_at {
            req.status = PairingStatus::Expired;
            return Err("Pairing request has expired".to_string());
        }

        let machine_id = req.machine_id.clone();
        req.status = PairingStatus::Accepted;
        req.device_id = Some(device_id.to_string());

        // Check machine device limit.
        let _machine = self.machines.get(&machine_id)
            .ok_or_else(|| format!("Machine {} no longer registered", machine_id))?;
        let existing_count = self.devices.values()
            .filter(|d| d.paired_machines.contains(&machine_id))
            .count();
        if existing_count >= self.config.max_devices_per_machine {
            return Err(format!(
                "Machine {} has reached max paired devices ({})",
                machine_id, self.config.max_devices_per_machine
            ));
        }

        // Register or update device.
        if let Some(device) = self.devices.get_mut(device_id) {
            if !device.paired_machines.contains(&machine_id) {
                if device.paired_machines.len() >= self.config.max_machines_per_device {
                    return Err(format!(
                        "Device {} has reached max paired machines ({})",
                        device_id, self.config.max_machines_per_device
                    ));
                }
                device.paired_machines.push(machine_id);
            }
            device.last_seen = now;
            if let Some(token) = push_token {
                device.push_token = Some(token);
            }
        } else {
            let device = PairedDevice {
                device_id: device_id.to_string(),
                device_name: device_name.to_string(),
                platform,
                push_token,
                paired_machines: vec![machine_id],
                paired_at: now,
                last_seen: now,
                app_version: app_version.to_string(),
                os_version: os_version.to_string(),
            };
            self.devices.insert(device_id.to_string(), device);
        }

        Ok(())
    }

    /// Verify a PIN for a pairing request.
    pub fn verify_pin(&self, pairing_id: &str, pin: &str) -> Result<bool, String> {
        let req = self.pairing_requests.iter()
            .find(|r| r.id == pairing_id)
            .ok_or_else(|| format!("Pairing request {} not found", pairing_id))?;

        if req.status != PairingStatus::Pending {
            return Err(format!("Pairing {} is not pending", pairing_id));
        }

        let now = now_epoch_secs();
        if now > req.expires_at {
            return Err("Pairing request has expired".to_string());
        }

        Ok(req.pin.as_deref() == Some(pin))
    }

    /// Reject a pairing request.
    pub fn reject_pairing(&mut self, pairing_id: &str) -> Result<(), String> {
        let req = self.pairing_requests.iter_mut()
            .find(|r| r.id == pairing_id)
            .ok_or_else(|| format!("Pairing request {} not found", pairing_id))?;
        req.status = PairingStatus::Rejected;
        Ok(())
    }

    /// Unpair a device from a machine.
    pub fn unpair_device(&mut self, device_id: &str, machine_id: &str) -> Result<(), String> {
        let device = self.devices.get_mut(device_id)
            .ok_or_else(|| format!("Device {} not found", device_id))?;
        device.paired_machines.retain(|m| m != machine_id);
        if device.paired_machines.is_empty() {
            self.devices.remove(device_id);
        }
        Ok(())
    }

    /// List all paired devices for a machine.
    pub fn list_devices_for_machine(&self, machine_id: &str) -> Vec<&PairedDevice> {
        self.devices.values()
            .filter(|d| d.paired_machines.contains(&machine_id.to_string()))
            .collect()
    }

    /// List all machines paired with a device.
    pub fn list_machines_for_device(&self, device_id: &str) -> Vec<&RegisteredMachine> {
        match self.devices.get(device_id) {
            Some(device) => device.paired_machines.iter()
                .filter_map(|mid| self.machines.get(mid))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Cleanup expired pairing requests.
    pub fn cleanup_expired_pairings(&mut self) -> usize {
        let now = now_epoch_secs();
        let before = self.pairing_requests.len();
        self.pairing_requests.retain(|r| {
            r.status == PairingStatus::Accepted || now <= r.expires_at
        });
        before - self.pairing_requests.len()
    }

    // ── Task Dispatch ────────────────────────────────────────────────────

    /// Dispatch a task from a mobile device to a machine.
    pub fn dispatch_task(
        &mut self,
        device_id: &str,
        machine_id: &str,
        dispatch_type: DispatchType,
        payload: &str,
    ) -> Result<&DispatchedTask, String> {
        // Validate device is paired with machine.
        let device = self.devices.get(device_id)
            .ok_or_else(|| format!("Device {} not found", device_id))?;
        if !device.paired_machines.contains(&machine_id.to_string()) {
            return Err(format!("Device {} is not paired with machine {}", device_id, machine_id));
        }

        // Validate machine is available.
        let machine = self.machines.get(machine_id)
            .ok_or_else(|| format!("Machine {} not found", machine_id))?;
        if machine.status == MachineStatus::Offline || machine.status == MachineStatus::Unreachable {
            return Err(format!("Machine {} is {}", machine_id, machine.status));
        }

        // Check dispatch type is allowed.
        if !self.config.allowed_dispatch_types.contains(&dispatch_type) {
            return Err(format!("Dispatch type {} is not allowed", dispatch_type));
        }

        // Check pending dispatch limit.
        let pending = self.dispatched_tasks.iter()
            .filter(|t| t.machine_id == machine_id && matches!(t.status, DispatchStatus::Queued | DispatchStatus::Sent | DispatchStatus::Running))
            .count();
        if pending >= self.config.max_pending_dispatches {
            return Err(format!("Machine {} has {} pending dispatches (max {})", machine_id, pending, self.config.max_pending_dispatches));
        }

        let task_id = self.next_id("dsp");
        let now = now_epoch_secs();

        let task = DispatchedTask {
            task_id: task_id.clone(),
            machine_id: machine_id.to_string(),
            device_id: device_id.to_string(),
            dispatch_type,
            payload: payload.to_string(),
            status: DispatchStatus::Queued,
            created_at: now,
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
            session_id: None,
            notify_on_complete: true,
            timeout_secs: 300,
        };

        self.dispatched_tasks.push(task);
        Ok(self.dispatched_tasks.last().expect("just pushed"))
    }

    /// Update the status of a dispatched task (called by the machine daemon).
    pub fn update_dispatch(
        &mut self,
        task_id: &str,
        status: DispatchStatus,
        result: Option<String>,
        error: Option<String>,
        session_id: Option<String>,
    ) -> Result<(), String> {
        let task = self.dispatched_tasks.iter_mut()
            .find(|t| t.task_id == task_id)
            .ok_or_else(|| format!("Task {} not found", task_id))?;

        let now = now_epoch_secs();

        match &status {
            DispatchStatus::Running => {
                task.started_at = Some(now);
            }
            DispatchStatus::Completed | DispatchStatus::Failed | DispatchStatus::Cancelled | DispatchStatus::TimedOut => {
                task.completed_at = Some(now);
            }
            _ => {}
        }

        task.status = status;
        if let Some(r) = result {
            task.result = Some(r);
        }
        if let Some(e) = error {
            task.error = Some(e);
        }
        if let Some(sid) = session_id {
            task.session_id = Some(sid);
        }

        // Queue push notification if task completed and notify enabled.
        if task.notify_on_complete
            && matches!(task.status, DispatchStatus::Completed | DispatchStatus::Failed)
        {
            let device_id = task.device_id.clone();
            let category = if task.status == DispatchStatus::Completed {
                NotificationCategory::TaskComplete
            } else {
                NotificationCategory::TaskFailed
            };
            let title = if task.status == DispatchStatus::Completed {
                "Task Completed".to_string()
            } else {
                "Task Failed".to_string()
            };
            let body = task.result.clone().or_else(|| task.error.clone())
                .unwrap_or_else(|| task.payload.chars().take(100).collect());

            let mut data = HashMap::new();
            data.insert("task_id".to_string(), task.task_id.clone());
            data.insert("machine_id".to_string(), task.machine_id.clone());

            let notif = PushNotification {
                id: generate_id("notif", now),
                device_id,
                title,
                body,
                category,
                data,
                created_at: now,
                sent: false,
                sent_at: None,
            };
            self.notifications.push(notif);
        }

        Ok(())
    }

    /// Get a dispatched task by ID.
    pub fn get_dispatch(&self, task_id: &str) -> Option<&DispatchedTask> {
        self.dispatched_tasks.iter().find(|t| t.task_id == task_id)
    }

    /// List dispatches for a device.
    pub fn list_dispatches_for_device(&self, device_id: &str) -> Vec<&DispatchedTask> {
        self.dispatched_tasks.iter()
            .filter(|t| t.device_id == device_id)
            .collect()
    }

    /// List dispatches for a machine.
    pub fn list_dispatches_for_machine(&self, machine_id: &str) -> Vec<&DispatchedTask> {
        self.dispatched_tasks.iter()
            .filter(|t| t.machine_id == machine_id)
            .collect()
    }

    /// List pending (queued) dispatches for a machine.
    pub fn pending_dispatches(&self, machine_id: &str) -> Vec<&DispatchedTask> {
        self.dispatched_tasks.iter()
            .filter(|t| t.machine_id == machine_id && t.status == DispatchStatus::Queued)
            .collect()
    }

    /// Cancel a pending or running dispatch.
    pub fn cancel_dispatch(&mut self, task_id: &str) -> Result<(), String> {
        let task = self.dispatched_tasks.iter_mut()
            .find(|t| t.task_id == task_id)
            .ok_or_else(|| format!("Task {} not found", task_id))?;

        if matches!(task.status, DispatchStatus::Completed | DispatchStatus::Failed | DispatchStatus::Cancelled) {
            return Err(format!("Task {} is already {}", task_id, task.status));
        }

        task.status = DispatchStatus::Cancelled;
        task.completed_at = Some(now_epoch_secs());
        Ok(())
    }

    /// Check for timed-out dispatches and mark them.
    pub fn check_timeouts(&mut self) -> Vec<String> {
        let now = now_epoch_secs();
        let mut timed_out = Vec::new();

        for task in &mut self.dispatched_tasks {
            if matches!(task.status, DispatchStatus::Running | DispatchStatus::Sent) {
                let started = task.started_at.unwrap_or(task.created_at);
                if now.saturating_sub(started) > task.timeout_secs {
                    task.status = DispatchStatus::TimedOut;
                    task.completed_at = Some(now);
                    task.error = Some("Dispatch timed out".to_string());
                    timed_out.push(task.task_id.clone());
                }
            }
        }

        timed_out
    }

    // ── Push Notifications ───────────────────────────────────────────────

    /// Update a device's push notification token.
    pub fn update_push_token(
        &mut self,
        device_id: &str,
        push_token: &str,
    ) -> Result<(), String> {
        let device = self.devices.get_mut(device_id)
            .ok_or_else(|| format!("Device {} not found", device_id))?;
        device.push_token = Some(push_token.to_string());
        Ok(())
    }

    /// Queue a push notification for a device.
    pub fn queue_notification(
        &mut self,
        device_id: &str,
        title: &str,
        body: &str,
        category: NotificationCategory,
    ) -> Result<&PushNotification, String> {
        if !self.devices.contains_key(device_id) {
            return Err(format!("Device {} not found", device_id));
        }

        let now = now_epoch_secs();
        let notif = PushNotification {
            id: generate_id("notif", now),
            device_id: device_id.to_string(),
            title: title.to_string(),
            body: body.to_string(),
            category,
            data: HashMap::new(),
            created_at: now,
            sent: false,
            sent_at: None,
        };

        self.notifications.push(notif);
        Ok(self.notifications.last().expect("just pushed"))
    }

    /// Get unsent notifications for a device.
    pub fn unsent_notifications(&self, device_id: &str) -> Vec<&PushNotification> {
        self.notifications.iter()
            .filter(|n| n.device_id == device_id && !n.sent)
            .collect()
    }

    /// Mark a notification as sent.
    pub fn mark_notification_sent(&mut self, notification_id: &str) -> Result<(), String> {
        let notif = self.notifications.iter_mut()
            .find(|n| n.id == notification_id)
            .ok_or_else(|| format!("Notification {} not found", notification_id))?;
        notif.sent = true;
        notif.sent_at = Some(now_epoch_secs());
        Ok(())
    }

    // ── Statistics & Summary ─────────────────────────────────────────────

    /// Get gateway statistics.
    pub fn stats(&self) -> GatewayStats {
        let total_machines = self.machines.len();
        let online_machines = self.machines.values()
            .filter(|m| matches!(m.status, MachineStatus::Online | MachineStatus::Idle | MachineStatus::Busy))
            .count();
        let total_devices = self.devices.len();
        let total_dispatches = self.dispatched_tasks.len();
        let active_dispatches = self.dispatched_tasks.iter()
            .filter(|t| matches!(t.status, DispatchStatus::Queued | DispatchStatus::Sent | DispatchStatus::Running))
            .count();
        let completed_dispatches = self.dispatched_tasks.iter()
            .filter(|t| t.status == DispatchStatus::Completed)
            .count();
        let failed_dispatches = self.dispatched_tasks.iter()
            .filter(|t| t.status == DispatchStatus::Failed)
            .count();
        let pending_notifications = self.notifications.iter()
            .filter(|n| !n.sent)
            .count();

        GatewayStats {
            total_machines,
            online_machines,
            total_devices,
            total_dispatches,
            active_dispatches,
            completed_dispatches,
            failed_dispatches,
            pending_notifications,
            pending_pairings: self.pairing_requests.iter()
                .filter(|r| r.status == PairingStatus::Pending)
                .count(),
        }
    }

    /// Get a brief summary of all machines for mobile display.
    pub fn machine_summaries(&self) -> Vec<MachineSummary> {
        self.machines.values().map(|m| {
            let dispatches = self.dispatched_tasks.iter()
                .filter(|t| t.machine_id == m.machine_id && matches!(t.status, DispatchStatus::Running))
                .count();
            let devices = self.devices.values()
                .filter(|d| d.paired_machines.contains(&m.machine_id))
                .count();

            MachineSummary {
                machine_id: m.machine_id.clone(),
                name: m.name.clone(),
                os: m.os.to_string(),
                status: m.status.to_string(),
                active_tasks: dispatches,
                paired_devices: devices,
                last_heartbeat: m.last_heartbeat,
                workspace: m.workspace_root.clone(),
            }
        }).collect()
    }
}

/// Gateway statistics.
#[derive(Debug, Clone)]
pub struct GatewayStats {
    pub total_machines: usize,
    pub online_machines: usize,
    pub total_devices: usize,
    pub total_dispatches: usize,
    pub active_dispatches: usize,
    pub completed_dispatches: usize,
    pub failed_dispatches: usize,
    pub pending_notifications: usize,
    pub pending_pairings: usize,
}

/// Brief machine summary for mobile list view.
#[derive(Debug, Clone)]
pub struct MachineSummary {
    pub machine_id: String,
    pub name: String,
    pub os: String,
    pub status: String,
    pub active_tasks: usize,
    pub paired_devices: usize,
    pub last_heartbeat: u64,
    pub workspace: String,
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gateway() -> MobileGateway {
        MobileGateway::with_config(GatewayConfig::local_dev())
    }

    fn register_test_machine(gw: &mut MobileGateway) -> String {
        let machine = gw.register_machine("Test Machine", "test-host", 7878, "/home/user/project", "secret123");
        machine.machine_id.clone()
    }

    fn pair_test_device(gw: &mut MobileGateway, machine_id: &str) -> (String, String) {
        let pairing = gw.create_pairing(machine_id, PairingMethod::Pin).unwrap();
        let pairing_id = pairing.id.clone();
        let pin = pairing.pin.clone().unwrap();
        let device_id = "device-001".to_string();
        gw.accept_pairing(&pairing_id, &device_id, "iPhone 16", PushPlatform::APNs, Some("apns-token-xyz".to_string()), "1.0.0", "18.0").unwrap();
        (device_id, pairing_id)
    }

    // ── Machine Registration ─────────────────────────────────────────

    #[test]
    fn test_register_machine() {
        let mut gw = make_gateway();
        let machine = gw.register_machine("My Mac", "mac-pro.local", 7878, "/Users/dev/project", "tok123");
        assert_eq!(machine.name, "My Mac");
        assert_eq!(machine.hostname, "mac-pro.local");
        assert_eq!(machine.daemon_port, 7878);
        assert_eq!(machine.status, MachineStatus::Online);
        assert!(!machine.capabilities.is_empty());
        assert!(machine.machine_id.starts_with("mach-"));
    }

    #[test]
    fn test_register_self() {
        let mut gw = make_gateway();
        let machine = gw.register_self(7878, "/tmp/workspace", "tok");
        assert!(!machine.name.is_empty());
        assert!(!machine.hostname.is_empty());
    }

    #[test]
    fn test_heartbeat() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        assert!(gw.heartbeat(&mid, None).is_ok());
        assert_eq!(gw.get_machine(&mid).unwrap().status, MachineStatus::Online);
    }

    #[test]
    fn test_heartbeat_with_metrics() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let metrics = MachineMetrics {
            machine_id: mid.clone(),
            timestamp: now_epoch_secs(),
            cpu_usage_pct: 45.0,
            memory_used_gb: 8.0,
            memory_total_gb: 16.0,
            disk_used_gb: 100.0,
            disk_total_gb: 500.0,
            active_agent_sessions: 2,
            queued_tasks: 1,
            uptime_secs: 3600,
            provider_name: "ollama".to_string(),
            provider_healthy: true,
        };
        gw.heartbeat(&mid, Some(metrics)).unwrap();
        let m = gw.get_machine(&mid).unwrap();
        assert_eq!(m.status, MachineStatus::Busy);
        assert_eq!(m.memory_gb, 16.0);
        assert_eq!(m.active_sessions, 2);
    }

    #[test]
    fn test_heartbeat_idle() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let metrics = MachineMetrics {
            machine_id: mid.clone(),
            timestamp: now_epoch_secs(),
            cpu_usage_pct: 5.0,
            memory_used_gb: 4.0,
            memory_total_gb: 16.0,
            disk_used_gb: 50.0,
            disk_total_gb: 500.0,
            active_agent_sessions: 0,
            queued_tasks: 0,
            uptime_secs: 7200,
            provider_name: "ollama".to_string(),
            provider_healthy: true,
        };
        gw.heartbeat(&mid, Some(metrics)).unwrap();
        assert_eq!(gw.get_machine(&mid).unwrap().status, MachineStatus::Idle);
    }

    #[test]
    fn test_heartbeat_unknown_machine() {
        let mut gw = make_gateway();
        assert!(gw.heartbeat("nonexistent", None).is_err());
    }

    #[test]
    fn test_unregister_machine() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        assert!(gw.unregister_machine(&mid).is_ok());
        assert!(gw.get_machine(&mid).is_none());
    }

    #[test]
    fn test_unregister_nonexistent() {
        let mut gw = make_gateway();
        assert!(gw.unregister_machine("nope").is_err());
    }

    #[test]
    fn test_list_machines() {
        let mut gw = make_gateway();
        register_test_machine(&mut gw);
        register_test_machine(&mut gw);
        assert_eq!(gw.list_machines().len(), 2);
    }

    #[test]
    fn test_list_available_machines() {
        let mut gw = make_gateway();
        let mid1 = register_test_machine(&mut gw);
        let mid2 = register_test_machine(&mut gw);
        gw.machines.get_mut(&mid1).unwrap().status = MachineStatus::Offline;
        assert_eq!(gw.list_available_machines().len(), 1);
    }

    #[test]
    fn test_check_stale_machines() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        // Force last heartbeat to be very old.
        gw.machines.get_mut(&mid).unwrap().last_heartbeat = 1000;
        let stale = gw.check_stale_machines();
        assert_eq!(stale.len(), 1);
        assert_eq!(gw.get_machine(&mid).unwrap().status, MachineStatus::Unreachable);
    }

    #[test]
    fn test_tag_machine() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        gw.tag_machine(&mid, vec!["prod".to_string(), "gpu".to_string()]).unwrap();
        assert_eq!(gw.get_machine(&mid).unwrap().tags, vec!["prod", "gpu"]);
    }

    #[test]
    fn test_tag_nonexistent() {
        let mut gw = make_gateway();
        assert!(gw.tag_machine("nope", vec![]).is_err());
    }

    // ── Device Pairing ───────────────────────────────────────────────

    #[test]
    fn test_create_pairing_pin() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let pairing = gw.create_pairing(&mid, PairingMethod::Pin).unwrap();
        assert!(pairing.pin.is_some());
        assert_eq!(pairing.pin.as_ref().unwrap().len(), 6);
        assert_eq!(pairing.status, PairingStatus::Pending);
    }

    #[test]
    fn test_create_pairing_qr() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let pairing = gw.create_pairing(&mid, PairingMethod::QrCode).unwrap();
        assert!(pairing.qr_data.is_some());
        assert!(pairing.qr_data.as_ref().unwrap().starts_with("vibecody://pair?"));
    }

    #[test]
    fn test_create_pairing_nonexistent_machine() {
        let mut gw = make_gateway();
        assert!(gw.create_pairing("nope", PairingMethod::Pin).is_err());
    }

    #[test]
    fn test_accept_pairing() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        assert!(gw.devices.contains_key(&device_id));
        let device = gw.devices.get(&device_id).unwrap();
        assert!(device.paired_machines.contains(&mid));
        assert_eq!(device.push_token.as_deref(), Some("apns-token-xyz"));
    }

    #[test]
    fn test_verify_pin() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let pairing = gw.create_pairing(&mid, PairingMethod::Pin).unwrap();
        let pid = pairing.id.clone();
        let pin = pairing.pin.clone().unwrap();
        assert!(gw.verify_pin(&pid, &pin).unwrap());
        assert!(!gw.verify_pin(&pid, "000000").unwrap());
    }

    #[test]
    fn test_reject_pairing() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let pairing = gw.create_pairing(&mid, PairingMethod::Pin).unwrap();
        let pid = pairing.id.clone();
        gw.reject_pairing(&pid).unwrap();
        assert_eq!(gw.pairing_requests.last().unwrap().status, PairingStatus::Rejected);
    }

    #[test]
    fn test_unpair_device() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        gw.unpair_device(&device_id, &mid).unwrap();
        // Device removed since no more paired machines.
        assert!(!gw.devices.contains_key(&device_id));
    }

    #[test]
    fn test_list_devices_for_machine() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        pair_test_device(&mut gw, &mid);
        assert_eq!(gw.list_devices_for_machine(&mid).len(), 1);
    }

    #[test]
    fn test_list_machines_for_device() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let machines = gw.list_machines_for_device(&device_id);
        assert_eq!(machines.len(), 1);
        assert_eq!(machines[0].machine_id, mid);
    }

    #[test]
    fn test_cleanup_expired_pairings() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        gw.create_pairing(&mid, PairingMethod::Pin).unwrap();
        // Force expiry.
        gw.pairing_requests.last_mut().unwrap().expires_at = 1000;
        let cleaned = gw.cleanup_expired_pairings();
        assert_eq!(cleaned, 1);
    }

    // ── Task Dispatch ────────────────────────────────────────────────

    #[test]
    fn test_dispatch_chat() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let task = gw.dispatch_task(&device_id, &mid, DispatchType::Chat, "Hello from mobile!").unwrap();
        assert!(task.task_id.starts_with("dsp-"));
        assert_eq!(task.status, DispatchStatus::Queued);
        assert_eq!(task.payload, "Hello from mobile!");
    }

    #[test]
    fn test_dispatch_agent_task() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let task = gw.dispatch_task(&device_id, &mid, DispatchType::AgentTask, "Fix the auth bug in login.rs").unwrap();
        assert_eq!(task.dispatch_type, DispatchType::AgentTask);
    }

    #[test]
    fn test_dispatch_unpaired() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        assert!(gw.dispatch_task("unknown-device", &mid, DispatchType::Chat, "hi").is_err());
    }

    #[test]
    fn test_dispatch_offline_machine() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        gw.machines.get_mut(&mid).unwrap().status = MachineStatus::Offline;
        assert!(gw.dispatch_task(&device_id, &mid, DispatchType::Chat, "hi").is_err());
    }

    #[test]
    fn test_update_dispatch_running() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let tid = gw.dispatch_task(&device_id, &mid, DispatchType::AgentTask, "task").unwrap().task_id.clone();
        gw.update_dispatch(&tid, DispatchStatus::Running, None, None, Some("sess-1".to_string())).unwrap();
        let task = gw.get_dispatch(&tid).unwrap();
        assert_eq!(task.status, DispatchStatus::Running);
        assert!(task.started_at.is_some());
        assert_eq!(task.session_id.as_deref(), Some("sess-1"));
    }

    #[test]
    fn test_update_dispatch_completed() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let tid = gw.dispatch_task(&device_id, &mid, DispatchType::Chat, "hi").unwrap().task_id.clone();
        gw.update_dispatch(&tid, DispatchStatus::Completed, Some("Done!".to_string()), None, None).unwrap();
        let task = gw.get_dispatch(&tid).unwrap();
        assert_eq!(task.status, DispatchStatus::Completed);
        assert_eq!(task.result.as_deref(), Some("Done!"));
        assert!(task.completed_at.is_some());
        // Should have queued a notification.
        assert_eq!(gw.notifications.len(), 1);
        assert_eq!(gw.notifications[0].category, NotificationCategory::TaskComplete);
    }

    #[test]
    fn test_update_dispatch_failed() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let tid = gw.dispatch_task(&device_id, &mid, DispatchType::Command, "ls").unwrap().task_id.clone();
        gw.update_dispatch(&tid, DispatchStatus::Failed, None, Some("permission denied".to_string()), None).unwrap();
        let task = gw.get_dispatch(&tid).unwrap();
        assert_eq!(task.status, DispatchStatus::Failed);
        assert_eq!(gw.notifications[0].category, NotificationCategory::TaskFailed);
    }

    #[test]
    fn test_cancel_dispatch() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let tid = gw.dispatch_task(&device_id, &mid, DispatchType::Chat, "hi").unwrap().task_id.clone();
        gw.cancel_dispatch(&tid).unwrap();
        assert_eq!(gw.get_dispatch(&tid).unwrap().status, DispatchStatus::Cancelled);
    }

    #[test]
    fn test_cancel_completed_dispatch() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let tid = gw.dispatch_task(&device_id, &mid, DispatchType::Chat, "hi").unwrap().task_id.clone();
        gw.update_dispatch(&tid, DispatchStatus::Completed, Some("done".to_string()), None, None).unwrap();
        assert!(gw.cancel_dispatch(&tid).is_err());
    }

    #[test]
    fn test_pending_dispatches() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        gw.dispatch_task(&device_id, &mid, DispatchType::Chat, "one").unwrap();
        gw.dispatch_task(&device_id, &mid, DispatchType::Chat, "two").unwrap();
        assert_eq!(gw.pending_dispatches(&mid).len(), 2);
    }

    #[test]
    fn test_check_timeouts() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let tid = gw.dispatch_task(&device_id, &mid, DispatchType::AgentTask, "long task").unwrap().task_id.clone();
        // Force running with old start time.
        let task = gw.dispatched_tasks.iter_mut().find(|t| t.task_id == tid).unwrap();
        task.status = DispatchStatus::Running;
        task.started_at = Some(1000);
        task.timeout_secs = 60;
        let timed_out = gw.check_timeouts();
        assert_eq!(timed_out.len(), 1);
        assert_eq!(gw.get_dispatch(&tid).unwrap().status, DispatchStatus::TimedOut);
    }

    #[test]
    fn test_list_dispatches_for_device() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        gw.dispatch_task(&device_id, &mid, DispatchType::Chat, "a").unwrap();
        gw.dispatch_task(&device_id, &mid, DispatchType::GitOp, "status").unwrap();
        assert_eq!(gw.list_dispatches_for_device(&device_id).len(), 2);
    }

    #[test]
    fn test_list_dispatches_for_machine() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        gw.dispatch_task(&device_id, &mid, DispatchType::FileOp, "ls /tmp").unwrap();
        assert_eq!(gw.list_dispatches_for_machine(&mid).len(), 1);
    }

    // ── Push Notifications ───────────────────────────────────────────

    #[test]
    fn test_update_push_token() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        gw.update_push_token(&device_id, "new-token").unwrap();
        assert_eq!(gw.devices.get(&device_id).unwrap().push_token.as_deref(), Some("new-token"));
    }

    #[test]
    fn test_queue_notification() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        gw.queue_notification(&device_id, "Test", "Hello", NotificationCategory::SessionEvent).unwrap();
        assert_eq!(gw.unsent_notifications(&device_id).len(), 1);
    }

    #[test]
    fn test_mark_notification_sent() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        gw.queue_notification(&device_id, "Test", "Hello", NotificationCategory::SessionEvent).unwrap();
        let nid = gw.notifications[0].id.clone();
        gw.mark_notification_sent(&nid).unwrap();
        assert!(gw.notifications[0].sent);
        assert_eq!(gw.unsent_notifications(&device_id).len(), 0);
    }

    // ── Statistics ───────────────────────────────────────────────────

    #[test]
    fn test_stats() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        gw.dispatch_task(&device_id, &mid, DispatchType::Chat, "hi").unwrap();
        let stats = gw.stats();
        assert_eq!(stats.total_machines, 1);
        assert_eq!(stats.online_machines, 1);
        assert_eq!(stats.total_devices, 1);
        assert_eq!(stats.total_dispatches, 1);
        assert_eq!(stats.active_dispatches, 1);
    }

    #[test]
    fn test_machine_summaries() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        pair_test_device(&mut gw, &mid);
        let summaries = gw.machine_summaries();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].paired_devices, 1);
        assert_eq!(summaries[0].status, "online");
    }

    // ── Config ───────────────────────────────────────────────────────

    #[test]
    fn test_default_config() {
        let config = GatewayConfig::default_config();
        assert!(config.bridge_url.starts_with("wss://"));
        assert_eq!(config.heartbeat_interval_secs, 30);
        assert_eq!(config.max_machines_per_device, 10);
        assert!(config.require_pin_confirmation);
    }

    #[test]
    fn test_local_dev_config() {
        let config = GatewayConfig::local_dev();
        assert!(config.bridge_url.starts_with("ws://"));
        assert!(!config.require_pin_confirmation);
        assert!(!config.enable_push_notifications);
    }

    // ── Enums Display ────────────────────────────────────────────────

    #[test]
    fn test_machine_os_display() {
        assert_eq!(MachineOS::MacOS.to_string(), "macOS");
        assert_eq!(MachineOS::Linux.to_string(), "Linux");
        assert_eq!(MachineOS::Docker.to_string(), "Docker");
        assert_eq!(MachineOS::WSL.to_string(), "WSL");
    }

    #[test]
    fn test_machine_status_display() {
        assert_eq!(MachineStatus::Online.to_string(), "online");
        assert_eq!(MachineStatus::Busy.to_string(), "busy");
        assert_eq!(MachineStatus::Unreachable.to_string(), "unreachable");
    }

    #[test]
    fn test_dispatch_type_display() {
        assert_eq!(DispatchType::Chat.to_string(), "chat");
        assert_eq!(DispatchType::AgentTask.to_string(), "agent_task");
        assert_eq!(DispatchType::ReplCommand.to_string(), "repl_command");
    }

    #[test]
    fn test_dispatch_status_display() {
        assert_eq!(DispatchStatus::Queued.to_string(), "queued");
        assert_eq!(DispatchStatus::Running.to_string(), "running");
        assert_eq!(DispatchStatus::TimedOut.to_string(), "timed_out");
    }

    #[test]
    fn test_push_platform_display() {
        assert_eq!(PushPlatform::APNs.to_string(), "apns");
        assert_eq!(PushPlatform::FCM.to_string(), "fcm");
    }

    #[test]
    fn test_pairing_method_display() {
        assert_eq!(PairingMethod::QrCode.to_string(), "qr_code");
        assert_eq!(PairingMethod::Tailscale.to_string(), "tailscale");
    }

    #[test]
    fn test_notification_category_display() {
        assert_eq!(NotificationCategory::TaskComplete.to_string(), "task_complete");
        assert_eq!(NotificationCategory::ApprovalRequired.to_string(), "approval_required");
    }

    // ── Edge Cases ───────────────────────────────────────────────────

    #[test]
    fn test_multiple_devices_one_machine() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);

        // Pair first device.
        let p1 = gw.create_pairing(&mid, PairingMethod::Pin).unwrap().id.clone();
        gw.accept_pairing(&p1, "dev-1", "iPhone", PushPlatform::APNs, None, "1.0", "18.0").unwrap();

        // Pair second device.
        let p2 = gw.create_pairing(&mid, PairingMethod::Pin).unwrap().id.clone();
        gw.accept_pairing(&p2, "dev-2", "Pixel", PushPlatform::FCM, None, "1.0", "14.0").unwrap();

        assert_eq!(gw.list_devices_for_machine(&mid).len(), 2);
    }

    #[test]
    fn test_one_device_multiple_machines() {
        let mut gw = make_gateway();
        let mid1 = register_test_machine(&mut gw);
        let mid2 = register_test_machine(&mut gw);

        // Pair same device with both machines.
        let p1 = gw.create_pairing(&mid1, PairingMethod::Pin).unwrap().id.clone();
        gw.accept_pairing(&p1, "dev-1", "iPhone", PushPlatform::APNs, None, "1.0", "18.0").unwrap();

        let p2 = gw.create_pairing(&mid2, PairingMethod::Pin).unwrap().id.clone();
        gw.accept_pairing(&p2, "dev-1", "iPhone", PushPlatform::APNs, None, "1.0", "18.0").unwrap();

        let device = gw.devices.get("dev-1").unwrap();
        assert_eq!(device.paired_machines.len(), 2);
        assert_eq!(gw.list_machines_for_device("dev-1").len(), 2);
    }

    #[test]
    fn test_dispatch_repl_command() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let task = gw.dispatch_task(&device_id, &mid, DispatchType::ReplCommand, "/status").unwrap();
        assert_eq!(task.dispatch_type, DispatchType::ReplCommand);
        assert_eq!(task.payload, "/status");
    }

    #[test]
    fn test_dispatch_git_op() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let task = gw.dispatch_task(&device_id, &mid, DispatchType::GitOp, "status").unwrap();
        assert_eq!(task.dispatch_type, DispatchType::GitOp);
    }

    #[test]
    fn test_dispatch_file_op() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let task = gw.dispatch_task(&device_id, &mid, DispatchType::FileOp, "list:/src").unwrap();
        assert_eq!(task.dispatch_type, DispatchType::FileOp);
    }

    #[test]
    fn test_dispatch_cancel_type() {
        let mut gw = make_gateway();
        let mid = register_test_machine(&mut gw);
        let (device_id, _) = pair_test_device(&mut gw, &mid);
        let task = gw.dispatch_task(&device_id, &mid, DispatchType::Cancel, "sess-123").unwrap();
        assert_eq!(task.dispatch_type, DispatchType::Cancel);
    }

    #[test]
    fn test_helpers() {
        let id = generate_id("test", 42);
        assert!(id.starts_with("test-"));

        let pin = generate_pin();
        assert_eq!(pin.len(), 6);
        assert!(pin.chars().all(|c| c.is_ascii_digit()));

        let hash = simple_hash("hello");
        assert_eq!(hash.len(), 16);

        let os = detect_os();
        assert_ne!(format!("{}", os), "");

        let arch = detect_arch();
        assert!(!arch.is_empty());
    }

    #[test]
    fn test_gateway_new() {
        let gw = MobileGateway::new();
        assert!(gw.machines.is_empty());
        assert!(gw.devices.is_empty());
        assert!(gw.config.bridge_url.starts_with("wss://"));
    }

    #[test]
    fn test_remote_file_entry() {
        let entry = RemoteFileEntry {
            path: "/src/main.rs".to_string(),
            is_dir: false,
            size_bytes: 1024,
            modified_at: now_epoch_secs(),
        };
        assert!(!entry.is_dir);
    }

    #[test]
    fn test_remote_git_status() {
        let status = RemoteGitStatus {
            branch: "main".to_string(),
            ahead: 2,
            behind: 0,
            staged: vec!["src/lib.rs".to_string()],
            modified: vec!["Cargo.toml".to_string()],
            untracked: vec![],
            has_conflicts: false,
        };
        assert_eq!(status.ahead, 2);
        assert!(!status.has_conflicts);
    }

    #[test]
    fn test_session_summary() {
        let summary = SessionSummary {
            session_id: "sess-1".to_string(),
            machine_id: "mach-1".to_string(),
            task: "Fix bug".to_string(),
            status: "running".to_string(),
            provider: "claude".to_string(),
            started_at: now_epoch_secs(),
            last_event_at: now_epoch_secs(),
            steps_completed: 3,
            has_pending_approval: true,
        };
        assert!(summary.has_pending_approval);
        assert_eq!(summary.steps_completed, 3);
    }
}
