//! sandbox_windows — ACL-based path/network policy sandbox (Windows-style).

#[derive(Debug, Clone, Default)]
pub struct NetworkPolicy {
    pub allow_internet: bool,
    pub allowed_hosts: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WindowsSandboxConfig {
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
    pub network: NetworkPolicy,
}

impl WindowsSandboxConfig {
    pub fn default_restricted() -> Self {
        Self { allowed_paths: vec![], denied_paths: vec![], network: NetworkPolicy { allow_internet: false, allowed_hosts: vec![] } }
    }

    pub fn allow_path(mut self, path: &str) -> Self {
        self.allowed_paths.push(path.to_string()); self
    }

    pub fn deny_path(mut self, path: &str) -> Self {
        self.denied_paths.push(path.to_string()); self
    }
}

#[derive(Debug, Clone)]
pub struct SandboxVerdict {
    pub allowed: bool,
    pub reason: String,
}

#[derive(Debug)]
pub struct WindowsSandbox { cfg: WindowsSandboxConfig }

impl WindowsSandbox {
    pub fn new(cfg: WindowsSandboxConfig) -> Self { Self { cfg } }

    pub fn check_path(&self, path: &str) -> SandboxVerdict {
        // Deny takes precedence
        for denied in &self.cfg.denied_paths {
            if path.starts_with(denied.as_str()) {
                return SandboxVerdict { allowed: false, reason: format!("path denied by rule: {}", denied) };
            }
        }
        for allowed in &self.cfg.allowed_paths {
            if path.starts_with(allowed.as_str()) {
                return SandboxVerdict { allowed: true, reason: format!("path allowed by rule: {}", allowed) };
            }
        }
        SandboxVerdict { allowed: false, reason: "no matching allow rule".to_string() }
    }

    pub fn check_network(&self, host: &str) -> SandboxVerdict {
        if self.cfg.network.allow_internet {
            return SandboxVerdict { allowed: true, reason: "internet allowed".to_string() };
        }
        if self.cfg.network.allowed_hosts.iter().any(|h| h == host) {
            return SandboxVerdict { allowed: true, reason: format!("host {} explicitly allowed", host) };
        }
        SandboxVerdict { allowed: false, reason: "internet disabled and host not in allowlist".to_string() }
    }
}
