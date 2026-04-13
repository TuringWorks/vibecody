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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_allowed_by_rule() {
        let cfg = WindowsSandboxConfig::default_restricted().allow_path("/allowed");
        let sb = WindowsSandbox::new(cfg);
        let v = sb.check_path("/allowed/file.txt");
        assert!(v.allowed);
    }

    #[test]
    fn test_path_denied_no_rule() {
        let cfg = WindowsSandboxConfig::default_restricted();
        let sb = WindowsSandbox::new(cfg);
        let v = sb.check_path("/some/path");
        assert!(!v.allowed);
        assert!(v.reason.contains("no matching allow rule"));
    }

    #[test]
    fn test_deny_takes_precedence_over_allow() {
        let cfg = WindowsSandboxConfig::default_restricted()
            .allow_path("/data")
            .deny_path("/data/secret");
        let sb = WindowsSandbox::new(cfg);
        let v = sb.check_path("/data/secret/file.txt");
        assert!(!v.allowed);
        assert!(v.reason.contains("denied"));
    }

    #[test]
    fn test_network_blocked_by_default() {
        let cfg = WindowsSandboxConfig::default_restricted();
        let sb = WindowsSandbox::new(cfg);
        let v = sb.check_network("example.com");
        assert!(!v.allowed);
    }

    #[test]
    fn test_network_allowed_when_internet_open() {
        let cfg = WindowsSandboxConfig {
            network: NetworkPolicy { allow_internet: true, allowed_hosts: vec![] },
            ..Default::default()
        };
        let sb = WindowsSandbox::new(cfg);
        assert!(sb.check_network("anything.com").allowed);
    }

    #[test]
    fn test_explicit_host_allowed() {
        let cfg = WindowsSandboxConfig {
            network: NetworkPolicy {
                allow_internet: false,
                allowed_hosts: vec!["api.example.com".to_string()],
            },
            ..Default::default()
        };
        let sb = WindowsSandbox::new(cfg);
        assert!(sb.check_network("api.example.com").allowed);
        assert!(!sb.check_network("other.example.com").allowed);
    }

    #[test]
    fn test_allow_path_builder_chain() {
        let cfg = WindowsSandboxConfig::default_restricted()
            .allow_path("/a")
            .allow_path("/b");
        assert_eq!(cfg.allowed_paths.len(), 2);
    }
}
