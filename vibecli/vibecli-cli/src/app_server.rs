//! app_server — Lightweight embedded HTTP server for serving generated web apps
//! on a local port. Manages server lifecycle (start/stop/status) and tracks
//! the current serving root directory and bound port.

use std::sync::{Arc, Mutex};

/// Current state of the embedded server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerState {
    Stopped,
    Running { port: u16, root: String },
}

impl Default for ServerState {
    fn default() -> Self { ServerState::Stopped }
}

/// Configuration for starting the app server.
#[derive(Debug, Clone)]
pub struct AppServerConfig {
    pub port: u16,
    pub root: String,
    pub open_browser: bool,
}

impl AppServerConfig {
    pub fn new(port: u16, root: impl Into<String>) -> Self {
        Self { port, root: root.into(), open_browser: false }
    }
}

impl Default for AppServerConfig {
    fn default() -> Self {
        Self { port: 8080, root: "./dist".to_string(), open_browser: false }
    }
}

/// Lifecycle error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppServerError {
    AlreadyRunning,
    NotRunning,
    PortInUse(u16),
}

impl std::fmt::Display for AppServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "server is already running"),
            Self::NotRunning => write!(f, "server is not running"),
            Self::PortInUse(p) => write!(f, "port {} is already in use", p),
        }
    }
}

/// Manages the embedded HTTP server lifecycle.
#[derive(Debug, Default)]
pub struct AppServer {
    state: Arc<Mutex<ServerState>>,
}

impl AppServer {
    pub fn new() -> Self { Self::default() }

    /// Start serving — returns an error if already running.
    pub fn start(&self, config: AppServerConfig) -> Result<(), AppServerError> {
        let mut st = self.state.lock().unwrap();
        if matches!(*st, ServerState::Running { .. }) {
            return Err(AppServerError::AlreadyRunning);
        }
        *st = ServerState::Running { port: config.port, root: config.root };
        Ok(())
    }

    /// Stop the server — returns an error if not running.
    pub fn stop(&self) -> Result<(), AppServerError> {
        let mut st = self.state.lock().unwrap();
        if matches!(*st, ServerState::Stopped) {
            return Err(AppServerError::NotRunning);
        }
        *st = ServerState::Stopped;
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        matches!(*self.state.lock().unwrap(), ServerState::Running { .. })
    }

    pub fn port(&self) -> Option<u16> {
        match *self.state.lock().unwrap() {
            ServerState::Running { port, .. } => Some(port),
            ServerState::Stopped => None,
        }
    }

    pub fn root(&self) -> Option<String> {
        match &*self.state.lock().unwrap() {
            ServerState::Running { root, .. } => Some(root.clone()),
            ServerState::Stopped => None,
        }
    }
}
